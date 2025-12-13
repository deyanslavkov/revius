use crate::core::models::objects::StagedFile;
use crate::error::ReviusError;
use rusqlite::{Transaction, Connection};

pub fn get_staged_file(tx: &Transaction, path: &str) -> Result<Option<StagedFile>, ReviusError> {
    let mut stmt = tx.prepare("SELECT path, file_hash, mode, size FROM Staging WHERE path = ?1")?;
    let mut rows = stmt.query(rusqlite::params![path])?;

    if let Some(row) = rows.next()? {
        let hash_blob: Vec<u8> = row.get(1)?;
        let hash = crate::utils::hash::vec_to_hash(&hash_blob)
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

pub fn upsert_staging(tx: &Transaction, path: &str, hash: &[u8; 32], mode: u32, size: u64, modified_at: i64)
-> Result<(), ReviusError> {
    tx.execute(
        "INSERT OR REPLACE INTO Staging (path, file_hash, mode, size, modified_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![path, &hash[..], mode as i64, size as i64, modified_at],
    )?;
    Ok(())
}

pub fn get_all_staged(conn: &Connection) -> Result<Vec<StagedFile>, ReviusError> {
    let mut stmt = conn
        .prepare("SELECT path, file_hash, mode, size FROM Staging ORDER BY path")
        .map_err(|e| ReviusError::Db(format!("Failed to prepare get all staged query: {}", e)))?;

    let rows = stmt
        .query_map([], |row| {
            let path: String = row.get(0)?;
            let hash_vec: Vec<u8> = row.get(1)?;
            let mode: u32 = row.get(2)?;
            let size: u64 = row.get(3)?;

            Ok(StagedFile {
                path,
                file_hash: crate::utils::hash::vec_to_hash(&hash_vec)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(
                    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))))?,
                mode,
                size,
            })
        })
        .map_err(|e| ReviusError::Db(format!("Failed to query all staged files: {}", e)))?;

    let mut files = Vec::new();
    for row in rows {
        files.push(row.map_err(|e| ReviusError::Db(format!("Failed to parse staged file row: {}", e)))?);
    }

    Ok(files)
}