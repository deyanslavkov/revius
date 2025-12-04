use crate::fs::lock::RepoLock;
use crate::core::config::Config;
use rusqlite::Connection;
use std::path::PathBuf;

/// Runtime repository handle: tiny and only contains state (no heavy logic here).
pub struct Repository {
    pub root: PathBuf,
    pub conn: Connection,
    pub lock: RepoLock,
    pub config: Config,
}
