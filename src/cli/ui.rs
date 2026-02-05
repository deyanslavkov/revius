use colored::Colorize;
use std::path::Path;
use crate::utils;
use crate::core::models::objects::{StatusInfo, CommitInfo, LogOptions, ReflogEntry, HeadState};
use crate::utils::hash::hash_to_short_hex;
use crate::utils::{hash, time};
use crate::core::merge::{ConflictType, MergeConflict};
use crate::core::gc::GcStats;

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

pub fn print_deleted_file(path: &str) {
    println!("{} {}", "-".red().bold(), path);
}

pub fn print_warn(msg: &str) {
    eprintln!("{} {}", "Warning:".yellow().bold(), msg);
}

pub fn print_add_summary(added: u64, changed: u64, deleted: u64, unchanged: u64, blobs: u64) {
    println!(
        "\n{} {} added, {} changed, {} deleted, {} unchanged, {} blob insertions",
        "✓".green().bold(),
        added,
        changed,
        deleted,
        unchanged,
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

pub fn print_reflog(entries: &[ReflogEntry]) {
    if entries.is_empty() {
        println!("Reflog is empty.");
        return;
    }

    for (i, entry) in entries.iter().enumerate() {
        let old_short = if let Some(h) = entry.old_hash {
            hash::hash_to_short_hex(&h)
        } else {
            "00000000".to_string()
        };

        let new_short = if let Some(h) = entry.new_hash {
            hash::hash_to_short_hex(&h)
        } else {
            "00000000".to_string()
        };

        println!(
            "{} {} ({} -> {}): {}",
            entry.ref_path.cyan(),
            format!("[{}]", i).dimmed(),
            old_short.red(),
            new_short.green(),
            entry.action
        );
    }
}

pub fn print_commit_detailed(commit: &CommitInfo) {
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

pub fn print_commit_oneline(commit: &CommitInfo) {
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
        println!("{} ({}) {}", short_hash.yellow(), ref_display.join(", ").cyan(), message_first_line);
    } else {
        println!("{} {}", short_hash.yellow(), message_first_line);
    }
}

/// A "true" ASCII graph renderer.
/// Tracks lanes of parent commits to draw connections.
pub fn print_commit_graph(commits: &[CommitInfo], oneline: bool) {
    // Tracks active parent hashes in each visual lane.
    // None means the lane is empty/reserved (rare in simple view).
    let mut lanes: Vec<[u8; 32]> = Vec::new();
    
    // Simple color cycling for graph lanes
    let colors = ["red", "green", "yellow", "blue", "magenta", "cyan"];

    for commit in commits {
        // 1. Find the lane containing this commit (if any)
        // If the commit is not in any lane (e.g., a branch tip we just encountered), 
        // we assign it a new lane (append).
        let lane_idx = lanes.iter().position(|&h| h == commit.hash).unwrap_or(lanes.len());

        // If it's a new tip, ensure we expand lanes
        if lane_idx == lanes.len() {
            lanes.push(commit.hash);
        }

        // 2. Prepare the node line (the "*")
        let mut graph_str = String::new();
        for (i, _hash) in lanes.iter().enumerate() {
            let color = colors[i % colors.len()];
            if i == lane_idx {
                graph_str.push_str(&"* ".color(color).to_string());
            } else {
                graph_str.push_str(&"| ".color(color).to_string());
            }
        }

        // 3. Print the commit info
        let short_hash = hash::hash_to_short_hex(&commit.hash);
        let msg = commit.message.lines().next().unwrap_or("");
        let refs = if !commit.refs.is_empty() {
             let names: Vec<String> = commit.refs.iter().map(|r| {
                if let Some(branch) = r.strip_prefix("refs/heads/") {
                    branch.to_string()
                } else {
                    r.to_string()
                }
            }).collect();
            format!(" ({})", names.join(", ").cyan())
        } else {
            "".to_string()
        };

        if oneline {
            println!("{}{} {}{}", graph_str, short_hash.yellow(), msg, refs);
        } else {
            println!("{}{} {}{}", graph_str, "commit".yellow(), short_hash, refs);
            // Print metadata with lane pipes
            let mut prefix = String::new();
            for (i, _) in lanes.iter().enumerate() {
                let color = colors[i % colors.len()];
                prefix.push_str(&"| ".color(color).to_string());
            }
            println!("{}Author: {} <{}>", prefix, commit.author_name, commit.author_email);
            println!("{}Date:   {}", prefix, time::format_timestamp(commit.timestamp));
            println!("{}", prefix); // Empty line
            println!("{}{}", prefix, msg); // Message (assumes 1 line for simplicity, or iterate lines)
            println!("{}", prefix); // Padding
        }

        // 4. Update lanes for next iteration (the connections)
        // Logic:
        // - The current commit is consumed.
        // - Its First Parent replaces it in the current lane.
        // - Its Second Parent (merge) is inserted/appended.
        
        // We capture parents before modifying lanes
        let p1 = commit.parent_hash;
        let p2 = commit.merge_parent_hash;

        // Visual connector logic (Simplified for robustness):
        // If we split (merge commit): Draw `| \` on next line?
        // If we merge (lanes converge): Draw `| /` ?
        // For now, we update the state. The "jump" in ASCII often suffices, 
        // but let's try to print a connector row if we have a split (2 parents).
        
        if let Some(parent1) = p1 {
            // Replace current with parent 1
            lanes[lane_idx] = parent1;
            
            if let Some(parent2) = p2 {
                // Merge Commit: We have a second parent.
                // We need to insert a new lane for parent2.
                // Standard git behavior: insert it immediately after current lane.
                lanes.insert(lane_idx + 1, parent2);
                
                // Draw connector: `| \`
                let mut conn_str = String::new();
                for (i, _) in lanes.iter().enumerate() {
                    let color = colors[i % colors.len()];
                    if i == lane_idx {
                         conn_str.push_str(&"| ".color(color).to_string()); 
                    } else if i == lane_idx + 1 {
                         conn_str.push_str(&"\\ ".color(color).to_string());
                    } else {
                         conn_str.push_str(&"| ".color(color).to_string());
                    }
                }
                println!("{}", conn_str);
            }
        } else {
            // No parents (Root), remove the lane
            lanes.remove(lane_idx);
            
            // Draw connector: `|` closing up (shifted left)
            // Actually, just leaving it empty next loop handles it visually.
        }
        
        // Handling "Lane Merging": 
        // If multiple lanes now point to the same parent hash (branches rejoining), 
        // we should deduplicate them.
        // E.g. Lane 0: HashA, Lane 1: HashA.
        // Real `git graph` draws a `|/` here.
        // Simplifying: We just iterate and see if duplicates exist.
        // If `lanes` has duplicates, we keep the leftmost one and remove the others.
        
        let mut dedup_lanes = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut removal_indices = Vec::new();

        for (i, hash) in lanes.iter().enumerate() {
            if !seen.insert(*hash) {
                removal_indices.push(i);
            } else {
                dedup_lanes.push(*hash);
            }
        }
        
        if !removal_indices.is_empty() {
             // If we removed lanes, we technically should draw a connector `|/`.
             // But strictly updating the lanes works for "Good enough" ASCII.
             lanes = dedup_lanes;
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
            println!("* {}", name.green());
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

pub fn print_switch_success(
    previous: &HeadState,
    new: &HeadState,
    files_changed: usize,
    files_deleted: usize,
) {
    match previous {
        HeadState::Branch(name, _) => {
            print!("Switched from branch '{}' ", name);
        }
        HeadState::Detached(hash) => {
            print!("Switched from detached HEAD at {} ", hash::hash_to_short_hex(hash));
        }
    }

    match new {
        HeadState::Branch(name, hash) => {
            println!("to branch '{}' ({})", name, hash::hash_to_short_hex(hash));
        }
        HeadState::Detached(hash) => {
            println!("to commit {}", hash::hash_to_short_hex(hash));
            println!("You are in 'detached HEAD' state.");
        }
    }

    if files_changed > 0 || files_deleted > 0 {
        println!(
            "{} file(s) changed, {} file(s) deleted",
            files_changed, files_deleted
        );
    }
}

pub fn print_branch_created_and_switched(branch_name: &str, commit_hash: &[u8; 32]) {
    println!(
        "Switched to a new branch '{}' at commit {}",
        branch_name,
        hash::hash_to_short_hex(commit_hash)
    );
}

pub fn print_merge_fast_forward(from: &[u8; 32], to: &[u8; 32]) {
    println!(
        "Fast-forward merge from {} to {}",
        crate::utils::hash::hash_to_short_hex(from),
        crate::utils::hash::hash_to_short_hex(to)
    );
}

pub fn print_merge_already_up_to_date() {
    println!("Already up to date.");
}

pub fn print_merge_success(commit_hash: &[u8; 32], files_changed: usize) {
    println!(
        "Merge commit created: {} ({} file(s) in merged tree)",
        crate::utils::hash::hash_to_short_hex(commit_hash),
        files_changed
    );
}

pub fn print_merge_conflicts(conflicts: &[MergeConflict]) {
    eprintln!("Merge conflicts detected in {} file(s):", conflicts.len());
    eprintln!();
    
    for conflict in conflicts {
        let conflict_desc = match conflict.conflict_type {
            ConflictType::BothModified => "both modified",
            ConflictType::DeletedByUsModifiedByThem => "deleted by us, modified by them",
            ConflictType::DeletedByThemModifiedByUs => "deleted by them, modified by us",
            ConflictType::BothAdded => "both added (different content)",
        };
        eprintln!("  {} ({})", conflict.path, conflict_desc);
    }
    
    eprintln!();
    eprintln!("Automatic merge failed. Please resolve conflicts manually.");
}

pub fn print_reset_success(mode: &str, commit_hash: &[u8; 32]) {
    println!(
        "HEAD is now at {} ({} reset)",
        hash::hash_to_short_hex(commit_hash).yellow(),
        mode
    );
}

pub fn print_restore_success(mode: &str, count: usize) {
    if count == 0 {
        println!("No files were changed.");
    } else {
        println!(
            "Restored {} file(s) ({})",
            count.to_string().yellow(),
            mode
        );
    }
}

pub fn print_gc_start(dry_run: bool) {
    if dry_run {
        println!("{}", "Running garbage collection (dry run)...".blue());
    } else {
        println!("{}", "Running garbage collection...".blue());
    }
}

pub fn print_gc_stats(stats: &GcStats) {
    println!("\nGarbage collection completed.");
    println!("Objects prune statistics:");
    println!("  - Commits deleted: {}", stats.commits_deleted.to_string().red());
    println!("  - Trees deleted:   {}", stats.trees_deleted.to_string().red());
    println!("  - Files deleted:   {}", stats.files_deleted.to_string().red());
    println!("  - Blobs deleted:   {}", stats.blobs_deleted.to_string().red());
    
    if stats.commits_deleted == 0 && stats.trees_deleted == 0 && stats.files_deleted == 0 && stats.blobs_deleted == 0 {
        println!("{}", "The repository is already optimized.".green());
    }
}

pub fn print_config_set_success(key: &str, value: &str, scope: &str) {
    println!(
        "{} Set {} to '{}' ({})",
        "✓".green().bold(),
        key.bold(),
        value,
        scope
    );
}

pub fn print_user_setup_success(name: &str, email: &str) {
    println!(
        "{} Configured global user identity:\n  Name:  {}\n  Email: {}",
        "✓".green().bold(),
        name.bold(),
        email.bold()
    );
}
