use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "revius", version = "0.1", about = "Revius VCS")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new Revius repository
    Init {
        /// Root directory (default: current directory)
        #[arg(short, long)]
        root: Option<PathBuf>,
    },
}

pub fn parse_args() -> Cli {
    Cli::parse()
}
