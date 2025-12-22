use rusqlite::Transaction;
use crate::error::ReviusError;

pub fn insert_reflog(
    tx: &Transaction,
    ref_path: &str,
    old_hash: Option<&[u8; 32]>,
    new_hash: Option<&[u8; 32]>,
    action: &str,
) -> Result<(), ReviusError> {
    tx.execute(
        "INSERT INTO Reflog (ref_path, old_hash, new_hash, action) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            ref_path,
            old_hash.map(|h| h.as_slice()),
            new_hash.map(|h| h.as_slice()),
            action,
        ],
    )
    .map_err(|e| ReviusError::Db(format!("Failed to insert reflog entry for {}: {}", ref_path, e)))?;

    Ok(())
}