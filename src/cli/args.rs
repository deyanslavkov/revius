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