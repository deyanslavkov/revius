use crate::core::models::objects::ReflogEntry;
use crate::error::ReviusError;
use rusqlite::{Connection, Transaction};

pub fn insert_reflog(
    tx: &Transaction,
    ref_path: &str,
    old_hash: Option<&[u8; 32]>,
    new_hash: Option<&[u8; 32]>,
    action: &str,
) -> Result<(), ReviusError> {
    let timestamp = crate::utils::time::unix_timestamp().unwrap_or(0);

    tx.execute(
        "INSERT INTO Reflog (ref_path, old_hash, new_hash, action, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            ref_path,
            old_hash.map(|h| h.as_slice()),
            new_hash.map(|h| h.as_slice()),
            action,
            timestamp
        ],
    )
    .map_err(|e| {
        ReviusError::Db(format!(
            "Failed to insert reflog entry for {}: {}",
            ref_path, e
        ))
    })?;

    Ok(())
}

pub fn get_reflog(
    conn: &Connection,
    ref_path_filter: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<ReflogEntry>, ReviusError> {
    let limit_clause = if let Some(l) = limit {
        format!("LIMIT {}", l)
    } else {
        String::new()
    };

    let (where_clause, _params) = if let Some(ref_path) = ref_path_filter {
        ("WHERE ref_path = ?1", vec![ref_path.to_string()])
    } else {
        ("", vec![])
    };

    let query = format!(
        "SELECT id, ref_path, old_hash, new_hash, action, timestamp 
         FROM Reflog 
         {} 
         ORDER BY id DESC {}",
        where_clause, limit_clause
    );

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| ReviusError::Db(format!("Failed to prepare reflog query: {}", e)))?;

    let rows = if let Some(ref_path) = ref_path_filter {
        stmt.query_map([ref_path], parse_reflog_row)
    } else {
        stmt.query_map([], parse_reflog_row)
    }
    .map_err(|e| ReviusError::Db(format!("Failed to query reflog: {}", e)))?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(
            row.map_err(|e| ReviusError::Db(format!("Failed to parse reflog row: {}", e)))?,
        );
    }

    Ok(entries)
}

fn parse_reflog_row(row: &rusqlite::Row) -> rusqlite::Result<ReflogEntry> {
    let old_hash_vec: Option<Vec<u8>> = row.get(2)?;
    let new_hash_vec: Option<Vec<u8>> = row.get(3)?;

    let old_hash = if let Some(v) = old_hash_vec {
        Some(crate::utils::hash::vec_to_hash(&v).map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e,
            )))
        })?)
    } else {
        None
    };

    let new_hash = if let Some(v) = new_hash_vec {
        Some(crate::utils::hash::vec_to_hash(&v).map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e,
            )))
        })?)
    } else {
        None
    };

    Ok(ReflogEntry {
        id: row.get(0)?,
        ref_path: row.get(1)?,
        old_hash,
        new_hash,
        action: row.get(4)?,
        timestamp: row.get(5)?,
    })
}