use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

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

    #[error("Unsupported format")]
    UnsupportedFormat,
}

pub type Result<T> = std::result::Result<T, VoiceError>;

// ============================================================================
// Voice Configuration
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub provider: VoiceProviderType,
    pub cache_dir: Option<PathBuf>,
    pub default_voice_id: Option<String>,
    pub elevenlabs: Option<ElevenLabsConfig>,
    pub fish_audio: Option<FishAudioConfig>,
    pub ollama: Option<OllamaConfig>,
    pub openai: Option<OpenAIVoiceConfig>,
    pub piper: Option<PiperConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiperConfig {
    pub models_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash)]
pub enum VoiceProviderType {
    ElevenLabs,
    FishAudio,
    Ollama,
    OpenAI,
    Piper,
    System,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElevenLabsConfig {
    pub api_key: String,
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FishAudioConfig {
    pub api_key: String,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIVoiceConfig {
    pub api_key: String,
    pub model: String,      // "tts-1" or "tts-1-hd"
    pub voice: String,      // "alloy", "echo", "fable", "onyx", "nova", "shimmer"
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            provider: VoiceProviderType::Disabled,
            cache_dir: None,
            default_voice_id: None,
            elevenlabs: None,
            fish_audio: None,
            ollama: None,
            openai: None,
            piper: None,
        }
    }
}

// ============================================================================
// Domain Types
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageInfo {
    pub characters_used: u64,
    pub characters_limit: u64,
    pub next_reset: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VoiceStatus {
    Pending,
    Processing,
    Playing,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedVoice {
    pub id: String,
    pub text: String,
    pub voice_id: String,
    pub status: VoiceStatus,
    pub created_at: String,
}
