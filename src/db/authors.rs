use crate::error::ReviusError;
use rusqlite::{OptionalExtension, Connection, Transaction};

/// Get or create an author, returning their ID
pub fn get_or_create_author(tx: &Transaction, name: &str, email: &str) -> Result<i64, ReviusError> {
    let existing: Option<i64> = tx
        .query_row(
            "SELECT id FROM Authors WHERE name = ?1 AND email = ?2",
            rusqlite::params![name, email],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| ReviusError::Db(format!("Failed to query author: {}", e)))?;

    if let Some(id) = existing {
        return Ok(id);
    }

    tx.execute(
        "INSERT INTO Authors (name, email) VALUES (?1, ?2)",
        rusqlite::params![name, email],
    )
    .map_err(|e| ReviusError::Db(format!("Failed to insert author '{}' <{}>: {}", name, email, e)))?;

    let id = tx.last_insert_rowid();
    Ok(id)
}

/// Get author details by ID. Returns (name, email)
pub fn get_author_by_id(conn: &Connection, author_id: i64) -> Result<(String, String), ReviusError> {
    let mut stmt = conn
        .prepare("SELECT name, email FROM Authors WHERE id = ?")
        .map_err(|e| {
            ReviusError::Db(format!("Failed to prepare query for Authors (id={}): {}", author_id, e))
        })?;

    let result = stmt
        .query_row([author_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| {
            ReviusError::Db(format!("Failed to get author with id={}: {}", author_id, e))
        })?;

    Ok(result)
}