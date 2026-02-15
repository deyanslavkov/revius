use crate::core::models::repository::Repository;
use crate::error::ReviusError;
use crate::utils::recipe::parse_recipe;
use crate::db;
use std::collections::HashSet;

#[derive(Default, Debug)]
pub struct GcStats {
    pub commits_deleted: usize,
    pub trees_deleted: usize,
    pub files_deleted: usize,
    pub blobs_deleted: usize,
}

pub fn run_garbage_collection(repo: &Repository, dry_run: bool) -> Result<GcStats, ReviusError> {
    let conn = &repo.conn;

    // We start a transaction immediately because we need to write to the temp KeepList table
    let tx = conn.unchecked_transaction()
        .map_err(|e| ReviusError::Db(format!("Failed to start transaction: {}", e)))?;

    // 1. Create Temporary Table
    db::gc::create_keep_list_table(&tx)?;

    // 2. Resolve Detached HEAD (if any)
    // We pass this explicitly because it's an application-level concept
    let detached_head = db::refs::resolve_head(conn)?;
    let detached_head_ref = detached_head.as_ref();

    // 3. Mark Phase (DB Side): Commits, Trees, and Files
    // This uses Recursive CTEs to find all reachable objects and populates KeepList (Types 1, 2, 3)
    // It returns the list of reachable Files so we can find Blobs
    let alive_files = db::gc::mark_repository_structure(&tx, detached_head_ref)?;

    // 4. Mark Phase (App Side): Blobs
    // We must parse file recipes to find which blobs are used.
    let mut alive_blobs = HashSet::new();

    for file_hash in alive_files {
        // We look up the recipe. 
        // Note: db::files::get_file does a SELECT. Since we are in a transaction, this is fine.
        match db::files::get_file(conn, &file_hash) {
            Ok(file_info) => {
                 match parse_recipe(&file_info.recipe) {
                    Ok(blob_hashes) => {
                        for blob_hash in blob_hashes {
                            alive_blobs.insert(blob_hash);
                        }
                    },
                    Err(_) => {
                        return Err(ReviusError::Db(format!("Corrupt recipe in file {}", hex::encode(&file_hash[..8]))));
                    }
                 }
            },
            Err(_) => {
                // If a file is in KeepList (reachable) but missing from Files table, 
                // the repo is corrupt, or it's a phantom reference. 
                // We proceed without crashing to clean up what we can.
            }
        }
    }

    // 5. Push Blobs to KeepList (Type 4)
    db::gc::populate_keep_list(&tx, &alive_blobs, 4)?;

    // 6. Sweep Phase
    let stats = if dry_run {
        GcStats {
            commits_deleted: db::gc::count_unused_commits(&tx)?,
            trees_deleted: db::gc::count_unused_trees(&tx)?,
            files_deleted: db::gc::count_unused_files(&tx)?,
            blobs_deleted: db::gc::count_unused_blobs(&tx)?,
        }
        // Rollback implicit
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