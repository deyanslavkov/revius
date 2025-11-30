use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReviusError {
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database Error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("Configuration Error: {0}")]
    Config(String),

    #[error("Integrity Error: {0}")]
    Integrity(String),

    #[error("Repository Locked: {0}")]
    Lock(String),

    #[error("Zstd Compression Error: {0}")]
    Compression(String),

    #[error("Repository not initialized. Run 'revius init' first.")]
    RepoNotFound,

    #[error("General Error: {0}")]
    General(String),
}
