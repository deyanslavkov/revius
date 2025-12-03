use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReviusError {
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("DB Error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("Serialization Error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Config Error: {0}")]
    Config(String),

    #[error("Repository corrupted: {0}")]
    Corrupt(String),

    #[error("Repository Locked: {0}")]
    Lock(String),

    #[error("Repository already initialized: {0}")]
    AlreadyInitialized(String),

    #[error("Zstd Compression Error: {0}")]
    Compression(String),

    #[error("Repository not initialized. Run 'revius init' first.")]
    RepoNotFound,

    #[error("Invalid reference: {0}")]
    InvalidRef(String),

    #[error("General Error: {0}")]
    General(String),
}
