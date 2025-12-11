use clap::Parser;
use revius::cli::args::{Cli, Commands};
use revius::cli::ui;
use revius::commands;
use revius::error::ReviusError;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init(args) => commands::init::run(args),
        Commands::Add(args) => commands::add::run(args),
    };

    if let Err(e) = result {
        match e {
            ReviusError::RepoAlreadyExists(path) => {
                // Print the friendly repo-exists message and exit non-zero
                ui::print_repo_already_exists(&path);
                std::process::exit(1);
            }
            other => {
                ui::print_error(&other.to_string());
                std::process::exit(1);
            }
        }
    }
}