use crate::core::models::repository::Repository;
use crate::core::models::objects::{SwitchResult, HeadState, TargetType, SwitchPlan};
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
    let (previous_head, current_commit) = get_current_head_state(&repo.conn)?;
    let (target_type, target_commit) = resolve_target(&repo.conn, target)?;
    
    // Check if already on target
    if let Some(current) = current_commit {
        if current == target_commit {
            return Err(ReviusError::Usage(
                format!("Already on {}", format_target_name(&target_type, target))
            ));
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
    let current_tree = if let Some(commit) = current_commit {
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
    let new_head_value = match &target_type {
        TargetType::Branch(name) => format!("ref: refs/heads/{}", name),
        TargetType::Commit => utils::hash::hash_to_hex(&target_commit),
    };
    db::meta::set_meta(&tx, "HEAD", &new_head_value)?;
    
    // Insert reflog entry
    let action = match &target_type {
        TargetType::Branch(name) => format!(r#"["switch", "{}"]"#, name),
        TargetType::Commit => format!(r#"["switch", "{}"]"#, utils::hash::hash_to_hex(&target_commit)),
    };
    db::reflog::insert_reflog(
        &tx,
        "HEAD",
        current_commit.as_ref(),
        Some(&target_commit),
        &action,
    )?;
    
    // Clear and rebuild staging
    db::staging::clear_staging(&tx)?;
    update_staging_from_tree(&tx, repo, target_tree)?;
    
    tx.commit()
        .map_err(|e| ReviusError::Db(format!("Failed to commit transaction: {}", e)))?;
    
    // Phase 4: Apply Workspace Changes
    let (files_changed, files_deleted) = apply_workspace_changes(repo, &plan)?;
    
    // Phase 5: Return Result
    let new_head = match target_type {
        TargetType::Branch(name) => HeadState::Branch(name, target_commit),
        TargetType::Commit => HeadState::Detached(target_commit),
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
    let new_head_value = format!("ref: refs/heads/{}", branch_name);
    db::meta::set_meta(&tx, "HEAD", &new_head_value)?;
    
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

pub fn get_current_head_state(conn: &Connection) -> Result<(HeadState, Option<[u8; 32]>), ReviusError> {
    let head_value = db::meta::get_meta(conn, "HEAD")?
        .ok_or_else(|| ReviusError::Db("HEAD not found in Meta table".to_string()))?;
    
    if head_value.starts_with("ref: refs/heads/") {
        let branch_name = head_value.strip_prefix("ref: refs/heads/")
            .ok_or_else(|| ReviusError::Db("Failed to parse HEAD branch reference".to_string()))?
            .to_string();
        let commit_hash = db::refs::get_ref(conn, &format!("refs/heads/{}", branch_name))?;
        
        if let Some(hash) = commit_hash {
            Ok((HeadState::Branch(branch_name, hash), Some(hash)))
        } else {
            // Branch exists but no commits yet
            Ok((HeadState::Branch(branch_name, [0; 32]), None))
        }
    } else {
        // Detached HEAD
        let hash_bytes = hex::decode(&head_value)
            .map_err(|e| ReviusError::Db(format!("Invalid HEAD value '{}': {}", head_value, e)))?;
        let hash = utils::hash::vec_to_hash(&hash_bytes)
            .map_err(|e| ReviusError::Db(format!("Invalid HEAD hash: {}", e)))?;
        Ok((HeadState::Detached(hash), Some(hash)))
    }
}

pub fn resolve_target(conn: &Connection, target: &str) -> Result<(TargetType, [u8; 32]), ReviusError> {
    // Try as branch name first
    let branch_ref = format!("refs/heads/{}", target);
    if let Some(commit_hash) = db::refs::get_ref(conn, &branch_ref)? {
        return Ok((TargetType::Branch(target.to_string()), commit_hash));
    }
    
    // Try as commit hash
    if let Ok(hash_bytes) = hex::decode(target) {
        if hash_bytes.len() == 32 {
            if let Ok(hash) = utils::hash::vec_to_hash(&hash_bytes) {
                // Verify commit exists
                if db::commits::commit_exists(conn, &hash)? {
                    return Ok((TargetType::Commit, hash));
                } else {
                    // Valid hash format but commit doesn't exist
                    return Err(ReviusError::CommitNotFound(target.to_string()));
                }
            }
        }
    }
    
    // Neither branch nor valid commit hash
    Err(ReviusError::TargetNotFound(target.to_string()))
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
    repo: &Repository,
    tree_hash: [u8; 32],
) -> Result<(), ReviusError> {
    let files = db::trees::get_all_files_in_tree(tx, &tree_hash)?;
    
    for (path, file_hash, mode, size) in files {
        // Get modified time from actual file if it exists, otherwise use 0
        let abs_path = fs::paths::to_absolute(&path, &repo.root);
        let modified_at = if fs::paths::path_exists(&abs_path) {
            fs::io::get_file_modified_time(&abs_path).unwrap_or(0)
        } else {
            0
        };
        
        db::staging::upsert_staging(tx, &path, &file_hash, mode, size, modified_at)?;
    }
    
    Ok(())
}

pub fn format_target_name(target_type: &TargetType, target: &str) -> String {
    match target_type {
        TargetType::Branch(_) => format!("branch '{}'", target),
        TargetType::Commit => format!("commit '{}'", target),
    }
}