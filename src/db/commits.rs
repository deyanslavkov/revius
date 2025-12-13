use crate::core::models::objects::Commit;
use crate::error::ReviusError;
use crate::utils::hash;
use rusqlite::{Connection, Transaction, OptionalExtension};

/// Insert a commit into the database
pub fn insert_commit(
    tx: &Transaction,
    hash: &[u8; 32],
    parent_hash: Option<&[u8; 32]>,
    merge_parent_hash: Option<&[u8; 32]>,
    tree_hash: &[u8; 32],
    message: &str,
    author_id: i64,
    timestamp: i64,
) -> Result<(), ReviusError> {
    tx.execute(
        "INSERT INTO Commits (hash, parent_hash, merge_parent_hash, tree_hash, message, author_id, timestamp) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            hash.as_slice(),
            parent_hash.map(|h| h.as_slice()),
            merge_parent_hash.map(|h| h.as_slice()),
            tree_hash.as_slice(),
            message,
            author_id,
            timestamp
        ],
    )
    .map_err(|e| ReviusError::Db(format!("Failed to insert commit (hash={}): {}", hex::encode(&hash[..8]), e)))?;

    Ok(())
}

/// Get a commit by hash
pub fn get_commit(conn: &Connection, hash: &[u8; 32]) -> Result<Option<Commit>, ReviusError> {
    let mut stmt = conn
        .prepare("SELECT hash, parent_hash, merge_parent_hash, tree_hash, message, author_id, timestamp FROM Commits WHERE hash = ?1")
        .map_err(|e| ReviusError::Db(format!("Failed to prepare get commit query: {}", e)))?;

    let result = stmt
        .query_row(rusqlite::params![hash.as_slice()], |row| {
            let hash_vec: Vec<u8> = row.get(0)?;
            let parent_vec: Option<Vec<u8>> = row.get(1)?;
            let merge_parent_vec: Option<Vec<u8>> = row.get(2)?;
            let tree_vec: Vec<u8> = row.get(3)?;

            Ok(Commit {
                hash: hash::vec_to_hash(&hash_vec).unwrap(),
                parent_hash: parent_vec.and_then(|v| hash::vec_to_hash(&v).ok()),
                merge_parent_hash: merge_parent_vec.and_then(|v| hash::vec_to_hash(&v).ok()),
                tree_hash: hash::vec_to_hash(&tree_vec).unwrap(),
                message: row.get(4)?,
                author_id: row.get(5)?,
                timestamp: row.get(6)?,
            })
        })
        .optional()
        .map_err(|e| ReviusError::Db(format!("Failed to get commit (hash={}): {}", hex::encode(&hash[..8]), e)))?;

    Ok(result)
}

/// Check if a commit exists by hash
pub fn commit_exists(conn: &Connection, hash: &[u8; 32]) -> Result<bool, ReviusError> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM Commits WHERE hash = ?1)",
            [hash.as_slice()],
            |row| row.get(0),
        )
        .map_err(|e| ReviusError::Db(format!("Failed to check commit existence (hash={}): {}", hex::encode(&hash[..8]), e)))?;
    
    Ok(exists)
}