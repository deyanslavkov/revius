use crate::core::models::config::Config;
use crate::fs::lock::LockFile;
use rusqlite::Connection;
use std::path::PathBuf;

pub struct Repository {
    pub root: PathBuf,
    pub config: Config,
    pub conn: Connection,
    pub _lock: LockFile, // The lock is held as long as this Repository struct is alive.
}

impl Repository {
    pub fn new(root: PathBuf, config: Config, conn: Connection, lock: LockFile) -> Self {
        Self {
            root,
            config,
            conn,
            _lock: lock,
        }
    }
}