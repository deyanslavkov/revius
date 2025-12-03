use crate::error::ReviusError;
use crate::fs::lock::RepoLock;
use crate::db::connection;
use crate::core::config::Config;
use crate::core::repository::state::Repository;
use rusqlite::params;
use std::path::{Path, PathBuf};
use std::fs;

/// Open repository at root. Validates basic things and acquires lock.
pub fn open_repository<P: AsRef<Path>>(root: P) -> Result<Repository, ReviusError> {
    let root = root.as_ref().to_path_buf();
    let rvs_dir = root.join(".rvs");
    let db_path = rvs_dir.join("repo.db");
    let lock_path = rvs_dir.join("lock");

    if !rvs_dir.exists() || !db_path.exists() {
        return Err(ReviusError::Corrupt("Not a Revius repository (missing .rvs or repo.db)".into()));
    }

    // Open DB
    let conn = connection::open(&db_path)?;

    // Read schema_version from Meta
    let version_str: String = conn.query_row(
        "SELECT value FROM Meta WHERE key = 'schema_version'",
        [],
        |row| row.get(0)
    ).map_err(|_| ReviusError::Corrupt("Missing schema_version in Meta".into()))?;

    let version = version_str.parse::<i32>()
        .map_err(|_| ReviusError::Corrupt("Invalid schema_version in Meta".into()))?;

    if version != 1 {
        return Err(ReviusError::Corrupt(format!("Unsupported repository version: {}", version)));
    }

    // Acquire lock (this will error if locked)
    let lock = RepoLock::acquire(lock_path)?;

    // Load config
    let canonical_root = fs::canonicalize(&root).unwrap_or(root.clone());
    let config = Config::load(&canonical_root)?;

    Ok(Repository {
        root: canonical_root,
        conn,
        lock,
        config,
    })
}
