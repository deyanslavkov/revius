use crate::core::models::objects::MODE_EXEC;
use crate::db;
use crate::error::ReviusError;
use crate::fs;
use crate::utils;
use rusqlite::Connection;
use std::path::Path;

/// Reconstruct file content from database (Files + Blobs + recipe)
pub fn reconstruct_file(
    conn: &Connection,
    file_hash: &[u8; 32],
) -> Result<Vec<u8>, ReviusError> {
    // Get file metadata and recipe
    let file_info = db::files::get_file(conn, file_hash)?;
    
    // Parse recipe to get blob hashes
    let blob_hashes = utils::recipe::parse_recipe(&file_info.recipe)
        .map_err(|e| ReviusError::Db(format!("Failed to parse recipe: {}", e)))?;
    
    // Reconstruct content by fetching, optionally decompressing, and concatenating blobs
    let mut content = Vec::with_capacity(file_info.size as usize);
    
    for blob_hash in blob_hashes {
        // Fetch data AND compression mode
        let (data, compression_algo) = db::blobs::get_blob(conn, &blob_hash)?;
        
        let chunk_data = if compression_algo == "none" {
            // No decompression needed
            data
        } else if compression_algo.starts_with("zstd") {
            // Decompress
            utils::compression::decompress(&data)?
        } else {
            // Unknown algorithm (future proofing)
            return Err(ReviusError::Db(format!(
                "Unsupported compression algorithm '{}' for blob {}", 
                compression_algo, 
                utils::hash::hash_to_short_hex(&blob_hash)
            )));
        };

        content.extend_from_slice(&chunk_data);
    }
    
    // Verify size
    if content.len() != file_info.size as usize {
        return Err(ReviusError::Db(format!(
            "File size mismatch: expected {}, got {}",
            file_info.size,
            content.len()
        )));
    }
    
    Ok(content)
}

/// Write reconstructed content to working directory
pub fn checkout_file(
    conn: &Connection,
    file_hash: &[u8; 32],
    target_path: &Path,
    mode: u32,
) -> Result<(), ReviusError> {
    // Reconstruct file content
    let content = reconstruct_file(conn, file_hash)?;
    
    // Ensure parent directory exists
    if let Some(parent) = target_path.parent()
        && !fs::paths::path_exists(parent) {
            fs::io::create_dir_all(parent)
                .map_err(|e| ReviusError::Io(parent.to_path_buf(), e))?;
        }
    
    // Write file
    fs::io::write_binary(target_path, &content)
        .map_err(|e| ReviusError::Io(target_path.to_path_buf(), e))?;
    
    // Set executable bit if needed
    if mode == MODE_EXEC {
        fs::io::set_executable(target_path)
            .map_err(|e| ReviusError::Io(target_path.to_path_buf(), e))?;
    }
    
    Ok(())
}