#[derive(Debug, Clone)]
pub struct Blob {
    pub hash: [u8; 32],
    pub data: Vec<u8>,
    pub compression: String,
    pub uncompressed_size: u64,
}

#[derive(Debug, Clone)]
pub struct File {
    pub hash: [u8; 32],
    pub size: u64,
    pub recipe_version: u32,
    pub chunk_count: u64,
    pub recipe: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct StagedFile {
    pub path: String,
    pub file_hash: [u8; 32],
    pub mode: u32,
    pub size: u64,
}