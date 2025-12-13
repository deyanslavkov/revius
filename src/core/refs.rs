use crate::db::{meta, refs};
use crate::error::ReviusError;
use rusqlite::Transaction;

/// Update HEAD to point to a new commit
/// Handles both branch refs and detached HEAD
pub fn update_head(tx: &Transaction, commit_hash: &[u8; 32]) -> Result<(), ReviusError> {
    let head_value = meta::get_meta(tx, "HEAD")?
        .ok_or_else(|| ReviusError::Db("HEAD not found in Meta table".to_string()))?;

    if head_value.starts_with("ref: ") {
        let ref_path = head_value
            .strip_prefix("ref: ")
            .ok_or_else(|| ReviusError::Db(format!("Malformed HEAD value: {}", head_value)))?;
        
        let ref_exists = refs::get_ref(tx, ref_path)?.is_some();
        
        if ref_exists {
            refs::update_ref(tx, ref_path, commit_hash)?;
        } else {
            // Create new ref (initial commit case)
            let ref_type = if ref_path.starts_with("refs/heads/") {
                0
            } else if ref_path.starts_with("refs/tags/") {
                1
            } else {
                2
            };
            refs::upsert_ref(tx, ref_path, ref_type, commit_hash)?;
        }
    } else {
        // Detached HEAD
        let commit_hex = hex::encode(commit_hash);
        meta::set_meta(tx, "HEAD", &commit_hex)?;
    }

    Ok(())
}