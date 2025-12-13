/// Serialization functions for computing canonical hashes of objects

/// Serialize a tree entry for hashing
/// Format: mode (4 bytes big-endian) || name (UTF-8 bytes) || null byte || hash (32 bytes)
pub fn serialize_tree_entry(mode: u32, name: &str, hash: &[u8; 32]) -> Vec<u8> {
    let mut result = Vec::new();
    
    // Mode as 4 bytes big-endian
    result.extend_from_slice(&mode.to_be_bytes());
    
    // Name as UTF-8 bytes
    result.extend_from_slice(name.as_bytes());
    
    // Null byte separator
    result.push(0);
    
    // Hash
    result.extend_from_slice(hash);
    
    result
}

/// Serialize a commit for hashing
/// Format:
/// tree <tree_hash_hex>\n
/// parent <parent_hash_hex>\n (if exists)
/// merge_parent <merge_parent_hash_hex>\n (if exists)
/// author <name> <email> <timestamp>\n
/// message\n
/// <message_text>
pub fn serialize_commit(
    tree_hash: &[u8; 32],
    parent_hash: Option<&[u8; 32]>,
    merge_parent_hash: Option<&[u8; 32]>,
    author_name: &str,
    author_email: &str,
    timestamp: i64,
    message: &str,
) -> Vec<u8> {
    let mut result = String::new();
    
    // Tree
    result.push_str(&format!("tree {}\n", hex::encode(tree_hash)));
    
    // Parent (if exists)
    if let Some(parent) = parent_hash {
        result.push_str(&format!("parent {}\n", hex::encode(parent)));
    }
    
    // Merge parent (if exists)
    if let Some(merge_parent) = merge_parent_hash {
        result.push_str(&format!("merge_parent {}\n", hex::encode(merge_parent)));
    }
    
    // Author
    result.push_str(&format!("author {} {} {}\n", author_name, author_email, timestamp));
    
    // Message header
    result.push_str("message\n");
    
    // Message content
    result.push_str(message);
    
    result.into_bytes()
}