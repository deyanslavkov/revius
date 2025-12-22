use crate::cli::args::BranchArgs;
use crate::cli::ui;
use crate::core;
use crate::error::ReviusError;
use crate::fs;
use crate::core::models::repository::Repository;

pub fn run(args: BranchArgs) -> Result<(), ReviusError> {
    let flag_count = [args.rename, args.delete, args.force_delete]
        .iter()
        .filter(|&&x| x)
        .count();

    if flag_count > 1 {
        return Err(ReviusError::Usage(
            "Cannot combine -m, -d, and -D flags".to_string(),
        ));
    }

    let current_dir = fs::paths::get_current_dir()?;
    let repo = core::open::open_repository(&current_dir)?;

    if args.rename {
        handle_rename(&repo, args)
    } else if args.delete {
        handle_delete(&repo, args, false)
    } else if args.force_delete {
        handle_delete(&repo, args, true)
    } else if args.name.is_some() {
        handle_create(&repo, args)
    } else {
        handle_list(&repo)
    }
}

/// Handle branch creation: rvs branch <name>
fn handle_create(repo: &Repository, args: BranchArgs) -> Result<(), ReviusError> {
    let branch_name = args.name.unwrap(); // Safe because we checked it's Some

    if args.new_name.is_some() {
        return Err(ReviusError::Usage(
            "Too many arguments for branch creation. Usage: rvs branch <branch_name>".to_string(),
        ));
    }

    let commit_hash = core::branch::create_branch(repo, &branch_name)?;
    ui::print_branch_created(&branch_name, &commit_hash);

    Ok(())
}

/// Handle branch listing: rvs branch
fn handle_list(repo: &Repository) -> Result<(), ReviusError> {
    let branches = core::branch::list_branches(repo)?;

    if branches.is_empty() {
        ui::print_no_branches();
    } else {
        ui::print_branch_list(&branches);
    }

    Ok(())
}

/// Handle branch rename: rvs branch -m [old_name] <new_name>
fn handle_rename(repo: &Repository, args: BranchArgs) -> Result<(), ReviusError> {
    let (old_name, new_name) = match (args.name, args.new_name) {
        (Some(old), Some(new)) => (Some(old), new),
        (Some(new), None) => (None, new),
        (None, _) => {
            return Err(ReviusError::Usage(
                "Branch name required for rename. Usage: rvs branch -m [<old_name>] <new_name>"
                    .to_string(),
            ));
        }
    };

    let (old_ref_path, _new_ref_path) =
        core::branch::rename_branch(repo, old_name.as_deref(), &new_name)?;

    let old_display_name = old_ref_path
        .strip_prefix("refs/heads/")
        .unwrap_or(&old_ref_path);

    ui::print_branch_renamed(old_display_name, &new_name);

    Ok(())
}

/// Handle branch deletion: rvs branch -d/-D <name>
fn handle_delete(repo: &Repository, args: BranchArgs, force: bool) -> Result<(), ReviusError> {
    let branch_name = args.name.ok_or_else(|| {
        ReviusError::Usage("Branch name required for deletion. Usage: rvs branch -d <branch_name>".to_string())
    })?;

    if args.new_name.is_some() {
        return Err(ReviusError::Usage(
            "Too many arguments for branch deletion. Usage: rvs branch -d <branch_name>"
                .to_string(),
        ));
    }

    let commit_hash = core::branch::delete_branch(repo, &branch_name, force)?;
    ui::print_branch_deleted(&branch_name, &commit_hash);

    Ok(())
}