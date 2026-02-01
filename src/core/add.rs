use crate::core::models::repository::Repository;
use crate::db;
use crate::error::ReviusError;
use crate::fs;
use crate::core::content;
use rusqlite::Transaction;
use std::path::PathBuf;

#[derive(Debug)]
pub enum StageOutcome {
    Added { blobs: u64 },
    Modified { blobs: u64 },
    Deleted,
    Unchanged,
}

pub fn stage_single_file(tx: &Transaction, repo: &Repository, path: &PathBuf)
-> Result<(PathBuf, StageOutcome), ReviusError> {
    let (file_data, file_hash) = content::read_and_hash_file(path)?;

    let repo_relative_path = fs::paths::make_repo_relative(path, &repo.root)?;

    let mode = fs::io::get_file_mode(path)
        .map_err(|e| ReviusError::Io(path.clone(), e))?;

    let mtime = fs::io::get_file_modified_time(path)
        .map_err(|e| ReviusError::Io(path.clone(), e))?;

    let previous_staged = db::staging::get_staged_file(tx, &repo_relative_path)?;

    let is_modified = previous_staged
        .as_ref()
        .map(|prev| prev.file_hash != file_hash)
        .unwrap_or(false);
    let is_new = previous_staged.is_none();

    if !is_new && !is_modified {
        return Ok((path.clone(), StageOutcome::Unchanged));
    }

    let blob_count = content::store_file_content(tx, path, &file_hash, &file_data, repo)?;

    db::staging::upsert_staging(tx, &repo_relative_path, &file_hash, mode, file_data.len() as u64, mtime)?;

    let outcome = if is_new {
        StageOutcome::Added { blobs: blob_count }
    } else {
        StageOutcome::Modified { blobs: blob_count }
    };

    Ok((path.clone(), outcome))
}

/// Stages files and detects deletions.
/// `found_files`: The list of files that currently exist on disk (result of expanding user paths).
/// `search_scopes`: The original paths provided by the user (to check for deletions within these folders).
pub fn stage_files(repo: &Repository, found_files: Vec<PathBuf>, search_scopes: Vec<PathBuf>)
-> Result<Vec<(PathBuf, StageOutcome)>, ReviusError> {
    let mut results = Vec::new();
    
    let tx = repo.conn.unchecked_transaction()
        .map_err(|e| ReviusError::Db(format!("Failed to start transaction for staging: {}", e)))?;

    // 1. Handle Additions and Modifications (Files that exist)
    for path in found_files {
        let result = stage_single_file(&tx, repo, &path)?;
        results.push(result);
    }

    // 2. Handle Deletions
    let all_staged = db::staging::get_all_staged(&repo.conn)?;
    
    // Pre-calculate relative scopes
    let mut relative_scopes = Vec::new();
    for scope in &search_scopes {
        if let Ok(rel) = fs::paths::make_repo_relative(scope, &repo.root) {
            relative_scopes.push(rel);
        }
    }

    for staged in all_staged {
        // Check if this staged file belongs to one of the user's requested scopes
        let in_scope = relative_scopes.iter().any(|scope| {
            // Case 1: Scope is root (empty string) - EVERYTHING is in scope
            if scope.is_empty() {
                return true;
            }
            // Case 2: Exact match (user added this specific file)
            if staged.path == *scope {
                return true;
            }
            // Case 3: Directory prefix (file is inside the added directory)
            // We check for the separator to avoid matching "file" against scope "f"
            staged.path.starts_with(scope) && staged.path.chars().nth(scope.len()) == Some('/')
        });

        if in_scope {
            // Construct absolute path to check existence on disk
            let abs_path = fs::paths::to_absolute(&staged.path, &repo.root);
            
            // If it doesn't exist on disk, we remove it from staging
            if !fs::paths::path_exists(&abs_path) {
                db::staging::remove_staged_file(&tx, &staged.path)?;
                results.push((abs_path, StageOutcome::Deleted));
            }
        }
    }

    tx.commit()
        .map_err(|e| ReviusError::Db(format!("Failed to commit staging transaction: {}", e)))?;

    Ok(results)
}