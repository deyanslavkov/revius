use blake3::Hasher;

// Type alias for 32-byte hash to ensure type safety across the app
pub type Hash = [u8; 32];

pub fn digest(data: &[u8]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(data); // It supports streams, but we don't need that right now.
    hasher.finalize().into()
}

// Helper to format hash as hex string (for UI/Logs)
pub fn to_hex(hash: &Hash) -> String {
    hex::encode(hash)
}
