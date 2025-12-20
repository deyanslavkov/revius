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