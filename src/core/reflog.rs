use crate::core::models::repository::Repository;
use crate::core::models::objects::{ReflogEntry, HeadReference};
use crate::core::refs;
use crate::db;
use crate::error::ReviusError;
use rusqlite::Transaction;

pub fn get_reflog(
    repo: &Repository,
    ref_name: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<ReflogEntry>, ReviusError> {
    let target_ref = match ref_name {
        Some("HEAD") => Some("HEAD".to_string()),
        Some(name) => {
            if name.starts_with("refs/") {
                Some(name.to_string())
            } else {
                // Assume branch if not qualified
                Some(format!("refs/heads/{}", name))
            }
        }
        None => None, // Retrieve all
    };

    db::reflog::get_reflog(&repo.conn, target_ref.as_deref(), limit)
}

/// Helper to log updates to HEAD.
/// If HEAD is attached to a branch, it logs to BOTH the branch and HEAD.
/// If HEAD is detached, it logs only to HEAD.
pub fn log_head_update(
    tx: &Transaction,
    old_hash: Option<&[u8; 32]>,
    new_hash: &[u8; 32],
    action: &str,
) -> Result<(), ReviusError> {
    // 1. Log to HEAD
    db::reflog::insert_reflog(tx, "HEAD", old_hash, Some(new_hash), action)?;

    // 2. Check if we are on a branch, and if so, log to the branch ref too.
    let head_state = refs::get_head_state(tx)?;

    if let HeadReference::Branch(ref_path) = head_state {
        // ref_path is already full path e.g. "refs/heads/main" from core::refs logic
        db::reflog::insert_reflog(tx, &ref_path, old_hash, Some(new_hash), action)?;
    }

    Ok(())
}