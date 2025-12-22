use crate::error::ReviusError;
use crate::utils::hash;
use rusqlite::{Connection, OptionalExtension, Transaction};

pub fn get_ref(conn: &Connection, path: &str) -> Result<Option<[u8; 32]>, ReviusError> {
    let result: Option<Vec<u8>> = conn
        .query_row(
            "SELECT commit_hash FROM Refs WHERE path = ?1",
            rusqlite::params![path],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| ReviusError::Db(format!("Failed to query ref '{}': {}", path, e)))?;

    match result {
        Some(vec) => {
            let hash_array = hash::vec_to_hash(&vec).map_err(|e| ReviusError::Db(e))?;
            Ok(Some(hash_array))
        }
        None => Ok(None),
    }
}

pub fn upsert_ref(tx: &Transaction, path: &str, ref_type: u8, commit_hash: &[u8; 32]) -> Result<(), ReviusError> {
    tx.execute(
        "INSERT INTO Refs (path, ref_type, commit_hash) VALUES (?1, ?2, ?3)
         ON CONFLICT(path) DO UPDATE SET commit_hash = excluded.commit_hash",
        rusqlite::params![path, ref_type, commit_hash.as_slice()],
    )
    .map_err(|e| ReviusError::Db(format!("Failed to upsert ref '{}': {}", path, e)))?;

    Ok(())
}

/// Update an existing ref - use when you know the ref exists (it doesn't take ref type as a parameter)
pub fn update_ref(tx: &Transaction, path: &str, commit_hash: &[u8; 32]) -> Result<(), ReviusError> {
    let rows = tx
        .execute(
            "UPDATE Refs SET commit_hash = ?1 WHERE path = ?2",
            rusqlite::params![commit_hash.as_slice(), path],
        )
        .map_err(|e| ReviusError::Db(format!("Failed to update ref '{}': {}", path, e)))?;

    if rows == 0 {
        return Err(ReviusError::Db(format!("Ref '{}' not found", path)));
    }

    Ok(())
}

/// Resolve HEAD to a commit hash. Returns None if HEAD points to non-existent ref (initial commit case)
pub fn resolve_head(conn: &Connection) -> Result<Option<[u8; 32]>, ReviusError> {
    // Get HEAD value from Meta
    let head_value: String = conn
        .query_row("SELECT value FROM Meta WHERE key = 'HEAD'", [], |row| {
            row.get(0)
        })
        .map_err(|e| ReviusError::Db(format!("Failed to get HEAD from Meta: {}", e)))?;

    if head_value.starts_with("ref: ") {
        // HEAD points to a ref
        let ref_path = head_value.strip_prefix("ref: ").unwrap();
        get_ref(conn, ref_path)
    } else {
        // Detached HEAD - parse as hex hash
        let hash_bytes = hex::decode(&head_value)
            .map_err(|e| ReviusError::Db(format!("Invalid HEAD hash format: {}", e)))?;
        
        Ok(Some(hash::vec_to_hash(&hash_bytes).map_err(|e| ReviusError::Db(e))?))
    }
}

/// Get all refs (branches and tags) with their commit hashes. Returns Vec<(ref_path, commit_hash)>
pub fn get_all_refs(conn: &Connection) -> Result<Vec<(String, [u8; 32])>, ReviusError> {
    let mut stmt = conn
        .prepare("SELECT path, commit_hash FROM Refs")
        .map_err(|e| ReviusError::Db(format!("Failed to prepare query for Refs: {}", e)))?;

    let refs = stmt
        .query_map([], |row| {
            let path: String = row.get(0)?;
            let hash_vec: Vec<u8> = row.get(1)?;
            Ok((path, hash_vec))
        })
        .map_err(|e| ReviusError::Db(format!("Failed to query Refs: {}", e)))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ReviusError::Db(format!("Failed to collect Refs results: {}", e)))?;

    let mut result = Vec::new();
    for (path, hash_vec) in refs {
        let hash = hash::vec_to_hash(&hash_vec)
            .map_err(|e| ReviusError::Db(format!("Invalid hash in Refs for {}: {}", path, e)))?;
        result.push((path, hash));
    }

    Ok(result)
}

/// Get all branch refs (starting with "refs/heads/"). Returns Vec<(branch_name_only, commit_hash)>
pub fn get_all_branches(conn: &Connection) -> Result<Vec<(String, [u8; 32])>, ReviusError> {
    let mut stmt = conn
        .prepare("SELECT path, commit_hash FROM Refs WHERE path LIKE 'refs/heads/%' ORDER BY path")
        .map_err(|e| ReviusError::Db(format!("Failed to prepare query for branches: {}", e)))?;

    let rows = stmt
        .query_map([], |row| {
            let path: String = row.get(0)?;
            let hash_vec: Vec<u8> = row.get(1)?;
            Ok((path, hash_vec))
        })
        .map_err(|e| ReviusError::Db(format!("Failed to query branches: {}", e)))?;

    let mut branches = Vec::new();
    for row_result in rows {
        let (path, hash_vec) = row_result
            .map_err(|e| ReviusError::Db(format!("Failed to read branch row: {}", e)))?;

        let branch_name = path
            .strip_prefix("refs/heads/")
            .ok_or_else(|| ReviusError::Db(format!("Invalid branch ref path: {}", path)))?
            .to_string();

        let hash = hash::vec_to_hash(&hash_vec)
            .map_err(|e| ReviusError::Db(format!("Invalid hash in branch {}: {}", branch_name, e)))?;

        branches.push((branch_name, hash));
    }

    Ok(branches)
}

pub fn delete_ref(tx: &Transaction, path: &str) -> Result<(), ReviusError> {
    let rows_affected = tx
        .execute("DELETE FROM Refs WHERE path = ?1", [path])
        .map_err(|e| ReviusError::Db(format!("Failed to delete ref {}: {}", path, e)))?;

    if rows_affected == 0 {
        return Err(ReviusError::BranchNotFound(path.to_string()));
    }

    Ok(())
}

pub fn ref_exists(conn: &Connection, path: &str) -> Result<bool, ReviusError> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM Refs WHERE path = ?1", [path], |row| {
            row.get(0)
        })
        .map_err(|e| ReviusError::Db(format!("Failed to check if ref exists {}: {}", path, e)))?;

    Ok(count > 0)
}