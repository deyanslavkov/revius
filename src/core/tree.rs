use crate::core::models::objects::{StagedFile, TreeEntry};
use crate::error::ReviusError;
use crate::fs::paths;
use crate::utils::hash;
use crate::core::models::serialization;
use rusqlite::{Connection, Transaction};
use std::collections::BTreeMap;
use crate::db;

#[derive(Debug)]
pub enum TreeNode {
    Dir {
        children: BTreeMap<String, TreeNode>,
    },
    File {
        hash: [u8; 32],
        mode: u32,
    },
}

impl TreeNode {
    pub fn new_dir() -> Self {
        TreeNode::Dir {
            children: BTreeMap::new(),
        }
    }

    pub fn new_file(hash: [u8; 32], mode: u32) -> Self {
        TreeNode::File { hash, mode }
    }
}

/// Build in-memory tree structure from a list of files with paths
pub fn build_tree_from_files(files: Vec<StagedFile>) -> Result<TreeNode, ReviusError> {
    let mut root = TreeNode::new_dir();

    for file in files {
        let path_parts = paths::split_path(&file.path);
        
        // Split path into directory components and the final filename
        // e.g., "src/main.rs" -> (["src"], "main.rs")
        if let Some((filename, dir_parts)) = path_parts.split_last() {
            let mut current = &mut root;

            // Phase 1: Navigate or create directories
            for (i, part) in dir_parts.iter().enumerate() {
                match current {
                    TreeNode::Dir { children } => {
                        current = children
                            .entry(part.to_string())
                            .or_insert_with(TreeNode::new_dir);
                    }
                    TreeNode::File { .. } => {
                        // Conflict: trying to treat a file as a directory
                        let conflict_path = dir_parts[..=i].join("/");
                        return Err(ReviusError::Path(format!(
                            "Path conflict: '{}' is a file, cannot create directory for '{}'",
                            conflict_path, file.path
                        )));
                    }
                }
            }

            // Phase 2: Insert the file at the leaf
            if let TreeNode::Dir { children } = current {
                children.insert(
                    filename.to_string(),
                    TreeNode::new_file(file.file_hash, file.mode),
                );
            } else {
                // Conflict: trying to insert a file into a node that became a file during traversal
                // (This branch is theoretically unreachable if the loop logic holds, but good for safety)
                let conflict_path = dir_parts.join("/");
                return Err(ReviusError::Path(format!(
                    "Path conflict: '{}' is a file, cannot create file '{}'",
                    conflict_path, file.path
                )));
            }
        }
    }

    Ok(root)
}

/// Recursively write tree entries to database and return parent_hash
pub fn write_tree_to_db(tx: &Transaction, node: &TreeNode) -> Result<[u8; 32], ReviusError> {
    match node {
        TreeNode::File { .. } => {
            Err(ReviusError::Db("Cannot write tree for file node".to_string()))
        }
        TreeNode::Dir { children } => {
            // 1. Recursively process children and collect data
            // Stores: (name, hash, mode, is_dir)
            let mut child_data = Vec::with_capacity(children.len());

            for (name, child) in children.iter() {
                match child {
                    TreeNode::File { hash, mode } => {
                        child_data.push((name.clone(), *hash, *mode, false));
                    }
                    TreeNode::Dir { .. } => {
                        let child_hash = write_tree_to_db(tx, child)?;
                        child_data.push((name.clone(), child_hash, 0o040000, true));
                    }
                }
            }

            // 2. Serialize for hashing (needs reference to names)
            let mut serialized = Vec::new();
            for (name, hash, mode, _is_dir) in &child_data {
                serialized.extend_from_slice(&serialization::serialize_tree_entry(*mode, name, hash));
            }

            let parent_hash = hash::hash_bytes(&serialized);

            // 3. Optimization: If tree already exists, don't write entries again
            if crate::db::trees::tree_exists(tx, &parent_hash)? {
                return Ok(parent_hash);
            }

            // 4. Create DB objects (consume child_data to avoid cloning strings again)
            let tree_entries: Vec<TreeEntry> = child_data
                .into_iter()
                .map(|(name, hash, mode, is_dir)| TreeEntry {
                    parent_hash,
                    name, // Move the string here
                    object_hash: hash,
                    mode,
                    is_dir,
                })
                .collect();

            crate::db::trees::batch_insert_tree_entries(tx, tree_entries)?;

            Ok(parent_hash)
        }
    }
}

// --- Traversal Wrappers ---

/// Returns a map of repo-relative path -> file_hash for all files in the tree
pub fn get_all_tree_files(conn: &Connection, tree_hash: &[u8; 32])
-> Result<BTreeMap<String, [u8; 32]>, ReviusError> {
    let mut result = BTreeMap::new();
    
    walk_tree(conn, tree_hash, "", &mut |path, entry| {
        if !entry.is_dir {
            result.insert(path.to_string(), entry.object_hash);
        }
        Ok(())
    })?;
    
    Ok(result)
}

/// Get the complete file tree for a commit as a flat map: path -> (file_hash, mode)
/// Returns None for file_hash if the entry is a directory
pub fn get_tree_snapshot(
    conn: &Connection,
    tree_hash: [u8; 32],
) -> Result<BTreeMap<String, (Option<[u8; 32]>, u32)>, ReviusError> {
    let mut snapshot = BTreeMap::new();

    walk_tree(conn, &tree_hash, "", &mut |path, entry| {
        if !entry.is_dir {
             snapshot.insert(path.to_string(), (Some(entry.object_hash), entry.mode));
        }
        Ok(())
    })?;

    Ok(snapshot)
}

/// Get all file entries from a tree (recursively) for staging reconstruction. 
/// Returns Vec<(relative_path, file_hash, mode, size)>
pub fn get_all_files_in_tree(
    conn: &Connection,
    tree_hash: &[u8; 32],
) -> Result<Vec<(String, [u8; 32], u32, u64)>, ReviusError> {
    let mut results = Vec::new();

    walk_tree(conn, tree_hash, "", &mut |path, entry| {
        if !entry.is_dir {
            // Only this use case needs file size
            let size = db::trees::get_file_size(conn, &entry.object_hash)?;
            results.push((path.to_string(), entry.object_hash, entry.mode, size));
        }
        Ok(())
    })?;

    Ok(results)
}

// --- The Single Recursion Engine ---

/// Generic recursive tree walker.
/// Visits every node and calls `callback`. Recurses automatically for directories.
fn walk_tree<F>(
    conn: &Connection,
    parent_hash: &[u8; 32],
    path_prefix: &str,
    callback: &mut F
) -> Result<(), ReviusError>
where
    F: FnMut(&str, &TreeEntry) -> Result<(), ReviusError>,
{
    let entries = db::trees::get_tree_entries(conn, parent_hash)?;

    for entry in entries {
        let full_path = if path_prefix.is_empty() {
            entry.name.clone()
        } else {
            format!("{}/{}", path_prefix, entry.name)
        };

        callback(&full_path, &entry)?;

        if entry.is_dir {
            walk_tree(conn, &entry.object_hash, &full_path, callback)?;
        }
    }

    Ok(())
}