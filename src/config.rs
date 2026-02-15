use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application configuration loaded from `~/.config/jdx/config.toml`.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// AI provider settings
    pub ai: AiConfig,
    /// Display settings
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    /// AI provider: "ollama", "openai", or "none"
    pub provider: String,
    /// Model name (e.g., "llama3.2", "gpt-4o-mini")
    pub model: String,
    /// API key (for OpenAI/Anthropic)
    pub api_key: String,
    /// Custom API endpoint URL
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    /// Monochrome mode (no colors)
    pub monochrome: bool,
    /// Max items to show in candidate popup
    pub max_candidates: usize,
    /// Max depth for schema inference sampling
    pub schema_max_samples: usize,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: "none".into(),
            model: String::new(),
            api_key: String::new(),
            endpoint: String::new(),
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            monochrome: false,
            max_candidates: 20,
            schema_max_samples: 10,
        }
    }
}

/// Get the configuration directory path.
pub fn config_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", "jdx").map(|dirs| dirs.config_dir().to_path_buf())
}

/// Get the config file path.
pub fn config_file_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("config.toml"))
}

/// Load configuration from disk, or return defaults.
pub fn load_config() -> AppConfig {
    match config_file_path() {
        Some(path) if path.exists() => match fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => AppConfig::default(),
        },
        _ => AppConfig::default(),
    }
}

/// Save the current configuration to disk.
pub fn save_config(config: &AppConfig) -> Result<()> {
    if let Some(dir) = config_dir() {
        fs::create_dir_all(&dir)?;
        let path = dir.join("config.toml");
        let content = toml::to_string_pretty(config)?;
        fs::write(path, content)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.ai.provider, "none");
        assert!(!config.display.monochrome);
        assert_eq!(config.display.max_candidates, 20);
    }

    #[test]
    fn test_parse_config_toml() {
        let toml_str = r#"
[ai]
provider = "ollama"
model = "llama3.2"

[display]
monochrome = true
max_candidates = 10
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.ai.provider, "ollama");
        assert_eq!(config.ai.model, "llama3.2");
        assert!(config.display.monochrome);
        assert_eq!(config.display.max_candidates, 10);
    }

    #[test]
    fn test_partial_config() {
        let toml_str = r#"
[ai]
provider = "openai"
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.ai.provider, "openai");
        // Defaults for unspecified
        assert!(!config.display.monochrome);
    }
}
