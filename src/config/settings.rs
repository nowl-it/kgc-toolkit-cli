//! Application settings

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Custom tools directory
    pub tools_directory: Option<PathBuf>,

    /// Default output directory
    pub output_directory: Option<PathBuf>,

    /// Proxy port
    pub proxy_port: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            tools_directory: None,
            output_directory: None,
            proxy_port: 8888,
        }
    }
}

impl Settings {
    /// Get config file path
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kgc-toolkit")
            .join("config.toml")
    }

    /// Load settings from config file
    pub fn load() -> Self {
        let path = Self::config_path();

        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save settings to config file
    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self).unwrap_or_default();
        fs::write(path, content)
    }

    /// Get data directory
    pub fn data_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kgc-toolkit")
    }

    /// Get cache directory
    pub fn cache_dir() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kgc-toolkit")
    }
}
