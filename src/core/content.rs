use crate::core::models::repository::Repository;
use crate::db;
use crate::error::ReviusError;
use crate::fs;
use crate::utils;
use rusqlite::Transaction;
use std::path::Path;

pub fn read_and_hash_file(path: &Path) -> Result<(Vec<u8>, [u8; 32]), ReviusError> {
    let data = fs::io::read_file(path)
        .map_err(|e| ReviusError::Io(path.to_path_buf(), e))?;
    let hash = utils::hash::hash_bytes(&data);
    Ok((data, hash))
}

/// True if new created, false if already exists
pub fn store_blob(tx: &Transaction, path: &Path, chunk: &[u8], chunk_hash: &[u8; 32], compression_enabled: bool, compression_level: u8)
-> Result<bool, ReviusError> {
    if db::blobs::blob_exists(tx, chunk_hash)? {
        return Ok(false);
    }

    let (data, compression) = if compression_enabled {
        let compressed = utils::compression::compress(chunk, compression_level as i32)
            .map_err(|e| {
                ReviusError::Io(
                    path.to_path_buf(),
                    std::io::Error::other(e),
                )
            })?;
        (compressed, format!("zstd{}", compression_level))
    } else {
        (chunk.to_vec(), "none".to_string())
    };

    db::blobs::insert_blob(tx, chunk_hash, &data, &compression, chunk.len() as u64)?;

    Ok(true)
}

/// Create file object in database (with chunking and compression). Returns the number of new blobs created
pub fn store_file_content(tx: &Transaction, path: &Path, file_hash: &[u8; 32], file_data: &[u8], repo: &Repository)
-> Result<u64, ReviusError> {
    if db::files::file_exists(tx, file_hash)? {
        return Ok(0);
    }

    let chunks = if repo.config.chunking {
        utils::cdc::chunk_data(
            file_data,
            repo.config.chunk_min,
            repo.config.chunk_avg,
            repo.config.chunk_max,
        )
    } else {
        vec![file_data]
    };

    let mut blob_count = 0;

    let chunk_hashes: Vec<[u8; 32]> = chunks.iter()
    .map(|chunk| utils::hash::hash_bytes(chunk))
    .collect();

    for (chunk, chunk_hash) in chunks.iter().zip(chunk_hashes.iter()) {
        let was_new = store_blob(tx, path, chunk, chunk_hash, repo.config.compression, repo.config.compression_level)?;
        if was_new {
            blob_count += 1;
        }
    }

    let recipe = utils::recipe::build_recipe(&chunk_hashes);
    db::files::insert_file(tx, file_hash, &recipe, chunks.len() as u64, file_data.len() as u64)?;

    Ok(blob_count)
}