use crate::error::ReviusError;
use std::path::{Path, PathBuf};
use std::io;
use std::fs;

pub fn get_current_dir() -> Result<PathBuf, ReviusError> {
    std::env::current_dir()
        .map_err(|e| ReviusError::Io(std::path::PathBuf::from("."), e))
}

pub fn get_rvs_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".rvs")
}

pub fn get_repo_db_path(repo_root: &Path) -> PathBuf {
    get_rvs_dir(repo_root).join("repo.db")
}

pub fn get_repo_lock_path(repo_root: &Path) -> PathBuf {
    get_rvs_dir(repo_root).join("lock")
}

pub fn get_repo_config_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".rvsconfig.toml")
}

pub fn get_repo_ignore_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".rvsignore")
}

pub fn get_user_config_path() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var("APPDATA")
            .ok()
            .map(|appdata| PathBuf::from(appdata).join("revius").join("config.toml"))
    } else {
        dirs::home_dir()
            .map(|home| home.join(".config").join("revius").join("config.toml"))
    }
}

/// Canonicalize a path (resolve symlinks, make absolute...). Fails if path doesn't exist
pub fn canonicalize(path: &Path) -> io::Result<PathBuf> {
    let canonical = fs::canonicalize(path)?;
    Ok(clean_path_display(&canonical))
}

/// Removes Windows UNC prefix
pub fn clean_path_display(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    
    if cfg!(windows) && path_str.starts_with(r"\\?\") {
        PathBuf::from(&path_str[4..])
    } else {
        path.to_path_buf()
    }
}

pub fn find_repo_root(start: &Path) -> Result<PathBuf, ReviusError> {
    let mut current = start.to_path_buf();
    loop {
        let rvs_dir = current.join(".rvs");
        if rvs_dir.exists() && rvs_dir.is_dir() {
            return Ok(current);
        }
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => {
                return Err(ReviusError::RepoNotFound(start.to_path_buf()));
            }
        }
    }
}

/// Also enforces UTF-8 encoding and forward slash separators
pub fn make_repo_relative(absolute_path: &Path, repo_root: &Path) -> Result<String, ReviusError> {
    let relative = absolute_path
        .strip_prefix(repo_root)
        .map_err(|_| {
            ReviusError::Path(format!(
                "Path {} is outside repository root {}. All files must be inside the repository.",
                absolute_path.display(),
                repo_root.display()
            ))
        })?;
    
    relative
        .to_str()
        .ok_or_else(|| ReviusError::Path(format!("Path is not valid UTF-8: {}", absolute_path.display())))
        .map(|s| s.replace('\\', "/"))
}

pub fn split_path(path: &str) -> Vec<&str> {
    path.split('/')
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn to_absolute(relative_path: &str, repo_root: &Path) -> PathBuf {
    repo_root.join(relative_path)
}

pub fn path_exists(path: &Path) -> bool {
    path.exists()
}