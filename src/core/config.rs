use crate::core::models::config::{Config, RepoConfig, UserConfig, UserInfo};
use crate::error::ReviusError;
use crate::fs;
use crate::fs::paths;
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

    if let Some(email) = &cfg.user_email
        && !email.contains('@') {
            return Err(ReviusError::Config("user.email appears invalid".to_string()));
        }

    Ok(())
}


pub fn load_default_repo_config() -> RepoConfig {
    RepoConfig::default()
}

pub fn load_user_config() -> Option<UserConfig> {
    fs::config::load_user_config().ok()
}

pub fn load_repo_config(repo_root: &Path) -> RepoConfig {
    fs::config::load_repo_config(repo_root).unwrap_or_default()
}

pub fn load_config(repo_root: &Path) -> Result<Config, ReviusError> {
    let repo_cfg = load_repo_config(repo_root);
    let user_cfg = load_user_config();
    merge(repo_cfg, user_cfg)
}

pub fn set_user_identity(name: &str, email: &str) -> Result<(), ReviusError> {
    let mut config = fs::config::load_user_config()?;
    
    let user_info = config.user.get_or_insert(UserInfo::default());
    user_info.name = Some(name.to_string());
    user_info.email = Some(email.to_string());
    
    let config_path = paths::get_user_config_path()
        .ok_or_else(|| ReviusError::Config("Could not determine user config path".to_string()))?;
        
    fs::config::write_user_config(&config_path, &config)
}

pub fn set_config_value(key: &str, value: &str) -> Result<String, ReviusError> {
    if key.starts_with("user.") {
        set_user_config_value(key, value)
    } else if key.starts_with("core.") {
        set_repo_config_value(key, value)
    } else {
         Err(ReviusError::Config(format!("Unknown configuration scope for key '{}'. Keys must start with 'user.' or 'core.'", key)))
    }
}

fn set_user_config_value(key: &str, value: &str) -> Result<String, ReviusError> {
     let mut config = fs::config::load_user_config()?;
     let user_info = config.user.get_or_insert(UserInfo::default());

     match key {
        "user.name" => user_info.name = Some(value.to_string()),
        "user.email" => user_info.email = Some(value.to_string()),
        _ => return Err(ReviusError::Config(format!("Unknown user config key: '{}'", key))),
     }
     
     let config_path = paths::get_user_config_path()
        .ok_or_else(|| ReviusError::Config("Could not determine user config path".to_string()))?;
        
     fs::config::write_user_config(&config_path, &config)?;
     Ok("global".to_string())
}

fn set_repo_config_value(key: &str, value: &str) -> Result<String, ReviusError> {
    let current_dir = paths::get_current_dir()?;
    let repo_root = paths::find_repo_root(&current_dir)?;
    
    let mut config = fs::config::load_repo_config(&repo_root)?;
    
    match key {
        "core.compression" => config.core.compression = parse_bool(value)?,
        "core.compression_level" => {
            let val = parse_u8(value)?;
            if !(1..=22).contains(&val) { return Err(ReviusError::Config("compression_level must be between 1 and 22".to_string())); }
            config.core.compression_level = val;
        },
        "core.chunking" => config.core.chunking = parse_bool(value)?,
        "core.chunk_min" => config.core.chunk_min = parse_u32(value)?,
        "core.chunk_avg" => config.core.chunk_avg = parse_u32(value)?,
        "core.chunk_max" => config.core.chunk_max = parse_u32(value)?,
        "core.case_sensitive" => config.core.case_sensitive = parse_bool(value)?,
        _ => return Err(ReviusError::Config(format!("Unknown core config key: '{}'", key))),
    }
    
    let config_path = paths::get_repo_config_path(&repo_root);
    fs::config::write_repo_config(&config_path, &config)?;
    Ok("local".to_string())
}

fn parse_bool(v: &str) -> Result<bool, ReviusError> {
    v.parse::<bool>().map_err(|_| ReviusError::Config(format!("Invalid boolean value: '{}' (expected true/false)", v)))
}

fn parse_u8(v: &str) -> Result<u8, ReviusError> {
    v.parse::<u8>().map_err(|_| ReviusError::Config(format!("Invalid number: '{}'", v)))
}

fn parse_u32(v: &str) -> Result<u32, ReviusError> {
    v.parse::<u32>().map_err(|_| ReviusError::Config(format!("Invalid number: '{}'", v)))
}