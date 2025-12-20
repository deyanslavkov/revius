use crate::error::ReviusError;
use rusqlite::{Connection, Transaction, OptionalExtension};

pub const CURRENT_SCHEMA_VERSION: i64 = 1;

pub fn check_schema_version(conn: &Connection) -> Result<(), ReviusError> {
    let version = get_schema_version(conn)?;
    
    if version > CURRENT_SCHEMA_VERSION {
        return Err(ReviusError::Config(format!(
            "Repository uses schema version {}, but this Revius version only supports up to {}.\n\
             Please upgrade Revius to open this repository.",
            version, CURRENT_SCHEMA_VERSION
        )));
    }
    
    if version < CURRENT_SCHEMA_VERSION {
        return Err(ReviusError::Config(format!(
            "Repository uses outdated schema version {} (current: {}).\n\
             Automatic migrations not yet implemented.\n\
             Options:\n\
             1. Use an older Revius version (schema {})\n\
             2. Recreate the repository\n\
             3. Wait for migration support in future release",
            version, CURRENT_SCHEMA_VERSION, version
        )));
    }
    
    Ok(())
}

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