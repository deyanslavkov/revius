use crate::core::config;
use crate::core::models::repository::Repository;
use crate::db;
use crate::error::ReviusError;
use crate::fs::paths;
use std::path::Path;

pub fn open_repository(start_path: &Path) -> Result<Repository, ReviusError> {
    let repo_root = paths::find_repo_root(start_path)?;
    let db_path = paths::get_repo_db_path(&repo_root);
    
    let conn = db::connection::open_db(&db_path)?;

    db::meta::check_schema_version(&conn)?;
    
    let config = config::load_config(&repo_root)?;
    
    Ok(Repository::new(repo_root, config, conn))
}