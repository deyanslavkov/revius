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
All shown functions, fields, and so on, are public and can be used as the architecture allows.
Some functions have docstrings above them, explaining some details that may be useful to know when using the function. The other functions are self-explanatory.

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
enum ReviusError {
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
    fn exit_code(&self) -> i32 {
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

### `commands/init.rs`

```rust
fn run(args: InitArgs) -> Result<(), ReviusError> {
    // (...)
    core::init::create_repository(&canonical_path)?;
    ui::print_init_success(&display_path);
    Ok(())
}
```

### `commands/add.rs`

```rust
fn run(args: AddArgs) -> Result<(), ReviusError> {
    // (...)
    let results = core::add::stage_files(&repo, file_paths)?;
    // (...)
    ui::print_add_summary(added_count + modified_count, unchanged_count, total_blobs);
    Ok(())
}
```

### `commands/commit.rs`

```rust
fn run(args: CommitArgs) -> Result<(), ReviusError> {
    // (...)
    let (commit_hash, files_changed) = core::commit::create_commit(&repo, &args.message)?;
    // (...)
    ui::print_commit_success(&commit_hash, &args.message, files_changed);
    Ok(())
}
```

## core

### `core/config.rs`

```rust
fn load_config(repo_root: &Path) -> Result<Config, ReviusError>
```

### `core/init.rs`

```rust
fn create_repository(path: &Path) -> Result<Repository, ReviusError>
```

### `core/open.rs`

```rust
fn open_repository(start_path: &Path) -> Result<Repository, ReviusError>
```

### `core/add.rs`

```rust
enum StageOutcome {Added { blobs: u64 }, Modified { blobs: u64 }, Unchanged}
fn stage_single_file(tx: &Transaction, repo: &Repository, path: &PathBuf) -> Result<(PathBuf, StageOutcome), ReviusError>
fn stage_files(repo: &Repository, paths: Vec<PathBuf>) -> Result<Vec<(PathBuf, StageOutcome)>, ReviusError>
```

### `core/content.rs`

```rust
fn read_and_hash_file(path: &Path) -> Result<(Vec<u8>, [u8; 32]), ReviusError>
/// True if new created, false if already exists
fn store_blob(tx: &Transaction, path: &Path, chunk: &[u8], chunk_hash: &[u8; 32], compression_enabled: bool, compression_level: u8) -> Result<bool, ReviusError>
/// Create file object in database (with chunking and compression). Returns the number of new blobs created
fn store_file_content(tx: &Transaction, path: &Path, file_hash: &[u8; 32], file_data: &[u8], repo: &Repository) -> Result<u64, ReviusError>
```

### `core/commit.rs`

```rust
fn create_commit(repo: &Repository, message: &str) -> Result<([u8; 32], usize), ReviusError>
/// Create and insert commit object (with hash)
fn create_commit_object(tx: &Transaction, tree_hash: &[u8; 32], parent_hash: Option<&[u8; 32]>, merge_parent_hash: Option<&[u8; 32]>, author_name: &str, author_email: &str, timestamp: i64, message: &str, author_id: i64) -> Result<[u8; 32], ReviusError>
```

### `core/tree.rs`

```rust
#[derive(Debug)]
enum TreeNode {
    Dir {
        children: BTreeMap<String, TreeNode>,
    },
    File {
        hash: [u8; 32],
        mode: u32,
    }
}
impl TreeNode {
    fn new_dir() -> Self {
        TreeNode::Dir {
            children: BTreeMap::new(),
        }
    }
    fn new_file(hash: [u8; 32], mode: u32) -> Self {
        TreeNode::File { hash, mode }
    }
}
/// Build in-memory tree structure from a list of files with paths
fn build_tree_from_files(files: Vec<StagedFile>) -> Result<TreeNode, ReviusError>
/// Recursively write tree entries to database and return parent_hash
fn write_tree_to_db(tx: &Transaction, node: &TreeNode) -> Result<[u8; 32], ReviusError>
```

### `core/refs.rs`

```rust
enum HeadState {
    Branch(String),  // e.g., "refs/heads/main"
    Detached([u8; 32]),  // commit hash
}
/// Update HEAD to point to a new commit. Handles both branch refs, detached HEAD, and initial commit case
fn update_head(tx: &Transaction, commit_hash: &[u8; 32]) -> Result<(), ReviusError>
fn get_head_state(conn: &Connection) -> Result<HeadState, ReviusError>
```

## core/models

### `core/models/config.rs`

```rust
/// The config struct, used in the Repository object
struct Config {
    compression: bool,
    compression_level: u8,
    chunking: bool,
    chunk_min: u32,
    chunk_avg: u32,
    chunk_max: u32,
    user_name: Option<String>,
    user_email: Option<String>,
}
```

### `core/models/objects.rs`

```rust
struct Blob {
    hash: [u8; 32],
    data: Vec<u8>,
    compression: String,
    uncompressed_size: u64,
}
struct File {
    hash: [u8; 32],
    size: u64,
    recipe_version: u32,
    chunk_count: u64,
    recipe: Vec<u8>,
}
struct StagedFile {
    path: String,
    file_hash: [u8; 32],
    mode: u32,
    size: u64,
}
struct Commit {
    hash: [u8; 32],
    parent_hash: Option<[u8; 32]>,
    merge_parent_hash: Option<[u8; 32]>,
    tree_hash: [u8; 32],
    message: String,
    author_id: i64,
    timestamp: i64,
}
struct TreeEntry {
    parent_hash: [u8; 32],
    name: String,
    object_hash: [u8; 32],
    mode: u32,
    is_dir: bool,
}
struct Author {
    id: i64,
    name: String,
    email: String,
}
```

### `core/models/repository.rs`

```rust
struct Repository {
    root: PathBuf,
    config: Config,
    conn: Connection,
}
impl Repository {
    fn new(root: PathBuf, config: Config, conn: Connection) -> Self
}
```

### `core/models/serialization.rs

```rust
fn serialize_tree_entry(mode: u32, name: &str, hash: &[u8; 32]) -> Vec<u8>
fn serialize_author(name: &str, email: &str, timestamp: i64) -> Result<String, String>
fn serialize_commit(tree_hash: &[u8; 32], parent_hash: Option<&[u8; 32]>, merge_parent_hash: Option<&[u8; 32]>, author_name: &str, author_email: &str, timestamp: i64, message: &str) -> Result<Vec<u8>, String>
```

## db

### `db/meta.rs`

```rust
fn get_schema_version(conn: &Connection) -> Result<i64, ReviusError>
fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>, ReviusError>
fn set_meta(tx: &Transaction, key: &str, value: &str) -> Result<(), ReviusError>
```

### `db/blobs.rs`

```rust
fn insert_blob(tx: &Transaction, hash: &[u8; 32], data: &[u8], compression: &str, uncompressed_size: u64) -> Result<(), ReviusError>
fn blob_exists(tx: &Transaction, hash: &[u8; 32]) -> Result<bool, ReviusError>
```

### `db/files.rs`

```rust
fn insert_file(tx: &Transaction, hash: &[u8; 32], recipe: &[u8], chunk_count: u64, size: u64) -> Result<(), ReviusError>
fn file_exists(tx: &Transaction, hash: &[u8; 32]) -> Result<bool, ReviusError>
```

### `db/staging.rs`

```rust
/// Returns StagedFile by repo-relative path
fn get_staged_file(tx: &Transaction, path: &str) -> Result<Option<StagedFile>, ReviusError>
fn upsert_staging(tx: &Transaction, path: &str, hash: &[u8; 32], mode: u32, size: u64, modified_at: i64) -> Result<(), ReviusError>
fn get_all_staged(conn: &Connection) -> Result<Vec<StagedFile>, ReviusError>
```

### `db/trees.rs`

```rust
fn tree_exists(conn: &Connection, parent_hash: &[u8; 32]) -> Result<bool, ReviusError>
fn insert_tree_entry(tx: &Transaction, parent_hash: &[u8; 32], name: &str, object_hash: &[u8; 32], mode: u32, is_dir: bool) -> Result<(), ReviusError>
/// Efficient batch insert by optimizing the query
fn batch_insert_tree_entries(tx: &Transaction, entries: Vec<TreeEntry>) -> Result<(), ReviusError>
```

### `db/commits.rs`

```rust
use crate::core::models::objects::Commit;
fn insert_commit(tx: &Transaction, hash: &[u8; 32], parent_hash: Option<&[u8; 32]>, merge_parent_hash: Option<&[u8; 32]>, tree_hash: &[u8; 32], message: &str, author_id: i64, timestamp: i64) -> Result<(), ReviusError>
fn get_commit(conn: &Connection, hash: &[u8; 32]) -> Result<Option<Commit>, ReviusError>
fn commit_exists(conn: &Connection, hash: &[u8; 32]) -> Result<bool, ReviusError>
```

### `db/authors.rs`

```rust
/// Get or create an author, returning their ID
fn get_or_create_author(tx: &Transaction, name: &str, email: &str) -> Result<i64, ReviusError>
```

### `db/refs.rs`

```rust
fn get_ref(conn: &Connection, path: &str) -> Result<Option<[u8; 32]>, ReviusError>
fn upsert_ref(tx: &Transaction, path: &str, ref_type: u8, commit_hash: &[u8; 32]) -> Result<(), ReviusError>
/// Update an existing ref - use when you know the ref exists (it doesn't take ref type as a parameter)
fn update_ref(tx: &Transaction, path: &str, commit_hash: &[u8; 32]) -> Result<(), ReviusError>
/// Resolve HEAD to a commit hash. Returns None if HEAD points to non-existent ref (initial commit case)
fn resolve_head(conn: &Connection) -> Result<Option<[u8; 32]>, ReviusError>
```

## fs

### `fs/config.rs`

```rust
/// You likely don't need to use those.
fn write_repo_config(path: &Path, config: &RepoConfig) -> Result<(), ReviusError>
fn write_user_config(path: &Path, config: &UserConfig) -> Result<(), ReviusError>
fn load_repo_config(repo_root: &Path) -> Result<RepoConfig, ReviusError>
fn load_user_config() -> Result<UserConfig, ReviusError>
```

### `fs/walk.rs`

```rust
/// Returns absolute paths to files using ignore::WalkBuilder
fn expand_paths(paths: Vec<PathBuf>, repo_root: &Path, ignore_path: &Path) -> Result<Vec<PathBuf>, ReviusError>
```

### `fs/io.rs`

```rust
fn create_dir(path: &Path) -> io::Result<()>
fn write_file(path: &Path, content: &str) -> io::Result<()>
fn write_binary(path: &Path, content: &[u8]) -> io::Result<()>
fn read_file(path: &Path) -> io::Result<Vec<u8>>
fn get_file_modified_time(path: &Path) -> io::Result<i64>
fn get_file_mode(path: &Path) -> io::Result<u32>
```

### `fs/paths.rs`

```rust
fn get_current_dir() -> Result<PathBuf, ReviusError>
fn get_rvs_dir(repo_root: &Path) -> PathBuf
fn get_repo_db_path(repo_root: &Path) -> PathBuf
fn get_repo_lock_path(repo_root: &Path) -> PathBuf
fn get_repo_config_path(repo_root: &Path) -> PathBuf
fn get_repo_ignore_path(repo_root: &Path) -> PathBuf
fn get_user_config_path() -> Option<PathBuf>
/// Canonicalize a path (resolve symlinks, make absolute...). Fails if path doesn't exist
fn canonicalize(path: &Path) -> io::Result<PathBuf>
/// Removes Windows UNC prefix
fn clean_path_display(path: &Path) -> PathBuf
fn find_repo_root(start: &Path) -> Result<PathBuf, ReviusError>
/// Also enforces UTF-8 encoding and forward slash separators
fn make_repo_relative(absolute_path: &Path, repo_root: &Path) -> Result<String, ReviusError>
fn split_path(path: &str) -> Vec<&str>
fn to_absolute(relative_path: &str, repo_root: &Path) -> PathBuf
```

## utils

### `utils/cdc.rs`

```rust
fn chunk_data(data: &[u8], min_size: u32, avg_size: u32, max_size: u32) -> Vec<&[u8]>
```

### `utils/compression.rs`

```rust
fn compress(data: &[u8], level: i32) -> io::Result<Vec<u8>>
fn decompress(data: &[u8]) -> Result<Vec<u8>, ReviusError>
```

### `utils/hash.rs`

```rust
fn hash_bytes(data: &[u8]) -> [u8; 32]
/// Takes a vector of bytes that's already a hash and enforces array size
fn vec_to_hash(vec: &[u8]) -> Result<[u8; 32], String>
fn hash_to_hex(hash: &[u8; 32]) -> String
/// Used for display in messages
fn hash_to_short_hex(hash: &[u8; 32]) -> String
```

### `utils/recipe.rs`

```rust
fn parse_recipe(recipe: &[u8]) -> Result<Vec<[u8; 32]>, String>
fn build_recipe(hashes: &[[u8; 32]]) -> Vec<u8>
```

### `utils/time.rs`

```rust
fn unix_timestamp() -> Result<i64, time::SystemTimeError>
```

## cli

### `cli/args.rs`

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "revius")]
#[command(about = "A content-addressed, single-file repository, lightweight VCS")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Initialize a new Revius repository")]
    Init(InitArgs),

    #[command(about = "Add file contents to the staging area")]
    Add(AddArgs),

    #[command(about = "Record changes to the repository")]
    Commit(CommitArgs),
}

#[derive(Parser)]
struct InitArgs {
    #[arg(default_value = ".", help = "Path where to initialize the repository")]
    path: PathBuf,
}

#[derive(Parser)]
struct AddArgs {
    #[arg(required = true, help = "Files or directories to add")]
    paths: Vec<PathBuf>,
}

#[derive(Parser)]
struct CommitArgs {
    #[arg(short, long, help = "Commit message")]
    message: String,
}
```

### `cli/ui.rs`

```rust
fn print_error(msg: &str)
fn print_no_user_configured()

fn print_init_success(path: &Path)

fn print_added_file(path: &str)
fn print_modified_file(path: &str)
fn print_add_summary(added: u64, skipped: u64, blobs: u64)

fn print_commit_success(hash: &[u8; 32], message: &str, files_changed: usize)
fn print_nothing_to_commit()
fn print_detached_head_warning(commit_hash: &[u8; 32])
```
