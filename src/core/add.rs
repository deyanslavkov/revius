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

pub fn stage_files(repo: &Repository, paths: Vec<PathBuf>)
-> Result<Vec<(PathBuf, StageOutcome)>, ReviusError> {
    let mut results = Vec::new();
    
    let tx = repo.conn.unchecked_transaction()
        .map_err(|e| ReviusError::Db(format!("Failed to start transaction for staging: {}", e)))?;

    for path in paths {
        let result = stage_single_file(&tx, repo, &path)?;
        results.push(result);
    }

    tx.commit()
        .map_err(|e| ReviusError::Db(format!("Failed to commit staging transaction: {}", e)))?;

    Ok(results)
}