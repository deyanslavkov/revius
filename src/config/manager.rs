use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::fs;
use crate::error::ReviusError;
use crate::core::cdc::CdcParams;

#[derive(Debug, Deserialize, Clone)]
pub struct UserConfig {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CoreConfig {
    pub cdc_min: Option<usize>,
    pub cdc_avg: Option<usize>,
    pub cdc_max: Option<usize>,
    pub zstd_level: Option<i32>,
    pub repository_format: Option<i32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ConfigFile {
    pub user: Option<UserConfig>,
    pub core: Option<CoreConfig>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub user_name: String,
    pub user_email: String,
    
    // Core settings
    pub cdc: CdcParams,
    pub zstd_level: i32,
    pub repo_format: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            user_name: "Unknown".to_string(),
            user_email: "unknown@example.com".to_string(),
            cdc: CdcParams {
                min_size: 16 * 1024,
                avg_size: 32 * 1024,
                max_size: 64 * 1024,
            },
            zstd_level: 3,
            repo_format: 1,
        }
    }
}

impl Config {
    pub fn load(repo_root: Option<&Path>) -> Result<Self, ReviusError> {
        let mut final_config = Config::default();

        // 1. Load User Global Config (~/.config/revius/config.toml)
        if let Some(config_dir) = dirs::config_dir() {
            let global_path = config_dir.join("revius/config.toml");
            if global_path.exists() {
                let content = fs::read_to_string(global_path)?;
                let partial: ConfigFile = toml::from_str(&content)
                    .map_err(|e| ReviusError::Config(format!("User config error: {}", e)))?;
                final_config.merge(partial);
            }
        }

        // 2. Load Repo Local Config (.rvsconfig.toml)
        if let Some(root) = repo_root {
            let repo_path = root.join(".rvsconfig.toml");
            if repo_path.exists() {
                let content = fs::read_to_string(repo_path)?;
                let partial: ConfigFile = toml::from_str(&content)
                    .map_err(|e| ReviusError::Config(format!("Repo config error: {}", e)))?;
                final_config.merge(partial);
            }
        }

        Ok(final_config)
    }

    /// Overlays values from a file on top of the current config
    fn merge(&mut self, file: ConfigFile) {
        if let Some(u) = file.user {
            if !u.name.is_empty() { self.user_name = u.name; }
            if !u.email.is_empty() { self.user_email = u.email; }
        }

        if let Some(c) = file.core {
            if let Some(v) = c.cdc_min { self.cdc.min_size = v; }
            if let Some(v) = c.cdc_avg { self.cdc.avg_size = v; }
            if let Some(v) = c.cdc_max { self.cdc.max_size = v; }
            if let Some(v) = c.zstd_level { self.zstd_level = v; }
            if let Some(v) = c.repository_format { self.repo_format = v; }
        }
    }
}