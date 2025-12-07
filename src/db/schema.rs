use crate::error::ReviusError;
use rusqlite::Connection;

pub fn create_all(conn: &Connection) -> Result<(), ReviusError> {
    create_meta_table(conn)?;
    initialize_meta(conn)?;
    create_blobs_table(conn)?;
    create_files_table(conn)?;
    create_trees_table(conn)?;
    create_authors_table(conn)?;
    create_commits_table(conn)?;
    create_refs_table(conn)?;
    create_staging_table(conn)?;
    create_reflog_table(conn)?;
    create_audit_table(conn)?;
    Ok(())
}

fn create_meta_table(conn: &Connection) -> Result<(), ReviusError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS Meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

fn initialize_meta(conn: &Connection) -> Result<(), ReviusError> {
    let uuid = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT OR IGNORE INTO Meta (key, value) VALUES ('schema_version', '1')",
        [],
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO Meta (key, value) VALUES ('repository_uuid', ?1)",
        [&uuid],
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO Meta (key, value) VALUES ('HEAD', 'ref: refs/heads/main')",
        [],
    )?;

    Ok(())
}

fn create_blobs_table(conn: &Connection) -> Result<(), ReviusError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS Blobs (
            hash BLOB PRIMARY KEY,
            data BLOB NOT NULL,
            compression TEXT NOT NULL DEFAULT 'zstd3',
            uncompressed_size INTEGER NOT NULL,
            CHECK(length(hash) = 32)
        )",
        [],
    )?;
    Ok(())
}

fn create_files_table(conn: &Connection) -> Result<(), ReviusError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS Files (
            hash BLOB PRIMARY KEY,
            size INTEGER NOT NULL,
            recipe_version INTEGER NOT NULL DEFAULT 1,
            chunk_count INTEGER NOT NULL,
            recipe BLOB NOT NULL,
            CHECK(length(hash) = 32),
            CHECK(length(recipe) % 32 == 0)
        )",
        [],
    )?;
    Ok(())
}

fn create_trees_table(conn: &Connection) -> Result<(), ReviusError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS Trees (
            object_hash BLOB NOT NULL PRIMARY KEY,
            parent_hash BLOB NOT NULL,
            name TEXT NOT NULL,
            mode INTEGER NOT NULL,
            CHECK(parent_hash IS NULL OR length(parent_hash) = 32),
            CHECK(length(object_hash) = 32)
        )",
        [],
    )?;
    Ok(())
}

fn create_authors_table(conn: &Connection) -> Result<(), ReviusError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS Authors (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT NOT NULL,
            UNIQUE(name, email)
        )",
        [],
    )?;
    Ok(())
}

fn create_commits_table(conn: &Connection) -> Result<(), ReviusError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS Commits (
            hash BLOB PRIMARY KEY,
            parent_hash BLOB REFERENCES Commits(hash),
            tree_hash BLOB NOT NULL,
            message TEXT NOT NULL,
            author_id INTEGER NOT NULL REFERENCES Authors(id),
            timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
            CHECK(length(hash) = 32)
        )",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_commits_parent ON Commits(parent_hash)",
        [],
    )?;
    
    Ok(())
}

fn create_refs_table(conn: &Connection) -> Result<(), ReviusError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS Refs (
            path TEXT PRIMARY KEY,
            ref_type INTEGER NOT NULL CHECK(ref_type IN (0, 1, 2)),
            commit_hash BLOB REFERENCES Commits(hash)
        )",
        [],
    )?;
    Ok(())
}

fn create_staging_table(conn: &Connection) -> Result<(), ReviusError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS Staging (
            path TEXT PRIMARY KEY,
            file_hash BLOB NOT NULL REFERENCES Files(hash),
            mode INTEGER NOT NULL,
            size INTEGER NOT NULL,
            modified_at INTEGER DEFAULT (strftime('%s', 'now')),
            CHECK(length(file_hash) = 32)
        )",
        [],
    )?;
    Ok(())
}

fn create_reflog_table(conn: &Connection) -> Result<(), ReviusError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS Reflog (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ref_path TEXT NOT NULL,
            old_hash BLOB,
            new_hash BLOB,
            action TEXT NOT NULL,
            timestamp INTEGER DEFAULT (strftime('%s', 'now'))
        )",
        [],
    )?;
    Ok(())
}

fn create_audit_table(conn: &Connection) -> Result<(), ReviusError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS Audit (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            action TEXT NOT NULL,
            args TEXT,
            output TEXT,
            exit_code INTEGER,
            author_id INTEGER REFERENCES Authors(id),
            timestamp INTEGER DEFAULT (strftime('%s', 'now')),
            duration_ms INTEGER
        )",
        [],
    )?;
    Ok(())
}