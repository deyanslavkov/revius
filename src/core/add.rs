use crate::core::models::repository::Repository;
use crate::db;
use crate::error::ReviusError;
use crate::fs;
use crate::utils;
use rusqlite::Transaction;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum StageOutcome {
    Added { blobs: u64 },
    Modified { blobs: u64 },
    Unchanged,
}

fn read_and_hash_file(path: &Path) -> Result<(Vec<u8>, [u8; 32]), ReviusError> {
    let data = fs::io::read_file(path)
        .map_err(|e| ReviusError::Io(path.to_path_buf(), e))?;
    let hash = utils::hash::hash_bytes(&data);
    Ok((data, hash))
}

fn create_file_object(tx: &Transaction, path: &Path, file_hash: &[u8; 32], file_data: &[u8], repo: &Repository)
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
        vec![&file_data[..]]
    };

    let mut recipe = Vec::new();
    let mut blob_count = 0;

    for chunk in &chunks {
        let chunk_hash = utils::hash::hash_bytes(chunk);

        if !db::blobs::blob_exists(tx, &chunk_hash)? {
            let (data, compression) = if repo.config.compression {
                let compressed = utils::compression::compress(
                    chunk,
                    repo.config.compression_level as i32,
                )
                .map_err(|e| {
                    ReviusError::Io(
                        path.to_path_buf(),
                        std::io::Error::new(std::io::ErrorKind::Other, e),
                    )
                })?;
                (compressed, format!("zstd{}", repo.config.compression_level))
            } else {
                (chunk.to_vec(), "none".to_string())
            };

            db::blobs::insert_blob(tx, &chunk_hash, &data, &compression, chunk.len() as u64)?;

            blob_count += 1;
        }

        recipe.extend_from_slice(&chunk_hash);
    }

    db::files::insert_file(tx, file_hash, &recipe, chunks.len() as u64, file_data.len() as u64)?;

    Ok(blob_count)
}

fn stage_single_file(tx: &Transaction, repo: &Repository, path: &PathBuf)
-> Result<(PathBuf, StageOutcome), ReviusError> {
    let (file_data, file_hash) = read_and_hash_file(path)?;

    let repo_relative_path = fs::paths::make_repo_relative(path, &repo.root)?;

    let mode = fs::io::get_file_mode(path)
        .map_err(|e| ReviusError::Io(path.clone(), e))?;

    let mtime = fs::io::get_file_modified_time(path)
        .map_err(|e| ReviusError::Io(path.clone(), e))?;

    let previous_staged = db::staging::get_staged_file(tx, &repo_relative_path)?;

    let is_modified = previous_staged
        .as_ref()
        .map(|prev| prev.file_hash != file_hash)
        .unwrap_or(false);
    let is_new = previous_staged.is_none();

    if !is_new && !is_modified {
        return Ok((path.clone(), StageOutcome::Unchanged));
    }

    let blob_count = create_file_object(tx, path, &file_hash, &file_data, repo)?;

    db::staging::upsert_staging(
        tx, 
        &repo_relative_path, 
        &file_hash, 
        mode, 
        file_data.len() as u64,
        mtime
    )?;

    let outcome = if is_new {
        StageOutcome::Added { blobs: blob_count }
    } else {
        StageOutcome::Modified { blobs: blob_count }
    };

    Ok((path.clone(), outcome))
}

pub fn stage_files(repo: &Repository, paths: Vec<PathBuf>)
-> Result<Vec<(PathBuf, StageOutcome)>, ReviusError> {
    let mut results = Vec::new();
    
    let tx = repo.conn.unchecked_transaction()?;

    for path in paths {
        let result = stage_single_file(&tx, repo, &path)?;
        results.push(result);
    }

    tx.commit()?;

    Ok(results)
}