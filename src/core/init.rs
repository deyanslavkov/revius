use crate::core::config;
use crate::core::models::repository::Repository;
use crate::db;
use crate::error::ReviusError;
use crate::fs;
use crate::fs::paths;
use std::path::Path;

pub fn create_repository(path: &Path) -> Result<Repository, ReviusError> {
    let rvs_dir = paths::get_rvs_dir(path);
    let db_path = paths::get_repo_db_path(path);
    let config_path = paths::get_repo_config_path(path);
    let ignore_path = paths::get_repo_ignore_path(path);
    
    if rvs_dir.exists() {
        return Err(ReviusError::RepoAlreadyExists(path.to_path_buf()));
    }

    fs::io::create_dir(&rvs_dir)
        .map_err(|e| ReviusError::Io(rvs_dir.clone(), e))?;

    let conn = db::connection::open_db(&db_path)?;
    db::schema::create_all(&conn)?;

    let lock = fs::lock::LockFile::acquire(&path)?;

    let repo_config = config::load_default_repo_config();
    fs::config::write_repo_config(&config_path, &repo_config)?;

    fs::io::write_file(&ignore_path, "")
        .map_err(|e| ReviusError::Io(ignore_path.clone(), e))?;

    let user_config = config::load_user_config();
    let merged_config = config::merge(repo_config, user_config)?;

    Ok(Repository::new(path.to_path_buf(), merged_config, conn, lock))
}