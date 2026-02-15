use crate::core::models::objects::TreeEntry;
use crate::error::ReviusError;
use crate::utils::hash::{hash_to_short_hex, vec_to_hash};
use rusqlite::{Transaction, Connection};

pub fn tree_exists(conn: &Connection, parent_hash: &[u8; 32]) -> Result<bool, ReviusError> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM Trees WHERE parent_hash = ?1 LIMIT 1)",
            [parent_hash.as_slice()],
            |row| row.get(0),
        )
        .map_err(|e| ReviusError::Db(format!("Failed to check tree existence (hash={}): {}", hex::encode(&parent_hash[..8]), e)))?;
    
    Ok(exists)
}

pub fn insert_tree_entry(tx: &Transaction, parent_hash: &[u8; 32], name: &str, object_hash: &[u8; 32], mode: u32, is_dir: bool) -> Result<(), ReviusError> {
    tx.execute(
        "INSERT INTO Trees (parent_hash, name, object_hash, mode, is_dir) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![parent_hash.as_slice(), name, object_hash.as_slice(), mode, is_dir as i32],
    )
    .map_err(|e| ReviusError::Db(format!("Failed to insert tree entry '{}': {}", name, e)))?;

    Ok(())
}

/// Efficient batch insert
pub fn batch_insert_tree_entries(
    tx: &Transaction,
    entries: Vec<TreeEntry>,
) -> Result<(), ReviusError> {
    let mut stmt = tx
        .prepare("INSERT INTO Trees (parent_hash, name, object_hash, mode, is_dir) VALUES (?1, ?2, ?3, ?4, ?5)")
        .map_err(|e| ReviusError::Db(format!("Failed to prepare tree batch insert: {}", e)))?;

    for entry in entries {
        stmt.execute(rusqlite::params![
            entry.parent_hash.as_slice(),
            entry.name,
            entry.object_hash.as_slice(),
            entry.mode,
            entry.is_dir as i32
        ])
        .map_err(|e| ReviusError::Db(format!("Failed to insert tree entry '{}': {}", entry.name, e)))?;
    }

    Ok(())
}

/// Get all direct children of a tree node (one level only).
/// Uses prepare_cached for performance during recursive traversals.
pub fn get_tree_entries(
    conn: &Connection,
    parent_hash: &[u8; 32],
) -> Result<Vec<TreeEntry>, ReviusError> {
    let mut stmt = conn
        .prepare_cached("SELECT parent_hash, name, object_hash, mode, is_dir FROM Trees WHERE parent_hash = ?")
        .map_err(|e| {
            ReviusError::Db(format!(
                "Failed to prepare tree entries query for parent_hash {}: {}",
                hash_to_short_hex(parent_hash),
                e
            ))
        })?;

    let entries = stmt
        .query_map([parent_hash], |row| {
            let parent_hash_vec = row.get::<_, Vec<u8>>(0)?;
            let object_hash_vec = row.get::<_, Vec<u8>>(2)?;

            Ok(TreeEntry {
                parent_hash: vec_to_hash(&parent_hash_vec)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    ))))?,
                name: row.get(1)?,
                object_hash: vec_to_hash(&object_hash_vec)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    ))))?,
                mode: row.get(3)?,
                is_dir: row.get(4)?,
            })
        })
        .map_err(|e| {
            ReviusError::Db(format!(
                "Failed to query tree entries for parent_hash {}: {}",
                hash_to_short_hex(parent_hash),
                e
            ))
        })?;

    let mut result = Vec::new();
    for entry_result in entries {
        let entry = entry_result.map_err(|e| {
            ReviusError::Db(format!(
                "Failed to read tree entry for parent_hash {}: {}",
                hash_to_short_hex(parent_hash),
                e
            ))
        })?;
        result.push(entry);
    }

    Ok(result)
}

/// Recursively fetches all files in a tree using a CTE.
/// Returns Vec<(path, hash, mode)>
pub fn get_recursive_files(
    conn: &Connection,
    root_hash: &[u8; 32],
) -> Result<Vec<(String, [u8; 32], u32)>, ReviusError> {
    let mut stmt = conn.prepare_cached(
        "WITH RECURSIVE tree_hierarchy(path, object_hash, mode, is_dir) AS (
            SELECT name, object_hash, mode, is_dir
            FROM Trees
            WHERE parent_hash = ?1
            
            UNION ALL
            
            SELECT th.path || '/' || t.name, t.object_hash, t.mode, t.is_dir
            FROM Trees t
            JOIN tree_hierarchy th ON t.parent_hash = th.object_hash
            WHERE th.is_dir = 1
        )
        SELECT path, object_hash, mode FROM tree_hierarchy WHERE is_dir = 0;"
    ).map_err(|e| ReviusError::Db(format!("Failed to prepare recursive tree query: {}", e)))?;

    let rows = stmt.query_map([root_hash.as_slice()], |row| {
        let path: String = row.get(0)?;
        let hash_vec: Vec<u8> = row.get(1)?;
        let mode: u32 = row.get(2)?;
        
        let hash = vec_to_hash(&hash_vec).map_err(|e| {
             rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                 std::io::ErrorKind::InvalidData, e
             )))
        })?;

        Ok((path, hash, mode))
    }).map_err(|e| ReviusError::Db(format!("Failed to execute recursive tree query: {}", e)))?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| ReviusError::Db(format!("Error reading recursive tree row: {}", e)))?);
    }

    Ok(result)
}

/// Recursively fetches files AND joins with Files table to get size in one pass.
/// Returns Vec<(path, hash, mode, size)>
pub fn get_recursive_files_with_size(
    conn: &Connection,
    root_hash: &[u8; 32],
) -> Result<Vec<(String, [u8; 32], u32, u64)>, ReviusError> {
    let mut stmt = conn.prepare_cached(
        "WITH RECURSIVE tree_hierarchy(path, object_hash, mode, is_dir) AS (
            SELECT name, object_hash, mode, is_dir
            FROM Trees
            WHERE parent_hash = ?1
            
            UNION ALL
            
            SELECT th.path || '/' || t.name, t.object_hash, t.mode, t.is_dir
            FROM Trees t
            JOIN tree_hierarchy th ON t.parent_hash = th.object_hash
            WHERE th.is_dir = 1
        )
        SELECT th.path, th.object_hash, th.mode, f.size
        FROM tree_hierarchy th
        JOIN Files f ON th.object_hash = f.hash
        WHERE th.is_dir = 0;"
    ).map_err(|e| ReviusError::Db(format!("Failed to prepare recursive tree size query: {}", e)))?;

    let rows = stmt.query_map([root_hash.as_slice()], |row| {
        let path: String = row.get(0)?;
        let hash_vec: Vec<u8> = row.get(1)?;
        let mode: u32 = row.get(2)?;
        let size: i64 = row.get(3)?;
        
        let hash = vec_to_hash(&hash_vec).map_err(|e| {
             rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                 std::io::ErrorKind::InvalidData, e
             )))
        })?;

        Ok((path, hash, mode, size as u64))
    }).map_err(|e| ReviusError::Db(format!("Failed to execute recursive tree size query: {}", e)))?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| ReviusError::Db(format!("Error reading recursive tree size row: {}", e)))?);
    }

    Ok(result)
}

/// Helper to get file size for staging reconstruction
pub fn get_file_size(conn: &Connection, file_hash: &[u8; 32]) -> Result<u64, ReviusError> {
    // Uses a simple query, cached by SQLite internally if frequent
    let size: i64 = conn
        .query_row(
            "SELECT size FROM Files WHERE hash = ?1",
            rusqlite::params![file_hash.as_slice()],
            |row| row.get(0),
        )
        .map_err(|e| {
            ReviusError::Db(format!(
                "Failed to get file size (hash={}): {}",
                hash_to_short_hex(file_hash),
                e
            ))
        })?;
    
    Ok(size as u64)
}