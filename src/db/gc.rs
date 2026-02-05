use crate::error::ReviusError;
use rusqlite::{Connection, Transaction};
use std::collections::HashSet;

/// Creates a temporary table to hold hashes that must be preserved.
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

/// Uses Recursive CTEs to find all reachable Commits, Trees, and Files directly in the DB.
/// Returns a list of all reachable File hashes so the application can process their recipes (Blobs).
pub fn mark_repository_structure(
    tx: &Transaction,
    detached_head: Option<&[u8; 32]>,
) -> Result<Vec<[u8; 32]>, ReviusError> {
    // 1. Seed the KeepList with the Detached HEAD (if any)
    if let Some(head) = detached_head {
        tx.execute(
            "INSERT OR IGNORE INTO KeepList (hash, type) VALUES (?1, 1)",
            [head.as_slice()],
        ).map_err(|e| ReviusError::Db(format!("Failed to insert detached HEAD: {}", e)))?;
    }

    // 2. Mark Commits (Recursive: Refs + Detached -> Parents)
    // We union with KeepList to pick up the detached head inserted above
    tx.execute(
        "INSERT OR IGNORE INTO KeepList (hash, type)
         WITH RECURSIVE Ancestors(hash) AS (
            SELECT commit_hash FROM Refs
            UNION
            SELECT hash FROM KeepList WHERE type = 1
            UNION
            SELECT c.parent_hash FROM Commits c JOIN Ancestors a ON c.hash = a.hash WHERE c.parent_hash IS NOT NULL
            UNION
            SELECT c.merge_parent_hash FROM Commits c JOIN Ancestors a ON c.hash = a.hash WHERE c.merge_parent_hash IS NOT NULL
         )
         SELECT hash, 1 FROM Ancestors",
        [],
    ).map_err(|e| ReviusError::Db(format!("Failed to mark reachable commits: {}", e)))?;

    // 3. Mark Trees and Files (Recursive: Commit Trees -> Subtrees & Files)
    // is_dir=1 -> Tree (Type 2), is_dir=0 -> File (Type 3)
    tx.execute(
        "INSERT OR IGNORE INTO KeepList (hash, type)
         WITH RECURSIVE TreeWalk(hash, is_dir) AS (
            SELECT tree_hash, 1 FROM Commits WHERE hash IN (SELECT hash FROM KeepList WHERE type=1)
            UNION
            SELECT t.object_hash, t.is_dir
            FROM Trees t
            JOIN TreeWalk p ON t.parent_hash = p.hash
            WHERE p.is_dir = 1
         )
         SELECT hash, CASE WHEN is_dir=1 THEN 2 ELSE 3 END
         FROM TreeWalk",
        [],
    ).map_err(|e| ReviusError::Db(format!("Failed to mark reachable trees and files: {}", e)))?;

    // 4. Mark Staged Files (Roots for files)
    tx.execute(
        "INSERT OR IGNORE INTO KeepList (hash, type)
         SELECT file_hash, 3 FROM Staging",
        [],
    ).map_err(|e| ReviusError::Db(format!("Failed to mark staged files: {}", e)))?;

    // 5. Retrieve all File hashes (Type 3) so we can check Blobs in Rust
    // (We cannot parse binary recipes in SQL)
    let mut stmt = tx.prepare("SELECT hash FROM KeepList WHERE type = 3")
        .map_err(|e| ReviusError::Db(format!("Failed to prepare file fetch: {}", e)))?;
    
    let file_hashes = stmt.query_map([], |row| {
        let vec: Vec<u8> = row.get(0)?;
        crate::utils::hash::vec_to_hash(&vec).map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
        })
    }).map_err(|e| ReviusError::Db(format!("Failed to query reachable files: {}", e)))?;

    let mut result = Vec::new();
    for h in file_hashes {
        result.push(h.map_err(|e| ReviusError::Db(format!("Error reading file hash: {}", e)))?);
    }

    Ok(result)
}

/// Batch inserts hashes into the keep list (used for Blobs)
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