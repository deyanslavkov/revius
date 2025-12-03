use crate::error::ReviusError;
use rusqlite::Connection;

/// Apply schema to the provided Connection.
/// For now: Meta and Audit tables.
pub fn apply(conn: &Connection) -> Result<(), ReviusError> {
    let sql = r#"
    BEGIN;
    CREATE TABLE IF NOT EXISTS Meta (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS Audit (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        timestamp TEXT NOT NULL,
        action TEXT NOT NULL,
        detail TEXT
    );
    COMMIT;
    "#;

    conn.execute_batch(sql).map_err(ReviusError::Db)
}
