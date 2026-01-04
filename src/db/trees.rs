use crate::core::models::objects::TreeEntry;
use crate::error::ReviusError;
use crate::utils::hash::{hash_to_short_hex, vec_to_hash};
use rusqlite::{Transaction, Connection};
use std::collections::VecDeque;

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

/// Efficient batch insert by optimizing the query
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

/// Get all direct children of a tree node (one level only)
pub fn get_tree_entries(
    conn: &Connection,
    parent_hash: &[u8; 32],
) -> Result<Vec<TreeEntry>, ReviusError> {
    let mut stmt = conn
        .prepare("SELECT parent_hash, name, object_hash, mode, is_dir FROM Trees WHERE parent_hash = ?")
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

/// Get all file entries from a tree (recursively) for staging reconstruction. Returns Vec<(relative_path, file_hash, mode, size)>
pub fn get_all_files_in_tree(
    conn: &Connection,
    tree_hash: &[u8; 32],
) -> Result<Vec<(String, [u8; 32], u32, u64)>, ReviusError> {
    let mut results = Vec::new();
    
    // Prepare statement once outside the loop
    let mut stmt = conn
        .prepare("SELECT name, object_hash, mode, is_dir FROM Trees WHERE parent_hash = ?1")
        .map_err(|e| {
            ReviusError::Db(format!(
                "Failed to prepare query for Trees: {}",
                e
            ))
        })?;
    
    // BFS to traverse tree structure
    let mut queue = VecDeque::new();
    queue.push_back((*tree_hash, String::new())); // (parent_hash, path_prefix)
    
    while let Some((parent_hash, path_prefix)) = queue.pop_front() {
        // Query all children of this parent using the pre-prepared statement
        let rows = stmt
            .query_map(rusqlite::params![parent_hash.as_slice()], |row| {
                let name: String = row.get(0)?;
                let object_hash_vec: Vec<u8> = row.get(1)?;
                let mode: i64 = row.get(2)?;
                let is_dir: i64 = row.get(3)?;
                Ok((name, object_hash_vec, mode as u32, is_dir == 1))
            })
            .map_err(|e| {
                ReviusError::Db(format!(
                    "Failed to query Trees (parent_hash={}): {}",
                    hash_to_short_hex(&parent_hash),
                    e
                ))
            })?;
        
        for row in rows {
            let (name, object_hash_vec, mode, is_dir) = row.map_err(|e| {
                ReviusError::Db(format!(
                    "Failed to read row from Trees (parent_hash={}): {}",
                    hash_to_short_hex(&parent_hash),
                    e
                ))
            })?;
            
            let object_hash = vec_to_hash(&object_hash_vec).map_err(|e| {
                ReviusError::Db(format!(
                    "Invalid object hash in Trees for name '{}': {}",
                    name, e
                ))
            })?;
            
            // Build full path
            let full_path = if path_prefix.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", path_prefix, name)
            };
            
            if is_dir {
                // It's a directory - add to queue for traversal
                queue.push_back((object_hash, full_path));
            } else {
                // It's a file - get size and add to results
                let size = get_file_size(conn, &object_hash)?;
                results.push((full_path, object_hash, mode, size));
            }
        }
    }
    
    Ok(results)
}

pub fn get_file_size(conn: &Connection, file_hash: &[u8; 32]) -> Result<u64, ReviusError> {
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