use crate::error::ReviusError;
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

/// Open or create repo.db at given path, set recommended PRAGMAs.
pub fn open<P: AsRef<Path>>(db_path: P) -> Result<Connection, ReviusError> {
    let db_path = db_path.as_ref();

    // Ensure parent exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Use default open flags: read/write/create
    let conn = Connection::open(db_path).map_err(ReviusError::Db)?;

    // Recommended pragmas
    conn.pragma_update(None, "foreign_keys", &"ON").map_err(ReviusError::Db)?;
    // Set WAL journal mode
    let _ = conn.pragma_update(None, "journal_mode", &"WAL");
    // synchronous to NORMAL (balance safety/perf)
    let _ = conn.pragma_update(None, "synchronous", &"NORMAL");

    Ok(conn)
}
