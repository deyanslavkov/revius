use crate::core::models::repository::Repository;
use crate::core::resolve::resolve_target;
use crate::core::tree::get_all_files_in_tree;
use crate::db;
use crate::error::ReviusError;
use crate::fs;
use crate::utils;
use rusqlite::Transaction;
use std::collections::HashSet;
use std::path::PathBuf;

/// Restore working tree from Staging area.
/// Only modifies files that exist in Staging and match the path patterns.
/// Does not delete files from working tree (matches git restore --worktree behavior regarding untracked files).
pub fn restore_worktree(repo: &Repository, paths: &[PathBuf]) -> Result<usize, ReviusError> {
    // 1. Parse patterns
    let patterns = normalize_patterns(repo, paths)?;

    // 2. Get Staged files
    // Since this is a read-only DB op (writing to FS), we don't strictly need a transaction, 
    // but it ensures consistent read.
    let conn = &repo.conn;
    let staged_files = db::staging::get_all_staged(conn)?;
    
    let mut restored_count = 0;

    // 3. Filter and Checkout
    for file in staged_files {
        if matches_any_pattern(&file.path, &patterns) {
            let abs_path = fs::paths::to_absolute(&file.path, &repo.root);
            crate::core::checkout::checkout_file(conn, &file.file_hash, &abs_path, file.mode)?;
            restored_count += 1;
        }
    }

    Ok(restored_count)
}

/// Restore Staging area from a Source Commit (HEAD by default).
/// Updates Staging to match the Source for the given paths.
/// Adds, Updates, and Removes entries in Staging.
pub fn restore_staged(repo: &Repository, paths: &[PathBuf], source: &str) -> Result<usize, ReviusError> {
    let patterns = normalize_patterns(repo, paths)?;
    
    // Transaction needed for Staging updates
    let tx = repo.conn.unchecked_transaction()
        .map_err(|e| ReviusError::Db(format!("Failed to begin transaction: {}", e)))?;

    // Get Source Tree Files
    let source_files = get_source_files(&tx, source)?;
    
    // Get Current Staged Files (for diffing)
    let current_staged = db::staging::get_all_staged(&tx)?;
    let mut staged_map = HashSet::new();
    for f in current_staged {
        staged_map.insert(f.path);
    }

    let mut count = 0;

    // 1. Update/Add files from Source to Staging
    // We iterate Source files. If they match pattern, we put them in Staging.
    for (path, hash, mode, size) in &source_files {
        if matches_any_pattern(path, &patterns) {
            // Use current time for modified_at, as we are technically resetting the index entry
            let now = utils::time::unix_timestamp().unwrap_or(0);
            db::staging::upsert_staging(&tx, path, hash, *mode, *size, now)?;
            count += 1;
        }
    }

    // 2. Remove files from Staging that are NOT in Source but match pattern
    // (Restoring "deletion" from the commit)
    for staged_path in staged_map {
        if matches_any_pattern(&staged_path, &patterns) {
            // If the file matches user pattern, but is NOT in the source list...
            // We verify by checking the source_files map/vec
            let exists_in_source = source_files.iter().any(|(p, _, _, _)| p == &staged_path);
            
            if !exists_in_source {
                db::staging::remove_staged_file(&tx, &staged_path)?;
                count += 1;
            }
        }
    }

    tx.commit()
        .map_err(|e| ReviusError::Db(format!("Failed to commit transaction: {}", e)))?;

    Ok(count)
}

/// Restore both Staging and Worktree from a Source Commit.
/// Adds/Updates/Deletes in both Staging and Disk.
pub fn restore_mixed(repo: &Repository, paths: &[PathBuf], source: &str) -> Result<usize, ReviusError> {
    let patterns = normalize_patterns(repo, paths)?;
    
    let tx = repo.conn.unchecked_transaction()
        .map_err(|e| ReviusError::Db(format!("Failed to begin transaction: {}", e)))?;

    let source_files = get_source_files(&tx, source)?;
    
    // Current Staged (to find deletions)
    let current_staged = db::staging::get_all_staged(&tx)?;
    let mut staged_paths = HashSet::new();
    for f in current_staged {
        staged_paths.insert(f.path);
    }

    let mut count = 0;

    // 1. Sync Source -> Staging
    // We build a plan for workspace updates to execute after DB commit
    let mut to_checkout = Vec::new();
    let mut to_delete = Vec::new();

    // Upsert from Source
    for (path, hash, mode, size) in &source_files {
        if matches_any_pattern(path, &patterns) {
            let now = utils::time::unix_timestamp().unwrap_or(0);
            db::staging::upsert_staging(&tx, path, hash, *mode, *size, now)?;
            to_checkout.push((path.clone(), *hash, *mode));
            count += 1;
        }
    }

    // Remove deleted
    for staged_path in staged_paths {
        if matches_any_pattern(&staged_path, &patterns) {
            let exists_in_source = source_files.iter().any(|(p, _, _, _)| p == &staged_path);
            if !exists_in_source {
                db::staging::remove_staged_file(&tx, &staged_path)?;
                to_delete.push(staged_path);
                count += 1;
            }
        }
    }

    tx.commit()
        .map_err(|e| ReviusError::Db(format!("Failed to commit transaction: {}", e)))?;

    // 2. Sync Worktree
    for path in to_delete {
        let abs_path = fs::paths::to_absolute(&path, &repo.root);
        if fs::paths::path_exists(&abs_path) {
            fs::io::delete_file(&abs_path)
                .map_err(|e| ReviusError::Io(abs_path, e))?;
        }
    }

    for (path, hash, mode) in to_checkout {
        let abs_path = fs::paths::to_absolute(&path, &repo.root);
        crate::core::checkout::checkout_file(&repo.conn, &hash, &abs_path, mode)?;
    }

    Ok(count)
}

fn normalize_patterns(repo: &Repository, paths: &[PathBuf]) -> Result<Vec<String>, ReviusError> {
    let current_dir = fs::paths::get_current_dir()?;
    let mut patterns = Vec::new();
    
    for p in paths {
        // Use absolutize instead of canonicalize to handle non-existent (deleted) files
        let abs = fs::paths::absolutize(p, &current_dir);
        let rel = fs::paths::make_repo_relative(&abs, &repo.root)?;
        patterns.push(rel);
    }
    Ok(patterns)
}

fn matches_any_pattern(path: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        // Dot matches everything (root)
        if pattern == "." || pattern.is_empty() {
            return true;
        }
        // Exact match
        if path == pattern {
            return true;
        }
        // Directory prefix match (e.g., "src" matches "src/main.rs")
        // We add a slash to ensure "src_backup" doesn't match "src"
        let dir_prefix = format!("{}/", pattern);
        if path.starts_with(&dir_prefix) {
            return true;
        }
    }
    false
}

fn get_source_files(conn: &Transaction, source: &str) -> Result<Vec<(String, [u8; 32], u32, u64)>, ReviusError> {
    let resolved = resolve_target(conn, source)?;
    let commit_hash = resolved.hash();
    let tree_hash = db::commits::get_commit_tree(conn, &commit_hash)?;
    get_all_files_in_tree(conn, &tree_hash)
}