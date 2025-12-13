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

#[derive(Debug, Clone)]
pub struct Commit {
    pub hash: [u8; 32],
    pub parent_hash: Option<[u8; 32]>,
    pub merge_parent_hash: Option<[u8; 32]>,
    pub tree_hash: [u8; 32],
    pub message: String,
    pub author_id: i64,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub parent_hash: [u8; 32],
    pub name: String,
    pub object_hash: [u8; 32],
    pub mode: u32,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub struct Author {
    pub id: i64,
    pub name: String,
    pub email: String,
}