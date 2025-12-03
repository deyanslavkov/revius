use crate::core::config::Config;
use crate::error::ReviusError;
use crate::fs::config as fs_config;
use crate::fs::lock::RepoLock;
use crate::db::{connection, schema};
use crate::core::repository::state::Repository;
use std::path::Path;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use uuid::Uuid;
use rusqlite::params;

/// Initialize a new repository at `root`.
/// Behavior:
/// - if `.rvs` exists and looks valid => AlreadyInitialized error
/// - create `.rvs`, create lock, create repo.db, apply schema, insert Meta, write default config & ignore (only if missing)
pub fn init<P: AsRef<Path>>(root: P) -> Result<Repository, ReviusError> {
    let root = root.as_ref().to_path_buf();
    let rvs_dir = root.join(".rvs");
    let db_path = rvs_dir.join("repo.db");
    let lock_path = rvs_dir.join("lock");
    let config_path = root.join(".rvsconfig.toml");
    let ignore_path = root.join(".rvsignore");

    // If .rvs already exists, try opening it
    if rvs_dir.exists() {
        match crate::core::repository::open::open_repository(&root) {
            Ok(repo) => {
                // it's already initialized
                return Err(ReviusError::AlreadyInitialized(repo.root.display().to_string()));
            }
            Err(ReviusError::Corrupt(msg)) => {
                return Err(ReviusError::Corrupt(format!("existing .rvs found but repo is corrupt: {}", msg)));
            }
            Err(_) => {
                // continue: maybe partially initialized; but safer to abort
                return Err(ReviusError::Corrupt("existing .rvs found but cannot open repository".into()));
            }
        }
    }

    // Create .rvs
    fs::create_dir_all(&rvs_dir)?;

    // Acquire lock (create .rvs/lock)
    let lock = RepoLock::acquire(lock_path.clone())?;

    // Open DB (this will create file if missing)
    let mut conn = connection::open(&db_path)?;

    // Apply schema (we keep schema.apply using Connection)
    schema::apply(&conn)?;

    // Insert initial Meta rows in a transaction
    {
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO Meta (key, value) VALUES (?1, ?2)",
            params!["schema_version", "1"],
        )?;
        tx.execute(
            "INSERT INTO Meta (key, value) VALUES (?1, ?2)",
            params!["repository_uuid", &Uuid::new_v4().to_string()],
        )?;
        tx.execute(
            "INSERT INTO Meta (key, value) VALUES (?1, ?2)",
            params!["HEAD", "ref: refs/heads/main"],
        )?;
        tx.commit()?;
    }

    // Canonicalize root for nice printing and defaults
    let canonical_root = fs::canonicalize(&root).unwrap_or(root.clone());

    // Write default .rvsconfig.toml if missing
    if !config_path.exists() {
        let defaults = fs_config::RepoConfigDefaults::for_root(&canonical_root);
        fs_config::write_default_repo_config(&config_path, &defaults)?;
    }

    // Write default .rvsignore if missing (do not overwrite)
    if !ignore_path.exists() {
        if let Ok(mut f) = OpenOptions::new().write(true).create_new(true).open(&ignore_path) {
            let _ = f.write_all(b".rvs/\n.rvsconfig.toml\n.rvsignore\n");
        }
    }

    // Load final runtime config
    let config = Config::load(&canonical_root)?;

    Ok(Repository {
        root: canonical_root,
        conn,
        lock,
        config,
    })
}
