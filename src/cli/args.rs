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