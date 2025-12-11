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

pub fn print_added_file(path: &str) {
    println!("{} {}", "+".green().bold(), path);
}

pub fn print_modified_file(path: &str) {
    println!("{} {}", "M".yellow().bold(), path);
}

pub fn print_warn(msg: &str) {
    eprintln!("{} {}", "Warning:".yellow().bold(), msg);
}

pub fn print_add_summary(added: u64, skipped: u64, blobs: u64) {
    println!(
        "\n{} {} added, {} unchanged, {} blob insertions",
        "✓".green().bold(),
        added,
        skipped,
        blobs
    );
}