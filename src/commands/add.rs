use crate::cli::args::AddArgs;
use crate::cli::ui;
use crate::core;
use crate::error::ReviusError;
use crate::fs;
use crate::fs::paths;

pub fn run(args: AddArgs) -> Result<(), ReviusError> {
    let current_dir = paths::get_current_dir()?;

    let repo = core::open::open_repository(&current_dir)?;

    // These are the scopes the user explicitly asked for
    let mut canonical_paths = Vec::new();
    for path in args.paths {
        let canonical = fs::paths::canonicalize(&path)
            .map_err(|e| ReviusError::Io(path.clone(), e))?;
        canonical_paths.push(canonical);
    }

    let ignore_path = fs::paths::get_repo_ignore_path(&repo.root);
    
    // These are the actual existing files found within those scopes
    // Note: If a file was deleted, it won't appear here.
    let found_files = fs::walk::expand_paths(canonical_paths.clone(), &repo.root, &ignore_path)?;

    // We pass both: found_files (to add/modify) and canonical_paths (to check for deletions)
    let results = core::add::stage_files(&repo, found_files, canonical_paths)?;

    let mut added_count = 0;
    let mut modified_count = 0;
    let mut deleted_count = 0;
    let mut unchanged_count = 0;
    let mut total_blobs = 0;

    for (path, outcome) in results {
        let repo_relative = fs::paths::make_repo_relative(&path, &repo.root)
            .unwrap_or_else(|_| path.to_string_lossy().to_string());

        match outcome {
            core::add::StageOutcome::Added { blobs } => {
                ui::print_added_file(&repo_relative);
                added_count += 1;
                total_blobs += blobs;
            }
            core::add::StageOutcome::Modified { blobs } => {
                ui::print_modified_file(&repo_relative);
                modified_count += 1;
                total_blobs += blobs;
            }
            core::add::StageOutcome::Deleted => {
                ui::print_deleted_file(&repo_relative);
                deleted_count += 1;
            }
            core::add::StageOutcome::Unchanged => {
                unchanged_count += 1;
            }
        }
    }

    ui::print_add_summary(added_count, modified_count, deleted_count, unchanged_count, total_blobs);

    Ok(())
}