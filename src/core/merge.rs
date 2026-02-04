use crate::core::models::repository::Repository;
use crate::core::{commit, refs, switch, tree, checkout, content};
use crate::db;
use crate::error::ReviusError;
use crate::utils::{hash, time};
use crate::fs;
use rusqlite::Connection;
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::str;

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

#[derive(Debug, Clone, Copy)]
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

/// Internal struct to track the merge plan
struct MergePlan {
    clean: Vec<(String, [u8; 32], u32)>,
    conflicts: Vec<ConflictEntry>,
}

struct ConflictEntry {
    path: String,
    // hashes (None if file doesn't exist in that version)
    _base: Option<[u8; 32]>,
    our: Option<[u8; 32]>,
    their: Option<[u8; 32]>,
    mode: u32,
    type_: ConflictType,
}

/// Perform a three-way merge logic
fn perform_three_way_merge(
    repo: &Repository,
    our_commit: [u8; 32],
    their_commit: [u8; 32],
    base_commit: [u8; 32],
) -> Result<MergeResult, ReviusError> {
    // Get tree objects
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

    // Calculate Merge Plan (Clean vs Conflicts)
    let plan = plan_three_way_merge(&base_tree, &our_tree, &their_tree);

    let files_changed = plan.clean.len();

    // Start transaction for DB updates
    let tx = repo.conn.unchecked_transaction()
        .map_err(|e| ReviusError::Db(format!("Failed to start transaction: {}", e)))?;

    // Clear staging to rebuild it with merged result
    db::staging::clear_staging(&tx)?;
    
    let mut conflict_list = Vec::new();

    // 1. Process Clean Merges
    for (path, file_hash, mode) in plan.clean {
        // Upsert to staging
        // We need file size, so fetch info
        let file_info = db::files::get_file(&tx, &file_hash)?;
        db::staging::upsert_staging(&tx, &path, &file_hash, mode, file_info.size as u64, 0)?;

        // Update Workspace (Write the file)
        let target_path = repo.root.join(&path);
        checkout::checkout_file(&tx, &file_hash, &target_path, mode)?;
    }

    // 2. Process Conflicts
    for conflict in plan.conflicts {
        // Generate conflict file content (with <<<<<< markers)
        let (merged_content, _) = generate_conflict_content(&tx, &conflict)?;
        
        // Write to workspace
        let target_path = repo.root.join(&conflict.path);
        if let Some(parent) = target_path.parent() {
            fs::io::create_dir_all(parent).ok(); // ignore if exists
        }
        
        fs::io::write_binary(&target_path, &merged_content)
            .map_err(|e| ReviusError::Io(target_path.clone(), e))?;
            
        // Store this "conflict version" as a new blob/file in DB
        // This effectively "stages" the conflict markers, which is valid safe behavior
        let content_hash = hash::hash_bytes(&merged_content);
        
        // We use store_file_content to handle chunking/storing of this new synthetic file
        content::store_file_content(&tx, &target_path, &content_hash, &merged_content, repo)?;
        
        // Add to staging (So the user sees it as "Modified" or "Added" in status)
        db::staging::upsert_staging(&tx, &conflict.path, &content_hash, conflict.mode, merged_content.len() as u64, 0)?;

        conflict_list.push(MergeConflict {
            path: conflict.path,
            conflict_type: conflict.type_,
        });
    }

    // If we have conflicts, we STOP here, saving the partial state.
    if !conflict_list.is_empty() {
        // Write MERGE_HEAD to signal we are in a merge state
        let merge_head_path = repo.root.join(".rvs").join("MERGE_HEAD");
        fs::io::write_binary(&merge_head_path, hex::encode(their_commit).as_bytes())
             .map_err(|e| ReviusError::Io(merge_head_path, e))?;

        // Commit the transaction to save Staging changes (partial merge)
        tx.commit().map_err(|e| ReviusError::Db(format!("Failed to commit conflict state: {}", e)))?;
        
        return Ok(MergeResult::Conflicts(conflict_list));
    }

    // --- NO CONFLICTS: Finalize Merge Commit ---

    // Build tree from the (now clean) staging
    let staged_files = db::staging::get_all_staged(&tx)?;
    let tree_node = tree::build_tree_from_files(staged_files)?;
    let tree_hash = tree::write_tree_to_db(&tx, &tree_node)?;

    // Get author info
    let author_name = repo.config.user_name.clone()
        .ok_or_else(|| ReviusError::Config("user_name not set".to_string()))?;
    let author_email = repo.config.user_email.clone()
        .ok_or_else(|| ReviusError::Config("user_email not set".to_string()))?;
    
    let author_id = db::authors::get_or_create_author(&tx, &author_name, &author_email)?;

    // Create merge commit
    let message = format!("Merge commit {}", hash::hash_to_short_hex(&their_commit));
    let timestamp = time::unix_timestamp().unwrap_or(0);

    let commit_hash = commit::create_commit_object(
        &tx,
        &tree_hash,
        Some(&our_commit),
        Some(&their_commit),
        &author_name,
        &author_email,
        timestamp,
        &message,
        author_id,
    )?;

    // Update HEAD
    refs::update_head(&tx, &commit_hash)?;

    // Commit transaction
    tx.commit()
        .map_err(|e| ReviusError::Db(format!("Failed to commit transaction: {}", e)))?;

    Ok(MergeResult::MergeCommit {
        commit_hash,
        files_changed,
    })
}

/// Helper to generate conflict content with markers
fn generate_conflict_content(conn: &Connection, conflict: &ConflictEntry) -> Result<(Vec<u8>, bool), ReviusError> {
    // Helper to fetch content safely
    let fetch = |h: Option<[u8;32]>| -> Vec<u8> {
        if let Some(hash) = h {
            checkout::reconstruct_file(conn, &hash).unwrap_or_default()
        } else {
            Vec::new()
        }
    };

    let ours = fetch(conflict.our);
    let theirs = fetch(conflict.their);

    // Try to treat as strings
    let our_str = str::from_utf8(&ours);
    let their_str = str::from_utf8(&theirs);

    if let (Ok(o), Ok(t)) = (our_str, their_str) {
        // Text file: Generate markers
        let content = format!(
            "<<<<<<< HEAD\n{}\n=======\n{}\n>>>>>>> MERGE_HEAD\n",
            o, t
        );
        Ok((content.into_bytes(), true))
    } else {
        // Binary file: We can't insert markers.
        // Strategy: Keep "Ours" content, but it will be marked as conflict in the return list.
        // The user will know there's a conflict but the file on disk is the pre-merge version.
        Ok((ours, false))
    }
}

/// Plan the merge: separate clean files from conflicts
fn plan_three_way_merge(
    base_tree: &BTreeMap<String, (Option<[u8; 32]>, u32)>,
    our_tree: &BTreeMap<String, (Option<[u8; 32]>, u32)>,
    their_tree: &BTreeMap<String, (Option<[u8; 32]>, u32)>,
) -> MergePlan {
    let mut clean = Vec::new();
    let mut conflicts = Vec::new();

    let mut all_paths = HashSet::new();
    all_paths.extend(base_tree.keys().cloned());
    all_paths.extend(our_tree.keys().cloned());
    all_paths.extend(their_tree.keys().cloned());

    for path in all_paths {
        let base = base_tree.get(&path).and_then(|(h, m)| h.map(|hash| (hash, *m)));
        let our = our_tree.get(&path).and_then(|(h, m)| h.map(|hash| (hash, *m)));
        let their = their_tree.get(&path).and_then(|(h, m)| h.map(|hash| (hash, *m)));

        match (base, our, their) {
            // 1. Clean Merges
            
            // All three exist and are identical - use any
            (Some(b), Some(o), Some(t)) if b.0 == o.0 && o.0 == t.0 => {
                clean.push((path.clone(), o.0, o.1));
            }

            // Only we changed (theirs same as base or base doesn't exist)
            (Some(b), Some(o), Some(t)) if b.0 == t.0 && b.0 != o.0 => {
                clean.push((path.clone(), o.0, o.1));
            }
            // Only we added
            (None, Some(o), None) => {
                clean.push((path.clone(), o.0, o.1));
            }

            // Only they changed (ours same as base or base doesn't exist)
            (Some(b), Some(o), Some(t)) if b.0 == o.0 && b.0 != t.0 => {
                clean.push((path.clone(), t.0, t.1));
            }
            // Only they added
            (None, None, Some(t)) => {
                clean.push((path.clone(), t.0, t.1));
            }

            // Both added identically or both modified to same result
            (None, Some(o), Some(t)) if o.0 == t.0 => {
                clean.push((path.clone(), o.0, o.1));
            }
            // Both modified to same result
            (Some(_), Some(o), Some(t)) if o.0 == t.0 => {
                clean.push((path.clone(), o.0, o.1));
            }

            // Both deleted (existed in base, now both None) - file disappears
            (Some(_), None, None) => {
                // No-op: file deleted by both sides
            }

            // 2. Conflicts

            // Both modified differently
            (Some(b), Some(o), Some(t)) => {
                conflicts.push(ConflictEntry {
                    path: path.clone(),
                    _base: Some(b.0), our: Some(o.0), their: Some(t.0),
                    mode: o.1,
                    type_: ConflictType::BothModified,
                });
            }

            // Both added differently
            (None, Some(o), Some(t)) => {
                conflicts.push(ConflictEntry {
                    path: path.clone(),
                    _base: None, our: Some(o.0), their: Some(t.0),
                    mode: o.1,
                    type_: ConflictType::BothAdded,
                });
            }

            // File existed in base, we modified, they deleted
            (Some(b), Some(o), None) => {
                // If base != ours, we modified it.
                if b.0 != o.0 {
                    conflicts.push(ConflictEntry {
                        path: path.clone(),
                        _base: Some(b.0), our: Some(o.0), their: None,
                        mode: o.1,
                        type_: ConflictType::DeletedByThemModifiedByUs,
                    });
                } else {
                    // We didn't modify it, they deleted it -> Clean delete (do nothing, effectively deleted)
                }
            }

            // File existed in base, we deleted, they modified - conflict
            (Some(b), None, Some(t)) => {
                // If base != theirs, they modified it.
                if b.0 != t.0 {
                     conflicts.push(ConflictEntry {
                        path: path.clone(),
                        _base: Some(b.0), our: None, their: Some(t.0),
                        mode: t.1, 
                        type_: ConflictType::DeletedByUsModifiedByThem,
                    });
                } else {
                    // They didn't modify, we deleted -> Clean delete
                }
            }
            
            // Should not happen if iteration logic is correct (all None)
            _ => {}
        }
    }
    
    MergePlan { clean, conflicts }
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