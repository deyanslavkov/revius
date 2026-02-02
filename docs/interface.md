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
        Commands::Status(args) => commands::status::run(args),
        Commands::Log(args) => commands::log::run(args),
        Commands::Branch(args) => commands::branch::run(args),
        Commands::Switch(args) => commands::switch::run(args),
        Commands::Merge(args) => commands::merge::run(args),
        Commands::Reset(args) => commands::reset::run(args),
        Commands::Restore(args) => commands::restore::run(args),
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

    #[error("Branch already exists: {0}")]
    BranchAlreadyExists(String),

    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    #[error("Invalid branch name: {0}")]
    InvalidBranchName(String),

    #[error("Cannot delete current branch: {0}")]
    CannotDeleteCurrentBranch(String),

    #[error("Not on any branch (detached HEAD at {0})")]
    DetachedHead(String),

    #[error("Cannot perform operation: no commits yet")]
    NoCommitsYet,

    #[error("Target not found: {0}")]
    TargetNotFound(String),

    #[error("Cannot switch: you have uncommitted changes. Use -f to force")]
    UncommittedChanges,

    #[error("Commit not found: {0}")]
    CommitNotFound(String),

    #[error("Ambiguous hash prefix '{0}': matches multiple commits. Please use a longer prefix.")]
    AmbiguousHashPrefix(String),

    #[error("Invalid hash prefix '{0}': must be 1-64 hex characters")]
    InvalidHashPrefix(String),

    #[error("Merge error: {0}")]
    MergeError(String),
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
    let results = core::add::stage_files(&repo, found_files, canonical_paths)?;
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

### `commands/status.rs`

```rust
fn run(_args: StatusArgs) -> Result<(), ReviusError> {
    let current_dir = fs::paths::get_current_dir()?;
    let repo = core::open::open_repository(&current_dir)?;
    let status_info = core::status::get_status_info(&repo)?;
    ui::print_status(&status_info);
    Ok(())
}
```

### `commands/log.rs`

```rust
fn run(args: LogArgs) -> Result<(), ReviusError> {
    // (...)
    let options = LogOptions { /*(...)*/ };
    let commits = core::log::get_commit_history(&repo.conn, &options)?;
    if commits.is_empty() {
        ui::print_no_commits();
        return Ok(());
    }
    ui::print_log(&commits, &options);
    Ok(())
}
```

### `commands/branch.rs`

```rust
fn run(args: BranchArgs) -> Result<(), ReviusError>
fn handle_create(repo: &Repository, args: BranchArgs) -> Result<(), ReviusError>
fn handle_list(repo: &Repository) -> Result<(), ReviusError>
fn handle_rename(repo: &Repository, args: BranchArgs) -> Result<(), ReviusError>
fn handle_delete(repo: &Repository, args: BranchArgs, force: bool) -> Result<(), ReviusError>
```

### `commands/switch.rs`

```rust
fn run(args: SwitchArgs) -> Result<(), ReviusError>
```

### `commands/merge.rs`

```rust
fn run(args: MergeArgs) -> Result<(), ReviusError> {
    let current_dir = fs::paths::get_current_dir()?;
    let repo = core::open::open_repository(&current_dir)?;
    let (_, target_commit) = resolve_target(&repo.conn, &args.target)?;
    match core::merge::perform_merge(&repo, target_commit)? {
        // (...)
    }
    Ok(())
}
```

### `commands/reset.rs`

```rust
fn run(args: ResetArgs) -> Result<(), ReviusError> {
    let current_dir = fs::paths::get_current_dir()?;
    let repo = open::open_repository(&current_dir)?;
    let target = args.target.as_deref().unwrap_or("HEAD");
    let mode_count = (args.soft as u8) + (args.mixed as u8) + (args.hard as u8);
    if mode_count > 1 {
        return Err(ReviusError::Usage("Cannot specify multiple reset modes (--soft, --mixed, --hard) at once".to_string()));
    }
    let final_hash = if args.hard {
        core::reset::reset_hard(&repo, target)?
    } else if args.soft {
        core::reset::reset_soft(&repo, target)?
    } else {
        core::reset::reset_mixed(&repo, target)?
    };
    let mode_str = if args.hard { "hard" } else if args.soft { "soft" } else { "mixed" };
    ui::print_reset_success(mode_str, &final_hash);
    Ok(())
}
```

### `commands/restore.rs`

```rust
fn run(args: RestoreArgs) -> Result<(), ReviusError> {
    let current_dir = fs::paths::get_current_dir()?;
    let repo = open::open_repository(&current_dir)?;
    let source = args.source.as_deref().unwrap_or("HEAD");
    let worktree = args.worktree || (!args.staged && !args.worktree);
    let staged = args.staged;
    let count = if staged && worktree {
        core::restore::restore_mixed(&repo, &args.paths, source)?
    } else if staged {
        core::restore::restore_staged(&repo, &args.paths, source)?
    } else {
        core::restore::restore_worktree(&repo, &args.paths)?
    };
    let mode_str = if staged && worktree { "mixed" } else if staged { "staged" } else { "worktree" };
    ui::print_restore_success(mode_str, count);
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
/// Finds repo root, opens DB connection, loads config, and checks schema version
fn open_repository(start_path: &Path) -> Result<Repository, ReviusError>
```

### `core/add.rs`

```rust
enum StageOutcome {Added { blobs: u64 }, Modified { blobs: u64 }, Deleted, Unchanged}
fn stage_single_file(tx: &Transaction, repo: &Repository, path: &PathBuf) -> Result<(PathBuf, StageOutcome), ReviusError>
fn stage_files(repo: &Repository, found_files: Vec<PathBuf>, search_scopes: Vec<PathBuf>) -> Result<Vec<(PathBuf, StageOutcome)>, ReviusError>
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
/// Returns a map of repo-relative path -> file_hash for all files in the tree
fn get_all_tree_files(conn: &Connection, tree_hash: &[u8; 32]) -> Result<BTreeMap<String, [u8; 32]>, ReviusError>
/// Get the complete file tree for a commit as a flat map: path -> (file_hash, mode). Returns None for file_hash if the entry is a directory
fn get_tree_snapshot(conn: &Connection, tree_hash: [u8; 32]) -> Result<BTreeMap<String, (Option<[u8; 32]>, u32)>, ReviusError>
/// Get all file entries from a tree (recursively) for staging reconstruction. Returns Vec<(relative_path, file_hash, mode, size)>
pub fn get_all_files_in_tree(conn: &Connection, tree_hash: &[u8; 32]) -> Result<Vec<(String, [u8; 32], u32, u64)>, ReviusError>
/// Generic recursive tree walker. Visits every node and calls `callback`. Recurses automatically for directories.
fn walk_tree<F>(conn: &Connection, parent_hash: &[u8; 32], path_prefix: &str, callback: &mut F) -> Result<(), ReviusError> where F: FnMut(&str, &TreeEntry) -> Result<(), ReviusError>
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
fn update_head_to_branch(tx: &Transaction, branch_name: &str) -> Result<(), ReviusError>
```

### `core/status.rs`

```rust
/// Get comprehensive status information by comparing HEAD, staging area, and working directory
fn get_status_info(repo: &Repository) -> Result<StatusInfo, ReviusError>
/// Get all files from HEAD commit with their hashes
fn get_head_files(conn: &rusqlite::Connection) -> Result<BTreeMap<String, [u8; 32]>, ReviusError>
/// Get all staged files with their hashes
fn get_staged_files(conn: &rusqlite::Connection) -> Result<BTreeMap<String, [u8; 32]>, ReviusError>
/// Get all working directory files with their hashes
fn get_workdir_files(repo: &Repository) -> Result<BTreeMap<String, [u8; 32]>, ReviusError>
```

### `core/log.rs`

```rust
/// Get commit history starting from HEAD, traversing the parent chain
fn get_commit_history(conn: &Connection, options: &LogOptions) -> Result<Vec<CommitInfo>, ReviusError>
```

### `core/branch.rs`

```rust
fn branch_ref_path(branch_name: &str) -> String
fn extract_branch_name(ref_path: &str) -> Result<String, ReviusError>
/// Create a branch within an existing transaction. This is used by switch -c to create and switch in one transaction
fn create_branch_in_tx(tx: &Transaction, branch_name: &str, commit_hash: &[u8; 32]) -> Result<(), ReviusError>
/// Create a new branch at the current commit. Returns the commit hash where the branch was created
fn create_branch(repo: &Repository, branch_name: &str) -> Result<[u8; 32], ReviusError>
/// Rename a branch. If old_name is None, renames the current branch. Returns (old_ref_path, new_ref_path)
fn rename_branch(repo: &Repository, old_name: Option<&str>, new_name: &str) -> Result<(String, String), ReviusError>
/// Delete a branch with safety checks (can't delete current, can't delete if unmerged). Returns the commit hash where the branch pointed
fn delete_branch(repo: &Repository, branch_name: &str, _force: bool) -> Result<[u8; 32], ReviusError>
/// List all branches with their commit hashes. Returns Vec<(branch_name, commit_hash, is_current)>
fn list_branches(repo: &Repository) -> Result<Vec<(String, [u8; 32], bool)>, ReviusError>
/// Get the current branch name (if on a branch). Returns None if in detached HEAD
fn get_current_branch_name(repo: &Repository) -> Result<Option<String>, ReviusError>
```

### `core/checkout.rs`

```rust
/// Reconstruct file content from database (Files + Blobs + recipe)
fn reconstruct_file(conn: &Connection, file_hash: &[u8; 32]) -> Result<Vec<u8>, ReviusError>
/// Write reconstructed content to working directory
fn checkout_file(conn: &Connection, file_hash: &[u8; 32], target_path: &Path, mode: u32) -> Result<(), ReviusError>
```

### `core/switch.rs`

```rust
/// Switch to a branch or commit. Updates HEAD, staging, and working directory.
/// Returns previous and new HEAD states with file change counts.
/// If workspace update fails after DB commit, DB remains consistent but workdir may be partial.
fn switch_to_target(repo: &Repository, target: &str, create: bool, force: bool) -> Result<SwitchResult, ReviusError>
/// Create a new branch at current commit and switch to it atomically.
/// Used by `switch -c`. No workspace changes since staying on same commit.
fn handle_create_and_switch(repo: &Repository, branch_name: &str) -> Result<SwitchResult, ReviusError>
/// Get current HEAD state (branch or detached) and the commit it points to.
/// Returns (HeadState, Option<commit_hash>). Option is None only if no commits exist yet.
fn get_current_head_state(conn: &Connection) -> Result<(HeadState, Option<[u8; 32]>), ReviusError>
/// Resolve target string to branch or commit. Tries in order: branch name, full hash (64 chars), hash prefix (1-63 chars).
/// Returns (TargetType, commit_hash) or error if ambiguous/not found.
fn resolve_target(conn: &Connection, target: &str) -> Result<(TargetType, [u8; 32]), ReviusError>
/// Check if working directory differs from staging area.
/// Returns true if there are modifications, additions, or deletions in workdir vs staging.
fn check_uncommitted_changes(repo: &Repository) -> Result<bool, ReviusError>
/// Build plan for switching from current_tree to target_tree.
/// Returns lists of files to add, modify, and delete. Current tree is None for initial commit case.
fn build_switch_plan(conn: &Connection, current_tree: Option<[u8; 32]>, target_tree: [u8; 32]) -> Result<SwitchPlan, ReviusError>
/// Execute workspace changes: delete files, then add/modify files by reconstructing from DB.
/// Returns (files_changed_count, files_deleted_count).
fn apply_workspace_changes(repo: &Repository, plan: &SwitchPlan) -> Result<(usize, usize), ReviusError>
/// Rebuild staging area from tree. Clears existing staging and populates with all files in tree.
fn update_staging_from_tree(tx: &rusqlite::Transaction, tree_hash: [u8; 32]) -> Result<(), ReviusError>
/// Format target type and name for user-facing messages.
/// Returns "branch 'name'" or "commit 'hash'".
fn format_target_name(target_type: &TargetType, target: &str) -> String
```

### `core/reset.rs`

```rust
/// Resets HEAD to the target commit. Does not touch staging or working directory.
pub fn reset_soft(repo: &Repository, target: &str) -> Result<[u8; 32], ReviusError>
/// Resets HEAD to the target commit and updates staging to match. Working directory is left unchanged.
pub fn reset_mixed(repo: &Repository, target: &str) -> Result<[u8; 32], ReviusError>
/// Resets HEAD, staging, and working directory to the target commit. Destructive operation.
pub fn reset_hard(repo: &Repository, target: &str) -> Result<[u8; 32], ReviusError>
/// Moves the HEAD or branch pointer. Adds a reflog entry.
fn move_head(tx: &Transaction, target_hash: [u8; 32], mode_str: &str) -> Result<(), ReviusError>
```

### `core/restore.rs`

```rust
/// Restore working tree from Staging area. Only modifies files that exist in Staging and match the path patterns. Does not delete files from working tree (matches git restore --worktree behavior regarding untracked files).
pub fn restore_worktree(repo: &Repository, paths: &[PathBuf]) -> Result<usize, ReviusError>
/// Restore Staging area from a Source Commit (HEAD by default). Updates Staging to match the Source for the given paths. Adds, Updates, and Removes entries in Staging.
pub fn restore_staged(repo: &Repository, paths: &[PathBuf], source: &str) -> Result<usize, ReviusError>
/// Restore both Staging and Worktree from a Source Commit. Adds/Updates/Deletes in both Staging and Disk.
pub fn restore_mixed(repo: &Repository, paths: &[PathBuf], source: &str) -> Result<usize, ReviusError>
fn normalize_patterns(repo: &Repository, paths: &[PathBuf]) -> Result<Vec<String>, ReviusError>
fn matches_any_pattern(path: &str, patterns: &[String]) -> bool
fn get_source_files(conn: &Transaction, source: &str) -> Result<Vec<(String, [u8; 32], u32, u64)>, ReviusError>
```

### `core/merge.rs`

```rust
/// Perform a merge of target_commit into current HEAD
fn perform_merge(repo: &Repository, target_commit: [u8; 32]) -> Result<MergeResult, ReviusError>
/// Perform a fast-forward merge by updating HEAD to target
fn perform_fast_forward(repo: &Repository, from: [u8; 32], to: [u8; 32]) -> Result<MergeResult, ReviusError>
/// Perform a three-way merge creating a merge commit
fn perform_three_way_merge(repo: &Repository, our_commit: [u8; 32], their_commit: [u8; 32], base_commit: [u8; 32]) -> Result<MergeResult, ReviusError>
/// Three-way merge algorithm. Returns Ok(merged_files) or Err(conflicts)
fn three_way_merge(
    base_tree: &BTreeMap<String, (Option<[u8; 32]>, u32)>,
    our_tree: &BTreeMap<String, (Option<[u8; 32]>, u32)>,
    their_tree: &BTreeMap<String, (Option<[u8; 32]>, u32)>,
    ) -> Result<Vec<(String, [u8; 32], u32)>, Vec<MergeConflict>>
/// Find the lowest common ancestor (merge base) of two commits using bidirectional BFS
fn find_merge_base(conn: &Connection, commit1: [u8; 32], commit2: [u8; 32]) -> Result<Option<[u8; 32]>, ReviusError>
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
struct FileInfo {
    size: i64,
    recipe: Vec<u8>,
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
/// Complete status information comparing HEAD, staging, and working directory
struct StatusInfo {
    branch_name: Option<String>,
    detached_commit: Option<[u8; 32]>,
    staged_new: Vec<String>,
    staged_modified: Vec<String>,
    staged_deleted: Vec<String>,
    unstaged_modified: Vec<String>,
    unstaged_deleted: Vec<String>,
    untracked: Vec<String>,
}
#[derive(Debug)]
struct LogOptions {
    limit: Option<usize>,
    show_graph: bool,
    oneline: bool,
    first_parent: bool,
}
#[derive(Debug, Clone)]
struct CommitInfo {
    hash: [u8; 32],
    parent_hash: Option<[u8; 32]>,
    merge_parent_hash: Option<[u8; 32]>,
    tree_hash: [u8; 32],
    author_name: String,
    author_email: String,
    timestamp: i64,
    message: String,
    refs: Vec<String>, // Branch/tag names pointing to this commit
}
struct SwitchResult {
    previous_head: HeadState,
    new_head: HeadState,
    files_changed: usize,
    files_deleted: usize,
}
enum HeadState {
    Branch(String, [u8; 32]),
    Detached([u8; 32]),
}
enum TargetType {
    Branch(String),
    Commit,
}
struct SwitchPlan {
    to_add: Vec<(String, [u8; 32], u32)>, // (path, file_hash, mode)
    to_modify: Vec<(String, [u8; 32], u32)>, // (path, file_hash, mode)
    to_delete: Vec<String>, // path
}
```

### `core/models/repository.rs`

```rust
struct Repository {
    root: PathBuf,
    config: Config,
    conn: Connection,
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
const CURRENT_SCHEMA_VERSION: i64 = 1;
/// Checks if current version in code mismatches that in DB
fn check_schema_version(conn: &Connection) -> Result<(), ReviusError>
fn get_schema_version(conn: &Connection) -> Result<i64, ReviusError>
fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>, ReviusError>
// Sets a meta value by key (both inserting and updating)
fn set_meta(tx: &Transaction, key: &str, value: &str) -> Result<(), ReviusError>
```

### `db/blobs.rs`

```rust
fn insert_blob(tx: &Transaction, hash: &[u8; 32], data: &[u8], compression: &str, uncompressed_size: u64) -> Result<(), ReviusError>
fn blob_exists(tx: &Transaction, hash: &[u8; 32]) -> Result<bool, ReviusError>
/// Get compressed blob data by hash
fn get_blob(conn: &Connection, blob_hash: &[u8; 32]) -> Result<Vec<u8>, ReviusError>
```

### `db/files.rs`

```rust
fn insert_file(tx: &Transaction, hash: &[u8; 32], recipe: &[u8], chunk_count: u64, size: u64) -> Result<(), ReviusError>
fn file_exists(tx: &Transaction, hash: &[u8; 32]) -> Result<bool, ReviusError>
fn get_file(conn: &Connection, file_hash: &[u8; 32]) -> Result<FileInfo, ReviusError>
```

### `db/staging.rs`

```rust
/// Returns StagedFile by repo-relative path
fn get_staged_file(tx: &Transaction, path: &str) -> Result<Option<StagedFile>, ReviusError>
fn upsert_staging(tx: &Transaction, path: &str, hash: &[u8; 32], mode: u32, size: u64, modified_at: i64) -> Result<(), ReviusError>
fn get_all_staged(conn: &Connection) -> Result<Vec<StagedFile>, ReviusError>
fn remove_staged_file(tx: &Transaction, path: &str) -> Result<(), ReviusError>
fn clear_staging(conn: &Transaction) -> Result<(), ReviusError>
```

### `db/trees.rs`

```rust
fn tree_exists(conn: &Connection, parent_hash: &[u8; 32]) -> Result<bool, ReviusError>
fn insert_tree_entry(tx: &Transaction, parent_hash: &[u8; 32], name: &str, object_hash: &[u8; 32], mode: u32, is_dir: bool) -> Result<(), ReviusError>
/// Efficient batch insert
fn batch_insert_tree_entries(tx: &Transaction, entries: Vec<TreeEntry>) -> Result<(), ReviusError>
/// Get all direct children of a tree node (one level only)
fn get_tree_entries(conn: &Connection, parent_hash: &[u8; 32]) -> Result<Vec<TreeEntry>, ReviusError>
fn get_file_size(conn: &Connection, file_hash: &[u8; 32]) -> Result<u64, ReviusError>
```

### `db/commits.rs`

```rust
use crate::core::models::objects::Commit;
fn insert_commit(tx: &Transaction, hash: &[u8; 32], parent_hash: Option<&[u8; 32]>, merge_parent_hash: Option<&[u8; 32]>, tree_hash: &[u8; 32], message: &str, author_id: i64, timestamp: i64) -> Result<(), ReviusError>
fn get_commit(conn: &Connection, hash: &[u8; 32]) -> Result<Option<Commit>, ReviusError>
fn commit_exists(conn: &Connection, hash: &[u8; 32]) -> Result<bool, ReviusError>
/// Get the tree hash for a commit
fn get_commit_tree(conn: &Connection, commit_hash: &[u8; 32]) -> Result<[u8; 32], ReviusError>
/// Find commits matching a hash prefix. Returns Vec<[u8; 32]> of matching commit hashes
fn find_commits_by_prefix(conn: &Connection, prefix: &str) -> Result<Vec<[u8; 32]>, ReviusError>
/// Check if a hash matches a given prefix. hex_len is the number of hex characters in the original prefix (not bytes)
fn hash_matches_prefix(hash: &[u8], prefix_bytes: &[u8], hex_len: usize) -> bool
/// Resolve a hash prefix to exactly one commit hash. Returns error if prefix is ambiguous or matches no commits
fn resolve_commit_prefix(conn: &Connection, prefix: &str) -> Result<[u8; 32], ReviusError>
/// Get all parent hashes for a commit (primary and merge parent if exists)
fn get_commit_parents(conn: &Connection, commit_hash: &[u8; 32]) -> Result<Vec<[u8; 32]>, ReviusError>
```

### `db/authors.rs`

```rust
/// Get or create an author, returning their ID
fn get_or_create_author(tx: &Transaction, name: &str, email: &str) -> Result<i64, ReviusError>
/// Get author details by ID. Returns (name, email)
fn get_author_by_id(conn: &Connection, author_id: i64) -> Result<(String, String), ReviusError>
```

### `db/refs.rs`

```rust
fn get_ref(conn: &Connection, path: &str) -> Result<Option<[u8; 32]>, ReviusError>
fn upsert_ref(tx: &Transaction, path: &str, ref_type: u8, commit_hash: &[u8; 32]) -> Result<(), ReviusError>
/// Update an existing ref - use when you know the ref exists (it doesn't take ref type as a parameter)
fn update_ref(tx: &Transaction, path: &str, commit_hash: &[u8; 32]) -> Result<(), ReviusError>
/// Resolve HEAD to a commit hash. Returns None if HEAD points to non-existent ref (initial commit case)
fn resolve_head(conn: &Connection) -> Result<Option<[u8; 32]>, ReviusError>
/// Get all refs (branches and tags) with their commit hashes. Returns Vec<(ref_path, commit_hash)>
fn get_all_refs(conn: &Connection) -> Result<Vec<(String, [u8; 32])>, ReviusError>
/// Get all branch refs (starting with "refs/heads/"). Returns Vec<(branch_name_only, commit_hash)>
fn get_all_branches(conn: &Connection) -> Result<Vec<(String, [u8; 32])>, ReviusError>
fn delete_ref(tx: &Transaction, path: &str) -> Result<(), ReviusError>
fn ref_exists(conn: &Connection, path: &str) -> Result<bool, ReviusError>
```

### `db/reflog.rs`

```rust
fn insert_reflog(tx: &Transaction, ref_path: &str, old_hash: Option<&[u8; 32]>, new_hash: Option<&[u8; 32]>, action: &str) -> Result<(), ReviusError>
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
/// Returns absolute paths to files
fn expand_paths(paths: Vec<PathBuf>, repo_root: &Path, ignore_path: &Path) -> Result<Vec<PathBuf>, ReviusError>
/// Get all unignored files in the working directory and return absolute paths to them
fn get_all_repo_files(repo_root: &Path, ignore_path: &Path) -> Result<Vec<PathBuf>, ReviusError>
/// Core directory walking implementation using ignore::WalkBuilder. Walks a directory tree and returns all unignored files
fn walk_directory(start_path: &Path, repo_root: &Path, ignore_path: &Path) -> Result<Vec<PathBuf>, ReviusError>
```

### `fs/io.rs`

```rust
fn create_dir(path: &Path) -> io::Result<()>
fn write_file(path: &Path, content: &str) -> io::Result<()>
fn write_binary(path: &Path, content: &[u8]) -> io::Result<()>
fn read_file(path: &Path) -> io::Result<Vec<u8>>
fn delete_file(path: &Path) -> io::Result<()>
/// Create directory and all parent directories
fn create_dir_all(path: &Path) -> io::Result<()>
fn get_file_modified_time(path: &Path) -> io::Result<i64>
fn get_file_mode(path: &Path) -> io::Result<u32>
fn set_file_mode(path: &Path, mode: u32) -> io::Result<()>
/// Set file as executable (Unix only, no-op on Windows)
fn set_executable(path: &Path) -> io::Result<()>
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
fn path_exists(path: &Path) -> bool
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
// Validate that a string is a valid hex prefix (1-64 hex chars)
fn is_valid_hash_prefix(prefix: &str) -> bool
/// Convert hex string to partial hash bytes (for prefix matching). Returns the bytes and the number of valid hex digits
fn hex_prefix_to_bytes(prefix: &str) -> Result<(Vec<u8>, usize), String>
```

### `utils/recipe.rs`

```rust
fn parse_recipe(recipe: &[u8]) -> Result<Vec<[u8; 32]>, String>
fn build_recipe(hashes: &[[u8; 32]]) -> Vec<u8>
```

### `utils/time.rs`

```rust
fn unix_timestamp() -> Result<i64, time::SystemTimeError>
/// Format Unix timestamp as human-readable string. Format: "Mon Dec 21 14:30:45 2024 +0000"
fn format_timestamp(timestamp: i64) -> String
```

### `utils/validation.rs`

```rust
fn validate_branch_name(name: &str) -> Result<(), ReviusError>
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

    #[command(about = "Show the working tree status")]
    Status(StatusArgs),

    #[command(about = "Show commit history")]
    Log(LogArgs),

    #[command(about = "List, create, rename, or delete branches")]
    Branch(BranchArgs),

    #[command(about = "Switch branches or restore working tree files")]
    Switch(SwitchArgs),

    #[command(about = "Join two development histories together")]
    Merge(MergeArgs),

    #[command(about = "Reset current HEAD to the specified state")]
    Reset(ResetArgs),

    #[command(about = "Restore working tree files")]
    Restore(RestoreArgs),
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

#[derive(Parser)]
struct StatusArgs {
    // Currently no arguments, but can add --short, --verbose, etc. later
}

#[derive(Parser)]
struct LogArgs {
    #[arg(short = 'n', long, help = "Limit number of commits to show")]
    limit: Option<usize>,
    
    #[arg(long, help = "Show commit graph with ASCII art")]
    graph: bool,
    
    #[arg(long, help = "Show each commit on a single line")]
    oneline: bool,
    
    #[arg(long, help = "Show only the first parent in merge commits")]
    first_parent: bool,
}

#[derive(Parser)]
struct BranchArgs {
    #[arg(help = "Branch name to create, or the first branch name when renaming/deleting")]
    name: Option<String>,

    #[arg(short = 'm', long, help = "Rename a branch")]
    rename: bool,

    #[arg(short = 'd', long, help = "Delete a branch")]
    delete: bool,

    #[arg(short = 'D', long, help = "Force delete a branch")]
    force_delete: bool,

    #[arg(help = "New name when renaming (optional second argument)")]
    new_name: Option<String>,
}

#[derive(Parser)]
struct SwitchArgs {
    #[arg(help = "Branch name or commit hash to switch to")]
    target: String,

    #[arg(short = 'c', long, help = "Create new branch from current state and switch to it")]
    create: bool,

    #[arg(short = 'f', long, help = "Force switch, discarding local changes")]
    force: bool,
}

#[derive(Parser)]
struct MergeArgs {
    #[arg(help = "Branch name or commit hash to merge")]
    target: String,
}

#[derive(Parser)]
pub struct ResetArgs {
    #[arg(help = "Commit hash or reference to reset to (defaults to HEAD)")]
    pub target: Option<String>,

    #[arg(short, long, help = "Reset HEAD but keep staging and working directory unchanged")]
    pub soft: bool,

    #[arg(short, long, help = "Reset HEAD and staging, but keep working directory unchanged (default)")]
    pub mixed: bool,

    #[arg(short = 'H', long, help = "Reset HEAD, staging, and working directory (destructive)")]
    pub hard: bool,
}

#[derive(Parser)]
pub struct RestoreArgs {
    #[arg(required = true, help = "Files or directories to restore")]
    pub paths: Vec<PathBuf>,

    #[arg(short, long, help = "Restore the repository's staging area from source")]
    pub staged: bool,

    #[arg(short, long, help = "Restore the working tree from staging area (or source if combined with --staged)")]
    pub worktree: bool,

    #[arg(long, help = "Commit to restore from. Defaults to HEAD. Ignored if only --worktree is used.")]
    pub source: Option<String>,
}
```

### `cli/ui.rs`

```rust
fn print_error(msg: &str)
pub fn print_warn(msg: &str)
fn print_no_user_configured()

fn print_init_success(path: &Path)

fn print_added_file(path: &str)
fn print_modified_file(path: &str)
pub fn print_deleted_file(path: &str)
fn print_add_summary(added: u64, changed: u64, deleted: u64, unchanged: u64, blobs: u64)

fn print_commit_success(hash: &[u8; 32], message: &str, files_changed: usize)
fn print_nothing_to_commit()
fn print_detached_head_warning(commit_hash: &[u8; 32])

fn print_status(status: &StatusInfo)

fn print_no_commits()
/// Print commit history based on options
fn print_log(commits: &[CommitInfo], options: &LogOptions)
fn print_commit_detailed(commit: &CommitInfo)
fn print_commit_oneline(commit: &CommitInfo)
/// For now implements a simple linear graph. Future enhancement: proper graph with branches
fn print_commit_graph(commits: &[CommitInfo], oneline: bool)

/// Print a list of branches with the current one marked
fn print_branch_list(branches: &[(String, [u8; 32], bool)])
fn print_branch_created(branch_name: &str, commit_hash: &[u8; 32])
fn print_branch_renamed(old_name: &str, new_name: &str)
fn print_branch_deleted(branch_name: &str, commit_hash: &[u8; 32])
fn print_current_branch(branch_name: &str)
fn print_detached_head_branch_warning(commit_hash: &[u8; 32])
fn print_no_branches()

fn print_switch_success(previous: &HeadState, new: &HeadState, files_changed: usize, files_deleted: usize)
fn print_branch_created_and_switched(branch_name: &str, commit_hash: &[u8; 32])

fn print_merge_fast_forward(from: &[u8; 32], to: &[u8; 32])
fn print_merge_already_up_to_date()
fn print_merge_success(commit_hash: &[u8; 32], files_changed: usize)
fn print_merge_conflicts(conflicts: &[MergeConflict])

pub fn print_reset_success(mode: &str, commit_hash: &[u8; 32])

pub fn print_restore_success(mode: &str, count: usize)
```
