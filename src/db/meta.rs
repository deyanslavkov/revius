use crate::error::ReviusError;
use rusqlite::Connection;

pub fn get_schema_version(conn: &Connection) -> Result<i64, ReviusError> {
    let version_str: String = conn.query_row(
        "SELECT value FROM Meta WHERE key = 'schema_version'",
        [],
        |row| row.get(0),
    ).map_err(|e| ReviusError::Db(format!("Failed to query schema version: {}", e)))?;
    
    version_str.parse::<i64>()
        .map_err(|_| ReviusError::Db(format!("Invalid schema version format: {}", version_str)))
}