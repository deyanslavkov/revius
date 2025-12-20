use crate::db::{meta, refs};
use crate::error::ReviusError;
use rusqlite::{Connection, Transaction};

pub enum HeadState {
    Branch(String),  // e.g., "refs/heads/main"
    Detached([u8; 32]),  // commit hash
}

pub const REF_TYPE_BRANCH: u8 = 0;
pub const REF_TYPE_TAG: u8 = 1;
pub const REF_TYPE_REMOTE: u8 = 2;

/// Update HEAD to point to a new commit. Handles both branch refs and detached HEAD, and initial commit case
pub fn update_head(tx: &Transaction, commit_hash: &[u8; 32]) -> Result<(), ReviusError> {
    let head_value = meta::get_meta(tx, "HEAD")?
        .ok_or_else(|| ReviusError::Db("HEAD not found in Meta table".to_string()))?;

    if head_value.is_empty() {
        return Err(ReviusError::Db("HEAD value is empty".to_string()));
    }

    if head_value.starts_with("ref: ") {
        // HEAD points to a branch
        let ref_path = head_value
            .strip_prefix("ref: ")
            .ok_or_else(|| ReviusError::Db(format!("Malformed HEAD value: {}", head_value)))?;
        
        let ref_exists = refs::get_ref(tx, ref_path)?.is_some();
        
        if ref_exists {
            // Update existing ref
            refs::update_ref(tx, ref_path, commit_hash)?;
        } else {
            // Create new ref (initial commit case)
            let ref_type = infer_ref_type(ref_path)?;
            refs::upsert_ref(tx, ref_path, ref_type, commit_hash)?;
        }
    } else {
        // Detached HEAD - update HEAD directly to new commit hash
        let commit_hex = hex::encode(commit_hash);
        meta::set_meta(tx, "HEAD", &commit_hex)?;
    }

    Ok(())
}

/// Infer ref type from ref path
fn infer_ref_type(ref_path: &str) -> Result<u8, ReviusError> {
    if ref_path.starts_with("refs/heads/") {
        Ok(REF_TYPE_BRANCH)
    } else if ref_path.starts_with("refs/tags/") {
        Ok(REF_TYPE_TAG)
    } else if ref_path.starts_with("refs/remotes/") {
        Ok(REF_TYPE_REMOTE)
    } else {
        Err(ReviusError::Db(format!(
            "Unknown ref type for path: '{}'. Must start with refs/heads/, refs/tags/, or refs/remotes/",
            ref_path
        )))
    }
}

pub fn get_head_state(conn: &Connection) -> Result<HeadState, ReviusError> {
    let head_value = meta::get_meta(conn, "HEAD")?
        .ok_or_else(|| ReviusError::Db("HEAD not found".to_string()))?;
    
    if head_value.starts_with("ref: ") {
        let ref_path = head_value.strip_prefix("ref: ").unwrap();
        Ok(HeadState::Branch(ref_path.to_string()))
    } else {
        let hash = hex::decode(&head_value)
            .map_err(|e| ReviusError::Db(format!("Invalid HEAD hash: {}", e)))?;
        let hash_array = crate::utils::hash::vec_to_hash(&hash)
            .map_err(|e| ReviusError::Db(e))?;
        Ok(HeadState::Detached(hash_array))
    }
}