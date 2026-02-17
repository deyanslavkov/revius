# Revius (rvs)

Revius is a version control system implemented in Rust. It is a learning-focused diploma project, providing all essential features and more.

## Project Goals

- Understand core concepts behind version control systems.
- Implement clean, maintainable, and well-structured Rust code.
- Explore hybrid ideas inspired by multiple VCS tools.
- Provide a system usable in practice.

## Technology Stack

- Programming Language: Rust
- Persistence layer: SQLite
- Algorithms used: BLAKE3, Zstd, FastCDC

## Status

Version 1.0 is out.

## Build & Run

```
git clone https://github.com/deyanslavkov/revius.git
cd revius
cargo build
cargo run -- help
Or with the binary:
rvs help
```

## `rvs help` output:

```
$ rvs help

A content-addressed, single-file repository, lightweight VCS

Usage: rvs.exe <COMMAND>

Commands:
  init     Initialize a new Revius repository
  add      Add file contents to the staging area
  commit   Record changes to the repository
  status   Show the working tree status
  log      Show commit history
  reflog   Manage reflog information
  branch   List, create, rename, or delete branches
  switch   Switch branches or restore working tree files
  merge    Join two development histories together
  reset    Reset current HEAD to the specified state
  restore  Restore working tree files
  gc       Cleanup unnecessary files and optimize the local repository
  config   Get and set repository or global options
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```
