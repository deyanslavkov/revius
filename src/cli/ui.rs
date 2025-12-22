use colored::Colorize;
use std::path::Path;
use crate::utils;
use crate::core::models::objects::{StatusInfo, CommitInfo, LogOptions};
use crate::utils::hash::hash_to_short_hex;
use crate::utils::{hash, time};

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

pub fn print_no_commits() {
    println!("No commits yet");
}

/// Print commit history based on options
pub fn print_log(commits: &[CommitInfo], options: &LogOptions) {
    if options.show_graph {
        print_commit_graph(commits, options.oneline);
    } else if options.oneline {
        for commit in commits {
            print_commit_oneline(commit);
        }
    } else {
        for commit in commits {
            print_commit_detailed(commit);
        }
    }
}

fn print_commit_detailed(commit: &CommitInfo) {
    print!("commit {}", hash::hash_to_hex(&commit.hash));

    if !commit.refs.is_empty() {
        print!(" (");
        for (i, ref_path) in commit.refs.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            let display_name = if let Some(branch) = ref_path.strip_prefix("refs/heads/") {
                branch
            } else if let Some(tag) = ref_path.strip_prefix("refs/tags/") {
                tag
            } else {
                ref_path
            };
            print!("{}", display_name);
        }
        print!(")");
    }
    println!();

    if let (Some(parent), Some(merge_parent)) = (commit.parent_hash, commit.merge_parent_hash) {
        println!(
            "Merge: {} {}",
            hash::hash_to_short_hex(&parent),
            hash::hash_to_short_hex(&merge_parent)
        );
    }

    println!("Author: {} <{}>", commit.author_name, commit.author_email);

    println!("Date:   {}", time::format_timestamp(commit.timestamp));

    println!();
    for line in commit.message.lines() {
        println!("    {}", line);
    }
    println!();
}

fn print_commit_oneline(commit: &CommitInfo) {
    let short_hash = hash::hash_to_short_hex(&commit.hash);

    let message_first_line = commit.message.lines().next().unwrap_or("");

    if !commit.refs.is_empty() {
        let ref_display: Vec<String> = commit.refs.iter().map(|r| {
            if let Some(branch) = r.strip_prefix("refs/heads/") {
                branch.to_string()
            } else if let Some(tag) = r.strip_prefix("refs/tags/") {
                tag.to_string()
            } else {
                r.to_string()
            }
        }).collect();
        println!("{} ({}) {}", short_hash, ref_display.join(", "), message_first_line);
    } else {
        println!("{} {}", short_hash, message_first_line);
    }
}

/// For now implements a simple linear graph. Future enhancement: proper graph with branches
fn print_commit_graph(commits: &[CommitInfo], oneline: bool) {
    for (i, commit) in commits.iter().enumerate() {
        let short_hash = hash::hash_to_short_hex(&commit.hash);
        let message_first_line = commit.message.lines().next().unwrap_or("");

        let graph_char = if i == 0 { "*" } else { "*" };

        let refs_display = if !commit.refs.is_empty() {
            let ref_names: Vec<String> = commit.refs.iter().map(|r| {
                if let Some(branch) = r.strip_prefix("refs/heads/") {
                    branch.to_string()
                } else if let Some(tag) = r.strip_prefix("refs/tags/") {
                    tag.to_string()
                } else {
                    r.to_string()
                }
            }).collect();
            format!(" ({})", ref_names.join(", "))
        } else {
            String::new()
        };

        if oneline {
            println!("{} {}{} {}", graph_char, short_hash, refs_display, message_first_line);
        } else {
            println!("{} commit {}{}", graph_char, hash::hash_to_hex(&commit.hash), refs_display);
            if let (Some(parent), Some(merge_parent)) = (commit.parent_hash, commit.merge_parent_hash) {
                println!("|\\  Merge: {} {}", 
                    hash::hash_to_short_hex(&parent),
                    hash::hash_to_short_hex(&merge_parent)
                );
            }
            println!("| Author: {} <{}>", commit.author_name, commit.author_email);
            println!("| Date:   {}", time::format_timestamp(commit.timestamp));
            println!("|");
            for line in commit.message.lines() {
                println!("|     {}", line);
            }
            println!("|");
        }

        if i < commits.len() - 1 && !oneline {
            if commits[i + 1].merge_parent_hash.is_some() {
                println!("|\\");
            } else {
                println!("|");
            }
        }
    }
}

/// Print a list of branches with the current one marked
pub fn print_branch_list(branches: &[(String, [u8; 32], bool)]) {
    if branches.is_empty() {
        println!("No branches found.");
        return;
    }

    for (name, _hash, is_current) in branches {
        if *is_current {
            println!("* {}", name);
        } else {
            println!("  {}", name);
        }
    }
}

pub fn print_branch_created(branch_name: &str, commit_hash: &[u8; 32]) {
    println!(
        "Branch '{}' created at commit {}",
        branch_name,
        hash::hash_to_short_hex(commit_hash)
    );
}

pub fn print_branch_renamed(old_name: &str, new_name: &str) {
    println!("Branch '{}' renamed to '{}'", old_name, new_name);
}

pub fn print_branch_deleted(branch_name: &str, commit_hash: &[u8; 32]) {
    println!(
        "Deleted branch '{}' (was at {})",
        branch_name,
        hash::hash_to_short_hex(commit_hash)
    );
}

pub fn print_current_branch(branch_name: &str) {
    println!("{}", branch_name);
}

pub fn print_detached_head_branch_warning(commit_hash: &[u8; 32]) {
    println!(
        "Not currently on any branch (detached HEAD at {})",
        hash::hash_to_short_hex(commit_hash)
    );
}

pub fn print_no_branches() {
    println!("No branches found.");
}