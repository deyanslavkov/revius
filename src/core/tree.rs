use crate::core::models::objects::{StagedFile, TreeEntry};
use crate::error::ReviusError;
use crate::fs::paths;
use crate::utils::hash;
use crate::core::models::serialization;
use rusqlite::Transaction;
use std::collections::BTreeMap;

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