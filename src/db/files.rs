use crate::{error::ReviusError, utils};
use crate::core::models::objects::FileInfo;
use rusqlite::{Connection, Transaction};

pub fn insert_file(tx: &Transaction, hash: &[u8; 32], recipe: &[u8], chunk_count: u64, size: u64) -> Result<(), ReviusError> {
    tx.execute(
        "INSERT OR IGNORE INTO Files (hash, size, recipe_version, chunk_count, recipe) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![&hash[..], size as i64, 1, chunk_count as i64, recipe],
    )
    .map_err(|e| ReviusError::Db(format!(
        "Failed to insert file (hash={}): {}",
        utils::hash::hash_to_short_hex(hash),
        e
    )))?;
    
    Ok(())
}

pub fn file_exists(tx: &Transaction, hash: &[u8; 32]) -> Result<bool, ReviusError> {
    let count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM Files WHERE hash = ?1",
        rusqlite::params![&hash[..]],
        |row| row.get(0),
    )
    .map_err(|e| ReviusError::Db(format!(
        "Failed to check file existence (hash={}): {}",
        utils::hash::hash_to_short_hex(hash),
        e
    )))?;
    
    Ok(count > 0)
}

pub fn get_file(conn: &Connection, file_hash: &[u8; 32]) -> Result<FileInfo, ReviusError> {
    let row = conn.query_row(
        "SELECT size, recipe FROM Files WHERE hash = ?1",
        rusqlite::params![file_hash.as_slice()],
        |row| {
            let size: i64 = row.get(0)?;
            let recipe: Vec<u8> = row.get(1)?;
            Ok(FileInfo { size, recipe })
        },
    )
    .map_err(|e| {
        ReviusError::Db(format!(
            "Failed to get file (hash={}): {}",
            utils::hash::hash_to_short_hex(file_hash),
            e
        ))
    })?;
    
    Ok(row)
}