use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to determine config directory")]
    NoConfigDir,
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("Failed to serialize config: {0}")]
    SerializeError(#[from] toml::ser::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Ollama API URL
    #[serde(default = "default_ollama_url")]
    pub ollama_url: String,

    /// Default model to use
    #[serde(default)]
    pub default_model: Option<String>,

    /// Default system prompt
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Copy result to clipboard automatically
    #[serde(default)]
    pub auto_copy: bool,

    /// Show GPU stats panel
    #[serde(default = "default_show_gpu_stats")]
    pub show_gpu_stats: bool,
}

fn default_ollama_url() -> String {
    "http://127.0.0.1:11434".to_string()
}

fn default_show_gpu_stats() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ollama_url: default_ollama_url(),
            default_model: None,
            system_prompt: None,
            auto_copy: false,
            show_gpu_stats: default_show_gpu_stats(),
        }
    }
}

impl Config {
    /// Get the config file path
    pub fn config_path() -> Result<PathBuf, ConfigError> {
        ProjectDirs::from("com", "locallm", "locallm")
            .map(|dirs| dirs.config_dir().join("config.toml"))
            .ok_or(ConfigError::NoConfigDir)
    }

    /// Load config from disk, or create default if it doesn't exist
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::config_path()?;

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save config to disk
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::config_path()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}
