use crate::error::ReviusError;
use crate::fs::paths::get_rvs_dir;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Returns absolute paths to files
pub fn expand_paths(
    paths: Vec<PathBuf>,
    repo_root: &Path,
    ignore_path: &Path,
) -> Result<Vec<PathBuf>, ReviusError> {
    let mut result = Vec::new();

    for path in paths {
        if !path.exists() {
            return Err(ReviusError::Path(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }

        if path.is_file() {
            result.push(path);
        } else if path.is_dir() {
            let files = walk_directory(&path, repo_root, ignore_path)?;
            result.extend(files);
        }
    }

    Ok(result)
}

/// Get all files in the working directory that aren't ignored
/// Returns absolute paths to all non-ignored files in the repository
pub fn get_all_repo_files(
    repo_root: &Path,
    ignore_path: &Path,
) -> Result<Vec<PathBuf>, ReviusError> {
    walk_directory(repo_root, repo_root, ignore_path)
}

/// Core directory walking implementation
/// Walks a directory tree and returns all non-ignored files
pub fn walk_directory(
    start_path: &Path,
    repo_root: &Path,
    ignore_path: &Path,
) -> Result<Vec<PathBuf>, ReviusError> {
    let rvs_dir = get_rvs_dir(repo_root);
    let mut result = Vec::new();

    let mut builder = WalkBuilder::new(start_path);
    builder.add_ignore(ignore_path);
    builder.hidden(false);
    builder.git_ignore(false);

    for entry in builder.build() {
        match entry {
            Ok(e) => {
                let entry_path = e.path();

                if entry_path.starts_with(&rvs_dir) {
                    continue;
                }

                if e.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                    result.push(entry_path.to_path_buf());
                }
            }
            Err(_) => continue,
        }
    }

    Ok(result)
}