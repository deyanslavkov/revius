use crate::error::ReviusError;
use crate::utils::hash;
use rusqlite::{Connection, OptionalExtension, Transaction};

pub fn get_ref(conn: &Connection, path: &str) -> Result<Option<[u8; 32]>, ReviusError> {
    let result: Option<Vec<u8>> = conn
        .query_row(
            "SELECT commit_hash FROM Refs WHERE path = ?1",
            rusqlite::params![path],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| ReviusError::Db(format!("Failed to query ref '{}': {}", path, e)))?;

    match result {
        Some(vec) => {
            let hash_array = hash::vec_to_hash(&vec).map_err(|e| ReviusError::Db(e))?;
            Ok(Some(hash_array))
        }
        None => Ok(None),
    }
}

pub fn upsert_ref(tx: &Transaction, path: &str, ref_type: u8, commit_hash: &[u8; 32]) -> Result<(), ReviusError> {
    tx.execute(
        "INSERT INTO Refs (path, ref_type, commit_hash) VALUES (?1, ?2, ?3)
         ON CONFLICT(path) DO UPDATE SET commit_hash = excluded.commit_hash",
        rusqlite::params![path, ref_type, commit_hash.as_slice()],
    )
    .map_err(|e| ReviusError::Db(format!("Failed to upsert ref '{}': {}", path, e)))?;

    Ok(())
}

/// Update an existing ref - use when you know the ref exists (it doesn't take ref type as a parameter)
pub fn update_ref(tx: &Transaction, path: &str, commit_hash: &[u8; 32]) -> Result<(), ReviusError> {
    let rows = tx
        .execute(
            "UPDATE Refs SET commit_hash = ?1 WHERE path = ?2",
            rusqlite::params![commit_hash.as_slice(), path],
        )
        .map_err(|e| ReviusError::Db(format!("Failed to update ref '{}': {}", path, e)))?;

    if rows == 0 {
        return Err(ReviusError::Db(format!("Ref '{}' not found", path)));
    }

    Ok(())
}

/// Resolve HEAD to a commit hash. Returns None if HEAD points to non-existent ref (initial commit case)
pub fn resolve_head(conn: &Connection) -> Result<Option<[u8; 32]>, ReviusError> {
    // Get HEAD value from Meta
    let head_value: String = conn
        .query_row("SELECT value FROM Meta WHERE key = 'HEAD'", [], |row| {
            row.get(0)
        })
        .map_err(|e| ReviusError::Db(format!("Failed to get HEAD from Meta: {}", e)))?;

    if head_value.starts_with("ref: ") {
        // HEAD points to a ref
        let ref_path = head_value.strip_prefix("ref: ").unwrap();
        get_ref(conn, ref_path)
    } else {
        // Detached HEAD - parse as hex hash
        let hash_bytes = hex::decode(&head_value)
            .map_err(|e| ReviusError::Db(format!("Invalid HEAD hash format: {}", e)))?;
        
        Ok(Some(hash::vec_to_hash(&hash_bytes).map_err(|e| ReviusError::Db(e))?))
    }
}