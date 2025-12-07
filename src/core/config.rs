use crate::core::models::config::{Config, RepoConfig, UserConfig};
use crate::error::ReviusError;
use crate::fs;
use std::path::Path;

pub fn merge(repo: RepoConfig, user: Option<UserConfig>) -> Result<Config, ReviusError> {
    let user_info = user.as_ref().and_then(|u| u.user.as_ref());

    let config = Config {
        compression: repo.core.compression,
        compression_level: repo.core.compression_level,
        chunking: repo.core.chunking,
        chunk_min: repo.core.chunk_min,
        chunk_avg: repo.core.chunk_avg,
        chunk_max: repo.core.chunk_max,
        case_sensitive: repo.core.case_sensitive,
        user_name: user_info.and_then(|u| u.name.clone()),
        user_email: user_info.and_then(|u| u.email.clone()),
    };

    validate(&config)?;
    Ok(config)
}

fn validate(cfg: &Config) -> Result<(), ReviusError> {
    if cfg.chunking {
        if cfg.chunk_min == 0 {
            return Err(ReviusError::Config(
                "chunk_min must be > 0".to_string(),
            ));
        }
        if cfg.chunk_avg < cfg.chunk_min {
            return Err(ReviusError::Config(
                "chunk_avg must be >= chunk_min".to_string(),
            ));
        }
        if cfg.chunk_max < cfg.chunk_avg {
            return Err(ReviusError::Config(
                "chunk_max must be >= chunk_avg".to_string(),
            ));
        }
    }

    if cfg.compression_level < 1 || cfg.compression_level > 22 {
        return Err(ReviusError::Config(
            "compression_level must be between 1 and 22".to_string(),
        ));
    }

    if let Some(email) = &cfg.user_email {
        if !email.contains('@') {
            return Err(ReviusError::Config("user.email appears invalid".to_string()));
        }
    }

    Ok(())
}


pub fn load_default_repo_config() -> RepoConfig {
    RepoConfig::default()
}

pub fn load_user_config() -> Option<UserConfig> {
    match fs::config::load_user_config() {
        Ok(cfg) => Some(cfg),
        Err(_) => None,
    }
}

pub fn load_repo_config(repo_root: &Path) -> RepoConfig {
    fs::config::load_repo_config(repo_root).unwrap_or_default()
}

pub fn load_config(repo_root: &Path) -> Result<Config, ReviusError> {
    let repo_cfg = load_repo_config(repo_root);
    let user_cfg = load_user_config();
    merge(repo_cfg, user_cfg)
}