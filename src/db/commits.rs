use crate::core::models::objects::Commit;
use crate::error::ReviusError;
use crate::utils::hash;
use rusqlite::{Connection, Transaction, OptionalExtension};

pub fn insert_commit(
    tx: &Transaction,
    hash: &[u8; 32],
    parent_hash: Option<&[u8; 32]>,
    merge_parent_hash: Option<&[u8; 32]>,
    tree_hash: &[u8; 32],
    message: &str,
    author_id: i64,
    timestamp: i64,
) -> Result<(), ReviusError> {
    tx.execute(
        "INSERT INTO Commits (hash, parent_hash, merge_parent_hash, tree_hash, message, author_id, timestamp) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            hash.as_slice(),
            parent_hash.map(|h| h.as_slice()),
            merge_parent_hash.map(|h| h.as_slice()),
            tree_hash.as_slice(),
            message,
            author_id,
            timestamp
        ],
    )
    .map_err(|e| ReviusError::Db(format!("Failed to insert commit (hash={}): {}", hex::encode(&hash[..8]), e)))?;

    Ok(())
}

pub fn get_commit(conn: &Connection, hash: &[u8; 32]) -> Result<Option<Commit>, ReviusError> {
    let mut stmt = conn
        .prepare("SELECT hash, parent_hash, merge_parent_hash, tree_hash, message, author_id, timestamp FROM Commits WHERE hash = ?1")
        .map_err(|e| ReviusError::Db(format!("Failed to prepare get commit query: {}", e)))?;

    let result = stmt
        .query_row(rusqlite::params![hash.as_slice()], |row| {
            let hash_vec: Vec<u8> = row.get(0)?;
            let parent_vec: Option<Vec<u8>> = row.get(1)?;
            let merge_parent_vec: Option<Vec<u8>> = row.get(2)?;
            let tree_vec: Vec<u8> = row.get(3)?;

            Ok(Commit {
                hash: hash::vec_to_hash(&hash_vec)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))))?,
                parent_hash: parent_vec
                    .and_then(|v| hash::vec_to_hash(&v).ok()),
                merge_parent_hash: merge_parent_vec
                    .and_then(|v| hash::vec_to_hash(&v).ok()),
                tree_hash: hash::vec_to_hash(&tree_vec)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))))?,
                message: row.get(4)?,
                author_id: row.get(5)?,
                timestamp: row.get(6)?,
            })
        })
        .optional()
        .map_err(|e| ReviusError::Db(format!("Failed to get commit (hash={}): {}", hex::encode(&hash[..8]), e)))?;

    Ok(result)
}

pub fn commit_exists(conn: &Connection, hash: &[u8; 32]) -> Result<bool, ReviusError> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM Commits WHERE hash = ?1)",
            [hash.as_slice()],
            |row| row.get(0),
        )
        .map_err(|e| ReviusError::Db(format!("Failed to check commit existence (hash={}): {}", hex::encode(&hash[..8]), e)))?;
    
    Ok(exists)
}

/// Get the tree hash for a commit
pub fn get_commit_tree(conn: &Connection, commit_hash: &[u8; 32]) -> Result<[u8; 32], ReviusError> {
    let tree_hash_vec: Vec<u8> = conn
        .query_row(
            "SELECT tree_hash FROM Commits WHERE hash = ?1",
            rusqlite::params![commit_hash.as_slice()],
            |row| row.get(0),
        )
        .map_err(|e| {
            ReviusError::CommitNotFound(format!(
                "Commit {} not found: {}",
                hash::hash_to_short_hex(commit_hash),
                e
            ))
        })?;
    
    hash::vec_to_hash(&tree_hash_vec).map_err(|e| {
        ReviusError::Db(format!(
            "Invalid tree hash for commit {}: {}",
            hash::hash_to_short_hex(commit_hash),
            e
        ))
    })
}

/// Find commits matching a hash prefix. Returns Vec<[u8; 32]> of matching commit hashes
pub fn find_commits_by_prefix(
    conn: &Connection,
    prefix: &str,
) -> Result<Vec<[u8; 32]>, ReviusError> {
    use crate::utils::hash;
    
    // Validate prefix
    if !hash::is_valid_hash_prefix(prefix) {
        return Err(ReviusError::InvalidHashPrefix(prefix.to_string()));
    }
    
    // Convert prefix to bytes for comparison
    let (prefix_bytes, hex_len) = hash::hex_prefix_to_bytes(prefix)
        .map_err(|e| ReviusError::Db(format!("Failed to parse hash prefix: {}", e)))?;
    
    // Query all commits
    let mut stmt = conn
        .prepare("SELECT hash FROM Commits")
        .map_err(|e| ReviusError::Db(format!("Failed to prepare query: {}", e)))?;
    
    let mut matches = Vec::new();
    let rows = stmt
        .query_map([], |row| {
            let hash_vec: Vec<u8> = row.get(0)?;
            Ok(hash_vec)
        })
        .map_err(|e| ReviusError::Db(format!("Failed to query commits: {}", e)))?;
    
    for row in rows {
        let hash_vec = row.map_err(|e| {
            ReviusError::Db(format!("Failed to read commit hash: {}", e))
        })?;
        
        // Check if this hash matches the prefix
        if hash_matches_prefix(&hash_vec, &prefix_bytes, hex_len) {
            let hash = hash::vec_to_hash(&hash_vec)
                .map_err(|e| ReviusError::Db(format!("Invalid hash in database: {}", e)))?;
            matches.push(hash);
        }
    }
    
    Ok(matches)
}

/// Check if a hash matches a given prefix. hex_len is the number of hex characters in the original prefix (not bytes)
fn hash_matches_prefix(hash: &[u8], prefix_bytes: &[u8], hex_len: usize) -> bool {
    let full_bytes = hex_len / 2;
    let has_nibble = hex_len % 2 == 1;
    
    // Compare full bytes
    if hash.len() < prefix_bytes.len() {
        return false;
    }
    
    for i in 0..full_bytes {
        if hash[i] != prefix_bytes[i] {
            return false;
        }
    }
    
    // If odd number of hex chars, compare the high nibble of the next byte
    if has_nibble {
        let hash_nibble = hash[full_bytes] >> 4;
        let prefix_nibble = prefix_bytes[full_bytes] >> 4;
        if hash_nibble != prefix_nibble {
            return false;
        }
    }
    
    true
}

/// Resolve a hash prefix to exactly one commit hash. Returns error if prefix is ambiguous or matches no commits
pub fn resolve_commit_prefix(
    conn: &Connection,
    prefix: &str,
) -> Result<[u8; 32], ReviusError> {
    let matches = find_commits_by_prefix(conn, prefix)?;
    
    match matches.len() {
        0 => Err(ReviusError::CommitNotFound(prefix.to_string())),
        1 => Ok(matches[0]),
        _ => Err(ReviusError::AmbiguousHashPrefix(prefix.to_string())),
    }
}

/// Get all parent hashes for a commit (primary and merge parent if exists)
pub fn get_commit_parents(
    conn: &Connection,
    commit_hash: &[u8; 32],
) -> Result<Vec<[u8; 32]>, ReviusError> {
    let mut stmt = conn
        .prepare("SELECT parent_hash, merge_parent_hash FROM Commits WHERE hash = ?")
        .map_err(|e| {
            ReviusError::Db(format!(
                "Failed to prepare query for commit parents (hash={}): {}",
                hash::hash_to_short_hex(commit_hash),
                e
            ))
        })?;

    let mut parents = Vec::new();

    let result = stmt.query_row([commit_hash.as_slice()], |row| {
        let parent_hash: Option<Vec<u8>> = row.get(0)?;
        let merge_parent_hash: Option<Vec<u8>> = row.get(1)?;
        Ok((parent_hash, merge_parent_hash))
    });

    match result {
        Ok((parent_hash, merge_parent_hash)) => {
            if let Some(ph) = parent_hash {
                parents.push(
                    hash::vec_to_hash(&ph)
                        .map_err(|e| ReviusError::Db(format!("Invalid parent hash: {}", e)))?,
                );
            }
            if let Some(mph) = merge_parent_hash {
                parents.push(
                    hash::vec_to_hash(&mph)
                        .map_err(|e| ReviusError::Db(format!("Invalid merge parent hash: {}", e)))?,
                );
            }
            Ok(parents)
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Err(ReviusError::CommitNotFound(hash::hash_to_short_hex(commit_hash)))
        }
        Err(e) => Err(ReviusError::Db(format!(
            "Failed to get commit parents (hash={}): {}",
            hash::hash_to_short_hex(commit_hash),
            e
        ))),
    }
}