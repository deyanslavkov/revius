use colored::Colorize;
use revius:: cli;
use revius::commands::init as init_cmd;
use revius::error::ReviusError;


fn main() {

    let cli = cli::args::parse_args();

    match &cli.command {
        Some(cli::args::Commands::Init { root }) => {
            let root_path = root.as_deref().unwrap_or(std::path::Path::new("."));
            match init_cmd::run(root_path) {
                Ok(()) => std::process::exit(0),
                Err(err) => {
                    // AlreadyInitialized is considered success in CLI surface
                    match err {
                        ReviusError::AlreadyInitialized(_) => {
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
