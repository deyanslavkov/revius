use crate::core::models::repository::Repository;
use crate::core::models::serialization;
use crate::db::{authors, commits, refs, staging};
use crate::error::ReviusError;
use crate::utils::{hash, time};
use crate::core;
use crate::fs;
use rusqlite::Transaction;

pub fn create_commit(repo: &Repository, message: &str) -> Result<([u8; 32], usize), ReviusError> {
    let tx = repo.conn.unchecked_transaction()
        .map_err(|e| ReviusError::Db(format!("Failed to start transaction for commit: {}", e)))?;

    let staged_files = staging::get_all_staged(&tx)?;
    if staged_files.is_empty() {
        return Err(ReviusError::Usage("Nothing to commit (staging area is empty)".to_string()));
    }

    let files_count = staged_files.len();

    let root = core::tree::build_tree_from_files(staged_files)?;

    let tree_hash = core::tree::write_tree_to_db(&tx, &root)?;

    let parent_hash = refs::resolve_head(&tx)?;

    // Check for MERGE_HEAD
    let merge_head_path = repo.root.join(".rvs").join("MERGE_HEAD");
    let mut merge_parent_hash: Option<[u8; 32]> = None;

    if merge_head_path.exists() {
        if let Ok(content) = fs::io::read_file(&merge_head_path) {
             let content_str = String::from_utf8_lossy(&content);
             if let Ok(hash_bytes) = hex::decode(content_str.trim()) {
                 if hash_bytes.len() == 32 {
                     let mut arr = [0u8; 32];
                     arr.copy_from_slice(&hash_bytes);
                     merge_parent_hash = Some(arr);
                 }
             }
        }
    }

    let user_name = repo.config.user_name.as_ref()
        .ok_or_else(|| ReviusError::Config("User name not configured".to_string()))?;
    let user_email = repo.config.user_email.as_ref()
        .ok_or_else(|| ReviusError::Config("User email not configured".to_string()))?;
    let author_id = authors::get_or_create_author(&tx, user_name, user_email)?;

    let timestamp = time::unix_timestamp()
        .map_err(|e| ReviusError::Db(format!("System time error: {}", e)))?;

    let commit_hash = create_commit_object(
        &tx, &tree_hash, parent_hash.as_ref(), merge_parent_hash.as_ref(), user_name, user_email, timestamp, message, author_id)?;

    core::refs::update_head(&tx, &commit_hash)?;

    // Reflog update
    let action_msg = if merge_parent_hash.is_some() {
        format!("merge: {}", message)
    } else {
        format!("commit: {}", message)
    };
    
    core::reflog::log_head_update(&tx, parent_hash.as_ref(), &commit_hash, &action_msg)?;

    tx.commit()
        .map_err(|e| ReviusError::Db(format!("Failed to commit transaction for commit: {}", e)))?;

    // Cleanup MERGE_HEAD if it existed
    if merge_head_path.exists() {
        let _ = fs::io::delete_file(&merge_head_path);
    }

    Ok((commit_hash, files_count))
}

/// Create and insert commit object (with hash)
pub fn create_commit_object(
    tx: &Transaction, tree_hash: &[u8; 32], parent_hash: Option<&[u8; 32]>, merge_parent_hash: Option<&[u8; 32]>,
    author_name: &str, author_email: &str, timestamp: i64, message: &str, author_id: i64)
-> Result<[u8; 32], ReviusError> {
    let serialized = serialization::
        serialize_commit(tree_hash, parent_hash, merge_parent_hash, author_name, author_email, timestamp, message)
        .map_err(|e| ReviusError::Db(format!("Failed to serialize commit: {}", e)))?;

    let commit_hash = hash::hash_bytes(&serialized);

    commits::insert_commit(tx, &commit_hash, parent_hash, merge_parent_hash, tree_hash, message, author_id, timestamp)?;

    Ok(commit_hash)
}