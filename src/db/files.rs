use crate::error::ReviusError;
use rusqlite::Transaction;

pub fn insert_file(
    tx: &Transaction,
    hash: &[u8; 32],
    recipe: &[u8],
    chunk_count: u64,
    size: u64,
) -> Result<(), ReviusError> {
    tx.execute(
        "INSERT OR IGNORE INTO Files (hash, size, recipe_version, chunk_count, recipe) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![&hash[..], size as i64, 1, chunk_count as i64, recipe],
    )?;
    Ok(())
}

pub fn file_exists(tx: &Transaction, hash: &[u8; 32]) -> Result<bool, ReviusError> {
    let count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM Files WHERE hash = ?1",
        rusqlite::params![&hash[..]],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}