# Meta

This document defines the full module interface (the public API) of the current state of the project. It provides:
- A high-level description of all the current modules and files existing
- All of each file's:
    - Public functions with their signatures and any relevant info
    - Defined structs and enums with their fields and types
    - Some examples of code

It aims to provide all needed knowledge for anyone contributing to the system, without having to view all internal code.
It is updated manually and constantly as new things get implemented.
Mod files are excluded for brevity, as their contents are obvious. You don't have to provide any mod.rs updates, either, I will fix them manually.

For some files, only the exported things will be included. For others, the whole files will be presented, in order to give an example of the code, and to know precisely how to implement new things in it. Some will leave out irrelevant code for brevity.
Everything added here is ALREADY implemented, so you can use it as the architecture rules allow.
Feel free to update relevant files in db, fs, utils, cli, errors, or main with new things if needed, if it follows the architecture and makes the code more modular and concern-separated. Do not modify old things, only do the needed changes to add the new thing. If it's simply adding new things rather than modifying old ones, give only the added part.

## Project Root

### `main.rs`

```rust
use clap::Parser;
use revius::cli::args::{Cli, Commands};
use revius::cli::ui;
use revius::commands;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init(args) => commands::init::run(args),
        Commands::Add(args) => commands::add::run(args),
        Commands::Commit(args) => commands::commit::run(args),
    };

    if let Err(e) = result {
        ui::print_error(&e.to_string());
        std::process::exit(e.exit_code());
    }
}
```

### `error.rs`

```rust
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

impl From<rusqlite::Error> for ReviusError {
    fn from(err: rusqlite::Error) -> Self {
        ReviusError::Db(err.to_string())
    }
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
```

## commands

### `commands\init.rs`

```rust
pub fn run(args: InitArgs) -> Result<(), ReviusError> {
    // (...)
    core::init::create_repository(&canonical_path)?;
    ui::print_init_success(&display_path);
    Ok(())
}
```

### `commands\add.rs`

```rust
pub fn run(args: AddArgs) -> Result<(), ReviusError> {
    // (...)
    let results = core::add::stage_files(&repo, file_paths)?;
    // (...)
    ui::print_add_summary(added_count + modified_count, unchanged_count, total_blobs);
    Ok(())
}
```

### `commands\commit.rs`

```rust
pub fn run(args: CommitArgs) -> Result<(), ReviusError> {
    // (...)
    let (commit_hash, files_changed) = core::commit::create_commit(&repo, &args.message)?;
    // (...)
    ui::print_commit_success(&commit_hash, &args.message, files_changed);
    Ok(())
}
```

## core

### `core\config.rs`

```rust
pub fn load_config(repo_root: &Path) -> Result<Config, ReviusError>
```

### `core\init.rs`

```rust
pub fn create_repository(path: &Path) -> Result<Repository, ReviusError>
```

### `core\open.rs`

```rust
pub fn open_repository(start_path: &Path) -> Result<Repository, ReviusError>
```

### `core\add.rs`

```rust
pub enum StageOutcome {Added { blobs: u64 }, Modified { blobs: u64 }, Unchanged}
pub fn stage_single_file(tx: &Transaction, repo: &Repository, path: &PathBuf) -> Result<(PathBuf, StageOutcome), ReviusError>
pub fn stage_files(repo: &Repository, paths: Vec<PathBuf>) -> Result<Vec<(PathBuf, StageOutcome)>, ReviusError>
```

### `core\content.rs`

```rust
/// Read a file from disk and compute its hash
pub fn read_and_hash_file(path: &Path) -> Result<(Vec<u8>, [u8; 32]), ReviusError>
/// Store a single blob in the database with optional compression. Returns true if a new blob was created, false if it already existed
pub fn store_blob(tx: &Transaction, path: &Path, chunk: &[u8], chunk_hash: &[u8; 32], compression_enabled: bool, compression_level: u8) -> Result<bool, ReviusError>
/// Create file object in database (with chunking and compression). Returns the number of new blobs created
pub fn store_file_content(tx: &Transaction, path: &Path, file_hash: &[u8; 32], file_data: &[u8], repo: &Repository) -> Result<u64, ReviusError>
```

### `core\commit.rs`

```rust
/// Main entry point for creating a commit
pub fn create_commit(repo: &Repository, message: &str) -> Result<([u8; 32], usize), ReviusError>
/// Create and insert commit object with hash
pub fn create_commit_object(tx: &Transaction, tree_hash: &[u8; 32], parent_hash: Option<&[u8; 32]>, merge_parent_hash: Option<&[u8; 32]>, author_name: &str, author_email: &str, timestamp: i64, message: &str, author_id: i64) -> Result<[u8; 32], ReviusError>
```

### `core\tree.rs`

```rust
#[derive(Debug)]
pub enum TreeNode {
    Dir {
        children: BTreeMap<String, TreeNode>,
    },
    File {
        hash: [u8; 32],
        mode: u32,
    }
}
impl TreeNode {
    pub fn new_dir() -> Self {
        TreeNode::Dir {
            children: BTreeMap::new(),
        }
    }
    pub fn new_file(hash: [u8; 32], mode: u32) -> Self {
        TreeNode::File { hash, mode }
    }
}
/// Build in-memory tree structure from a list of files with paths
pub fn build_tree_from_files(files: Vec<StagedFile>) -> Result<TreeNode, ReviusError>
/// Recursively write tree entries to database and return parent_hash
pub fn write_tree_to_db(tx: &Transaction, node: &TreeNode) -> Result<[u8; 32], ReviusError>
```

### `core\refs.rs`

```rust
/// Update HEAD to point to a new commit. Handles both branch refs and detached HEAD
pub fn update_head(tx: &Transaction, commit_hash: &[u8; 32]) -> Result<(), ReviusError>
```

## core\models

### `core\models\config.rs`

```rust
pub struct Config { // The config struct, used in the Repository object
    pub compression: bool,
    pub compression_level: u8,
    pub chunking: bool,
    pub chunk_min: u64,
    pub chunk_avg: u64,
    pub chunk_max: u64,
    pub case_sensitive: bool,
    pub user_name: Option<String>,
    pub user_email: Option<String>,
}
```

### `core\models\objects.rs`

```rust
pub struct Blob {
    pub hash: [u8; 32],
    pub data: Vec<u8>,
    pub compression: String,
    pub uncompressed_size: u64,
}
pub struct File {
    pub hash: [u8; 32],
    pub size: u64,
    pub recipe_version: u32,
    pub chunk_count: u64,
    pub recipe: Vec<u8>,
}
pub struct StagedFile {
    pub path: String,
    pub file_hash: [u8; 32],
    pub mode: u32,
    pub size: u64,
}
pub struct Commit {
    pub hash: [u8; 32],
    pub parent_hash: Option<[u8; 32]>,
    pub merge_parent_hash: Option<[u8; 32]>,
    pub tree_hash: [u8; 32],
    pub message: String,
    pub author_id: i64,
    pub timestamp: i64,
}
pub struct TreeEntry {
    pub parent_hash: [u8; 32],
    pub name: String,
    pub object_hash: [u8; 32],
    pub mode: u32,
    pub is_dir: bool,
}
pub struct Author {
    pub id: i64,
    pub name: String,
    pub email: String,
}
```

### `core\models\repository.rs`

```rust
pub struct Repository {
    pub root: PathBuf,
    pub config: Config,
    pub conn: Connection,
}
impl Repository {
    pub fn new(root: PathBuf, config: Config, conn: Connection) -> Self
}
```

### `core\models\serialization.rs

```rust
pub fn serialize_tree_entry(mode: u32, name: &str, hash: &[u8; 32]) -> Vec<u8>
pub fn serialize_commit(tree_hash: &[u8; 32], parent_hash: Option<&[u8; 32]>, merge_parent_hash: Option<&[u8; 32]>, author_name: &str, author_email: &str, timestamp: i64, message: &str) -> Vec<u8>
```

## db

### `db\meta.rs`

```rust
pub fn get_schema_version(conn: &Connection) -> Result<i64, ReviusError>
pub fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>, ReviusError>
pub fn set_meta(tx: &Transaction, key: &str, value: &str) -> Result<(), ReviusError>
```

### `db\blobs.rs`

```rust
pub fn insert_blob(tx: &Transaction, hash: &[u8; 32], data: &[u8], compression: &str, uncompressed_size: u64) -> Result<(), ReviusError>
pub fn blob_exists(tx: &Transaction, hash: &[u8; 32]) -> Result<bool, ReviusError>
```

### `db\files.rs`

```rust
pub fn insert_file(tx: &Transaction, hash: &[u8; 32], recipe: &[u8], chunk_count: u64, size: u64) -> Result<(), ReviusError>
pub fn file_exists(tx: &Transaction, hash: &[u8; 32]) -> Result<bool, ReviusError>
```

### `db\staging.rs`

```rust
pub fn get_staged_file(tx: &Transaction, path: &str) -> Result<Option<StagedFile>, ReviusError>
pub fn upsert_staging(tx: &Transaction, path: &str, hash: &[u8; 32], mode: u32, size: u64, modified_at: i64) -> Result<(), ReviusError>
pub fn get_all_staged(conn: &Connection) -> Result<Vec<StagedFile>, ReviusError>
```

### `db\trees.rs`

```rust
pub fn tree_exists(conn: &Connection, parent_hash: &[u8; 32]) -> Result<bool, ReviusError>
pub fn insert_tree_entry(tx: &Transaction, parent_hash: &[u8; 32], name: &str, object_hash: &[u8; 32], mode: u32, is_dir: bool) -> Result<(), ReviusError>
pub fn batch_insert_tree_entries(tx: &Transaction, entries: Vec<TreeEntry>) -> Result<(), ReviusError> // For efficiency
```

### `db\commits.rs`

```rust
use crate::core::models::objects::Commit;
pub fn insert_commit(tx: &Transaction, hash: &[u8; 32], parent_hash: Option<&[u8; 32]>, merge_parent_hash: Option<&[u8; 32]>, tree_hash: &[u8; 32], message: &str, author_id: i64, timestamp: i64) -> Result<(), ReviusError>
pub fn get_commit(conn: &Connection, hash: &[u8; 32]) -> Result<Option<Commit>, ReviusError>
pub fn commit_exists(conn: &Connection, hash: &[u8; 32]) -> Result<bool, ReviusError>
```

### `db\authors.rs`

```rust
/// Get or create an author, returning their ID
pub fn get_or_create_author(tx: &Transaction, name: &str, email: &str) -> Result<i64, ReviusError>
```

### `db\refs.rs`

```rust
/// Update HEAD to point to a new commit
/// Handles both branch refs and detached HEAD
pub fn update_head(tx: &Transaction, commit_hash: &[u8; 32]) -> Result<(), ReviusError>
```

## fs

### `fs\config.rs`

```rust
pub fn write_repo_config(path: &Path, config: &RepoConfig) -> Result<(), ReviusError>
pub fn write_user_config(path: &Path, config: &UserConfig) -> Result<(), ReviusError>
pub fn load_repo_config(repo_root: &Path) -> Result<RepoConfig, ReviusError>
pub fn load_user_config() -> Result<UserConfig, ReviusError>
```

### `fs\ignore.rs`

```rust
pub fn expand_paths(paths: Vec<PathBuf>, repo_root: &Path, ignore_path: &Path) -> Result<Vec<PathBuf>, ReviusError> // Returns all files as paths in a vector
```

### `fs\io.rs`

```rust
pub fn canonicalize(path: &Path) -> io::Result<PathBuf>
pub fn clean_path_display(path: &Path) -> PathBuf // Removes Windows prefix
pub fn create_dir(path: &Path) -> io::Result<()>
pub fn write_file(path: &Path, content: &str) -> io::Result<()>
pub fn write_binary(path: &Path, content: &[u8]) -> io::Result<()>
pub fn read_file(path: &Path) -> io::Result<Vec<u8>>
pub fn get_file_modified_time(path: &Path) -> io::Result<i64>
pub fn get_file_mode(path: &Path) -> io::Result<u32>
```

### `fs\paths.rs`

```rust
pub fn get_rvs_dir(repo_root: &Path) -> PathBuf
pub fn get_repo_db_path(repo_root: &Path) -> PathBuf
pub fn get_repo_lock_path(repo_root: &Path) -> PathBuf
pub fn get_repo_config_path(repo_root: &Path) -> PathBuf
pub fn get_repo_ignore_path(repo_root: &Path) -> PathBuf
pub fn get_user_config_path() -> Option<PathBuf>
pub fn find_repo_root(start: &Path) -> Result<PathBuf, ReviusError>
pub fn make_repo_relative(absolute_path: &Path, repo_root: &Path) -> Result<String, ReviusError>
pub fn split_path(path: &str) -> Vec<&str>
```

## utils

### `utils\cdc.rs`

```rust
pub fn chunk_data(data: &[u8], min_size: u64, avg_size: u64, max_size: u64) -> Vec<&[u8]>
```

### `utils\compression.rs`

```rust
pub fn compress(data: &[u8], level: i32) -> io::Result<Vec<u8>>
pub fn decompress(data: &[u8]) -> io::Result<Vec<u8>>
```

### `utils\hash.rs`

```rust
pub fn hash_bytes(data: &[u8]) -> [u8; 32]
pub fn vec_to_hash(vec: &[u8]) -> Result<[u8; 32], String>
```

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

    #[command(about = "Add file contents to the staging area")]
    Add(AddArgs),

    #[command(about = "Record changes to the repository")]
    Commit(CommitArgs),
}

#[derive(Parser)]
pub struct InitArgs {
    #[arg(default_value = ".", help = "Path where to initialize the repository")]
    pub path: PathBuf,
}

#[derive(Parser)]
pub struct AddArgs {
    #[arg(required = true, help = "Files or directories to add")]
    pub paths: Vec<PathBuf>,
}

#[derive(Parser)]
pub struct CommitArgs {
    #[arg(short, long, help = "Commit message")]
    pub message: String,
}
```

### `cli\ui.rs`

```rust
pub fn print_error(msg: &str)
pub fn print_no_user_configured()

pub fn print_init_success(path: &Path)

pub fn print_added_file(path: &str)
pub fn print_modified_file(path: &str)
pub fn print_add_summary(added: u64, skipped: u64, blobs: u64)

pub fn print_commit_success(hash: &[u8; 32], message: &str, files_changed: usize)
pub fn print_nothing_to_commit()
```
