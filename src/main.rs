mod cli;
mod commands;
mod core;
mod db;
mod fs;

use clap::Parser;
use colored::Colorize;
use cli::args::Cli;
use commands::init as init_cmd;

fn main() {
    let cli = cli::args::parse_args();

    match &cli.command {
        Some(cli::args::Commands::Init { root }) => {
            let root_path = root.as_deref().unwrap_or(std::path::Path::new("."));
            match init_cmd::run(root_path) {
                Ok(()) => std::process::exit(0),
                Err(err) => {
                    // AlreadyInitialized is considered success in CLI surface
                    use core::repository::init::InitErrorKind;
                    match err {
                        crate::error::ReviusError::AlreadyInitialized(_) => {
                            println!("{}", format!("Repository already exists at {}", root_path.display()).yellow());
                            std::process::exit(0)
                        }
                        _ => {
                            eprintln!("{} {}", "error:".red(), err);
                            std::process::exit(1)
                        }
                    }
                }
            }
        }
        None => {
            println!("{}", "No command given. Use `revius --help`".yellow());
            std::process::exit(0);
        }
    }
}
