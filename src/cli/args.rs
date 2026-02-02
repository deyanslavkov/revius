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

    #[command(about = "Cleanup unnecessary files and optimize the local repository")]
    Gc(GcArgs),
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

#[derive(Parser)]
pub struct StatusArgs {
    // Currently no arguments, but can add --short, --verbose, etc. later
}

#[derive(Parser)]
pub struct LogArgs {
    #[arg(short = 'n', long, help = "Limit number of commits to show")]
    pub limit: Option<usize>,
    
    #[arg(long, help = "Show commit graph with ASCII art")]
    pub graph: bool,
    
    #[arg(long, help = "Show each commit on a single line")]
    pub oneline: bool,
    
    #[arg(long, help = "Show only the first parent in merge commits")]
    pub first_parent: bool,
}

#[derive(Parser)]
pub struct BranchArgs {
    #[arg(help = "Branch name to create, or the first branch name when renaming/deleting")]
    pub name: Option<String>,

    #[arg(short = 'm', long, help = "Rename a branch")]
    pub rename: bool,

    #[arg(short = 'd', long, help = "Delete a branch")]
    pub delete: bool,

    #[arg(short = 'D', long, help = "Force delete a branch")]
    pub force_delete: bool,

    #[arg(help = "New name when renaming (optional second argument)")]
    pub new_name: Option<String>,
}

#[derive(Parser)]
pub struct SwitchArgs {
    #[arg(help = "Branch name or commit hash to switch to")]
    pub target: String,

    #[arg(short = 'c', long, help = "Create new branch from current state and switch to it")]
    pub create: bool,

    #[arg(short = 'f', long, help = "Force switch, discarding local changes")]
    pub force: bool,
}

#[derive(Parser)]
pub struct MergeArgs {
    #[arg(help = "Branch name or commit hash to merge")]
    pub target: String,
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

#[derive(Parser)]
pub struct GcArgs {
    #[arg(long, help = "Do not delete anything, just show what would be deleted")]
    pub dry_run: bool,
}