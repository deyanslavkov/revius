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

pub struct FileInfo {
    pub size: i64,
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

/// Complete status information comparing HEAD, staging, and working directory
#[derive(Debug)]
pub struct StatusInfo {
    pub branch_name: Option<String>,
    pub detached_commit: Option<[u8; 32]>,
    pub staged_new: Vec<String>,
    pub staged_modified: Vec<String>,
    pub staged_deleted: Vec<String>,
    pub unstaged_modified: Vec<String>,
    pub unstaged_deleted: Vec<String>,
    pub untracked: Vec<String>,
}

/// Helper impl for usage in status display
impl StatusInfo {
    pub fn has_changes(&self) -> bool {
        !self.staged_new.is_empty()
            || !self.staged_modified.is_empty()
            || !self.staged_deleted.is_empty()
            || !self.unstaged_modified.is_empty()
            || !self.unstaged_deleted.is_empty()
            || !self.untracked.is_empty()
    }

    pub fn has_staged_changes(&self) -> bool {
        !self.staged_new.is_empty()
            || !self.staged_modified.is_empty()
            || !self.staged_deleted.is_empty()
    }
}

#[derive(Debug)]
pub struct LogOptions {
    pub limit: Option<usize>,
    pub show_graph: bool,
    pub oneline: bool,
    pub first_parent: bool,
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: [u8; 32],
    pub parent_hash: Option<[u8; 32]>,
    pub merge_parent_hash: Option<[u8; 32]>,
    pub tree_hash: [u8; 32],
    pub author_name: String,
    pub author_email: String,
    pub timestamp: i64,
    pub message: String,
    pub refs: Vec<String>, // Branch/tag names pointing to this commit
}

#[derive(Debug)]
pub struct SwitchResult {
    pub previous_head: HeadState,
    pub new_head: HeadState,
    pub files_changed: usize,
    pub files_deleted: usize,
}

#[derive(Debug, Clone)]
pub enum HeadState {
    Branch(String, [u8; 32]),
    Detached([u8; 32]),
}

#[derive(Debug)]
pub enum TargetType {
    Branch(String),
    Commit,
}

pub struct SwitchPlan {
    pub to_add: Vec<(String, [u8; 32], u32)>, // (path, file_hash, mode)
    pub to_modify: Vec<(String, [u8; 32], u32)>, // (path, file_hash, mode)
    pub to_delete: Vec<String>, // path
}