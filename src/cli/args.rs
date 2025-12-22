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