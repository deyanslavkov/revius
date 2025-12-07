use colored::Colorize;
use std::path::Path;

pub fn print_init_success(path: &Path) {
    println!(
        "{} Initialized empty Revius repository in {}",
        "✓".green().bold(),
        path.display()
    );
}

pub fn print_repo_already_exists(path: &Path) {
    eprintln!(
        "{} Repository already exists at {}",
        "✗".red().bold(),
        path.display()
    );
}

pub fn print_error(msg: &str) {
    eprintln!("{} {}", "Error:".red().bold(), msg);
}