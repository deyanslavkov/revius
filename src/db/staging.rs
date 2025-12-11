use crate::core::models::objects::StagedFile;
use crate::error::ReviusError;
use crate::utils;
use rusqlite::Transaction;

pub fn get_staged_file(tx: &Transaction, path: &str) -> Result<Option<StagedFile>, ReviusError> {
    let mut stmt = tx.prepare("SELECT path, file_hash, mode, size FROM Staging WHERE path = ?1")?;
    let mut rows = stmt.query(rusqlite::params![path])?;

    if let Some(row) = rows.next()? {
        let hash_blob: Vec<u8> = row.get(1)?;
        let hash = utils::hash::vec_to_hash(&hash_blob)
            .map_err(|e| ReviusError::Db(format!("Invalid hash in staging: {}", e)))?;

        Ok(Some(StagedFile {
            path: row.get(0)?,
            file_hash: hash,
            mode: row.get::<_, i64>(2)? as u32,
            size: row.get::<_, i64>(3)? as u64,
        }))
    } else {
        Ok(None)
    }
}

pub fn upsert_staging(
    tx: &Transaction,
    path: &str,
    hash: &[u8; 32],
    mode: u32,
    size: u64,
    modified_at: i64,
) -> Result<(), ReviusError> {
    tx.execute(
        "INSERT OR REPLACE INTO Staging (path, file_hash, mode, size, modified_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![path, &hash[..], mode as i64, size as i64, modified_at],
    )?;
    Ok(())
}