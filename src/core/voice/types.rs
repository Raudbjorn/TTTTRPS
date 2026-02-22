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
    // Cloud providers
    pub elevenlabs: Option<ElevenLabsConfig>,
    pub fish_audio: Option<FishAudioConfig>,
    pub openai: Option<OpenAIVoiceConfig>,
    pub piper: Option<PiperConfig>,
    // Self-hosted providers
    pub ollama: Option<OllamaConfig>,
    pub chatterbox: Option<ChatterboxConfig>,
    pub gpt_sovits: Option<GptSoVitsConfig>,
    pub xtts_v2: Option<XttsV2Config>,
    pub fish_speech: Option<FishSpeechConfig>,
    pub dia: Option<DiaConfig>,
    pub coqui: Option<CoquiConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiperConfig {
    pub models_dir: Option<PathBuf>,
    /// Speech rate: 1.0 = normal, >1.0 = slower, <1.0 = faster
    #[serde(default = "default_length_scale")]
    pub length_scale: f32,
    /// Phoneme variability (0.0 - 1.0)
    #[serde(default = "default_noise_scale")]
    pub noise_scale: f32,
    /// Phoneme width variability (0.0 - 1.0)
    #[serde(default = "default_noise_w")]
    pub noise_w: f32,
    /// Pause between sentences in seconds (0.0 - 2.0)
    #[serde(default = "default_sentence_silence")]
    pub sentence_silence: f32,
    /// Speaker ID for multi-speaker models (0-N)
    #[serde(default)]
    pub speaker_id: u32,
}

fn default_length_scale() -> f32 { 1.0 }
fn default_noise_scale() -> f32 { 0.667 }
fn default_noise_w() -> f32 { 0.8 }
fn default_sentence_silence() -> f32 { 0.2 }

impl Default for PiperConfig {
    fn default() -> Self {
        Self {
            models_dir: None,
            length_scale: default_length_scale(),
            noise_scale: default_noise_scale(),
            noise_w: default_noise_w(),
            sentence_silence: default_sentence_silence(),
            speaker_id: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoquiConfig {
    pub port: u16,
    /// Model name (e.g., "tts_models/en/ljspeech/tacotron2-DDC")
    #[serde(default = "default_coqui_model")]
    pub model: String,
    /// Speaker name for multi-speaker models
    #[serde(default)]
    pub speaker: Option<String>,
    /// Language code for multilingual models
    #[serde(default)]
    pub language: Option<String>,
    /// Speech speed factor (0.5 - 2.0)
    #[serde(default = "default_speed")]
    pub speed: f32,
    /// Path to reference audio for voice cloning (XTTS)
    #[serde(default)]
    pub speaker_wav: Option<String>,
    /// Generation temperature for XTTS (0.0 - 1.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Top-k sampling for XTTS (1 - 100)
    #[serde(default = "default_top_k")]
    pub top_k: u32,
    /// Top-p/nucleus sampling for XTTS (0.0 - 1.0)
    #[serde(default = "default_top_p")]
    pub top_p: f32,
    /// Repetition penalty for XTTS (1.0 - 3.0)
    #[serde(default = "default_repetition_penalty")]
    pub repetition_penalty: f32,
}

fn default_coqui_model() -> String { "tts_models/en/ljspeech/tacotron2-DDC".to_string() }
fn default_speed() -> f32 { 1.0 }
fn default_temperature() -> f32 { 0.65 }
fn default_top_k() -> u32 { 50 }
fn default_top_p() -> f32 { 0.85 }
fn default_repetition_penalty() -> f32 { 2.0 }

impl Default for CoquiConfig {
    fn default() -> Self {
        Self {
            port: 5002,
            model: default_coqui_model(),
            speaker: None,
            language: None,
            speed: default_speed(),
            speaker_wav: None,
            temperature: default_temperature(),
            top_k: default_top_k(),
            top_p: default_top_p(),
            repetition_penalty: default_repetition_penalty(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum VoiceProviderType {
    // Cloud providers
    ElevenLabs,
    FishAudio,
    OpenAI,
    Piper,
    // Self-hosted providers
    Ollama,
    Chatterbox,
    GptSoVits,
    XttsV2,
    FishSpeech,
    Dia,
    Coqui,
    // System/disabled
    System,
    Disabled,
}

impl VoiceProviderType {
    /// Returns the default endpoint for self-hosted providers.
    /// Note: Each provider uses a unique port to avoid conflicts.
    pub fn default_endpoint(&self) -> Option<&'static str> {
        match self {
            Self::Ollama => Some("http://localhost:11434"),
            Self::Chatterbox => Some("http://localhost:8000"),
            Self::GptSoVits => Some("http://localhost:9880"),
            Self::FishSpeech => Some("http://localhost:7860"), // Fish Speech default
            Self::Dia => Some("http://localhost:8003"),
            Self::Coqui => Some("http://localhost:5002"),
            _ => None,
        }
    }

    /// Returns true if provider runs locally
    pub fn is_local(&self) -> bool {
        matches!(
            self,
            Self::Ollama | Self::Chatterbox | Self::GptSoVits | Self::XttsV2 | Self::FishSpeech | Self::Dia | Self::Piper | Self::Coqui
        )
    }

    /// Human-readable display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ElevenLabs => "ElevenLabs",
            Self::FishAudio => "Fish Audio (Cloud)",
            Self::OpenAI => "OpenAI TTS",
            Self::Ollama => "Ollama",
            Self::Chatterbox => "Chatterbox",
            Self::GptSoVits => "GPT-SoVITS",
            Self::XttsV2 => "XTTS-v2 (Coqui)",
            Self::FishSpeech => "Fish Speech",
            Self::Dia => "Dia",
            Self::Coqui => "Coqui TTS Server",
            Self::Piper => "Piper (Local)",
            Self::System => "System TTS",
            Self::Disabled => "Disabled",
        }
    }
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

// ============================================================================
// Self-Hosted Provider Configurations
// ============================================================================

/// Chatterbox TTS - Resemble AI's open-source model
/// GitHub: https://github.com/resemble-ai/chatterbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatterboxConfig {
    pub base_url: String,
    /// Reference audio for voice cloning (5 seconds minimum)
    pub reference_audio: Option<String>,
    /// Exaggeration factor for voice characteristics (0.0-1.0)
    pub exaggeration: Option<f32>,
    /// CFG/pace control
    pub cfg_weight: Option<f32>,
}

impl Default for ChatterboxConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8000".to_string(),
            reference_audio: None,
            exaggeration: Some(0.5),
            cfg_weight: Some(0.5),
        }
    }
}

/// GPT-SoVITS - Zero-shot voice cloning TTS
/// GitHub: https://github.com/RVC-Boss/GPT-SoVITS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GptSoVitsConfig {
    pub base_url: String,
    /// Reference audio path for voice cloning
    pub reference_audio: Option<String>,
    /// Reference text (transcript of reference audio)
    pub reference_text: Option<String>,
    /// Target language: zh/en/ja/ko/yue/auto
    pub language: Option<String>,
    /// Speaker ID for multi-speaker models
    pub speaker_id: Option<String>,
}

impl Default for GptSoVitsConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:9880".to_string(),
            reference_audio: None,
            reference_text: None,
            language: Some("en".to_string()),
            speaker_id: None,
        }
    }
}

/// XTTS-v2 - Coqui TTS multilingual model
/// GitHub: https://github.com/coqui-ai/TTS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XttsV2Config {
    pub base_url: String,
    /// Speaker WAV file for voice cloning (6 seconds ideal)
    pub speaker_wav: Option<String>,
    /// Target language code (17 languages supported)
    pub language: Option<String>,
}

impl Default for XttsV2Config {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:5002".to_string(), // Coqui TTS default port
            speaker_wav: None,
            language: Some("en".to_string()),
        }
    }
}

/// Fish Speech S1 - Self-hosted variant
/// GitHub: https://github.com/fishaudio/fish-speech
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FishSpeechConfig {
    pub base_url: String,
    /// Reference audio for voice cloning
    pub reference_audio: Option<String>,
    /// Reference text (transcript)
    pub reference_text: Option<String>,
}

impl Default for FishSpeechConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:7860".to_string(), // Fish Speech default port
            reference_audio: None,
            reference_text: None,
        }
    }
}

/// Dia - Nari Labs dialogue TTS, great for podcasts
/// GitHub: https://github.com/nari-labs/dia
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaConfig {
    pub base_url: String,
    /// Voice preset or cloned voice ID
    pub voice_id: Option<String>,
    /// Enable dialogue mode for multi-speaker content
    pub dialogue_mode: Option<bool>,
}

impl Default for DiaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8003".to_string(),
            voice_id: None,
            dialogue_mode: Some(false),
        }
    }
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            provider: VoiceProviderType::Disabled,
            cache_dir: None,
            default_voice_id: None,
            elevenlabs: None,
            fish_audio: None,
            openai: None,
            piper: None,
            ollama: None,
            chatterbox: None,
            gpt_sovits: None,
            xtts_v2: None,
            fish_speech: None,
            dia: None,
            coqui: None,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
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

// ============================================================================
// Provider Detection Types
// ============================================================================

/// Status of a single voice provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub provider: VoiceProviderType,
    pub available: bool,
    pub endpoint: Option<String>,
    pub version: Option<String>,
    pub error: Option<String>,
}

/// All detected voice providers
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceProviderDetection {
    pub providers: Vec<ProviderStatus>,
    pub detected_at: Option<String>,
}

