use crate::error::ReviusError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;

/// Repo-level config file representation (fields optional for partial overrides)
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RepoConfigFile {
    pub core: RepoConfigCore,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RepoConfigCore {
    pub repository_format: Option<i32>,
    pub name: Option<String>,
    pub cdc_min: Option<usize>,
    pub cdc_avg: Option<usize>,
    pub cdc_max: Option<usize>,
    pub zstd_level: Option<i32>,
}

/// Defaults used when writing the initial file
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RepoConfigDefaults {
    pub core: RepoConfigCore,
}

impl RepoConfigDefaults {
    pub fn for_root(root: impl AsRef<std::path::Path>) -> Self {
        let name = root.as_ref()
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("repository")
            .to_string();

        RepoConfigDefaults {
            core: RepoConfigCore {
                repository_format: Some(1),
                name: Some(name),
                cdc_min: Some(16 * 1024),
                cdc_avg: Some(32 * 1024),
                cdc_max: Some(64 * 1024),
                zstd_level: Some(3),
            }
        }
    }
}

/// Return the canonical path to the repo config file for a root.
pub fn repo_config_path(root: impl AsRef<Path>) -> PathBuf {
    root.as_ref().join(".rvsconfig.toml")
}

/// Read repo config file if present.
pub fn read_repo_config(path: impl AsRef<Path>) -> Result<Option<RepoConfigFile>, ReviusError> {
    let p = path.as_ref();
    if !p.exists() {
        return Ok(None);
    }
    let s = fs::read_to_string(p)?;
    let cfg: RepoConfigFile = toml::from_str(&s).map_err(|e| ReviusError::Config(format!("Failed to parse repo config: {}", e)))?;
    Ok(Some(cfg))
}

/// Write default config only if missing.
pub fn write_default_repo_config(path: impl AsRef<Path>, defaults: &RepoConfigDefaults) -> Result<(), ReviusError> {
    let p = path.as_ref();
    if p.exists() {
        return Ok(());
    }
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    let toml = toml::to_string_pretty(&defaults).map_err(|e| ReviusError::Config(format!("Failed to serialize config: {}", e)))?;
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(p)?;
    f.write_all(toml.as_bytes())?;
    Ok(())
}
