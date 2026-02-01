use crate::core::models::repository::Repository;
use crate::core::models::objects::{SwitchResult, HeadState, SwitchPlan};
use crate::core::resolve::{resolve_target, ResolvedTarget};
use crate::core::refs::{self as core_refs};
use crate::db;
use crate::error::ReviusError;
use crate::fs;
use crate::utils;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

pub fn switch_to_target(
    repo: &Repository,
    target: &str,
    create: bool,
    force: bool,
) -> Result<SwitchResult, ReviusError> {
    // Handle create flag: create new branch from current state and switch to it
    if create {
        return handle_create_and_switch(repo, target);
    }
    
    // Phase 1: Preparation and Validation
    let (previous_head, current_commit_opt) = get_current_head_state(&repo.conn)?;
    let resolved_target = resolve_target(&repo.conn, target)?;
    let target_commit = resolved_target.hash();
    
    // Check if already on target
    // If we are on a branch, and the target is that branch name
    if let HeadState::Branch(ref current_name, _) = previous_head {
        if let ResolvedTarget::Branch(ref target_name, _) = resolved_target {
            if current_name == target_name {
                 return Err(ReviusError::Usage(
                    format!("Already on {}", format_target_name(&resolved_target))
                ));
            }
        }
    }
    // If we are detached, and switching to the same commit hash
    if let HeadState::Detached(current_hash) = previous_head {
        if current_hash == target_commit {
             // Note: Git allows "checking out" the same commit again to refresh files, 
             // but for now we block it to avoid confusion unless we want to support --force for that.
             // If target was a branch name, we proceed (attaching HEAD). 
             // If target was a hash/tag, we are already there.
             if let ResolvedTarget::Commit(_) = resolved_target {
                 return Err(ReviusError::Usage(
                    format!("Already on {}", format_target_name(&resolved_target))
                ));
             }
        }
    }
    
    // Check for uncommitted changes if not force
    if !force {
        let has_changes = check_uncommitted_changes(repo)?;
        if has_changes {
            return Err(ReviusError::UncommittedChanges);
        }
    }
    
    // Phase 2: Plan Workspace Changes
    let current_tree = if let Some(commit) = current_commit_opt {
        Some(db::commits::get_commit_tree(&repo.conn, &commit)?)
    } else {
        None
    };
    
    let target_tree = db::commits::get_commit_tree(&repo.conn, &target_commit)?;
    let plan = build_switch_plan(&repo.conn, current_tree, target_tree)?;
    
    // Phase 3: Database Transaction
    let tx = repo.conn.unchecked_transaction()
        .map_err(|e| ReviusError::Db(format!("Failed to begin transaction: {}", e)))?;
    
    // Update HEAD
    match &resolved_target {
        ResolvedTarget::Branch(name, _) => {
            core_refs::update_head_to_branch(&tx, name)?;
        }
        ResolvedTarget::Commit(hash) => {
            core_refs::update_head(&tx, hash)?;
        }
    };
    
    // Insert reflog entry
    // We use the raw target string from the resolved target for the log
    let target_display = match &resolved_target {
        ResolvedTarget::Branch(name, _) => name.clone(),
        ResolvedTarget::Commit(hash) => utils::hash::hash_to_hex(hash),
    };

    let action = format!(r#"["switch", "{}"]"#, target_display);
    
    db::reflog::insert_reflog(
        &tx,
        "HEAD",
        current_commit_opt.as_ref(),
        Some(&target_commit),
        &action,
    )?;
    
    // Clear and rebuild staging
    db::staging::clear_staging(&tx)?;
    update_staging_from_tree(&tx, target_tree)?;
    
    tx.commit()
        .map_err(|e| ReviusError::Db(format!("Failed to commit transaction: {}", e)))?;
    
    // Phase 4: Apply Workspace Changes
    // Note: If this fails, DB is already committed. This is acceptable because:
    // - DB integrity is maintained
    // - User can manually fix working directory or re-run switch with --force
    let (files_changed, files_deleted) = apply_workspace_changes(repo, &plan)?;
    
    // Phase 5: Return Result
    let new_head = match resolved_target {
        ResolvedTarget::Branch(name, hash) => HeadState::Branch(name, hash),
        ResolvedTarget::Commit(hash) => HeadState::Detached(hash),
    };
    
    Ok(SwitchResult {
        previous_head,
        new_head,
        files_changed,
        files_deleted,
    })
}

pub fn handle_create_and_switch(repo: &Repository, branch_name: &str) -> Result<SwitchResult, ReviusError> {
    // Validate branch name
    crate::utils::validation::validate_branch_name(branch_name)?;
    
    // Get current commit (must exist)
    let current_commit = db::refs::resolve_head(&repo.conn)?
        .ok_or(ReviusError::NoCommitsYet)?;
    
    // Get current HEAD state
    let (previous_head, _) = get_current_head_state(&repo.conn)?;
    
    // Create branch at current commit and switch HEAD to it
    let tx = repo.conn.unchecked_transaction()
        .map_err(|e| ReviusError::Db(format!("Failed to begin transaction: {}", e)))?;
    
    // Create the branch
    crate::core::branch::create_branch_in_tx(&tx, branch_name, &current_commit)?;
    
    // Switch HEAD to new branch
    core_refs::update_head_to_branch(&tx, branch_name)?;
    
    // Log to reflog
    let action = format!(r#"["switch", "-c", "{}"]"#, branch_name);
    db::reflog::insert_reflog(
        &tx,
        "HEAD",
        Some(&current_commit),
        Some(&current_commit),
        &action,
    )?;
    
    tx.commit()
        .map_err(|e| ReviusError::Db(format!("Failed to commit transaction: {}", e)))?;
    
    // No workspace changes needed
    Ok(SwitchResult {
        previous_head,
        new_head: HeadState::Branch(branch_name.to_string(), current_commit),
        files_changed: 0,
        files_deleted: 0,
    })
}

/// Helper to get the full HeadState (including hash) which is needed for SwitchResult
pub fn get_current_head_state(conn: &Connection) -> Result<(HeadState, Option<[u8; 32]>), ReviusError> {
    // We use core::refs to parse the meta value, but we need to enrich it with the hash
    let simple_state = core_refs::get_head_state(conn)?;
    
    match simple_state {
        core_refs::HeadState::Branch(name) => {
            let ref_path = format!("refs/heads/{}", name);
            let hash = db::refs::get_ref(conn, &ref_path)?;
            
            if let Some(h) = hash {
                Ok((HeadState::Branch(name, h), Some(h)))
            } else {
                // Branch exists but no commits yet (initial state)
                Ok((HeadState::Branch(name, [0; 32]), None))
            }
        }
        core_refs::HeadState::Detached(hash) => {
            Ok((HeadState::Detached(hash), Some(hash)))
        }
    }
}

pub fn check_uncommitted_changes(repo: &Repository) -> Result<bool, ReviusError> {
    // Get all files in working directory
    let ignore_path = fs::paths::get_repo_ignore_path(&repo.root);
    let workdir_files = fs::walk::get_all_repo_files(&repo.root, &ignore_path)?;
    
    // Get all files in staging
    let staged_files = db::staging::get_all_staged(&repo.conn)?;
    
    // Create maps for easier comparison
    let mut staged_map: HashMap<String, ([u8; 32], u32)> = HashMap::new();
    for staged_file in staged_files {
        staged_map.insert(staged_file.path, (staged_file.file_hash, staged_file.mode));
    }
    
    let mut workdir_map: HashSet<String> = HashSet::new();
    for abs_path in workdir_files {
        let rel_path = fs::paths::make_repo_relative(&abs_path, &repo.root)?;
        workdir_map.insert(rel_path);
    }
    
    // Check for modifications or additions in workdir
    for rel_path in &workdir_map {
        let abs_path = fs::paths::to_absolute(rel_path, &repo.root);
        let (_file_data, file_hash) = crate::core::content::read_and_hash_file(&abs_path)?;
        
        if let Some((staged_hash, _staged_mode)) = staged_map.get(rel_path) {
            // File exists in staging - check if modified
            if &file_hash != staged_hash {
                return Ok(true); // Modified
            }
        } else {
            // File not in staging - new file
            return Ok(true);
        }
    }
    
    // Check for deletions (in staging but not in workdir)
    for staged_path in staged_map.keys() {
        if !workdir_map.contains(staged_path) {
            return Ok(true); // Deleted
        }
    }
    
    Ok(false)
}

pub fn build_switch_plan(
    conn: &Connection,
    current_tree: Option<[u8; 32]>,
    target_tree: [u8; 32],
) -> Result<SwitchPlan, ReviusError> {
    // Get files in current tree (if exists)
    let current_files: HashMap<String, ([u8; 32], u32)> = if let Some(tree_hash) = current_tree {
        let files = db::trees::get_all_files_in_tree(conn, &tree_hash)?;
        files.into_iter()
            .map(|(path, hash, mode, _size)| (path, (hash, mode)))
            .collect()
    } else {
        HashMap::new()
    };
    
    // Get files in target tree
    let target_files_vec = db::trees::get_all_files_in_tree(conn, &target_tree)?;
    let target_files: HashMap<String, ([u8; 32], u32)> = target_files_vec.iter()
        .map(|(path, hash, mode, _size)| (path.clone(), (*hash, *mode)))
        .collect();
    
    let mut to_add = Vec::new();
    let mut to_modify = Vec::new();
    let mut to_delete = Vec::new();
    
    // Files in target
    for (path, hash, mode, _size) in target_files_vec {
        if let Some((current_hash, _current_mode)) = current_files.get(&path) {
            if current_hash != &hash {
                to_modify.push((path, hash, mode));
            }
            // If hashes match, no change needed
        } else {
            to_add.push((path, hash, mode));
        }
    }
    
    // Files in current but not in target
    for (path, _) in current_files {
        if !target_files.contains_key(&path) {
            to_delete.push(path);
        }
    }
    
    Ok(SwitchPlan {
        to_add,
        to_modify,
        to_delete,
    })
}

pub fn apply_workspace_changes(
    repo: &Repository,
    plan: &SwitchPlan,
) -> Result<(usize, usize), ReviusError> {
    // Delete files first
    for path in &plan.to_delete {
        let abs_path = fs::paths::to_absolute(path, &repo.root);
        if fs::paths::path_exists(&abs_path) {
            fs::io::delete_file(&abs_path)
                .map_err(|e| ReviusError::Io(abs_path.clone(), e))?;
        }
    }
    
    // Add and modify files
    for (path, file_hash, mode) in plan.to_add.iter().chain(plan.to_modify.iter()) {
        let abs_path = fs::paths::to_absolute(path, &repo.root);
        crate::core::checkout::checkout_file(&repo.conn, file_hash, &abs_path, *mode)?;
    }
    
    let files_changed = plan.to_add.len() + plan.to_modify.len();
    let files_deleted = plan.to_delete.len();
    
    Ok((files_changed, files_deleted))
}

pub fn update_staging_from_tree(
    tx: &rusqlite::Transaction,
    tree_hash: [u8; 32],
) -> Result<(), ReviusError> {
    let files = db::trees::get_all_files_in_tree(tx, &tree_hash)?;
    
    // Use current timestamp for all staged files since they'll be written to disk after this transaction
    let current_time = crate::utils::time::unix_timestamp()
        .unwrap_or(0);
    
    for (path, file_hash, mode, size) in files {
        db::staging::upsert_staging(tx, &path, &file_hash, mode, size, current_time)?;
    }
    
    Ok(())
}

pub fn format_target_name(target: &ResolvedTarget) -> String {
    match target {
        ResolvedTarget::Branch(name, _) => format!("branch '{}'", name),
        ResolvedTarget::Commit(hash) => format!("commit '{}'", utils::hash::hash_to_short_hex(hash)),
    }
}