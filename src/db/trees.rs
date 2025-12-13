use crate::core::models::objects::TreeEntry;
use crate::error::ReviusError;
use rusqlite::{Transaction, Connection};

/// Check if a tree with given parent_hash already exists in the database
pub fn tree_exists(conn: &Connection, parent_hash: &[u8; 32]) -> Result<bool, ReviusError> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM Trees WHERE parent_hash = ?1 LIMIT 1)",
            [parent_hash.as_slice()],
            |row| row.get(0),
        )
        .map_err(|e| ReviusError::Db(format!("Failed to check tree existence (hash={}): {}", hex::encode(&parent_hash[..8]), e)))?;
    
    Ok(exists)
}

/// Insert a single tree entry
pub fn insert_tree_entry(
    tx: &Transaction,
    parent_hash: &[u8; 32],
    name: &str,
    object_hash: &[u8; 32],
    mode: u32,
    is_dir: bool,
) -> Result<(), ReviusError> {
    tx.execute(
        "INSERT INTO Trees (parent_hash, name, object_hash, mode, is_dir) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![parent_hash.as_slice(), name, object_hash.as_slice(), mode, is_dir as i32],
    )
    .map_err(|e| ReviusError::Db(format!("Failed to insert tree entry '{}': {}", name, e)))?;

    Ok(())
}

/// Batch insert multiple tree entries efficiently
pub fn batch_insert_tree_entries(
    tx: &Transaction,
    entries: Vec<TreeEntry>,
) -> Result<(), ReviusError> {
    let mut stmt = tx
        .prepare("INSERT INTO Trees (parent_hash, name, object_hash, mode, is_dir) VALUES (?1, ?2, ?3, ?4, ?5)")
        .map_err(|e| ReviusError::Db(format!("Failed to prepare tree batch insert: {}", e)))?;

    for entry in entries {
        stmt.execute(rusqlite::params![
            entry.parent_hash.as_slice(),
            entry.name,
            entry.object_hash.as_slice(),
            entry.mode,
            entry.is_dir as i32
        ])
        .map_err(|e| ReviusError::Db(format!("Failed to insert tree entry '{}': {}", entry.name, e)))?;
    }

    Ok(())
}