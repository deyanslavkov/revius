use crate::core::models::repository::Repository;
use crate::core::{commit, refs, switch, tree};
use crate::db;
use crate::error::ReviusError;
use crate::utils::{hash, time};
use rusqlite::Connection;
use std::collections::{BTreeMap, HashSet, VecDeque};

#[derive(Debug)]
pub enum MergeResult {
    FastForward { from: [u8; 32], to: [u8; 32] },
    AlreadyUpToDate,
    MergeCommit { commit_hash: [u8; 32], files_changed: usize },
    Conflicts(Vec<MergeConflict>),
}

#[derive(Debug)]
pub struct MergeConflict {
    pub path: String,
    pub conflict_type: ConflictType,
}

#[derive(Debug)]
pub enum ConflictType {
    BothModified,
    DeletedByUsModifiedByThem,
    DeletedByThemModifiedByUs,
    BothAdded,
}

/// Perform a merge of target_commit into current HEAD
pub fn perform_merge(
    repo: &Repository,
    target_commit: [u8; 32],
) -> Result<MergeResult, ReviusError> {
    // Get current HEAD commit
    let current_commit = db::refs::resolve_head(&repo.conn)?
        .ok_or(ReviusError::NoCommitsYet)?;

    // Check if we're trying to merge the same commit
    if current_commit == target_commit {
        return Ok(MergeResult::AlreadyUpToDate);
    }

    // Find merge base (lowest common ancestor)
    let merge_base = find_merge_base(&repo.conn, current_commit, target_commit)?
        .ok_or_else(|| ReviusError::MergeError("No common ancestor found".to_string()))?;

    // Determine merge strategy based on ancestry
    if merge_base == target_commit {
        // Target is ancestor of current - already up to date
        return Ok(MergeResult::AlreadyUpToDate);
    }

    if merge_base == current_commit {
        // Current is ancestor of target - can fast-forward
        return perform_fast_forward(repo, current_commit, target_commit);
    }

    // Perform three-way merge
    perform_three_way_merge(repo, current_commit, target_commit, merge_base)
}

/// Perform a fast-forward merge by updating HEAD to target
fn perform_fast_forward(
    repo: &Repository,
    from: [u8; 32],
    to: [u8; 32],
) -> Result<MergeResult, ReviusError> {
    // Get target commit to access its tree
    let target_commit = db::commits::get_commit(&repo.conn, &to)?
        .ok_or_else(|| ReviusError::CommitNotFound(hash::hash_to_short_hex(&to)))?;

    // Get current tree
    let current_commit = db::commits::get_commit(&repo.conn, &from)?
        .ok_or_else(|| ReviusError::CommitNotFound(hash::hash_to_short_hex(&from)))?;
    let current_tree = Some(current_commit.tree_hash);

    // Build switch plan
    let plan = switch::build_switch_plan(&repo.conn, current_tree, target_commit.tree_hash)?;

    // Update HEAD and staging in transaction
    let tx = repo.conn.unchecked_transaction()
        .map_err(|e| ReviusError::Db(format!("Failed to start transaction: {}", e)))?;
    
    refs::update_head(&tx, &to)?;
    switch::update_staging_from_tree(&tx, target_commit.tree_hash)?;
    
    tx.commit()
        .map_err(|e| ReviusError::Db(format!("Failed to commit transaction: {}", e)))?;

    // Apply workspace changes
    switch::apply_workspace_changes(repo, &plan)?;

    Ok(MergeResult::FastForward { from, to })
}

/// Perform a three-way merge creating a merge commit
fn perform_three_way_merge(
    repo: &Repository,
    our_commit: [u8; 32],
    their_commit: [u8; 32],
    base_commit: [u8; 32],
) -> Result<MergeResult, ReviusError> {
    // Get tree hashes for all three commits
    let our_commit_obj = db::commits::get_commit(&repo.conn, &our_commit)?
        .ok_or_else(|| ReviusError::CommitNotFound(hash::hash_to_short_hex(&our_commit)))?;
    let their_commit_obj = db::commits::get_commit(&repo.conn, &their_commit)?
        .ok_or_else(|| ReviusError::CommitNotFound(hash::hash_to_short_hex(&their_commit)))?;
    let base_commit_obj = db::commits::get_commit(&repo.conn, &base_commit)?
        .ok_or_else(|| ReviusError::CommitNotFound(hash::hash_to_short_hex(&base_commit)))?;

    // Get tree snapshots
    let base_tree = tree::get_tree_snapshot(&repo.conn, base_commit_obj.tree_hash)?;
    let our_tree = tree::get_tree_snapshot(&repo.conn, our_commit_obj.tree_hash)?;
    let their_tree = tree::get_tree_snapshot(&repo.conn, their_commit_obj.tree_hash)?;

    // Perform three-way merge
    let merge_result = three_way_merge(&base_tree, &our_tree, &their_tree);
    
    // Check for conflicts
    let merged_files = match merge_result {
        Ok(files) => files,
        Err(conflicts) => return Ok(MergeResult::Conflicts(conflicts)),
    };

    // Start transaction for creating merge commit
    let tx = repo.conn.unchecked_transaction()
        .map_err(|e| ReviusError::Db(format!("Failed to start transaction: {}", e)))?;

    // Clear and populate staging with merged files
    db::staging::clear_staging(&tx)?;
    
    // Convert merged files to StagedFile format and populate staging
    for (path, file_hash, mode) in &merged_files {
        let file = db::files::get_file(&tx, file_hash)?;
        db::staging::upsert_staging(&tx, path, file_hash, *mode, file.size as u64, 0)?;
    }

    // Get staged files for tree building
    let staged_files = db::staging::get_all_staged(&tx)?;
    
    // Build tree from staged files
    let tree_node = tree::build_tree_from_files(staged_files)?;
    let tree_hash = tree::write_tree_to_db(&tx, &tree_node)?;

    // Get author
    let author_name = repo.config.user_name
        .as_ref()
        .ok_or_else(|| ReviusError::Config("user_name not set".to_string()))?;
    let author_email = repo.config.user_email
        .as_ref()
        .ok_or_else(|| ReviusError::Config("user_email not set".to_string()))?;
    
    let author_id = db::authors::get_or_create_author(&tx, author_name, author_email)?;

    // Create merge commit message
    let message = format!(
        "Merge commit {}",
        hash::hash_to_short_hex(&their_commit)
    );

    // Get timestamp
    let timestamp = time::unix_timestamp()
        .map_err(|e| ReviusError::Db(format!("Failed to get timestamp: {}", e)))?;

    // Create merge commit with both parents
    let commit_hash = commit::create_commit_object(
        &tx,
        &tree_hash,
        Some(&our_commit),
        Some(&their_commit),
        author_name,
        author_email,
        timestamp,
        &message,
        author_id,
    )?;

    // Update HEAD
    refs::update_head(&tx, &commit_hash)?;

    // Commit transaction
    tx.commit()
        .map_err(|e| ReviusError::Db(format!("Failed to commit transaction: {}", e)))?;

    // Build switch plan and update working directory
    let plan = switch::build_switch_plan(&repo.conn, Some(our_commit_obj.tree_hash), tree_hash)?;
    switch::apply_workspace_changes(repo, &plan)?;

    let files_changed = merged_files.len();

    Ok(MergeResult::MergeCommit {
        commit_hash,
        files_changed,
    })
}

/// Three-way merge algorithm
/// Returns Ok(merged_files) or Err(conflicts)
fn three_way_merge(
    base_tree: &BTreeMap<String, (Option<[u8; 32]>, u32)>,
    our_tree: &BTreeMap<String, (Option<[u8; 32]>, u32)>,
    their_tree: &BTreeMap<String, (Option<[u8; 32]>, u32)>,
) -> Result<Vec<(String, [u8; 32], u32)>, Vec<MergeConflict>> {
    let mut merged_files = Vec::new();
    let mut conflicts = Vec::new();

    // Get all unique paths from all three trees
    let mut all_paths = HashSet::new();
    all_paths.extend(base_tree.keys().cloned());
    all_paths.extend(our_tree.keys().cloned());
    all_paths.extend(their_tree.keys().cloned());

    for path in all_paths {
        let base_file = base_tree.get(&path).and_then(|(h, m)| h.map(|hash| (hash, *m)));
        let our_file = our_tree.get(&path).and_then(|(h, m)| h.map(|hash| (hash, *m)));
        let their_file = their_tree.get(&path).and_then(|(h, m)| h.map(|hash| (hash, *m)));

        match (base_file, our_file, their_file) {
            // All three exist and are identical - use any
            (Some(b), Some(o), Some(t)) if b == o && o == t => {
                merged_files.push((path.clone(), o.0, o.1));
            }

            // Only we changed (theirs same as base or base doesn't exist)
            (Some(b), Some(o), Some(t)) if b == t && b != o => {
                merged_files.push((path.clone(), o.0, o.1));
            }
            (None, Some(o), None) => {
                // Only we added
                merged_files.push((path.clone(), o.0, o.1));
            }

            // Only they changed (ours same as base or base doesn't exist)
            (Some(b), Some(o), Some(t)) if b == o && b != t => {
                merged_files.push((path.clone(), t.0, t.1));
            }
            (None, None, Some(t)) => {
                // Only they added
                merged_files.push((path.clone(), t.0, t.1));
            }

            // Both added identically or both modified to same result
            (None, Some(o), Some(t)) if o == t => {
                merged_files.push((path.clone(), o.0, o.1));
            }
            (Some(_), Some(o), Some(t)) if o == t => {
                // Both modified to same result
                merged_files.push((path.clone(), o.0, o.1));
            }

            // Both deleted (existed in base, now both None) - file disappears
            (Some(_), None, None) => {
                // No-op: file deleted by both sides
            }

            // Both modified differently - conflict
            (Some(_), Some(_), Some(_)) => {
                conflicts.push(MergeConflict {
                    path: path.clone(),
                    conflict_type: ConflictType::BothModified,
                });
            }

            // Both added differently - conflict
            (None, Some(_), Some(_)) => {
                conflicts.push(MergeConflict {
                    path: path.clone(),
                    conflict_type: ConflictType::BothAdded,
                });
            }

            // File existed in base, we modified, they deleted - conflict
            (Some(_), Some(_), None) => {
                conflicts.push(MergeConflict {
                    path: path.clone(),
                    conflict_type: ConflictType::DeletedByThemModifiedByUs,
                });
            }

            // File existed in base, we deleted, they modified - conflict
            (Some(_), None, Some(_)) => {
                conflicts.push(MergeConflict {
                    path: path.clone(),
                    conflict_type: ConflictType::DeletedByUsModifiedByThem,
                });
            }

            // Doesn't exist anywhere - no-op (shouldn't happen since we iterate over union of keys)
            (None, None, None) => {}
        }
    }

    if !conflicts.is_empty() {
        return Err(conflicts);
    }

    Ok(merged_files)
}

/// Find the lowest common ancestor (merge base) of two commits using bidirectional BFS
pub fn find_merge_base(
    conn: &Connection,
    commit1: [u8; 32],
    commit2: [u8; 32],
) -> Result<Option<[u8; 32]>, ReviusError> {
    // Handle the case where commits are identical
    if commit1 == commit2 {
        return Ok(Some(commit1));
    }

    // Use two-way BFS to find the first common ancestor
    let mut visited1 = HashSet::new();
    let mut visited2 = HashSet::new();
    let mut queue1 = VecDeque::new();
    let mut queue2 = VecDeque::new();

    queue1.push_back(commit1);
    queue2.push_back(commit2);
    visited1.insert(commit1);
    visited2.insert(commit2);

    // Alternate between the two searches
    while !queue1.is_empty() || !queue2.is_empty() {
        // Search from commit1
        if !queue1.is_empty() {
            if let Some(lca) = bfs_step(conn, &mut queue1, &mut visited1, &visited2)? {
                return Ok(Some(lca));
            }
        }

        // Search from commit2
        if !queue2.is_empty() {
            if let Some(lca) = bfs_step(conn, &mut queue2, &mut visited2, &visited1)? {
                return Ok(Some(lca));
            }
        }
    }

    Ok(None)
}

/// Perform one BFS step, returns Some(lca) if found
fn bfs_step(
    conn: &Connection,
    queue: &mut VecDeque<[u8; 32]>,
    visited: &mut HashSet<[u8; 32]>,
    other_visited: &HashSet<[u8; 32]>,
) -> Result<Option<[u8; 32]>, ReviusError> {
    if let Some(current) = queue.pop_front() {
        // Get parents of current commit
        let parents = db::commits::get_commit_parents(conn, &current)?;
        
        for parent in parents {
            // Check if this parent was visited by the other search
            if other_visited.contains(&parent) {
                return Ok(Some(parent));
            }
            
            if visited.insert(parent) {
                queue.push_back(parent);
            }
        }
    }

    Ok(None)
}