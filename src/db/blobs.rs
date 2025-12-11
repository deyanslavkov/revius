use crate::error::ReviusError;
use rusqlite::Transaction;

pub fn insert_blob(
    tx: &Transaction,
    hash: &[u8; 32],
    data: &[u8],
    compression: &str,
    uncompressed_size: u64,
) -> Result<(), ReviusError> {
    tx.execute(
        "INSERT OR IGNORE INTO Blobs (hash, data, compression, uncompressed_size) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![&hash[..], data, compression, uncompressed_size as i64],
    )?;
    Ok(())
}

pub fn blob_exists(tx: &Transaction, hash: &[u8; 32]) -> Result<bool, ReviusError> {
    let count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM Blobs WHERE hash = ?1",
        rusqlite::params![&hash[..]],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}