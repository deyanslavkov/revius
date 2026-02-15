use crate::core::content::read_and_hash_file;
use crate::core::models::objects::{StagedFile, StatusInfo, HeadReference};
use crate::core::models::repository::Repository;
use crate::core::refs::get_head_state;
use crate::core::tree::get_all_tree_files;
use crate::db;
use crate::error::ReviusError;
use crate::fs;
use crate::utils::hash::hash_to_short_hex;
use std::collections::{BTreeMap, HashSet};

/// Get comprehensive status information by comparing HEAD, staging area, and working directory
pub fn get_status_info(repo: &Repository) -> Result<StatusInfo, ReviusError> {
    let conn = &repo.conn;

    let head_state = get_head_state(conn)?;
    let (branch_name, detached_commit) = match head_state {
        HeadReference::Branch(ref_path) => {
            let branch = ref_path
                .strip_prefix("refs/heads/")
                .unwrap_or(&ref_path)
                .to_string();
            (Some(branch), None)
        }
        HeadReference::Detached(commit_hash) => (None, Some(commit_hash)),
    };

    let head_files = get_head_files(conn)?;

    let staged_files_map = get_staged_files_map(conn)?;

    // Pass the staged files map to workdir scanning for optimization
    let workdir_files = get_workdir_files(repo, &staged_files_map)?;

    let mut staged_new = Vec::new();
    let mut staged_modified = Vec::new();
    let mut staged_deleted = Vec::new();

    for (path, staged_file) in &staged_files_map {
        match head_files.get(path) {
            None => staged_new.push(path.clone()),
            Some(head_hash) => {
                if head_hash != &staged_file.file_hash {
                    staged_modified.push(path.clone());
                }
            }
        }
    }

    for path in head_files.keys() {
        if !staged_files_map.contains_key(path) {
            staged_deleted.push(path.clone());
        }
    }

    let mut unstaged_modified = Vec::new();
    let mut unstaged_deleted = Vec::new();

    for (path, staged_file) in &staged_files_map {
        match workdir_files.get(path) {
            None => unstaged_deleted.push(path.clone()),
            Some(workdir_hash) => {
                if workdir_hash != &staged_file.file_hash {
                    unstaged_modified.push(path.clone());
                }
            }
        }
    }

    let mut untracked = Vec::new();
    let tracked: HashSet<&String> = head_files
        .keys()
        .chain(staged_files_map.keys())
        .collect();

    for path in workdir_files.keys() {
        if !tracked.contains(path) {
            untracked.push(path.clone());
        }
    }

    staged_new.sort();
    staged_modified.sort();
    staged_deleted.sort();
    unstaged_modified.sort();
    unstaged_deleted.sort();
    untracked.sort();

    Ok(StatusInfo {
        branch_name,
        detached_commit,
        staged_new,
        staged_modified,
        staged_deleted,
        unstaged_modified,
        unstaged_deleted,
        untracked,
    })
}

/// Get all files from HEAD commit with their hashes
pub fn get_head_files(conn: &rusqlite::Connection) -> Result<BTreeMap<String, [u8; 32]>, ReviusError> {
    let commit_hash = match db::refs::resolve_head(conn)? {
        Some(hash) => hash,
        None => return Ok(BTreeMap::new()), // No commits yet
    };

    let commit = db::commits::get_commit(conn, &commit_hash)?.ok_or_else(|| {
        ReviusError::Db(format!(
            "HEAD points to non-existent commit {}",
            hash_to_short_hex(&commit_hash)
        ))
    })?;

    get_all_tree_files(conn, &commit.tree_hash)
}

/// Get all staged files as a map
pub fn get_staged_files_map(
    conn: &rusqlite::Connection,
) -> Result<BTreeMap<String, StagedFile>, ReviusError> {
    let staged = db::staging::get_all_staged(conn)?;
    let mut map = BTreeMap::new();

    for file in staged {
        map.insert(file.path.clone(), file);
    }

    Ok(map)
}

/// Get all working directory files with their hashes
/// Skips hashing if metadata matches Staging
pub fn get_workdir_files(
    repo: &Repository, 
    staged_files: &BTreeMap<String, StagedFile>
) -> Result<BTreeMap<String, [u8; 32]>, ReviusError> {
    let ignore_path = fs::paths::get_repo_ignore_path(&repo.root);
    let all_files = fs::walk::get_all_repo_files(&repo.root, &ignore_path)?;

    let mut map = BTreeMap::new();

    for abs_path in all_files {
        let rel_path = fs::paths::make_repo_relative(&abs_path, &repo.root)?;

        // Check against staged metadata
        let hash = if let Some(staged) = staged_files.get(&rel_path) {
             let mtime = fs::io::get_file_modified_time(&abs_path).unwrap_or(0);
             let size = std::fs::metadata(&abs_path).map(|m| m.len()).unwrap_or(0);
             
             // If metadata matches, trust the staged hash (Skip IO read/hash)
             if staged.modified_at == mtime && staged.size == size {
                 staged.file_hash
             } else {
                 // Metadata changed, we must re-hash to verify content change
                 let (_, h) = read_and_hash_file(&abs_path)?;
                 h
             }
        } else {
            // Untracked file: must hash
             let (_, h) = read_and_hash_file(&abs_path)?;
             h
        };

        map.insert(rel_path, hash);
    }

    Ok(map)
}