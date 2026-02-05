use crate::core::models::repository::Repository;
use crate::core::refs::{get_head_state, update_head_to_branch, HeadState};
use crate::db;
use crate::error::ReviusError;
use crate::utils::{hash, validation};
use rusqlite::Transaction;

pub fn branch_ref_path(branch_name: &str) -> String {
    format!("refs/heads/{}", branch_name)
}

pub fn extract_branch_name(ref_path: &str) -> Result<String, ReviusError> {
    ref_path
        .strip_prefix("refs/heads/")
        .ok_or_else(|| ReviusError::Db(format!("Invalid branch ref path: {}", ref_path)))
        .map(|s| s.to_string())
}

/// Create a branch within an existing transaction. This is used by switch -c to create and switch in one transaction
pub fn create_branch_in_tx(
    tx: &Transaction,
    branch_name: &str,
    commit_hash: &[u8; 32],
) -> Result<(), ReviusError> {
    validation::validate_branch_name(branch_name)?;

    if !db::commits::commit_exists(tx, commit_hash)? {
        return Err(ReviusError::CommitNotFound(
            hash::hash_to_hex(commit_hash)
        ));
    }
    
    let branch_ref = branch_ref_path(branch_name);
    if db::refs::ref_exists(tx, &branch_ref)? {
        return Err(ReviusError::BranchAlreadyExists(branch_name.to_string()));
    }
    
    db::refs::upsert_ref(tx, &branch_ref, 0, commit_hash)?;
    
    Ok(())
}

/// Create a new branch at the current commit. Returns the commit hash where the branch was created
pub fn create_branch(repo: &Repository, branch_name: &str) -> Result<[u8; 32], ReviusError> {
    validation::validate_branch_name(branch_name)?;

    let ref_path = branch_ref_path(branch_name);

    if db::refs::ref_exists(&repo.conn, &ref_path)? {
        return Err(ReviusError::BranchAlreadyExists(branch_name.to_string()));
    }

    let current_commit = db::refs::resolve_head(&repo.conn)?
        .ok_or_else(|| ReviusError::NoCommitsYet)?;

    let tx = repo.conn.unchecked_transaction().map_err(|e| {
        ReviusError::Db(format!("Failed to begin transaction for branch creation: {}", e))
    })?;

    db::refs::upsert_ref(&tx, &ref_path, 0, &current_commit)?;

    let action = format!("branch: Created from {}", hash::hash_to_short_hex(&current_commit));
    db::reflog::insert_reflog(&tx, &ref_path, None, Some(&current_commit), &action)?;

    tx.commit().map_err(|e| {
        ReviusError::Db(format!("Failed to commit transaction for branch creation: {}", e))
    })?;

    Ok(current_commit)
}

/// Rename a branch. If old_name is None, renames the current branch. Returns (old_ref_path, new_ref_path)
pub fn rename_branch(repo: &Repository, old_name: Option<&str>, new_name: &str) -> Result<(String, String), ReviusError> {
    validation::validate_branch_name(new_name)?;

    let old_branch_name = match old_name {
        Some(name) => name.to_string(),
        None => {
            get_current_branch_name(repo)?.ok_or_else(|| {
                match db::refs::resolve_head(&repo.conn) {
                    Ok(Some(commit_hash)) => ReviusError::DetachedHead(hash::hash_to_short_hex(&commit_hash)),
                    _ => ReviusError::DetachedHead("unknown".to_string()),
                }
            })?
        }
    };

    let old_ref_path = branch_ref_path(&old_branch_name);
    let new_ref_path = branch_ref_path(new_name);

    if !db::refs::ref_exists(&repo.conn, &old_ref_path)? {
        return Err(ReviusError::BranchNotFound(old_branch_name));
    }

    if db::refs::ref_exists(&repo.conn, &new_ref_path)? {
        return Err(ReviusError::BranchAlreadyExists(new_name.to_string()));
    }

    let commit_hash = db::refs::get_ref(&repo.conn, &old_ref_path)?
        .ok_or_else(|| ReviusError::BranchNotFound(old_branch_name.clone()))?;

    let is_current = get_current_branch_name(repo)?
        .map_or(false, |name| name == old_branch_name);

    let tx = repo.conn.unchecked_transaction().map_err(|e| {
        ReviusError::Db(format!("Failed to begin transaction for branch rename: {}", e))
    })?;

    db::refs::delete_ref(&tx, &old_ref_path)?;

    db::refs::upsert_ref(&tx, &new_ref_path, 0, &commit_hash)?;

    if is_current {
        update_head_to_branch(&tx, new_name)?;
    }

    let action_old = format!("branch: renamed to {}", new_name);
    let action_new = format!("branch: renamed from {}", old_branch_name);
    db::reflog::insert_reflog(&tx, &old_ref_path, Some(&commit_hash), None, &action_old)?;
    db::reflog::insert_reflog(&tx, &new_ref_path, None, Some(&commit_hash), &action_new)?;

    tx.commit().map_err(|e| {
        ReviusError::Db(format!("Failed to commit transaction for branch rename: {}", e))
    })?;

    Ok((old_ref_path, new_ref_path))
}

/// Delete a branch with safety checks (can't delete current, can't delete if unmerged). Returns the commit hash where the branch pointed
pub fn delete_branch(repo: &Repository, branch_name: &str, force: bool) -> Result<[u8; 32], ReviusError> {
    let ref_path = branch_ref_path(branch_name);

    if !db::refs::ref_exists(&repo.conn, &ref_path)? {
        return Err(ReviusError::BranchNotFound(branch_name.to_string()));
    }

    if let Some(current) = get_current_branch_name(repo)? {
        if current == branch_name {
            return Err(ReviusError::CannotDeleteCurrentBranch(
                branch_name.to_string(),
            ));
        }
    }

    let commit_hash = db::refs::get_ref(&repo.conn, &ref_path)?
        .ok_or_else(|| ReviusError::BranchNotFound(branch_name.to_string()))?;

    // Check if merged (Safety)
    if !force {
        // Resolve HEAD (should exist if we have branches)
        let head_commit = db::refs::resolve_head(&repo.conn)?
             .ok_or(ReviusError::NoCommitsYet)?;

        // If the branch tip is an ancestor of HEAD, it is fully merged.
        // We reuse the merge-base logic.
        match crate::core::merge::find_merge_base(&repo.conn, head_commit, commit_hash)? {
            Some(base) if base == commit_hash => {
                // Fully merged, allow delete
            },
            _ => {
                return Err(ReviusError::Usage(format!(
                    "The branch '{}' is not fully merged. If you are sure you want to delete it, run 'rvs branch -D {}'.",
                    branch_name, branch_name
                )));
            }
        }
    }

    let tx = repo.conn.unchecked_transaction().map_err(|e| {
        ReviusError::Db(format!("Failed to begin transaction for branch deletion: {}", e))
    })?;

    db::refs::delete_ref(&tx, &ref_path)?;

    let action = format!(
        "branch: deleted (was at {})",
        hash::hash_to_short_hex(&commit_hash)
    );
    db::reflog::insert_reflog(&tx, &ref_path, Some(&commit_hash), None, &action)?;

    tx.commit().map_err(|e| {
        ReviusError::Db(format!(
            "Failed to commit transaction for branch deletion: {}",
            e
        ))
    })?;

    Ok(commit_hash)
}

/// List all branches with their commit hashes. Returns Vec<(branch_name, commit_hash, is_current)>
pub fn list_branches(repo: &Repository) -> Result<Vec<(String, [u8; 32], bool)>, ReviusError> {
    let branches = db::refs::get_all_branches(&repo.conn)?;

    let current_branch = get_current_branch_name(repo)?;

    let result = branches
        .into_iter()
        .map(|(name, hash)| {
            let is_current = current_branch.as_ref().map_or(false, |c| c == &name);
            (name, hash, is_current)
        })
        .collect();

    Ok(result)
}

/// Get the current branch name (if on a branch). Returns None if in detached HEAD
pub fn get_current_branch_name(repo: &Repository) -> Result<Option<String>, ReviusError> {
    match get_head_state(&repo.conn)? {
        HeadState::Branch(ref_path) => Ok(Some(extract_branch_name(&ref_path)?)),
        HeadState::Detached(_) => Ok(None),
    }
}