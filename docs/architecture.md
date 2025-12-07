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
    object_hash BLOB NOT NULL PRIMARY KEY, -- Points either to Files (hash), or Trees (parent_hash) recursively
    parent_hash BLOB NOT NULL, -- Hash of the containing folder (dependent on sorted all children's hashes)
    name TEXT NOT NULL, -- Name of the element
    mode INTEGER NOT NULL, -- 100644 (file), 100755 (exec), 040000 (dir) - the Unix codes
    CHECK(length(parent_hash) = 32),
    CHECK(length(object_hash) = 32)
    -- The pure relational solution would be to have 3 tables (trees with a general id, and treefiles and treedirs pointing to trees), for clean FK enforcement.
    -- But that would make the algorithm unnecessarily complex and slow. Here integrity is managed by the hash system.
    -- Note: The root folder itself is not stored as an entry, it's only the parent hash of all top-level elements.
);

CREATE TABLE IF NOT EXISTS Authors ( -- Author's name and email are set in an external configuration file.
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    UNIQUE(name, email)
);

CREATE TABLE IF NOT EXISTS Commits (
    hash BLOB PRIMARY KEY,
    parent_hash BLOB REFERENCES Commits(hash), -- NULL for the first commit
    tree_hash BLOB NOT NULL, -- Root tree hash (we don't have FK but use an abstraction, as explained above)
    message TEXT NOT NULL,
    author_id INTEGER NOT NULL REFERENCES Authors(id),
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')), -- Unix timestamp
    CHECK(length(hash) = 32)
);
CREATE INDEX IF NOT EXISTS idx_commits_parent ON Commits(parent_hash); -- Optimizing the queries for commits with parent X, for visualizing the branches in a graph.

CREATE TABLE IF NOT EXISTS Refs (
    path TEXT PRIMARY KEY,  -- "refs/heads/main", "refs/tags/v1"
    ref_type INTEGER NOT NULL CHECK(ref_type IN (0, 1, 2)), -- 0 for branch, 1 for tag, 2 for remote
    commit_hash BLOB NOT NULL REFERENCES Commits(hash) -- The commit this ref points to currently
);

CREATE TABLE IF NOT EXISTS Staging (
    path TEXT PRIMARY KEY, -- Full path to file from root of repo
    file_hash BLOB NOT NULL REFERENCES Files(hash),
    mode INTEGER NOT NULL, -- Same format as in Trees
    size INTEGER NOT NULL,
    modified_at INTEGER DEFAULT (strftime('%s', 'now')), -- Timestamp of the file (for the status subcommand)
    CHECK(length(file_hash) = 32)
);

CREATE TABLE IF NOT EXISTS Reflog (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ref_path TEXT NOT NULL, -- The ref in question
    old_hash BLOB, -- Old commit hash
    new_hash BLOB, -- New commit hash
    action TEXT NOT NULL, -- The specific command used: commit, checkout, reset, and its parameters, list in a JSON string (like in Audit)
    timestamp INTEGER DEFAULT (strftime('%s', 'now'))
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
main -> commands -> core -> { fs, db, utils }
Each layer has strict responsibilities and must not reach down or across improperly. This enables consistency, safety, predictable code creation, and the DRY principle.

# Code structure

## Modules and files

```text
src/
    main.rs -- Parses commands with clap, turning args into a variant of an enum; Constructs a Repository object (except for init); Dispatch to appropriate command module, passing Repository and respective struct
    lib.rs
    error.rs -- Defines the ReviusError enum
    commands/ -- Application controllers; Receive their individual parameter struct, validate them, call the right core functions, and handle high-level orchestration - call core operations, handle success/failure, and call cli::ui to print results
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
    db/ -- Holds the SQL for the various DB operations that will be used by the other modules - one file per table, plus the connection and the schema; Internally works with SQL, externally with model structs; Receives the connection object
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
        ignore.rs -- Ignore wrappers (works with path and ignore string, not FS)
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
- Deconstruct arguments and calls the appropriate core function(s) with the relevant fields
- Translate core results into human output with cli::ui
- Pass errors up to main with Err()
- Potentially building a "plan" of operations for the user to confirm before beginning (like in switch and any other destructive operations)
- NO: heavy domain logic, DB operations

Core:
- All domain logic:
    - High-level repository operations: init, open, add-file logic, commit creation, tree building, ref updating, staging logic, status check, etc. - for each subcommand, everything not covered by the commands module
    - Cross-cutting logic like config management (and merging)
- All domain models and data types:
    - Repository struct (represents repository context - root path, config, DB connection)
    - Model for each table
    - All other structs and enums that belong to domain logic
- Transaction boundaries - core chooses which operations require transactions, commands don't manage them
- Carries out the logic
- Uses FS, DB, utils modules as needed
- NO: UI, CLI args, user prompting, using external crates directly for which utils exist, manually working with paths rather than using fs

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

# Dependencies

Cargo.toml, as of now. Can be modified at need.

```toml
[package]
name = "revius"
version = "0.1.0"
edition = "2024"

[lib]
name = "revius"
path = "src/lib.rs"

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
ignore = "0.4.18"

chrono = "0.4.42"
uuid = { version = "1.19.0", features = ["v4"] }
similar = "2.7.0"
dirs = "6.0.0"
colored = "3.0.0"
comfy-table = "7.2.1"
```

# Miscellaneous rules

1. Modular code which separates concerns.
2. The OS, architecture, and any other environment shouldn't matter.
3. Console output should be informative and nicely formatted.

# Repository lifecycle

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

# Path handling

Rules for handling file paths:
1. Commands must convert user paths → absolute paths.
2. Core converts absolute → repo-relative paths before storing in DB.
3. Ignore matching always uses repo-relative paths.
4. .rvs directory is excluded automatically.
5. FS module provides canonicalization/normalization utilities.
6. FS module provides paths for the various things, so no hardcoding in other parts of the file.

# Configuration system

Revius uses 2 TOML configuration files:
1. Repository-local config at `<repo>/.rvsconfig.toml`
2. User-level config at `%APPDATA%\revius\config.toml` on Windows and `$HOME/.config/revius/config.toml` on Linux.
These are independent sources that are merged into a Config struct at repository open/init time. If a field does not exist (or the whole config file does not exist), assume defaults, as listed in core/config_models.rs.

RepoConfig:

```toml
[core]
compression = true # Whether to compress new blobs
compression_level = 3 # Zstd compression level (from -7 to 22 but not 0)
chunking = true # Whether to chunk files using CDC or save them as one blob
chunk_min = 2048 # FastCDC minimum chunk size
chunk_avg = 8192 # FastCDC average chunk size
chunk_max = 16384 # FastCDC maximum chunk size
case_sensitive = true # Determines path comparison semantics inside the repository. If false, internal keys are normalized to lowercase, external paths retain original case
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
    chunk_min: usize,
    chunk_avg: usize,
    chunk_max: usize,
    case_sensitive: bool,
    user_name: String,
    user_email: String,
}
```

# Transactions

Rules:
1. Only core starts and commits transactions, not commands.
2. A core function performing multiple DB writes must wrap them in a single transaction.
3. DB writes accept &Transaction and DB reads accept &Connection.
4. Transactions are short-lived: begin, perform writes, commit/rollback.

# Error handling

Uniform error strategy:
1. All errors use ReviusError.
2. Core returns errors upward without formatting them.
3. Commands catch and format errors for UI.
4. main only prints fallback errors (unexpected or uncaught).
5. No unwrap, expect, or panics outside tests.
6. FS and DB operations add path/SQL context to errors.

# Determinism

Revius must be deterministic:
1. Identical input produces identical commits, trees, and blob hashes.
2. Tree entries sorted lexicographically.
3. Canonical encoding is stable across:
    - OS
    - architecture
    - library versions (within reason)
Hashing is always content-based, independent of file metadata.

# Workspace changes & operation plans

Some commands like restore and switch modify the workspace.

Rules:
1. Commands build a “plan” describing which files will be overwritten or deleted.
2. UI displays the plan for confirmation (prompt for each question)
3. Core executes the plan transactionally (if it modifies DB).

This ensures safe, predictable workspace manipulation.

# Testing Architecture (future)

Even if implemented later, testing rules must be stable:
1. Unit tests for utils, DB modules, pure core logic.
2. Integration tests for commands + core + FS + DB.
3. End-to-end tests to validate command behavior and UI output.
