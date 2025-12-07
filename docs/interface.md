## Meta

This document defines the full module interface (the public API) of the current state of the project. It provides:
- A high-level description of all the current modules and files existing
- All of each file's:
    - Public functions with their signatures and any relevant info
    - Defined structs and enums with their fields and types
    - All DB tables with their column names and types (the schema)

It aims to provide all needed knowledge for anyone contributing to the system, without having to view all internal code.
It is updated manually and constantly as new things get implemented.

For some files, only the exported things will be included. For others, the whole files will be presented, in order to give an example of the code.
Everything added here is ALREADY implemented, so you can use it as the architecture rules allow.

## Project Root

### `error.rs`

```rust
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReviusError {
    #[error("Repository already exists at {0}")]
    RepoAlreadyExists(PathBuf),

    #[error("IO error at {0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),

    #[error("Database error: {0}")]
    Db(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

impl From<rusqlite::Error> for ReviusError {
    fn from(err: rusqlite::Error) -> Self {
        ReviusError::Db(err.to_string())
    }
}
```

### `main.rs`

```rust
use clap::Parser;
use revius::cli::args::{Cli, Commands};
use revius::cli::ui;
use revius::commands;
use revius::error::ReviusError;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init(args) => commands::init::run(args),
    };

    if let Err(e) = result {
        match e {
            ReviusError::RepoAlreadyExists(path) => {
                // Print the friendly repo-exists message and exit non-zero
                ui::print_repo_already_exists(&path);
                std::process::exit(1);
            }
            other => {
                ui::print_error(&other.to_string());
                std::process::exit(1);
            }
        }
    }
}
```


---


## cli

### `cli\args.rs`

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "revius")]
#[command(about = "A content-addressed, single-file repository, lightweight VCS")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Initialize a new Revius repository")]
    Init(InitArgs),
}

#[derive(Parser)]
pub struct InitArgs {
    #[arg(default_value = ".", help = "Path where to initialize the repository")]
    pub path: PathBuf,
}
```

### `cli\mod.rs`

```rust
pub mod args;
pub mod ui;
```

### `cli\ui.rs`

```rust
use colored::Colorize;
use std::path::Path;

pub fn print_init_success(path: &Path) {
    println!(
        "{} Initialized empty Revius repository in {}",
        "✓".green().bold(),
        path.display()
    );
}

pub fn print_repo_already_exists(path: &Path) {
    eprintln!(
        "{} Repository already exists at {}",
        "✗".red().bold(),
        path.display()
    );
}

pub fn print_error(msg: &str) {
    eprintln!("{} {}", "Error:".red().bold(), msg);
}
```


---


## commands

### `commands\init.rs`

```rust
use crate::cli::args::InitArgs;
use crate::cli::ui;
use crate::core;
use crate::error::ReviusError;
use crate::fs;

pub fn run(args: InitArgs) -> Result<(), ReviusError> {
    let canonical_path = fs::io::canonicalize(&args.path)
        .map_err(|e| ReviusError::Io(args.path.clone(), e))?;
    let display_path = fs::io::clean_path_display(&canonical_path);
    
    core::init::create_repository(&canonical_path)?;

    ui::print_init_success(&display_path);
    Ok(())
}
```

### `commands\mod.rs`

```rust
pub mod init;
```


---


## core

### `core\config.rs`

```rust
pub fn load_config(repo_root: &Path) -> Result<Config, ReviusError>
```

### `core\init.rs`

```rust
use crate::core::config;
use crate::core::models::repository::Repository;
use crate::db;
use crate::error::ReviusError;
use crate::fs;
use crate::fs::paths;
use std::path::Path;

pub fn create_repository(path: &Path) -> Result<Repository, ReviusError> {
    let rvs_dir = paths::get_rvs_dir(path);
    let db_path = paths::get_repo_db_path(path);
    let lock_path = paths::get_repo_lock_path(path);
    let config_path = paths::get_repo_config_path(path);
    let ignore_path = paths::get_repo_ignore_path(path);
    
    if rvs_dir.exists() {
        return Err(ReviusError::RepoAlreadyExists(path.to_path_buf()));
    }

    fs::io::create_dir(&rvs_dir)
        .map_err(|e| ReviusError::Io(rvs_dir.clone(), e))?;

    let conn = db::connection::open_db(&db_path)?;
    db::schema::create_all(&conn)?;

    fs::lock::init_lockfile(&lock_path)
        .map_err(|e| ReviusError::Io(lock_path.clone(), e))?;

    let repo_config = config::load_default_repo_config();
    fs::config::write_repo_config(&config_path, &repo_config)?;

    fs::io::write_file(&ignore_path, "")
        .map_err(|e| ReviusError::Io(ignore_path.clone(), e))?;

    let user_config = config::load_user_config();
    let merged_config = config::merge(repo_config, user_config)?;

    Ok(Repository::new(path.to_path_buf(), merged_config, conn))
}
```

### `core\mod.rs`

```rust
pub mod config;
pub mod init;
pub mod models;
```

### `core\open.rs`

```rust

```


---


## db

### `db\connection.rs`

```rust
pub fn open_db(path: &Path) -> Result<Connection, ReviusError>
```

### `db\mod.rs`

```rust
pub mod connection;
pub mod schema;
```

### `db\schema.rs`

```rust
pub fn create_all(conn: &Connection) -> Result<(), ReviusError>
```


---


## fs

### `fs\config.rs`

```rust
pub fn write_repo_config(path: &Path, config: &RepoConfig) -> Result<(), ReviusError>
pub fn write_user_config(path: &Path, config: &UserConfig) -> Result<(), ReviusError>
pub fn load_repo_config(repo_root: &Path) -> Result<RepoConfig, ReviusError>
pub fn load_user_config() -> Result<UserConfig, ReviusError>
```

### `fs\io.rs`

```rust
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
pub fn canonicalize(path: &Path) -> io::Result<PathBuf>
pub fn clean_path_display(path: &Path) -> PathBuf
pub fn create_dir(path: &Path) -> io::Result<()>
pub fn write_file(path: &Path, content: &str) -> io::Result<()>
pub fn write_binary(path: &Path, content: &[u8]) -> io::Result<()>
```

### `fs\mod.rs`

```rust
pub mod config;
pub mod io;
pub mod lock;
pub mod paths;
```

### `fs\paths.rs`

Exports:

```rust
pub fn get_rvs_dir(repo_root: &Path) -> PathBuf
pub fn get_repo_db_path(repo_root: &Path) -> PathBuf
pub fn get_repo_lock_path(repo_root: &Path) -> PathBuf
pub fn get_repo_config_path(repo_root: &Path) -> PathBuf
pub fn get_repo_ignore_path(repo_root: &Path) -> PathBuf
pub fn get_user_config_path() -> Option<PathBuf>
```

---


### core\models

### `core\models\config.rs`

Exported struct:

```rust
#[derive(Debug, Clone)]
pub struct Config {
    pub compression: bool,
    pub compression_level: u8,
    pub chunking: bool,
    pub chunk_min: usize,
    pub chunk_avg: usize,
    pub chunk_max: usize,
    pub case_sensitive: bool,
    pub user_name: Option<String>,
    pub user_email: Option<String>,
}
```

### `core\models\mod.rs`

```rust
pub mod config;
pub mod repository;
pub mod objects;
pub mod serialization;
```

### `core\models\repository.rs`

```rust
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
```

---
