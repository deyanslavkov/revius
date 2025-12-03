use crate::core::hash::{self, Hash};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileRecipe {
    pub size: u64,
    pub chunks: Vec<Hash>, 
}

impl FileRecipe {
    /// Serializes the recipe to bytes for storage in the DB (packed hashes).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.chunks.len() * 32);
        for hash in &self.chunks {
            out.extend_from_slice(hash);
        }
        out
    }

    /// Reconstructs a recipe from raw DB bytes.
    pub fn from_bytes(size: u64, bytes: &[u8]) -> Self {
        let mut chunks = Vec::new();
        for chunk in bytes.chunks_exact(32) {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(chunk);
            chunks.push(hash);
        }
        Self { size, chunks }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeEntry {
    pub mode: u32,       // 100644 (file), 100755 (executable file), 040000 (dir)
    pub name: String,
    pub hash: Hash,      // Points to File or another Tree
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

impl Tree {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn add_entry(&mut self, mode: u32, name: String, hash: Hash) {
        self.entries.push(TreeEntry { mode, name, hash });
    }

    /// Canonical serialization for hashing: sort_by_name -> "mode name\0hash"
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut sorted_entries = self.entries.clone();
        sorted_entries.sort_by(|a, b| a.name.cmp(&b.name));

        let mut buffer = Vec::new();
        for entry in sorted_entries {
            buffer.extend_from_slice(format!("{} ", entry.mode).as_bytes());
            buffer.extend_from_slice(entry.name.as_bytes());
            buffer.push(b'\0');
            buffer.extend_from_slice(&entry.hash);
        }
        buffer
    }

    pub fn get_hash(&self) -> Hash {
        hash::digest(&self.to_canonical_bytes())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub tree_hash: Hash,
    pub parent_hash: Option<Hash>,
    pub author_id: i64, 
    pub timestamp: i64, 
    pub message: String,
}

impl Commit {
    /// Canonical serialization:
    /// "tree {hash}\n"
    /// "parent {hash}\n" (optional)
    /// "author {id} {timestamp}\n"
    /// "message\n{content}"
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        buffer.extend_from_slice(b"tree ");
        buffer.extend_from_slice(&self.tree_hash);
        buffer.push(b'\n');

        if let Some(parent) = self.parent_hash {
            buffer.extend_from_slice(b"parent ");
            buffer.extend_from_slice(&parent);
            buffer.push(b'\n');
        }

        let author_line = format!("author {} {}\n", self.author_id, self.timestamp);
        buffer.extend_from_slice(author_line.as_bytes());

        buffer.extend_from_slice(b"message\n");
        buffer.extend_from_slice(self.message.as_bytes());

        buffer
    }

    pub fn get_hash(&self) -> Hash {
        hash::digest(&self.to_canonical_bytes())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum RefType {
    Branch = 0,
    Tag = 1,
    Remote = 2,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Reference {
    pub path: String,
    pub ref_type: RefType,
    pub commit_hash: Hash,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StagingEntry {
    pub path: String,
    pub file_hash: Hash,
    pub mode: u32,
    pub size: u64,
    pub modified_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Author {
    pub id: i64,
    pub name: String,
    pub email: String,
}