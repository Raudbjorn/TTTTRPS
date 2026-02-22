use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Top-level application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub tui: TuiConfig,
    pub data: DataConfig,
}

/// TUI-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TuiConfig {
    /// Tick interval in milliseconds for the event loop.
    pub tick_rate_ms: u64,
    /// Enable mouse support in the terminal.
    pub mouse_enabled: bool,
    /// Theme name (reserved for future use).
    pub theme: String,
}

/// Data directory configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DataConfig {
    /// Override the default data directory.
    pub data_dir: Option<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            tui: TuiConfig::default(),
            data: DataConfig::default(),
        }
    }
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            tick_rate_ms: 50,
            mouse_enabled: false,
            theme: "default".to_string(),
        }
    }
}

impl Default for DataConfig {
    fn default() -> Self {
        Self { data_dir: None }
    }
}

impl AppConfig {
    /// Load configuration from `~/.config/ttttrps/config.toml`.
    /// Returns `Default` if the file is missing or unparseable.
    pub fn load() -> Self {
        let config_path = Self::config_path();
        match std::fs::read_to_string(&config_path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => {
                    log::info!("Loaded config from {}", config_path.display());
                    config
                }
                Err(e) => {
                    log::warn!(
                        "Failed to parse config at {}: {e} — using defaults",
                        config_path.display()
                    );
                    Self::default()
                }
            },
            Err(_) => {
                log::debug!(
                    "No config file at {} — using defaults",
                    config_path.display()
                );
                Self::default()
            }
        }
    }

    /// Resolved data directory (override or XDG default).
    pub fn data_dir(&self) -> PathBuf {
        self.data
            .data_dir
            .clone()
            .unwrap_or_else(|| {
                dirs::data_dir()
                    .map(|d| d.join("ttrpg-assistant"))
                    .unwrap_or_else(|| PathBuf::from("data"))
            })
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .map(|d| d.join("ttttrps").join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.tui.tick_rate_ms, 50);
        assert!(!config.tui.mouse_enabled);
        assert_eq!(config.tui.theme, "default");
        assert!(config.data.data_dir.is_none());
    }

    #[test]
    fn test_config_load_missing_file() {
        // Should return defaults without panicking
        let config = AppConfig::load();
        assert_eq!(config.tui.tick_rate_ms, 50);
    }

    #[test]
    fn test_data_dir_default() {
        let config = AppConfig::default();
        let dir = config.data_dir();
        assert!(dir.to_string_lossy().contains("ttrpg-assistant") || dir == PathBuf::from("data"));
    }

    #[test]
    fn test_data_dir_override() {
        let mut config = AppConfig::default();
        config.data.data_dir = Some(PathBuf::from("/tmp/custom"));
        assert_eq!(config.data_dir(), PathBuf::from("/tmp/custom"));
    }

    #[test]
    fn test_toml_roundtrip() {
        let config = AppConfig::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: AppConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.tui.tick_rate_ms, config.tui.tick_rate_ms);
    }
}
