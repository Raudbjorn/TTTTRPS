use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::llm::providers::ProviderConfig;
use crate::core::search::embeddings::EmbeddingConfig;
use crate::core::voice::types::VoiceConfig;

/// Top-level application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub tui: TuiConfig,
    pub data: DataConfig,
    pub llm: LlmConfig,
    pub voice: VoiceConfig,
    pub embedding: EmbeddingConfig,
    pub budget: BudgetConfig,
    pub transcription: TranscriptionConfig,
}

/// Budget enforcement configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BudgetConfig {
    /// Enable budget enforcement.
    pub enabled: bool,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self { enabled: false }
    }
}

/// Transcription provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TranscriptionConfig {
    /// Preferred provider: "whisper" or "groq".
    pub provider: String,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            provider: "whisper".to_string(),
        }
    }
}

/// LLM provider configuration (persisted to disk).
///
/// API keys are NOT stored here — they live in the system keyring.
/// This only stores provider type, model, host, and non-secret settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    /// Configured providers, keyed by provider ID (e.g., "ollama", "openai").
    pub providers: HashMap<String, ProviderConfig>,
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
            llm: LlmConfig::default(),
            voice: VoiceConfig::default(),
            embedding: EmbeddingConfig::default(),
            budget: BudgetConfig::default(),
            transcription: TranscriptionConfig::default(),
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

    /// Save configuration to `~/.config/ttttrps/config.toml`.
    pub fn save(&self) -> Result<(), String> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {e}"))?;
        }
        let contents =
            toml::to_string_pretty(self).map_err(|e| format!("Failed to serialize config: {e}"))?;
        std::fs::write(&config_path, contents)
            .map_err(|e| format!("Failed to write config: {e}"))?;
        log::info!("Saved config to {}", config_path.display());
        Ok(())
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

    #[test]
    fn test_llm_config_default_empty() {
        let config = AppConfig::default();
        assert!(config.llm.providers.is_empty());
    }

    #[test]
    fn test_llm_config_roundtrip() {
        use crate::core::llm::providers::ProviderConfig;

        let mut config = AppConfig::default();
        config.llm.providers.insert(
            "ollama".to_string(),
            ProviderConfig::Ollama {
                host: "http://localhost:11434".to_string(),
                model: "llama3.2".to_string(),
            },
        );
        config.llm.providers.insert(
            "openai".to_string(),
            ProviderConfig::OpenAI {
                api_key: String::new(), // Not persisted with actual key
                model: "gpt-4o".to_string(),
                max_tokens: 4096,
                organization_id: None,
                base_url: None,
            },
        );

        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: AppConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.llm.providers.len(), 2);
        assert!(deserialized.llm.providers.contains_key("ollama"));
        assert!(deserialized.llm.providers.contains_key("openai"));
    }
}
