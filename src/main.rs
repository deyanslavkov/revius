use clap::Parser;
use revius::cli::args::{Cli, Commands};
use revius::cli::ui;
use revius::commands;

pub fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init(args) => commands::init::run(args),
        Commands::Add(args) => commands::add::run(args),
        Commands::Commit(args) => commands::commit::run(args),
        Commands::Status(args) => commands::status::run(args),
        Commands::Log(args) => commands::log::run(args),
        Commands::Branch(args) => commands::branch::run(args),
        Commands::Switch(args) => commands::switch::run(args),
        Commands::Merge(args) => commands::merge::run(args),
        Commands::Reset(args) => commands::reset::run(args),
        Commands::Restore(args) => commands::restore::run(args),
        Commands::Gc(args) => commands::gc::run(args),
    };

    if let Err(e) = result {
        ui::print_error(&e.to_string());
        std::process::exit(e.exit_code());
    }
}