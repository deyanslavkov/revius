use crate::core::models::config::Config;
use rusqlite::Connection;
use std::path::PathBuf;

pub struct Repository {
    pub root: PathBuf,
    pub config: Config,
    pub conn: Connection,
}

impl Repository {
    pub fn new(root: PathBuf, config: Config, conn: Connection) -> Self {
        Self { root, config, conn }
    }
}