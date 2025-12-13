use clap::Parser;
use revius::cli::args::{Cli, Commands};
use revius::cli::ui;
use revius::commands;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init(args) => commands::init::run(args),
        Commands::Add(args) => commands::add::run(args),
        Commands::Commit(args) => commands::commit::run(args),
    };

    if let Err(e) = result {
        ui::print_error(&e.to_string());
        std::process::exit(e.exit_code());
    }
}