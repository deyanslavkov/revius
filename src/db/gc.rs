use crate::error::ReviusError;
use rusqlite::{Connection, Transaction};
use std::collections::HashSet;

/// Creates a temporary table to hold hashes that must be preserved.
/// We use a (hash, type) composite PK to allow different object types to share hashes 
/// (though unlikely in practice, it's correct) and to reuse one table.
/// Types: 1=Commit, 2=Tree, 3=File, 4=Blob
pub fn create_keep_list_table(tx: &Transaction) -> Result<(), ReviusError> {
    tx.execute(
        "CREATE TEMPORARY TABLE IF NOT EXISTS KeepList (
            hash BLOB NOT NULL,
            type INTEGER NOT NULL,
            PRIMARY KEY (hash, type)
        )",
        [],
    )
    .map_err(|e| ReviusError::Db(format!("Failed to create temporary GC table: {}", e)))?;
    
    // Clear it just in case connection pooling kept it alive
    tx.execute("DELETE FROM KeepList", [])
        .map_err(|e| ReviusError::Db(format!("Failed to clear temporary GC table: {}", e)))?;

    Ok(())
}

/// Batch inserts hashes into the keep list.
pub fn populate_keep_list(
    tx: &Transaction,
    hashes: &HashSet<[u8; 32]>,
    object_type: u8,
) -> Result<(), ReviusError> {
    if hashes.is_empty() {
        return Ok(());
    }

    let mut stmt = tx
        .prepare("INSERT OR IGNORE INTO KeepList (hash, type) VALUES (?1, ?2)")
        .map_err(|e| ReviusError::Db(format!("Failed to prepare GC batch insert: {}", e)))?;

    for hash in hashes {
        stmt.execute(rusqlite::params![&hash[..], object_type])
            .map_err(|e| ReviusError::Db(format!("Failed to insert into keep list: {}", e)))?;
    }

    Ok(())
}

pub fn delete_unused_commits(tx: &Transaction) -> Result<usize, ReviusError> {
    let count = tx.execute(
        "DELETE FROM Commits WHERE hash NOT IN (SELECT hash FROM KeepList WHERE type = 1)",
        [],
    )
    .map_err(|e| ReviusError::Db(format!("Failed to prune commits: {}", e)))?;
    Ok(count)
}

pub fn delete_unused_trees(tx: &Transaction) -> Result<usize, ReviusError> {
    // For trees, we delete rows where the *parent_hash* (the tree's identity) is not in the keep list.
    let count = tx.execute(
        "DELETE FROM Trees WHERE parent_hash NOT IN (SELECT hash FROM KeepList WHERE type = 2)",
        [],
    )
    .map_err(|e| ReviusError::Db(format!("Failed to prune trees: {}", e)))?;
    Ok(count)
}

pub fn delete_unused_files(tx: &Transaction) -> Result<usize, ReviusError> {
    let count = tx.execute(
        "DELETE FROM Files WHERE hash NOT IN (SELECT hash FROM KeepList WHERE type = 3)",
        [],
    )
    .map_err(|e| ReviusError::Db(format!("Failed to prune files: {}", e)))?;
    Ok(count)
}

pub fn delete_unused_blobs(tx: &Transaction) -> Result<usize, ReviusError> {
    let count = tx.execute(
        "DELETE FROM Blobs WHERE hash NOT IN (SELECT hash FROM KeepList WHERE type = 4)",
        [],
    )
    .map_err(|e| ReviusError::Db(format!("Failed to prune blobs: {}", e)))?;
    Ok(count)
}

/// Runs VACUUM to reclaim physical disk space.
/// Note: VACUUM cannot run inside a transaction.
pub fn vacuum_db(conn: &Connection) -> Result<(), ReviusError> {
    conn.execute("VACUUM", [])
        .map_err(|e| ReviusError::Db(format!("Failed to vacuum database: {}", e)))?;
    Ok(())
}

// Queries for Dry Run stats

pub fn count_unused_commits(tx: &Transaction) -> Result<usize, ReviusError> {
    let count: usize = tx.query_row(
        "SELECT COUNT(*) FROM Commits WHERE hash NOT IN (SELECT hash FROM KeepList WHERE type = 1)",
        [],
        |row| row.get(0),
    ).map_err(|e| ReviusError::Db(format!("Failed to count unused commits: {}", e)))?;
    Ok(count)
}

pub fn count_unused_trees(tx: &Transaction) -> Result<usize, ReviusError> {
    // We count distinct parent_hashes that are not kept
    let count: usize = tx.query_row(
        "SELECT COUNT(DISTINCT parent_hash) FROM Trees WHERE parent_hash NOT IN (SELECT hash FROM KeepList WHERE type = 2)",
        [],
        |row| row.get(0),
    ).map_err(|e| ReviusError::Db(format!("Failed to count unused trees: {}", e)))?;
    Ok(count)
}

pub fn count_unused_files(tx: &Transaction) -> Result<usize, ReviusError> {
    let count: usize = tx.query_row(
        "SELECT COUNT(*) FROM Files WHERE hash NOT IN (SELECT hash FROM KeepList WHERE type = 3)",
        [],
        |row| row.get(0),
    ).map_err(|e| ReviusError::Db(format!("Failed to count unused files: {}", e)))?;
    Ok(count)
}

pub fn count_unused_blobs(tx: &Transaction) -> Result<usize, ReviusError> {
    let count: usize = tx.query_row(
        "SELECT COUNT(*) FROM Blobs WHERE hash NOT IN (SELECT hash FROM KeepList WHERE type = 4)",
        [],
        |row| row.get(0),
    ).map_err(|e| ReviusError::Db(format!("Failed to count unused blobs: {}", e)))?;
    Ok(count)
}