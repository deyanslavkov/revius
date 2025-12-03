use crate::fs::config as fs_config;
use crate::error::ReviusError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Runtime configuration used throughout the repo.
/// This is the resolved configuration after merging defaults, user config, and repo config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub repository_format: i32,
    pub name: String,
    pub cdc_min: usize,
    pub cdc_avg: usize,
    pub cdc_max: usize,
    pub zstd_level: i32,
}

impl Config {
    /// Defaults for a repo; name derived from root folder.
    pub fn default_for_root(root: impl AsRef<Path>) -> Self {
        let name = root.as_ref()
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed_repo")
            .to_string();

        Self {
            repository_format: 1,
            name,
            cdc_min: 16 * 1024,
            cdc_avg: 32 * 1024,
            cdc_max: 64 * 1024,
            zstd_level: 3,
        }
    }

    /// Load configuration for repo at root.
    /// Merge order: defaults < user-global (TODO) < repo-file < CLI overrides (future).
    pub fn load(root: impl AsRef<Path>) -> Result<Self, ReviusError> {
        let mut cfg = Config::default_for_root(&root);

        // TODO: Load user/global config and merge (XDG paths / %APPDATA%)
        // For now we only load repo-level file if present.
        let repo_config_path = fs_config::repo_config_path(&root);
        if let Ok(Some(file_cfg)) = fs_config::read_repo_config(&repo_config_path) {
            // apply optional fields
            if let Some(v) = file_cfg.core.repository_format { cfg.repository_format = v; }
            if let Some(v) = file_cfg.core.name { cfg.name = v; }
            if let Some(v) = file_cfg.core.cdc_min { cfg.cdc_min = v; }
            if let Some(v) = file_cfg.core.cdc_avg { cfg.cdc_avg = v; }
            if let Some(v) = file_cfg.core.cdc_max { cfg.cdc_max = v; }
            if let Some(v) = file_cfg.core.zstd_level { cfg.zstd_level = v; }
        }

        validate(&cfg)?;
        Ok(cfg)
    }
}

fn validate(cfg: &Config) -> Result<(), ReviusError> {
    if !(cfg.cdc_min < cfg.cdc_avg && cfg.cdc_avg < cfg.cdc_max) {
        return Err(ReviusError::Config("cdc_min < cdc_avg < cdc_max must hold".into()));
    }
    if !(1..=22).contains(&cfg.zstd_level) {
        return Err(ReviusError::Config("zstd_level must be between 1 and 22".into()));
    }
    if cfg.name.trim().is_empty() {
        return Err(ReviusError::Config("repository name must be non-empty".into()));
    }
    Ok(())
}
