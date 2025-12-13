# Meta

Revius is a content-addressed, single-file repository, lightweight VCS, offering speed, safety and modernization through its implementation in Rust.

This document defines the overall architecture of the Revius VCS.
- A fixed reference so that code remains stable and consistent.
- Description of all modules, responsibilities, workflows, and design constraints.
- Rules for how new features, commands, modules, or DB changes must behave.
- Architectural invariants that must never be violated.

# Storage

## The repo on disk
On disk, the repository is a folder. It is like so:

```text
(root folder's name)/
    -- Any files and folders stored in the repository
    .rvsconfig.toml -- Contains the per-repo TOML configuration
    .rvsignore -- Git-style ignoring
    .rvs/ -- The contents of this folder are exclusively managed by the system, the user shouldn't manually tamper with them.
        repo.db -- The SQLite database which contains all internal info about the repository (see schema below)
```

Separately, a config.toml file is going to be stored per-user. On Windows it's saved at "%APPDATA%\revius\config.toml". On Linux and MacOS it's saved at "~/.config/revius/config.toml".

## SQL storage

"repo.db" contains all internal repo information.
Large files are divided using CDC. Blobs are compressed with Zstd. I use a merkle tree, and each record can be either a file or a directory.
Refs -> Commits -> Trees -> Files -> Blobs form a directed acyclic graph of content-addressed immutable objects (except Refs, which are mutable).
Hashes are stable - identical content yields identical hash.
Blobs, Files, Trees, Commits are immutable. If identical content appears twice, the system must not duplicate storage.
Commits, trees, files and blobs are append-only. Only the garbage collector will prune unused objects.
Important for deduplication: When inserting one of these objects, it might already exist. That's not a problem, that's how deduplication works.

## Full starter SQL schema

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL; -- Write-ahead logging - changes are written in a separate file and then get merged, allows long writing sessions without blocking reading of the database.

-- Config (anything that can be modified by the user directly) is an external TOML file.
-- Meta: Inner data which probably shouldn't be messed with directly.
CREATE TABLE IF NOT EXISTS Meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
-- Fields include: schema_version set to 1, repository_uuid set to a randomly generated uuid, and HEAD initially set to "ref: refs/heads/main". Add these 3 fields in the schema.
-- Any code that opens a repo must check schema and repository version. If repo version > code supported, program must exit with a message.

CREATE TABLE IF NOT EXISTS Blobs (
    hash BLOB PRIMARY KEY, -- BLAKE3 (32 bytes)
    data BLOB NOT NULL, -- Compressed data
    compression TEXT NOT NULL DEFAULT 'zstd3', -- In case of format changes - default for now is Zstd on level 3
    uncompressed_size INTEGER NOT NULL,
    CHECK(length(hash) = 32)
);

CREATE TABLE IF NOT EXISTS Files (
    hash BLOB PRIMARY KEY, -- BLAKE3 of the entire file
    size INTEGER NOT NULL, -- Uncompressed size
    recipe_version INTEGER NOT NULL DEFAULT 1, -- In case of format changes
    chunk_count INTEGER NOT NULL,
    recipe BLOB NOT NULL, -- Packed binary chunk hashes (or the same single hash if not segmented) - concatenated hashes in order
    CHECK(length(hash) = 32),
    CHECK(length(recipe) % 32 == 0)
    -- There's no FK for Blobs, because we pack them in one recipe, we'll parse them afterward.
);

CREATE TABLE IF NOT EXISTS Trees (
    parent_hash BLOB NOT NULL, -- Hash of the containing folder (dependent on sorted all children's hashes)
    name TEXT NOT NULL, -- Name of the element
    object_hash BLOB NOT NULL, -- Points either to Files (hash) if it's a file, or Trees (parent_hash) recursively if it's a dir
    mode INTEGER NOT NULL, -- 100644 (file), 100755 (exec), 040000 (dir) - the Unix codes (used also to differentiate between files and dirs)
    is_dir INTEGER NOT NULL CHECK(is_dir IN (0, 1)), -- 0 = file, 1 = directory - a bit of info duplication with mode, but it's important for the compound PK
    PRIMARY KEY (parent_hash, name, is_dir),
    CHECK(length(parent_hash) = 32),
    CHECK(length(object_hash) = 32)
    -- The pure relational solution would be to have 3 tables (trees with a general id, and treefiles and treedirs pointing to trees), for clean FK enforcement.
    -- But that would make the algorithm unnecessarily complex and slow. Here integrity is managed by the hash system.
    -- Note: The repo root folder itself is not stored as an entry, it's only the parent hash of all top-level elements. Commit's tree_hash points to the parent_hash of the top-level elements. For every object_hash that belongs to a folder, search this as a parent_hash and so on.
);
CREATE INDEX idx_trees_object ON Trees(object_hash);
CREATE INDEX idx_trees_parent ON Trees(parent_hash);

CREATE TABLE IF NOT EXISTS Authors ( -- Author's name and email are set in an external configuration file.
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    UNIQUE(name, email)
);

CREATE TABLE IF NOT EXISTS Commits (
    hash BLOB PRIMARY KEY,
    parent_hash BLOB REFERENCES Commits(hash), -- Primary parent (the branch you were on when you committed) - NULL for the first commit
    merge_parent_hash BLOB REFERENCES Commits(hash), -- The secondary parent (branch you merged in). NULL unless this is a merge commit.
    tree_hash BLOB NOT NULL, -- Root tree hash (we don't have FK but use an abstraction, as explained above) - points to the parent_hash of all top-level tree objects in the repo.
    message TEXT NOT NULL,
    author_id INTEGER NOT NULL REFERENCES Authors(id),
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')), -- Unix timestamp
    CHECK(length(hash) = 32)
);
CREATE INDEX IF NOT EXISTS idx_commits_parent ON Commits(parent_hash);
CREATE INDEX IF NOT EXISTS idx_commits_merge_parent ON Commits(merge_parent_hash);
-- Optimizing the queries for commits with parent X, for visualizing the branches in a graph.

CREATE TABLE IF NOT EXISTS Refs (
    path TEXT PRIMARY KEY,  -- "refs/heads/main", "refs/tags/v1"
    ref_type INTEGER NOT NULL CHECK(ref_type IN (0, 1, 2)), -- 0 for branch, 1 for tag, 2 for remote
    commit_hash BLOB NOT NULL REFERENCES Commits(hash) -- The commit this ref points to currently. For new refs, the commit should be existing first.
);

CREATE TABLE IF NOT EXISTS Staging (
    path TEXT PRIMARY KEY, -- Full path to file from root of repo
    file_hash BLOB NOT NULL REFERENCES Files(hash),
    mode INTEGER NOT NULL, -- Same format as in Trees
    size INTEGER NOT NULL,
    modified_at INTEGER, -- "modified at" timestamp of the file (for the status subcommand) (not the moment it was staged, but the mtime of the file)
    CHECK(length(file_hash) = 32)
);

CREATE TABLE IF NOT EXISTS Reflog (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ref_path TEXT NOT NULL, -- The ref in question
    old_hash BLOB, -- Old commit hash
    new_hash BLOB, -- New commit hash
    action TEXT NOT NULL, -- The specific command used: commit/switch/reset, and its parameters, both in a list in a JSON string (like in Audit)
    timestamp INTEGER DEFAULT (strftime('%s', 'now')),
    CHECK(length(old_hash) = 32),
    CHECK(length(new_hash) = 32)
);

CREATE TABLE IF NOT EXISTS Audit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    action TEXT NOT NULL, -- The subcommand used
    args TEXT, -- The parameters in a list as a JSON string
    output TEXT, -- The console output
    exit_code INTEGER, -- 0, 1, 2, 126, 130 -- POSIX codes
    author_id INTEGER REFERENCES Authors(id),
    timestamp INTEGER DEFAULT (strftime('%s', 'now')), -- Start time in seconds
    duration_ms INTEGER -- Execution time in milliseconds (excluding input awaiting)
);
```

# High-level architecture

Revius is structured into clear layers:
```text
main.rs -> commands -> core -> {db -> rusqlite, fs -> std::fs, utils -> pure functions, cli::ui -> clap and UI stuff}
```
Each layer has strict responsibilities and must not reach down or across improperly. This enables consistency, safety, predictable code creation, and the DRY principle.

Rules:
- `core` may import: db, fs, utils
- `commands` may import: core, cli, fs
- `db` may import: models, utils (no core, no fs)
- `fs` may import: utils (no core, no db)
- `utils` may import external crates only
- `cli` modules: args imports clap, ui imports io stuff
- All may import error, and any other needed common Rust imports for utilities

Violation of these rules = architectural violation.

# Code modularity and function extraction

## Core principle: Don't hardcode, extract

When implementing features, never hardcode logic that belongs in another module. If you need functionality that doesn't exist yet, create it in the appropriate place:

## When to Create New Functions

Create functions in existing modules when:
- db/: You need a new database operation (query, insert, update, delete)
- fs/: You need filesystem operations or path manipulations
- utils/: You need pure data transformations or algorithm wrappers, or you need to use utilities provided by the system (create wrappers for them here)
- cli/ui: You need to print a new type of message
- error: You need a new error variant

Create helper functions within the same file when:
- A function becomes long or complex
- You're repeating similar logic multiple times
- A chunk of code has a clear single responsibility
- Breaking it out would make the parent function easier to read

Create files in core/ when:
- The logic is complex domain logic that will be reused across commands
- Example: Building a tree from staging needs to be used by commit, status, diff...

## Decision Tree

When writing code, ask yourself:

1. Is this logic I'm about to write already a responsibility of db/, fs/, utils/, or cli/?
   - YES -> Create a function there
2. Is this logic I'm about to write likely to be needed elsewhere?
   - YES -> Extract to appropriate module
3. Is my function getting long (>50 lines), too complex, or doing multiple things?
   - YES -> Break into smaller functions
4. Am I repeating similar code within this file?
   - YES -> Extract common logic to a helper function

## Remember

- Separation of concerns is not optional - it's a core architectural requirement
- When in doubt, extract - small, focused functions are easier to test, debug, and reuse
- Don't wait for functions to exist - if you need something, create it in the right place
- Think about the next developer - will they know where to find this logic? Will they accidentally duplicate it?

This modularity keeps the codebase maintainable, testable, and prevents the core logic from becoming a tangled mess of mixed concerns.

# Code structure

## Modules and files

```text
src/
    main.rs -- Parses commands with clap, turning args into a variant of an enum; Constructs a Repository object (except for init); Dispatch to appropriate command module, passing Repository and respective struct
    lib.rs
    error.rs -- Defines the ReviusError enum
    commands/ -- Application controllers; Receive their individual parameter struct, validate them, call the right core functions, and handle high-level orchestration - call core operations, handle success/failure (ReviusError gets passed up to main), and call cli::ui to print results
        mod.rs
        init.rs
        add.rs
        commit.rs
        ... (one for each subcommand)
    core/ -- Domain logic, the various algorithms which work with the other modules, also handling edge cases
        models/
            mod.rs
            repository.rs -- Repo state, the context struct - DB connection, config struct, root path
            objects.rs -- The DB models used throughout the system, one per each table
            config.rs -- The various structures used for the config
            serialization.rs -- Canonical binary serialization of the objects which need it (commit, tree...)
            (anything else, if needed...)
        init.rs -- Initialized a Revius repository at a specific path, if not existing
        open.rs -- Opens the repo at a specific path (if it exists) and returns the Repository struct
        config.rs -- Manages config structs and merging, validating, defaults...
        (and more...)
    db/ -- Holds the SQL for the various DB operations that will be used by the other modules - one file per table, plus the connection and the schema; Internally works with SQL, externally with parameters or model structs (whichever suits the use case); Receives the connection object (or as a transaction)
        mod.rs
        connection.rs
        schema.rs
        meta.rs
        blobs.rs
        files.rs
        trees.rs
        commits.rs
        refs.rs
        staging.rs
        reflog.rs
        audit.rs
        authors.rs
    fs/ -- Handles filesystem operations
        mod.rs
        io.rs -- Basic I/O wrappers
        paths.rs -- Provides functions to get absolute paths to various repo stuff from root path (or user configs)
        ignore.rs -- Ignore pattern matching (wrapper for ignore crate)
        config.rs -- For serializing TOML files
        walk.rs -- Walks dir trees, respecting ignore, expands user path list into flat list of files
        lock.rs -- Handles the lock (for future feature)
    cli/ -- User interface - pure UI, responsible only for converting user input and output
        mod.rs
        args.rs -- Clap Command enum, defining CLI grammar; Each subcommand has its own parameters struct
        ui.rs -- Printing helpers; Prevents hardcoding messages in other files - contains functions for displaying messages for any case needed by the system
    utils/ -- Pure general-purpose helper functions
        hash.rs -- BLAKE3 wrappers
        cdc.rs -- FastCDC wrappers
        compression.rs -- Zstd wrappers
        (and others if needed...)
```

## The logic split:
Main:
- Parses CLI args via clap and matches on the command enum
- Opens repository for commands requiring it
- Dispatches to the respective commands::X::run(), passing reference to repository and specific args struct
- Handles top-level error printing
- NO: domain logic, DB/FS operations, command-specific output

Commands:
- Validates parsed CLI parameters
- Expand user-supplied path lists to file lists with ignoring via FS
- Deconstruct arguments and calls the appropriate core function(s) with the Repository object and the relevant fields
- Translate core results into human output with cli::ui
- Pass errors up to main with Err()
- Potentially building a "plan" of operations for the user to confirm before beginning (like in switch and any other destructive operations)
- NO: heavy domain logic, DB operations

Core:
- All domain logic:
    - High-level repository operations: init, open, add logic, commit creation, tree building, ref updating, staging logic, status check, etc. - for each subcommand, everything not covered by the commands module
    - Cross-cutting logic like opening and config management (and merging)
- All domain models and data types:
    - Repository struct (represents repository context - root path, config, DB connection)
    - Model for each table
    - All other structs and enums that belong to domain logic
- Transaction boundaries - core chooses which operations require transactions, commands don't manage them
- Carries out the logic
- Uses FS, DB, utils modules as needed
- NO: UI, CLI args, user prompting, using external crates directly for which utils exist, manually working with paths rather than using fs, or anything that can be put as a separate function anywhere else (if a function gets too big, you can divide it in multiple functions, too)

FS:
- Raw filesystem reads/writes
- Directory walking
- Path normalization and utilities
- Provides the actual absolute paths to various repo stuff based on root path (or gets user config)
- Ignore rules file loading
- Config file loading and serialization/deserialization
- Lock file management (future)
- NO: domain logic, DB operations, UI

DB:
- Defines the schema
- SQL CRUD
- One file per table - exposes needed operations
- Model to row and row to model conversion (internally works with SQL, externally works with structs)
- Operate only on the active connection provided by core
- NO: domain logic, FS operations, managing transactions, UI

CLI:
- Args holds clap struct and individual structs for each subcommand
- UI holds various functions for printing various things, and contain the actual messages (so no message is hardcoded outside of here) - may also receive parameters for the message if needed
- NO: domain logic, DB/FS, data retrieval other than the given parameters

Utils:
- Only pure, stateless helper functions
- NO: FS/DB/UI, Repository dependence

# External dependencies (Cargo.toml)

Cargo.toml, as of now. Can be modified at need.

```toml
[package]
name = "revius"
version = "0.1.0"
edition = "2024"

# library crate
[lib]
name = "revius"
path = "src/lib.rs"

# binary crate
[[bin]]
name = "revius"
path = "src/main.rs"

[dependencies]

rusqlite = { version = "0.37.0", features = ["bundled", "serde_json"] }

clap = { version = "4.5.53", features = ["derive"] }

thiserror = "2.0.17"

serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.145"
toml = "0.9.8"

blake3 = "1.8.2"
zstd = "0.13.3"
fastcdc = "3.2.1"
ignore = "0.4.25"

chrono = "0.4.42"
uuid = { version = "1.19.0", features = ["v4"] }
similar = "2.7.0"
dirs = "6.0.0"
hex = "0.4.3"
colored = "3.0.0"
comfy-table = "7.2.1"
```

# Details about sppecifics

## Miscellaneous rules

1. Modular code which separates concerns.
2. The OS, architecture, and any other environment shouldn't matter.
3. Console output should be informative and nicely formatted.
4. The individual functions must be generally short and modular.
5. Important: While developing something (especially commands and core) feel free to create new things in db, fs, utils, cli, core as needed, so that the separation of concerns is strong. If part of the logic can be extracted and reused, it probably should.

## Repository lifecycle

A Repository is a domain context containing:
- Root path
- Repo configuration struct (loaded from core::config::load_config(root_path))
- Database connection

Creation of this object occurs only in core::init and core::open.

The core::open function is called ONLY in commands, those which need it. It returns the repository (in a result). Internally, it calls an up traversal function, looking for the first parent containing a `.rvs` folder.
```rust
let repo = Repository::open(std::env::current_dir()?)?;
```

Lifecycle invariants:
1. All paths are canonical absolute paths internally.
2. Paths written to the DB are repo-relative.
3. Repository is never mutated outside of core.
4. .rvs layout is constant; no command may change its structure.
5. (future) Lock file acquisition is handled by core; commands never touch locking directly.

## Path handling

Rules for handling file paths:
1. Commands must convert user paths → absolute paths.
2. Core converts absolute → repo-relative paths before storing in DB.
3. Ignore matching always uses repo-relative paths.
4. .rvs directory is excluded automatically.
5. FS module provides canonicalization/normalization and other path and filesystem utilities.
6. FS module provides paths for the various things, so no hardcoding in other parts of the file.

## Configuration system

Revius uses 2 TOML configuration files:
1. Repository-local config at `<repo>/.rvsconfig.toml`
2. User-level config at `%APPDATA%\revius\config.toml` on Windows and `$HOME/.config/revius/config.toml` on Linux.
These are independent sources that are merged into a Config struct at repository open/init time. If a field does not exist (or the whole config file does not exist), assume defaults, as listed in core/models/config.rs.
No precedence needed, as the two files contain different fields.

RepoConfig:

```toml
[core]
compression = true # Whether to compress new blobs
compression_level = 3 # Zstd compression level (from -7 to 22 but not 0)
chunking = true # Whether to chunk files using CDC or save them as one blob
chunk_min = 8192 # FastCDC minimum chunk size
chunk_avg = 16384 # FastCDC average chunk size
chunk_max = 32768 # FastCDC maximum chunk size
```

UserConfig:

```toml
[user]
name = "None" # User's name to save in Authors table upon commit
email = "none@example.com" # User's email to save in Authors table upon commit
```

At repository open time, the two configs are merged - missing fields use defaults.
The merged struct:
```rust
struct Config {
    compression: bool,
    compression_level: u8,
    chunking: bool,
    chunk_min: u64,
    chunk_avg: u64,
    chunk_max: u64,
    user_name: Optional<String>,
    user_email: Optional<String>,
}
```

## Transactions

Rules:
1. Only core starts and commits transactions, not commands.
2. A core function performing multiple DB writes must wrap them in a single transaction.
3. DB operations accept &Connection. Transaction objects dereference to Connection, allowing the same functions to work within or outside transactions.
4. Transactions are short-lived: begin, perform writes, commit/rollback.
5. If the command requires workspace changes, begin them only after the DB operations are done. If it fails, rollback the DB, too.

## Error handling

Uniform error strategy:
1. All errors use ReviusError.
2. Core returns errors upward without formatting them.
3. Commands pass errors to main.
4. Main handles the ReviusError by printing it (with UI, depending on the error).
5. Important: NO unwrap, expect, or panics outside tests.
6. FS and DB operations add path/SQL context to all errors.

All errors must include sufficient context for debugging:
FS operations:
- Include the full path attempted
- Example: `Failed to read file: /path/to/file: Permission denied`
DB operations:
- Include table name and operation type
- For hash lookups: include the hash (first 8 hex chars)
- Example: `Failed to insert into Blobs (hash=a1b2c3d4...): constraint violation`
Core operations:
- Include the command context
- Example: `Failed to create commit: no files staged`
Use error chaining with thiserror's `#[source]` attribute.

## Determinism and hash computation

Revius must be deterministic:
1. Identical input produces identical commits, trees, and blob hashes.
2. Tree entries sorted lexicographically (uses BTree).
3. Canonical encoding is stable across:
    - OS
    - architecture
    - library versions (within reason)
Hashing is always content-based, independent of file metadata.

All hashes are BLAKE3, 32 bytes. Hash inputs must be deterministic and canonical:

Blob hashing:
- Hash the uncompressed data directly
- `hash = BLAKE3(uncompressed_data)`

File hashing:
- Hash the uncompressed file directly
- Recipe stores concatenated blob hashes in chunk order separately
- Hashing the pure file content is important for staging, too.

Tree hashing:
- `tree_hash = BLAKE3(concat(all_serialized_entries))`
- Handled by `core/models/serialization.rs` (for entries)

Commit hashing:
- Handled by `core/models/serialization.rs`

## Staging Area Semantics

The Staging table represents the next commit's snapshot:

1. `add <file>` computes file hash, stores in Staging (replaces existing entry for same path) (if not ignored)
2. `add <dir>` recursively adds all non-ignored files in directory
3. Staging stores flattened file list, not directory structure
4. At commit time:
   - Build Trees bottom-up from staged file paths
   - Retain Staging after successful commit
5. Deleted files: must be explicitly removed from Staging (future: `rm` command)
6. Modified files not re-added remain at old version in Staging
7. Unstaged changes: compare working tree vs Staging for `status`

Important: Staging contains Files (their full relative to root paths, also hashed by content), not Blobs directly.

## HEAD and Branch Mechanics

HEAD is stored in Meta table with key "HEAD":

Formats:
- Detached: HEAD = `<commit_hash>` (32-byte blob)
- Branch: HEAD = `ref: refs/heads/<branch_name>` (text)

Current commit resolution:
1. Read HEAD from Meta
2. If starts with "ref: ", parse branch name and look up in Refs table
3. Otherwise, treat as direct commit hash
4. Functions:
    - `db::refs::resolve_head(conn: &Connection) -> Result<Option<[u8; 32]>, ReviusError>`
    - `core::refs::update_head(tx: &Transaction, commit_hash: &[u8; 32]) -> Result<(), ReviusError>` // Requires more complex logic, so is in core.

Branch operations:
- Create branch: Insert into Refs, don't change HEAD
- Switch branch: Update HEAD to `ref: refs/heads/<branch>`
- Commit: Update the commit_hash in Refs for current branch (or HEAD if detached)

First commit special case:
- Before first commit, HEAD points to "ref: refs/heads/main" but that ref doesn't exist
- First commit creates the main ref

## Path Normalization Contract

User input -> Absolute:
- All paths from CLI converted to absolute (if not already) in commands module
- Use `fs::paths::canonicalize(user_path, repo_root)`
- Handle ".", "..", "~", relative paths, symlinks, and Windows/Unix differences (like the separator)

Absolute -> Repo-relative:
- Before DB storage: `fs::paths::make_relative(absolute_path, repo_root)`
- Always use forward slashes (/) even on Windows
- Never store paths starting with "/" or containing ".."

Repo-relative -> Absolute:
- For FS operations: `fs::paths::to_absolute(relative_path, repo_root)`

Validation:
- All paths must be within repo root (no escaping via ..) (return Path ReviusError at need)
- .rvs directory automatically excluded everywhere, regardless of ignore file

## Workspace changes & operation plans

Some commands like restore and switch modify the workspace.

Rules:
1. Commands build a “plan” describing which files will be overwritten or deleted.
2. UI displays the plan for confirmation (prompt for each question regarding destructive actions (like delete dirty files))
3. Core executes the plan transactionally (if it modifies DB).
4. Begin workspace changes only after the DB operations are done. If it fails, rollback the DB, too. (Workdir restoring can be done afterward manually, so a broken workdir state is not fatal.)

This ensures safe, predictable workspace manipulation.

## Exit Code Standards

All commands must return consistent exit codes:

- 0: Success
- 1: General error (file not found, invalid state, etc.)
- 2: Usage error (invalid arguments, missing required args)
- 126: Permission denied
- 130: User cancellation (Ctrl+C)

Commands return `Result<()>` where:
- `Ok(())` -> exit 0
- `Err(ReviusError::Permission(..))` -> exit 126
- `Err(ReviusError::Usage(..))` -> exit 2
- `Err(_)` -> exit 1

Main converts these to process exit codes.

## Testing Architecture (future)

Even if implemented later, testing rules must be stable:
1. Unit tests for utils, DB modules, pure core logic.
2. Integration tests for commands + core + FS + DB.
3. End-to-end tests to validate command behavior and UI output.

# 
