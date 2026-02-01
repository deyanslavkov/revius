use crate::core::models::repository::Repository;
use crate::core::models::objects::HeadState;
use crate::core::resolve::resolve_target;
use crate::core::refs;
use crate::core::switch;
use crate::db;
use crate::error::ReviusError;
use crate::utils;
use rusqlite::Transaction;

/// Resets HEAD to the target commit. Does not touch staging or working directory.
pub fn reset_soft(repo: &Repository, target: &str) -> Result<[u8; 32], ReviusError> {
    let conn = repo.conn.unchecked_transaction()
        .map_err(|e| ReviusError::Db(format!("Failed to begin transaction: {}", e)))?;
    
    // 1. Resolve target
    let target_hash = resolve_and_get_hash(&conn, target)?;
    
    // 2. Move HEAD (this handles both branch and detached HEAD updates)
    move_head(&conn, target_hash, "soft")?;
    
    conn.commit()
        .map_err(|e| ReviusError::Db(format!("Failed to commit transaction: {}", e)))?;
        
    Ok(target_hash)
}

/// Resets HEAD to the target commit and updates staging to match. Working directory is left unchanged.
pub fn reset_mixed(repo: &Repository, target: &str) -> Result<[u8; 32], ReviusError> {
    let conn = repo.conn.unchecked_transaction()
        .map_err(|e| ReviusError::Db(format!("Failed to begin transaction: {}", e)))?;
    
    // 1. Resolve target
    let target_hash = resolve_and_get_hash(&conn, target)?;
    
    // 2. Move HEAD
    move_head(&conn, target_hash, "mixed")?;
    
    // 3. Update Staging
    // Clear staging and repopulate from the target tree
    let tree_hash = db::commits::get_commit_tree(&conn, &target_hash)?;
    db::staging::clear_staging(&conn)?;
    switch::update_staging_from_tree(&conn, tree_hash)?;
    
    conn.commit()
        .map_err(|e| ReviusError::Db(format!("Failed to commit transaction: {}", e)))?;
        
    Ok(target_hash)
}

/// Resets HEAD, staging, and working directory to the target commit. Destructive operation.
pub fn reset_hard(repo: &Repository, target: &str) -> Result<[u8; 32], ReviusError> {
    // 1. Resolve target (Read-only first)
    // We resolve before transaction to get the hashes needed for planning
    let target_hash = {
        let conn = &repo.conn;
        let resolved = resolve_target(conn, target)?;
        resolved.hash()
    };
    
    // 2. Plan Workspace Changes
    let (current_tree_hash, target_tree_hash) = {
        let conn = &repo.conn;
        
        let current_head_opt = db::refs::resolve_head(conn)?;
        let current_tree = if let Some(h) = current_head_opt {
            Some(db::commits::get_commit_tree(conn, &h)?)
        } else {
            None
        };
        
        let target_tree = db::commits::get_commit_tree(conn, &target_hash)?;
        
        (current_tree, target_tree)
    };
    
    let plan = switch::build_switch_plan(&repo.conn, current_tree_hash, target_tree_hash)?;
    
    // 3. Execute DB Transaction
    let tx = repo.conn.unchecked_transaction()
        .map_err(|e| ReviusError::Db(format!("Failed to begin transaction: {}", e)))?;
    
    move_head(&tx, target_hash, "hard")?;
    
    // Update staging
    db::staging::clear_staging(&tx)?;
    switch::update_staging_from_tree(&tx, target_tree_hash)?;
    
    tx.commit()
        .map_err(|e| ReviusError::Db(format!("Failed to commit transaction: {}", e)))?;
    
    // 4. Apply Workspace Changes (After DB commit)
    switch::apply_workspace_changes(repo, &plan)?;
    
    Ok(target_hash)
}

fn resolve_and_get_hash(conn: &Transaction, target: &str) -> Result<[u8; 32], ReviusError> {
    let resolved = resolve_target(conn, target)?;
    Ok(resolved.hash())
}

/// Moves the HEAD or branch pointer. Adds a reflog entry.
fn move_head(tx: &Transaction, target_hash: [u8; 32], mode_str: &str) -> Result<(), ReviusError> {
    // Get current state for reflog
    // This returns core::models::objects::HeadState
    let (head_state, current_hash_opt) = switch::get_current_head_state(tx)?;
    
    // Update the pointer
    // update_head handles both "ref: refs/heads/..." and detached hashes automatically
    refs::update_head(tx, &target_hash)?;
    
    let ref_path = match head_state {
        HeadState::Branch(ref name, _) => format!("refs/heads/{}", name),
        HeadState::Detached(_) => "HEAD".to_string(),
    };
    
    let action_json = format!(r#"["reset", "--{}", "{}"]"#, mode_str, utils::hash::hash_to_hex(&target_hash));
    
    db::reflog::insert_reflog(
        tx,
        &ref_path,
        current_hash_opt.as_ref(),
        Some(&target_hash),
        &action_json
    )?;
    
    Ok(())
}