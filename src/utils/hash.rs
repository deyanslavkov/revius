use blake3;

pub fn hash_bytes(data: &[u8]) -> [u8; 32] {
    blake3::hash(data).into()
}

/// Takes a vector of bytes that's already a hash and enforces array size
pub fn vec_to_hash(vec: &[u8]) -> Result<[u8; 32], String> {
    if vec.len() != 32 {
        return Err(format!("Invalid hash length: expected 32, got {}", vec.len()));
    }
    let mut hash = [0u8; 32];
    hash.copy_from_slice(vec);
    Ok(hash)
}

pub fn hash_to_hex(hash: &[u8; 32]) -> String {
    hex::encode(hash)
}

/// Used for display in messages
pub fn hash_to_short_hex(hash: &[u8; 32]) -> String {
    hex::encode(&hash[..8])  // First 8 bytes means 16 hex chars
}

/// Validate that a string is a valid hex prefix (1-64 hex chars)
pub fn is_valid_hash_prefix(prefix: &str) -> bool {
    if prefix.is_empty() || prefix.len() > 64 {
        return false;
    }
    prefix.chars().all(|c| c.is_ascii_hexdigit())
}

/// Convert hex string to partial hash bytes (for prefix matching). Returns the bytes and the number of valid hex digits
pub fn hex_prefix_to_bytes(prefix: &str) -> Result<(Vec<u8>, usize), String> {
    if !is_valid_hash_prefix(prefix) {
        return Err(format!("Invalid hex prefix: {}", prefix));
    }
    
    // Decode as much as we can (pad with 0 if odd length)
    let padded = if prefix.len() % 2 == 1 {
        format!("{}0", prefix)
    } else {
        prefix.to_string()
    };
    
    let bytes = hex::decode(&padded)
        .map_err(|e| format!("Failed to decode hex: {}", e))?;
    
    Ok((bytes, prefix.len()))
}