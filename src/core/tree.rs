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
        
        if path_parts.is_empty() {
            continue;
        }

        let mut current = &mut root;

        // Navigate/create directories
        for (i, part) in path_parts.iter().enumerate() {
            let is_last = i == path_parts.len() - 1;

            if is_last {
                // This is the file - insert it
                if let TreeNode::Dir { children } = current {
                    children.insert(
                        part.to_string(),
                        TreeNode::new_file(file.file_hash, file.mode),
                    );
                } else {
                    // Path conflict
                    let conflict_path = path_parts[..i].join("/");
                    return Err(ReviusError::Path(format!(
                        "Path conflict: '{}' is a file, cannot create '{}'",
                        conflict_path,
                        file.path
                    )));
                }
            } else {
                // This is a directory - navigate into it
                match current {
                    TreeNode::Dir { children } => {
                        current = children
                            .entry(part.to_string())
                            .or_insert_with(TreeNode::new_dir);
                    }
                    TreeNode::File { .. } => {
                        // Path conflict
                        let conflict_path = path_parts[..i].join("/");
                        return Err(ReviusError::Path(format!(
                            "Path conflict: '{}' is a file, cannot create '{}'",
                            conflict_path,
                            file.path
                        )));
                    }
                }
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
            let mut entries = Vec::new();

            // Process all children
            for (name, child) in children.iter() {
                match child {
                    TreeNode::File { hash, mode } => {
                        entries.push((name.clone(), *hash, *mode, false));
                    }
                    TreeNode::Dir { .. } => {
                        let child_hash = write_tree_to_db(tx, child)?;
                        entries.push((name.clone(), child_hash, 0o040000, true));
                    }
                }
            }

            let mut serialized = Vec::new();
            for (name, hash, mode, _is_dir) in &entries {
                serialized.extend_from_slice(&serialization::serialize_tree_entry(*mode, name, hash));
            }

            let parent_hash = hash::hash_bytes(&serialized);

            if crate::db::trees::tree_exists(tx, &parent_hash)? {
                return Ok(parent_hash);
            }

            let tree_entries: Vec<TreeEntry> = entries
                .iter()
                .map(|(name, hash, mode, is_dir)| TreeEntry {
                    parent_hash,
                    name: name.clone(),
                    object_hash: *hash,
                    mode: *mode,
                    is_dir: *is_dir,
                })
                .collect();

            crate::db::trees::batch_insert_tree_entries(tx, tree_entries)?;

            Ok(parent_hash)
        }
    }
}

/// Recursively traverse a tree and return all file paths with their hashes
/// Returns a map of repo-relative path -> file_hash for all files in the tree
pub fn get_all_tree_files(conn: &Connection, tree_hash: &[u8; 32])
-> Result<BTreeMap<String, [u8; 32]>, ReviusError> {
    let mut result = BTreeMap::new();
    traverse_tree_recursive(conn, tree_hash, "", &mut result)?;
    Ok(result)
}

fn traverse_tree_recursive(
    conn: &Connection,
    parent_hash: &[u8; 32],
    current_path: &str,
    result: &mut BTreeMap<String, [u8; 32]>,
) -> Result<(), ReviusError> {
    let entries = db::trees::get_tree_entries(conn, parent_hash)?;

    for entry in entries {
        let full_path = if current_path.is_empty() {
            entry.name.clone()
        } else {
            format!("{}/{}", current_path, entry.name)
        };

        if entry.is_dir {
            traverse_tree_recursive(conn, &entry.object_hash, &full_path, result)?;
        } else {
            result.insert(full_path, entry.object_hash);
        }
    }

    Ok(())
}

/// Get the complete file tree for a commit as a flat map: path -> (file_hash, mode)
/// Returns None for file_hash if the entry is a directory
pub fn get_tree_snapshot(
    conn: &Connection,
    tree_hash: [u8; 32],
) -> Result<BTreeMap<String, (Option<[u8; 32]>, u32)>, ReviusError> {
    let mut snapshot = BTreeMap::new();
    collect_tree_entries(conn, tree_hash, String::new(), &mut snapshot)?;
    Ok(snapshot)
}

fn collect_tree_entries(
    conn: &Connection,
    parent_hash: [u8; 32],
    current_path: String,
    snapshot: &mut BTreeMap<String, (Option<[u8; 32]>, u32)>,
) -> Result<(), ReviusError> {
    let entries = db::trees::get_tree_entries(conn, &parent_hash)?;

    for entry in entries {
        let full_path = if current_path.is_empty() {
            entry.name.clone()
        } else {
            format!("{}/{}", current_path, entry.name)
        };

        if entry.is_dir {
            // It's a directory - recurse
            collect_tree_entries(conn, entry.object_hash, full_path, snapshot)?;
        } else {
            // It's a file - add to snapshot
            snapshot.insert(full_path, (Some(entry.object_hash), entry.mode));
        }
    }

    Ok(())
}

/// Get all file entries from a tree (recursively) for staging reconstruction. 
/// Returns Vec<(relative_path, file_hash, mode, size)>
pub fn get_all_files_in_tree(
    conn: &Connection,
    tree_hash: &[u8; 32],
) -> Result<Vec<(String, [u8; 32], u32, u64)>, ReviusError> {
    let mut results = Vec::new();
    collect_files_recursive(conn, tree_hash, "", &mut results)?;
    Ok(results)
}

fn collect_files_recursive(
    conn: &Connection,
    parent_hash: &[u8; 32],
    prefix: &str,
    results: &mut Vec<(String, [u8; 32], u32, u64)>,
) -> Result<(), ReviusError> {
    let entries = db::trees::get_tree_entries(conn, parent_hash)?;

    for entry in entries {
        let full_path = if prefix.is_empty() {
            entry.name.clone()
        } else {
            format!("{}/{}", prefix, entry.name)
        };

        if entry.is_dir {
            collect_files_recursive(conn, &entry.object_hash, &full_path, results)?;
        } else {
            // Fetch size for files to support staging reconstruction
            let size = db::trees::get_file_size(conn, &entry.object_hash)?;
            results.push((full_path, entry.object_hash, entry.mode, size));
        }
    }
    
    Ok(())
}