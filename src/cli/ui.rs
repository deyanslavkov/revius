use colored::Colorize;
use std::path::Path;
use crate::utils;
use crate::core::models::objects::StatusInfo;
use crate::utils::hash::hash_to_short_hex;

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
    println!("{} file(s) committed", files_changed);
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

pub fn print_detached_head_warning(commit_hash: &[u8; 32]) {
    let short_hash = utils::hash::hash_to_short_hex(&commit_hash);

    eprintln!("Warning: You are in a detached HEAD state");
    eprintln!();
    eprintln!("You are currently not on any branch.");
    eprintln!("If you commit now, the commit will not belong to any branch.");
    eprintln!();
    eprintln!("Current commit: {}", short_hash);
    eprintln!();
    eprintln!("To retain this commit, consider creating a branch:");
    eprintln!("rvs branch <name>");
}

pub fn print_status(status: &StatusInfo) {
    if let Some(ref branch) = status.branch_name {
        println!("On branch {}", branch);
    } else if let Some(commit_hash) = status.detached_commit {
        println!("HEAD detached at {}", hash_to_short_hex(&commit_hash));
    }

    println!();

    if status.has_staged_changes() {
        println!("Changes to be committed:");
        println!("  (use \"rvs reset <file>...\" to unstage)");
        println!();

        for path in &status.staged_new {
            println!("        \x1b[32mnew file:   {}\x1b[0m", path);
        }
        for path in &status.staged_modified {
            println!("        \x1b[32mmodified:   {}\x1b[0m", path);
        }
        for path in &status.staged_deleted {
            println!("        \x1b[32mdeleted:    {}\x1b[0m", path);
        }

        println!();
    }

    if !status.unstaged_modified.is_empty() || !status.unstaged_deleted.is_empty() {
        println!("Changes not staged for commit:");
        println!("  (use \"rvs add <file>...\" to update what will be committed)");
        println!("  (use \"rvs restore <file>...\" to discard changes in working directory)");
        println!();

        for path in &status.unstaged_modified {
            println!("        \x1b[31mmodified:   {}\x1b[0m", path);
        }
        for path in &status.unstaged_deleted {
            println!("        \x1b[31mdeleted:    {}\x1b[0m", path);
        }

        println!();
    }

    if !status.untracked.is_empty() {
        println!("Untracked files:");
        println!("  (use \"rvs add <file>...\" to include in what will be committed)");
        println!();

        for path in &status.untracked {
            println!("        \x1b[31m{}\x1b[0m", path);
        }

        println!();
    }

    if !status.has_changes() {
        println!("nothing to commit, working tree clean");
    } else if !status.has_staged_changes() {
        if !status.untracked.is_empty() {
            println!("no changes added to commit (use \"rvs add\" and/or \"rvs commit\")");
        } else {
            println!("no changes added to commit (use \"rvs add\")");
        }
    }
}