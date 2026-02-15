use crate::core::models::objects::StagedFile;
use crate::error::ReviusError;
use rusqlite::{params, Transaction, Connection};

/// Returns StagedFile by repo-relative path
pub fn get_staged_file(tx: &Transaction, path: &str) -> Result<Option<StagedFile>, ReviusError> {
    let mut stmt = tx.prepare("SELECT path, file_hash, mode, size, modified_at FROM Staging WHERE path = ?1")
        .map_err(|e| ReviusError::Db(format!("Failed to prepare get staged file query: {}", e)))?;
    
    let mut rows = stmt.query(params![path])
        .map_err(|e| ReviusError::Db(format!("Failed to query staged file '{}': {}", path, e)))?;

    if let Some(row) = rows.next()
        .map_err(|e| ReviusError::Db(format!("Failed to fetch staged file row '{}': {}", path, e)))? {
        
        let hash_blob: Vec<u8> = row.get(1)
            .map_err(|e| ReviusError::Db(format!("Failed to get hash from staged file '{}': {}", path, e)))?;
        
        let hash = crate::utils::hash::vec_to_hash(&hash_blob)
            .map_err(|e| ReviusError::Db(format!("Invalid hash in staging for '{}': {}", path, e)))?;

        Ok(Some(StagedFile {
            path: row.get(0)
                .map_err(|e| ReviusError::Db(format!("Failed to get path from staged file: {}", e)))?,
            file_hash: hash,
            mode: row.get::<_, i64>(2)
                .map_err(|e| ReviusError::Db(format!("Failed to get mode from staged file '{}': {}", path, e)))? 
                as u32,
            size: row.get::<_, i64>(3)
                .map_err(|e| ReviusError::Db(format!("Failed to get size from staged file '{}': {}", path, e)))? 
                as u64,
            modified_at: row.get::<_, i64>(3)
                .map_err(|e| ReviusError::Db(format!("Failed to get mtime from staged file '{}': {}", path, e)))?,
        }))
    } else {
        Ok(None)
    }
}

pub fn upsert_staging(tx: &Transaction, path: &str, hash: &[u8; 32], mode: u32, size: u64, modified_at: i64) -> Result<(), ReviusError> {
    tx.execute(
        "INSERT OR REPLACE INTO Staging (path, file_hash, mode, size, modified_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![path, &hash[..], mode as i64, size as i64, modified_at],
    )
    .map_err(|e| ReviusError::Db(format!("Failed to upsert staging for '{}': {}", path, e)))?;
    
    Ok(())
}

pub fn get_all_staged(conn: &Connection) -> Result<Vec<StagedFile>, ReviusError> {
    let mut stmt = conn
        .prepare("SELECT path, file_hash, mode, size, modified_at FROM Staging ORDER BY path")
        .map_err(|e| ReviusError::Db(format!("Failed to prepare get all staged query: {}", e)))?;

    let rows = stmt
        .query_map([], |row| {
            let path: String = row.get(0)?;
            let hash_vec: Vec<u8> = row.get(1)?;
            let mode: i64 = row.get(2)?;
            let size: i64 = row.get(3)?;
            let modified_at: i64 = row.get(4)?;

            let file_hash = crate::utils::hash::vec_to_hash(&hash_vec)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(
                    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                ))?;

            Ok(StagedFile {
                path,
                file_hash,
                mode: mode as u32,
                size: size as u64,
                modified_at,
            })
        })
        .map_err(|e| ReviusError::Db(format!("Failed to query all staged files: {}", e)))?;

    let mut files = Vec::new();
    for row in rows {
        files.push(row.map_err(|e| ReviusError::Db(format!("Failed to parse staged file row: {}", e)))?);
    }

    Ok(files)
}

pub fn remove_staged_file(tx: &Transaction, path: &str) -> Result<(), ReviusError> {
    tx.execute(
        "DELETE FROM Staging WHERE path = ?1",
        params![path],
    ).map_err(|e| ReviusError::Db(format!("Failed to remove file from staging: {}", e)))?;
    Ok(())
}

pub fn clear_staging(conn: &Transaction) -> Result<(), ReviusError> {
    conn.execute("DELETE FROM Staging", [])
        .map_err(|e| ReviusError::Db(format!("Failed to clear Staging table: {}", e)))?;
    
    Ok(())
}

pub fn is_staged(conn: &Connection, path: &str) -> Result<bool, ReviusError> {
    let mut stmt = conn.prepare("SELECT 1 FROM Staging WHERE path = ?1")
        .map_err(|e| ReviusError::Db(format!("Failed to prepare is_staged query: {}", e)))?;
    
    let exists = stmt.exists(params![path])
        .map_err(|e| ReviusError::Db(format!("Failed to check if staged: {}", e)))?;
        
    Ok(exists)
}