use colored::Colorize;
use std::path::Path;

pub fn print_init_success(path: &Path) {
    println!(
        "{} Initialized empty Revius repository in {}",
        "✓".green().bold(),
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

pub fn print_commit_success(hash: &[u8; 32], message: &str, files_changed: usize) {
    let short_hash = hex::encode(&hash[..8]);
    let first_line = message.lines().next().unwrap_or("");
    println!("[{}] {}", short_hash, first_line);
    println!("{} file(s) changed", files_changed);
}

pub fn print_nothing_to_commit() {
    println!("Nothing to commit (staging area is empty)");
    println!("Use 'revius add <file>' to add files to the staging area");
}

pub fn print_no_user_configured() {
    eprintln!("Error: User name and email not configured");
    eprintln!();
    eprintln!("Please configure your identity:");
    eprintln!("  Edit your user config file with:");
    eprintln!();
    eprintln!("  [user]");
    eprintln!("  name = \"Your Name\"");
    eprintln!("  email = \"your.email@example.com\"");
    eprintln!();
    
    #[cfg(target_os = "windows")]
    eprintln!("  Config location: %APPDATA%\\revius\\config.toml");
    
    #[cfg(not(target_os = "windows"))]
    eprintln!("  Config location: ~/.config/revius/config.toml");
}