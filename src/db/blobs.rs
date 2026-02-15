use crate::{error::ReviusError, utils};
use rusqlite::{Connection, Transaction};

pub fn insert_blob(tx: &Transaction, hash: &[u8; 32], data: &[u8], compression: &str, uncompressed_size: u64) -> Result<(), ReviusError> {
    tx.execute(
        "INSERT OR IGNORE INTO Blobs (hash, data, compression, uncompressed_size) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![&hash[..], data, compression, uncompressed_size as i64],
    )
    .map_err(|e| ReviusError::Db(format!(
        "Failed to insert blob (hash={}): {}",
        utils::hash::hash_to_short_hex(hash),
        e
    )))?;
    
    Ok(())
}

pub fn blob_exists(tx: &Transaction, hash: &[u8; 32]) -> Result<bool, ReviusError> {
    let count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM Blobs WHERE hash = ?1",
        rusqlite::params![&hash[..]],
        |row| row.get(0),
    )
    .map_err(|e| ReviusError::Db(format!(
        "Failed to check blob existence (hash={}): {}",
        utils::hash::hash_to_short_hex(hash),
        e
    )))?;
    
    Ok(count > 0)
}

/// Get blob data and its compression method by hash
/// Returns (data, compression_string)
pub fn get_blob(conn: &Connection, blob_hash: &[u8; 32]) -> Result<(Vec<u8>, String), ReviusError> {
    let (data, compression): (Vec<u8>, String) = conn
        .query_row(
            "SELECT data, compression FROM Blobs WHERE hash = ?1",
            rusqlite::params![blob_hash.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| {
            ReviusError::Db(format!(
                "Failed to get blob (hash={}): {}",
                utils::hash::hash_to_short_hex(blob_hash),
                e
            ))
        })?;
    
    Ok((data, compression))
}