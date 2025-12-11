use crate::error::ReviusError;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

pub fn expand_paths(
    paths: Vec<PathBuf>,
    repo_root: &Path,
    ignore_path: &Path,
) -> Result<Vec<PathBuf>, ReviusError> {
    let mut result = Vec::new();
    let rvs_dir = repo_root.join(".rvs");

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
            let mut builder = WalkBuilder::new(&path);
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
        }
    }

    Ok(result)
}