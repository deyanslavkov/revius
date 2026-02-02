use std::collections::{HashSet, VecDeque};
use rusqlite::Connection;

use crate::core::models::repository::Repository;
use crate::error::ReviusError;
use crate::utils::recipe::parse_recipe;
use crate::db;

#[derive(Default, Debug)]
pub struct GcStats {
    pub commits_deleted: usize,
    pub trees_deleted: usize,
    pub files_deleted: usize,
    pub blobs_deleted: usize,
}

pub fn run_garbage_collection(repo: &Repository, dry_run: bool) -> Result<GcStats, ReviusError> {
    let conn = &repo.conn;
    
    // 1. Mark Phase: Identification of all reachable objects
    // We do this in-memory. For extremely large repos, this might need optimization,
    // but for a lightweight VCS, in-memory sets of 32-byte hashes are efficient enough.
    
    let (alive_commits, alive_trees, alive_files, alive_blobs) = mark_reachable_objects(conn)?;

    // 2. Sweep Phase: DB operations
    
    // We must ensure the transaction is mutable for the Temp table operations
    let tx = conn.unchecked_transaction()
        .map_err(|e| ReviusError::Db(format!("Failed to start transaction: {}", e)))?;

    db::gc::create_keep_list_table(&tx)?;

    // Type IDs: 1=Commit, 2=Tree, 3=File, 4=Blob
    db::gc::populate_keep_list(&tx, &alive_commits, 1)?;
    db::gc::populate_keep_list(&tx, &alive_trees, 2)?;
    db::gc::populate_keep_list(&tx, &alive_files, 3)?;
    db::gc::populate_keep_list(&tx, &alive_blobs, 4)?;

    let stats = if dry_run {
        GcStats {
            commits_deleted: db::gc::count_unused_commits(&tx)?,
            trees_deleted: db::gc::count_unused_trees(&tx)?,
            files_deleted: db::gc::count_unused_files(&tx)?,
            blobs_deleted: db::gc::count_unused_blobs(&tx)?,
        }
        // Rollback implicit when tx is dropped, or explicit here
    } else {
        let commits = db::gc::delete_unused_commits(&tx)?;
        let trees = db::gc::delete_unused_trees(&tx)?;
        let files = db::gc::delete_unused_files(&tx)?;
        let blobs = db::gc::delete_unused_blobs(&tx)?;
        
        tx.commit().map_err(|e| ReviusError::Db(format!("Failed to commit GC transaction: {}", e)))?;

        // VACUUM must be run outside transaction
        db::gc::vacuum_db(&repo.conn)?;

        GcStats {
            commits_deleted: commits,
            trees_deleted: trees,
            files_deleted: files,
            blobs_deleted: blobs,
        }
    };

    Ok(stats)
}

fn mark_reachable_objects(conn: &Connection) 
    -> Result<(HashSet<[u8; 32]>, HashSet<[u8; 32]>, HashSet<[u8; 32]>, HashSet<[u8; 32]>), ReviusError> 
{
    let mut alive_commits = HashSet::new();
    let mut alive_trees = HashSet::new();
    let mut alive_files = HashSet::new();
    let mut alive_blobs = HashSet::new();

    // -- ROOTS --

    // 1. Refs (Branches, Tags)
    let refs = db::refs::get_all_refs(conn)?;
    for (_, commit_hash) in refs {
        alive_commits.insert(commit_hash);
    }

    // 2. HEAD (if detached)
    if let Some(head_hash) = db::refs::resolve_head(conn)? {
        alive_commits.insert(head_hash);
    }

    // 3. Staging Area (Files currently staged are roots for files)
    let staged = db::staging::get_all_staged(conn)?;
    for file in staged {
        alive_files.insert(file.file_hash);
    }

    // -- TRAVERSAL --

    // 1. Commits -> Parents (Commits) & Roots (Trees)
    let mut commit_queue: VecDeque<[u8; 32]> = alive_commits.iter().copied().collect();
    
    // We might have visited some commits if multiple refs point to them, 
    // but the set handles uniqueness. The queue needs to process them.
    // To avoid reprocessing, we track visited commits in the set (already done) 
    // and only push parents if they weren't in the set.
    
    // Actually, queue needs to be populated carefully. 
    // Optimization: Use a separate visited set for queue processing? 
    // No, `alive_commits` IS the visited set.
    // If we find a parent that is NOT in `alive_commits`, we add to set AND queue.
    
    while let Some(hash) = commit_queue.pop_front() {
        // Get commit info
        if let Some(commit) = db::commits::get_commit(conn, &hash)? {
            // Mark Tree
            alive_trees.insert(commit.tree_hash);

            // Mark Parents
            if let Some(parent) = commit.parent_hash {
                if alive_commits.insert(parent) {
                    commit_queue.push_back(parent);
                }
            }
            if let Some(merge_parent) = commit.merge_parent_hash {
                if alive_commits.insert(merge_parent) {
                    commit_queue.push_back(merge_parent);
                }
            }
        }
    }

    // 2. Trees -> Subtrees (Trees) & Files (Files)
    let mut tree_queue: VecDeque<[u8; 32]> = alive_trees.iter().copied().collect();

    while let Some(hash) = tree_queue.pop_front() {
        // Get children
        let entries = db::trees::get_tree_entries(conn, &hash)?;
        for entry in entries {
            if entry.is_dir {
                if alive_trees.insert(entry.object_hash) {
                    tree_queue.push_back(entry.object_hash);
                }
            } else {
                alive_files.insert(entry.object_hash);
            }
        }
    }

    // 3. Files -> Blobs
    // Files don't recurse, so no queue needed. Just iterate the set.
    for file_hash in &alive_files {
        // We need to parse the recipe to find blobs.
        // db::files::get_file returns FileInfo { size, recipe }
        match db::files::get_file(conn, file_hash) {
            Ok(file_info) => {
                 match parse_recipe(&file_info.recipe) {
                    Ok(blob_hashes) => {
                        for blob_hash in blob_hashes {
                            alive_blobs.insert(blob_hash);
                        }
                    },
                    Err(_) => {
                        // If recipe is invalid, we can't find blobs. 
                        // We log error but don't fail GC completely?
                        // For safety, we should probably fail.
                        return Err(ReviusError::Db(format!("Corrupt recipe in file {}", hex::encode(&file_hash[..8]))));
                    }
                 }
            },
            Err(_) => {
                // If file is missing but referenced, it's a corrupt repo state.
                // We skip it (it's already gone, so we can't mark its blobs).
            }
        }
    }

    Ok((alive_commits, alive_trees, alive_files, alive_blobs))
}