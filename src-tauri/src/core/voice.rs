//! Voice Synthesis Module
//!
//! Integrates with TTS providers for NPC voice generation:
//! - ElevenLabs (cloud-based, high quality)
//! - Ollama (local, open source models)
//! - System TTS (fallback)

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use reqwest::Client;
use tokio::fs;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum VoiceError {
    #[error("Provider not configured: {0}")]
    NotConfigured(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid voice ID: {0}")]
    InvalidVoiceId(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Quota exceeded")]
    QuotaExceeded,
}

pub type Result<T> = std::result::Result<T, VoiceError>;

// ============================================================================
// Voice Configuration
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub provider: VoiceProvider,
    pub cache_dir: Option<PathBuf>,
    pub default_voice_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VoiceProvider {
    ElevenLabs {
        api_key: String,
        model_id: Option<String>,
    },
    Ollama {
        base_url: String,
        model: String,
    },
    System,
    Disabled,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            provider: VoiceProvider::Disabled,
            cache_dir: None,
            default_voice_id: None,
        }
    }
}

// ============================================================================
// Voice Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voice {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub description: Option<String>,
    pub preview_url: Option<String>,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSettings {
    pub stability: f32,        // 0.0 - 1.0
    pub similarity_boost: f32, // 0.0 - 1.0
    pub style: f32,            // 0.0 - 1.0 (ElevenLabs v2 only)
    pub use_speaker_boost: bool,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            stability: 0.5,
            similarity_boost: 0.75,
            style: 0.0,
            use_speaker_boost: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisRequest {
    pub text: String,
    pub voice_id: String,
    pub settings: Option<VoiceSettings>,
    pub output_format: OutputFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum OutputFormat {
    #[default]
    Mp3,
    Wav,
    Ogg,
    Pcm,
}

impl OutputFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::Ogg => "ogg",
            Self::Pcm => "pcm",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Mp3 => "audio/mpeg",
            Self::Wav => "audio/wav",
            Self::Ogg => "audio/ogg",
            Self::Pcm => "audio/pcm",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisResult {
    pub audio_path: PathBuf,
    pub duration_ms: Option<u64>,
    pub format: OutputFormat,
    pub cached: bool,
}

// ============================================================================
// Voice Client
// ============================================================================

pub struct VoiceClient {
    config: VoiceConfig,
    http_client: Client,
    cache_dir: PathBuf,
}

impl VoiceClient {
    pub fn new(config: VoiceConfig) -> Self {
        let cache_dir = config.cache_dir.clone()
            .unwrap_or_else(|| PathBuf::from("./voice_cache"));

        Self {
            config,
            http_client: Client::new(),
            cache_dir,
        }
    }

    /// Synthesize speech from text
    pub async fn synthesize(&self, request: SynthesisRequest) -> Result<SynthesisResult> {
        // Check cache first
        let cache_key = self.cache_key(&request);
        let cache_path = self.cache_dir.join(&cache_key);

        if cache_path.exists() {
            return Ok(SynthesisResult {
                audio_path: cache_path,
                duration_ms: None,
                format: request.output_format,
                cached: true,
            });
        }

        // Ensure cache directory exists
        fs::create_dir_all(&self.cache_dir).await?;

        // Synthesize based on provider
        let audio_data = match &self.config.provider {
            VoiceProvider::ElevenLabs { api_key, model_id } => {
                self.synthesize_elevenlabs(
                    &request,
                    api_key,
                    model_id.as_deref(),
                ).await?
            }
            VoiceProvider::Ollama { base_url, model } => {
                self.synthesize_ollama(&request, base_url, model).await?
            }
            VoiceProvider::System => {
                // System TTS would need platform-specific implementation
                return Err(VoiceError::NotConfigured("System TTS not implemented".to_string()));
            }
            VoiceProvider::Disabled => {
                return Err(VoiceError::NotConfigured("Voice synthesis disabled".to_string()));
            }
        };

        // Save to cache
        fs::write(&cache_path, &audio_data).await?;

        Ok(SynthesisResult {
            audio_path: cache_path,
            duration_ms: None,
            format: request.output_format,
            cached: false,
        })
    }

    /// List available voices
    pub async fn list_voices(&self) -> Result<Vec<Voice>> {
        match &self.config.provider {
            VoiceProvider::ElevenLabs { api_key, .. } => {
                self.list_elevenlabs_voices(api_key).await
            }
            VoiceProvider::Ollama { base_url, .. } => {
                self.list_ollama_voices(base_url).await
            }
            VoiceProvider::System => {
                // Return some default system voices
                Ok(vec![
                    Voice {
                        id: "system-default".to_string(),
                        name: "System Default".to_string(),
                        provider: "system".to_string(),
                        description: Some("Default system TTS voice".to_string()),
                        preview_url: None,
                        labels: vec!["system".to_string()],
                    }
                ])
            }
            VoiceProvider::Disabled => {
                Ok(vec![])
            }
        }
    }

    /// Check usage/quota
    pub async fn check_usage(&self) -> Result<UsageInfo> {
        match &self.config.provider {
            VoiceProvider::ElevenLabs { api_key, .. } => {
                self.check_elevenlabs_usage(api_key).await
            }
            _ => Ok(UsageInfo::default()),
        }
    }

    /// Clear the voice cache
    pub async fn clear_cache(&self) -> Result<usize> {
        let mut count = 0;

        if self.cache_dir.exists() {
            let mut entries = fs::read_dir(&self.cache_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                if entry.path().is_file() {
                    fs::remove_file(entry.path()).await?;
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    // ========================================================================
    // ElevenLabs Implementation
    // ========================================================================

    async fn synthesize_elevenlabs(
        &self,
        request: &SynthesisRequest,
        api_key: &str,
        model_id: Option<&str>,
    ) -> Result<Vec<u8>> {
        let url = format!(
            "https://api.elevenlabs.io/v1/text-to-speech/{}",
            request.voice_id
        );

        let settings = request.settings.clone().unwrap_or_default();

        let body = serde_json::json!({
            "text": request.text,
            "model_id": model_id.unwrap_or("eleven_monolingual_v1"),
            "voice_settings": {
                "stability": settings.stability,
                "similarity_boost": settings.similarity_boost,
                "style": settings.style,
                "use_speaker_boost": settings.use_speaker_boost
            }
        });

        let response = self.http_client
            .post(&url)
            .header("xi-api-key", api_key)
            .header("Content-Type", "application/json")
            .header("Accept", request.output_format.mime_type())
            .json(&body)
            .send()
            .await?;

        if response.status() == 429 {
            return Err(VoiceError::RateLimitExceeded);
        }

        if response.status() == 401 {
            return Err(VoiceError::ApiError("Invalid API key".to_string()));
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(VoiceError::ApiError(format!(
                "ElevenLabs API error: {}", error_text
            )));
        }

        Ok(response.bytes().await?.to_vec())
    }

    async fn list_elevenlabs_voices(&self, api_key: &str) -> Result<Vec<Voice>> {
        let response = self.http_client
            .get("https://api.elevenlabs.io/v1/voices")
            .header("xi-api-key", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(VoiceError::ApiError("Failed to list voices".to_string()));
        }

        let data: serde_json::Value = response.json().await?;

        let voices = data["voices"].as_array()
            .map(|arr| {
                arr.iter().filter_map(|v| {
                    Some(Voice {
                        id: v["voice_id"].as_str()?.to_string(),
                        name: v["name"].as_str()?.to_string(),
                        provider: "elevenlabs".to_string(),
                        description: v["description"].as_str().map(String::from),
                        preview_url: v["preview_url"].as_str().map(String::from),
                        labels: v["labels"].as_object()
                            .map(|obj| obj.values()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect())
                            .unwrap_or_default(),
                    })
                }).collect()
            })
            .unwrap_or_default();

        Ok(voices)
    }

    async fn check_elevenlabs_usage(&self, api_key: &str) -> Result<UsageInfo> {
        let response = self.http_client
            .get("https://api.elevenlabs.io/v1/user/subscription")
            .header("xi-api-key", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(VoiceError::ApiError("Failed to check usage".to_string()));
        }

        let data: serde_json::Value = response.json().await?;

        Ok(UsageInfo {
            characters_used: data["character_count"].as_u64().unwrap_or(0),
            characters_limit: data["character_limit"].as_u64().unwrap_or(0),
            next_reset: data["next_character_count_reset_unix"]
                .as_i64()
                .map(|ts| chrono::DateTime::from_timestamp(ts, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()),
        })
    }

    // ========================================================================
    // Ollama TTS Implementation
    // ========================================================================

    async fn synthesize_ollama(
        &self,
        request: &SynthesisRequest,
        base_url: &str,
        model: &str,
    ) -> Result<Vec<u8>> {
        // Ollama doesn't have native TTS, but some models like Bark can be used
        // This is a placeholder for when Ollama adds TTS support or for custom models

        let url = format!("{}/api/tts", base_url.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": model,
            "input": request.text,
            "voice": request.voice_id,
        });

        let response = self.http_client
            .post(&url)
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                Ok(resp.bytes().await?.to_vec())
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                Err(VoiceError::ApiError(format!("Ollama TTS error: {}", error)))
            }
            Err(e) => {
                // Ollama might not have TTS endpoint
                Err(VoiceError::NotConfigured(format!(
                    "Ollama TTS not available: {}", e
                )))
            }
        }
    }

    async fn list_ollama_voices(&self, base_url: &str) -> Result<Vec<Voice>> {
        // Return some default voices for Ollama-based TTS
        Ok(vec![
            Voice {
                id: "default".to_string(),
                name: "Default".to_string(),
                provider: "ollama".to_string(),
                description: Some("Default Ollama TTS voice".to_string()),
                preview_url: None,
                labels: vec!["local".to_string()],
            },
        ])
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    fn cache_key(&self, request: &SynthesisRequest) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        request.text.hash(&mut hasher);
        request.voice_id.hash(&mut hasher);

        format!(
            "{:x}.{}",
            hasher.finish(),
            request.output_format.extension()
        )
    }
}

// ============================================================================
// Usage Information
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageInfo {
    pub characters_used: u64,
    pub characters_limit: u64,
    pub next_reset: Option<String>,
}

impl UsageInfo {
    pub fn remaining(&self) -> u64 {
        self.characters_limit.saturating_sub(self.characters_used)
    }

    pub fn usage_percent(&self) -> f32 {
        if self.characters_limit == 0 {
            0.0
        } else {
            (self.characters_used as f32 / self.characters_limit as f32) * 100.0
        }
    }
}

// ============================================================================
// Voice Presets for NPCs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NPCVoicePreset {
    pub name: String,
    pub voice_id: String,
    pub settings: VoiceSettings,
    pub description: String,
    pub suitable_for: Vec<String>,
}

pub fn get_voice_presets() -> Vec<NPCVoicePreset> {
    vec![
        NPCVoicePreset {
            name: "Wise Elder".to_string(),
            voice_id: "pNInz6obpgDQGcFmaJgB".to_string(), // ElevenLabs "Adam"
            settings: VoiceSettings {
                stability: 0.7,
                similarity_boost: 0.8,
                style: 0.3,
                use_speaker_boost: true,
            },
            description: "Deep, measured voice suitable for mentors and sages".to_string(),
            suitable_for: vec!["mentor".to_string(), "elder".to_string(), "wizard".to_string()],
        },
        NPCVoicePreset {
            name: "Tavern Keeper".to_string(),
            voice_id: "yoZ06aMxZJJ28mfd3POQ".to_string(), // ElevenLabs "Sam"
            settings: VoiceSettings {
                stability: 0.5,
                similarity_boost: 0.7,
                style: 0.5,
                use_speaker_boost: true,
            },
            description: "Friendly, warm voice for innkeepers and merchants".to_string(),
            suitable_for: vec!["merchant".to_string(), "innkeeper".to_string(), "commoner".to_string()],
        },
        NPCVoicePreset {
            name: "Mysterious Stranger".to_string(),
            voice_id: "21m00Tcm4TlvDq8ikWAM".to_string(), // ElevenLabs "Rachel"
            settings: VoiceSettings {
                stability: 0.3,
                similarity_boost: 0.9,
                style: 0.7,
                use_speaker_boost: true,
            },
            description: "Enigmatic voice for rogues and mysterious figures".to_string(),
            suitable_for: vec!["rogue".to_string(), "spy".to_string(), "informant".to_string()],
        },
        NPCVoicePreset {
            name: "Noble Authority".to_string(),
            voice_id: "AZnzlk1XvdvUeBnXmlld".to_string(), // ElevenLabs "Domi"
            settings: VoiceSettings {
                stability: 0.8,
                similarity_boost: 0.8,
                style: 0.2,
                use_speaker_boost: true,
            },
            description: "Commanding voice for nobles and authority figures".to_string(),
            suitable_for: vec!["noble".to_string(), "guard".to_string(), "authority".to_string()],
        },
        NPCVoicePreset {
            name: "Sinister Villain".to_string(),
            voice_id: "VR6AewLTigWG4xSOukaG".to_string(), // ElevenLabs "Arnold"
            settings: VoiceSettings {
                stability: 0.4,
                similarity_boost: 0.9,
                style: 0.8,
                use_speaker_boost: true,
            },
            description: "Menacing voice for villains and antagonists".to_string(),
            suitable_for: vec!["villain".to_string(), "boss".to_string(), "enemy".to_string()],
        },
    ]
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_config_default() {
        let config = VoiceConfig::default();
        assert!(matches!(config.provider, VoiceProvider::Disabled));
    }

    #[test]
    fn test_output_format() {
        assert_eq!(OutputFormat::Mp3.extension(), "mp3");
        assert_eq!(OutputFormat::Wav.mime_type(), "audio/wav");
    }

    #[test]
    fn test_voice_settings_default() {
        let settings = VoiceSettings::default();
        assert_eq!(settings.stability, 0.5);
        assert_eq!(settings.similarity_boost, 0.75);
    }

    #[test]
    fn test_usage_info() {
        let usage = UsageInfo {
            characters_used: 5000,
            characters_limit: 10000,
            next_reset: None,
        };

        assert_eq!(usage.remaining(), 5000);
        assert_eq!(usage.usage_percent(), 50.0);
    }

    #[test]
    fn test_voice_presets() {
        let presets = get_voice_presets();
        assert!(!presets.is_empty());
        assert!(presets.iter().any(|p| p.suitable_for.contains(&"merchant".to_string())));
    }

    #[tokio::test]
    async fn test_voice_client_disabled() {
        let config = VoiceConfig::default();
        let client = VoiceClient::new(config);

        let request = SynthesisRequest {
            text: "Hello world".to_string(),
            voice_id: "test".to_string(),
            settings: None,
            output_format: OutputFormat::Mp3,
        };

        let result = client.synthesize(request).await;
        assert!(result.is_err());
    }
}
