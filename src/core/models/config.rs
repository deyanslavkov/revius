use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    #[serde(default)]
    pub core: CoreConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    #[serde(default = "default_compression")]
    pub compression: bool,
    
    #[serde(default = "default_compression_level")]
    pub compression_level: u8,
    
    #[serde(default = "default_chunking")]
    pub chunking: bool,
    
    #[serde(default = "default_chunk_min")]
    pub chunk_min: u32,
    
    #[serde(default = "default_chunk_avg")]
    pub chunk_avg: u32,
    
    #[serde(default = "default_chunk_max")]
    pub chunk_max: u32,
    
    #[serde(default = "default_case_sensitive")]
    pub case_sensitive: bool,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            compression: default_compression(),
            compression_level: default_compression_level(),
            chunking: default_chunking(),
            chunk_min: default_chunk_min(),
            chunk_avg: default_chunk_avg(),
            chunk_max: default_chunk_max(),
            case_sensitive: default_case_sensitive(),
        }
    }
}

impl Default for RepoConfig {
    fn default() -> Self {
        Self {
            core: CoreConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    #[serde(default)]
    pub user: Option<UserInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub email: Option<String>,
}

impl Default for UserInfo {
    fn default() -> Self {
        Self {
            name: None,
            email: None,
        }
    }
}

/// The config struct, used in the Repository object
#[derive(Debug, Clone)]
pub struct Config {
    pub compression: bool,
    pub compression_level: u8,
    pub chunking: bool,
    pub chunk_min: u32,
    pub chunk_avg: u32,
    pub chunk_max: u32,
    pub user_name: Option<String>,
    pub user_email: Option<String>,
}

fn default_compression() -> bool {
    true
}

fn default_compression_level() -> u8 {
    3
}

fn default_chunking() -> bool {
    true
}

fn default_chunk_min() -> u32 {
    8192
}

fn default_chunk_avg() -> u32 {
    16384
}

fn default_chunk_max() -> u32 {
    32768
}

fn default_case_sensitive() -> bool {
    true
}