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
    
    // Reconstruct content by fetching, decompressing, and concatenating blobs
    let mut content = Vec::with_capacity(file_info.size as usize);
    
    for blob_hash in blob_hashes {
        let compressed_data = db::blobs::get_blob(conn, &blob_hash)?;
        let decompressed_data = utils::compression::decompress(&compressed_data)?;
        content.extend_from_slice(&decompressed_data);
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
    if let Some(parent) = target_path.parent() {
        if !fs::paths::path_exists(parent) {
            fs::io::create_dir_all(parent)
                .map_err(|e| ReviusError::Io(parent.to_path_buf(), e))?;
        }
    }
    
    // Write file
    fs::io::write_binary(target_path, &content)
        .map_err(|e| ReviusError::Io(target_path.to_path_buf(), e))?;
    
    // Set executable bit if needed (mode 100755)
    if mode == 100755 {
        fs::io::set_executable(target_path)
            .map_err(|e| ReviusError::Io(target_path.to_path_buf(), e))?;
    }
    
    Ok(())
}