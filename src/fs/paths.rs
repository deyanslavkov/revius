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