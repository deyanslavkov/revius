use crate::error::ReviusError;
use rusqlite::{OptionalExtension, Transaction};

/// Get or create an author, returning their ID
pub fn get_or_create_author(
    tx: &Transaction,
    name: &str,
    email: &str,
) -> Result<i64, ReviusError> {
    // Try to find existing author
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

    // Create new author
    tx.execute(
        "INSERT INTO Authors (name, email) VALUES (?1, ?2)",
        rusqlite::params![name, email],
    )
    .map_err(|e| ReviusError::Db(format!("Failed to insert author '{}' <{}>: {}", name, email, e)))?;

    let id = tx.last_insert_rowid();
    Ok(id)
}