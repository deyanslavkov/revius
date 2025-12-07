use crate::error::ReviusError;
use rusqlite::Connection;
use std::path::Path;

pub fn open_db(path: &Path) -> Result<Connection, ReviusError> {
    let conn = Connection::open(path)
        .map_err(|e| ReviusError::Db(format!("Failed to open database at {}: {}", path.display(), e)))?;
    
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| ReviusError::Db(format!("Failed to enable foreign keys: {}", e)))?;
    
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| ReviusError::Db(format!("Failed to set WAL mode: {}", e)))?;
    
    Ok(conn)
}