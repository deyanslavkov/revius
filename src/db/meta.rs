use crate::error::ReviusError;
use rusqlite::{Connection, Transaction, OptionalExtension};

pub fn get_schema_version(conn: &Connection) -> Result<i64, ReviusError> {
    let version_str: String = conn.query_row(
        "SELECT value FROM Meta WHERE key = 'schema_version'",
        [],
        |row| row.get(0),
    ).map_err(|e| ReviusError::Db(format!("Failed to query schema version: {}", e)))?;
    
    version_str.parse::<i64>()
        .map_err(|_| ReviusError::Db(format!("Invalid schema version format: {}", version_str)))
}

pub fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>, ReviusError> {
    let result = conn
        .query_row(
            "SELECT value FROM Meta WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| ReviusError::Db(format!("Failed to get meta key '{}': {}", key, e)))?;

    Ok(result)
}

pub fn set_meta(tx: &Transaction, key: &str, value: &str) -> Result<(), ReviusError> {
    tx.execute(
        "INSERT INTO Meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )
    .map_err(|e| ReviusError::Db(format!("Failed to set meta key '{}': {}", key, e)))?;

    Ok(())
}