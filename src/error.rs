use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReviusError {
    #[error("Repository already exists at {0}")]
    RepoAlreadyExists(PathBuf),

    #[error("Repository not found (no .rvs directory found in {0} or any parent)")]
    RepoNotFound(PathBuf),

    #[error("IO error at {0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),

    #[error("Database error: {0}")]
    Db(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Path error: {0}")]
    Path(String),

    #[error("Usage error: {0}")]
    Usage(String),

    #[error("Permission denied: {0}")]
    Permission(PathBuf),

    #[error("Operation cancelled by user")]
    Cancelled,
}

impl ReviusError {
    pub fn exit_code(&self) -> i32 {
        match self {
            ReviusError::Usage(_) => 2,
            ReviusError::Permission(_) => 126,
            ReviusError::Cancelled => 130,
            _ => 1,
        }
    }
}