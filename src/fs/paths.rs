use crate::error::ReviusError;
use std::path::{Path, PathBuf};

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

pub fn make_repo_relative(absolute_path: &Path, repo_root: &Path) -> Result<String, ReviusError> {
    let relative = absolute_path
        .strip_prefix(repo_root)
        .map_err(|_| {
            ReviusError::Path(format!(
                "Path {} is not inside repository {}",
                absolute_path.display(),
                repo_root.display()
            ))
        })?;
    
    relative
        .to_str()
        .ok_or_else(|| ReviusError::Path(format!("Path is not valid UTF-8: {}", absolute_path.display())))
        .map(|s| s.replace('\\', "/"))
}