use crate::core::models::config::{RepoConfig, UserConfig};
use crate::error::ReviusError;
use crate::fs::paths;
use std::path::Path;

pub fn write_repo_config(path: &Path, config: &RepoConfig) -> Result<(), ReviusError> {
    let toml_string = toml::to_string_pretty(config)
        .map_err(|e| ReviusError::Config(format!("Failed to serialize repo config: {}", e)))?;
    
    std::fs::write(path, toml_string)
        .map_err(|e| ReviusError::Io(path.to_path_buf(), e))?;
    
    Ok(())
}

pub fn write_user_config(path: &Path, config: &UserConfig) -> Result<(), ReviusError> {
    let toml_string = toml::to_string_pretty(config)
        .map_err(|e| ReviusError::Config(format!("Failed to serialize user config: {}", e)))?;
    
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ReviusError::Io(parent.to_path_buf(), e))?;
    }
    
    std::fs::write(path, toml_string)
        .map_err(|e| ReviusError::Io(path.to_path_buf(), e))?;
    
    Ok(())
}

pub fn load_repo_config(repo_root: &Path) -> Result<RepoConfig, ReviusError> {
    let config_path = paths::get_repo_config_path(repo_root);
    
    if !config_path.exists() {
        return Ok(RepoConfig::default());
    }
    
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| ReviusError::Io(config_path.clone(), e))?;
    
    toml::from_str(&content)
        .map_err(|e| ReviusError::Config(format!("Failed to parse repo config at {}: {}", config_path.display(), e)))
}

pub fn load_user_config() -> Result<UserConfig, ReviusError> {
    let config_path = paths::get_user_config_path()
        .ok_or_else(|| ReviusError::Config("Could not determine user config path".to_string()))?;
    
    if !config_path.exists() {
        return Ok(UserConfig::default());
    }
    
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| ReviusError::Io(config_path.clone(), e))?;
    
    toml::from_str(&content)
        .map_err(|e| ReviusError::Config(format!("Failed to parse user config at {}: {}", config_path.display(), e)))
}