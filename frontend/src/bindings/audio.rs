use serde::{Deserialize, Serialize};
use super::core::{invoke, invoke_void, invoke_no_args, invoke_void_no_args};

// ============================================================================
// Voice Configuration & Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub provider: String,
    pub cache_dir: Option<String>,
    pub default_voice_id: Option<String>,
    pub elevenlabs: Option<ElevenLabsConfig>,
    pub fish_audio: Option<FishAudioConfig>,
    pub ollama: Option<OllamaConfig>,
    pub openai: Option<OpenAIVoiceConfig>,
    pub piper: Option<PiperConfig>,
    pub coqui: Option<CoquiConfig>,
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
    pub model: String,
    pub voice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiperConfig {
    pub models_dir: Option<String>,
    pub length_scale: f32,
    pub noise_scale: f32,
    pub noise_w: f32,
    pub sentence_silence: f32,
    pub speaker_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoquiConfig {
    pub port: u16,
    pub model: String,
    pub speaker: Option<String>,
    pub language: Option<String>,
    pub speed: f32,
    pub speaker_wav: Option<String>,
    pub temperature: f32,
    pub top_k: u32,
    pub top_p: f32,
    pub repetition_penalty: f32,
}

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
pub struct SpeakResult {
    pub audio_data: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailablePiperVoice {
    pub key: String,
    pub name: String,
    pub language: PiperLanguage,
    pub quality: String,
    pub num_speakers: u32,
    pub sample_rate: u32,
    pub files: PiperVoiceFiles,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiperLanguage {
    pub code: String,
    pub family: String,
    pub region: String,
    pub name_native: String,
    pub name_english: String,
    pub country_english: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiperVoiceFiles {
    pub model: PiperFileInfo,
    pub config: PiperFileInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiperFileInfo {
    pub size_bytes: u64,
    pub md5_digest: String,
}

pub type PopularPiperVoice = (String, String, String);

// ============================================================================
// Voice Commands
// ============================================================================

pub async fn list_openai_voices() -> Result<Vec<Voice>, String> {
    invoke_no_args("list_openai_voices").await
}

pub async fn list_openai_tts_models() -> Result<Vec<(String, String)>, String> {
    invoke_no_args("list_openai_tts_models").await
}

pub async fn list_elevenlabs_voices(api_key: String) -> Result<Vec<Voice>, String> {
    #[derive(Serialize)]
    struct Args {
        api_key: String,
    }
    invoke("list_elevenlabs_voices", &Args { api_key }).await
}

pub async fn list_available_voices() -> Result<Vec<Voice>, String> {
    invoke_no_args("list_available_voices").await
}

pub async fn list_all_voices() -> Result<Vec<Voice>, String> {
    invoke_no_args("list_all_voices").await
}

pub async fn play_tts(text: String, voice_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        text: String,
        voice_id: String,
    }
    invoke_void("play_tts", &Args { text, voice_id }).await
}

pub async fn configure_voice(config: VoiceConfig) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        config: VoiceConfig,
    }
    invoke("configure_voice", &Args { config }).await
}

pub async fn get_voice_config() -> Result<VoiceConfig, String> {
    invoke_no_args("get_voice_config").await
}

pub async fn list_downloadable_piper_voices() -> Result<Vec<AvailablePiperVoice>, String> {
    invoke_no_args("list_downloadable_piper_voices").await
}

pub async fn get_popular_piper_voices() -> Result<Vec<PopularPiperVoice>, String> {
    invoke_no_args("get_popular_piper_voices").await
}

pub async fn download_piper_voice(voice_key: String, quality: Option<String>) -> Result<String, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        voice_key: String,
        quality: Option<String>,
    }
    invoke("download_piper_voice", &Args { voice_key, quality }).await
}

pub async fn speak(text: String) -> Result<Option<SpeakResult>, String> {
    #[derive(Serialize)]
    struct Args {
        text: String,
    }
    invoke("speak", &Args { text }).await
}

// ============================================================================
// Voice Provider Installation
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallStatus {
    pub provider: VoiceProviderType,
    pub installed: bool,
    pub version: Option<String>,
    pub binary_path: Option<String>,
    pub voices_available: u32,
    pub install_method: InstallMethod,
    pub install_instructions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallMethod {
    PackageManager(String),
    Python(String),
    Binary(String),
    Docker(String),
    Manual(String),
    AppManaged,
}

pub async fn check_voice_provider_status(provider: VoiceProviderType) -> Result<InstallStatus, String> {
    #[derive(Serialize)]
    struct Args {
        provider: VoiceProviderType,
    }
    invoke("check_voice_provider_status", &Args { provider }).await
}

pub async fn check_voice_provider_installations() -> Result<Vec<InstallStatus>, String> {
    invoke_no_args("check_voice_provider_installations").await
}

pub async fn install_voice_provider(provider: VoiceProviderType) -> Result<InstallStatus, String> {
    #[derive(Serialize)]
    struct Args {
        provider: VoiceProviderType,
    }
    invoke("install_voice_provider", &Args { provider }).await
}

// ============================================================================
// Voice Queue
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VoiceStatus {
    Queued,
    Playing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueuedVoice {
    pub id: String,
    pub text: String,
    pub voice_id: Option<String>,
    pub status: VoiceStatus,
    pub created_at: String,
}

pub async fn get_voice_queue() -> Result<Vec<QueuedVoice>, String> {
    invoke_no_args("get_voice_queue").await
}

pub async fn queue_voice(text: String, voice_id: Option<String>) -> Result<QueuedVoice, String> {
    #[derive(Serialize)]
    struct Args {
        text: String,
        voice_id: Option<String>,
    }
    invoke("queue_voice", &Args { text, voice_id }).await
}

// ============================================================================
// Voice Profiles
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgeRange {
    Child,
    YoungAdult,
    Adult,
    MiddleAged,
    Elderly,
}

impl Default for AgeRange {
    fn default() -> Self {
        Self::Adult
    }
}

impl AgeRange {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Child => "Child (0-12)",
            Self::YoungAdult => "Young Adult (13-25)",
            Self::Adult => "Adult (26-45)",
            Self::MiddleAged => "Middle-Aged (46-65)",
            Self::Elderly => "Elderly (65+)",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Child,
            Self::YoungAdult,
            Self::Adult,
            Self::MiddleAged,
            Self::Elderly,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
    Neutral,
    NonBinary,
}

impl Default for Gender {
    fn default() -> Self {
        Self::Neutral
    }
}

impl Gender {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Male => "Male",
            Self::Female => "Female",
            Self::Neutral => "Neutral",
            Self::NonBinary => "Non-Binary",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::Male, Self::Female, Self::Neutral, Self::NonBinary]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileMetadata {
    pub age_range: AgeRange,
    pub gender: Gender,
    pub personality_traits: Vec<String>,
    pub linked_npc_ids: Vec<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSettings {
    pub stability: f32,
    pub similarity_boost: f32,
    pub style: f32,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoiceProviderType {
    ElevenLabs,
    FishAudio,
    OpenAI,
    Piper,
    Ollama,
    Chatterbox,
    GptSoVits,
    XttsV2,
    FishSpeech,
    Dia,
    Coqui,
    System,
    Disabled,
}

impl Default for VoiceProviderType {
    fn default() -> Self {
        Self::Disabled
    }
}

impl VoiceProviderType {
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
            Self::Coqui => "Coqui TTS",
            Self::Piper => "Piper (Local)",
            Self::System => "System TTS",
            Self::Disabled => "Disabled",
        }
    }

    pub fn to_string_key(&self) -> &'static str {
        match self {
            Self::ElevenLabs => "elevenlabs",
            Self::FishAudio => "fish_audio",
            Self::OpenAI => "openai",
            Self::Ollama => "ollama",
            Self::Chatterbox => "chatterbox",
            Self::GptSoVits => "gpt_sovits",
            Self::XttsV2 => "xtts_v2",
            Self::FishSpeech => "fish_speech",
            Self::Dia => "dia",
            Self::Coqui => "coqui",
            Self::Piper => "piper",
            Self::System => "system",
            Self::Disabled => "disabled",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::OpenAI,
            Self::ElevenLabs,
            Self::FishAudio,
            Self::Piper,
            Self::Coqui,
            Self::Ollama,
            Self::Chatterbox,
            Self::GptSoVits,
            Self::XttsV2,
            Self::FishSpeech,
            Self::Dia,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProfile {
    pub id: String,
    pub name: String,
    pub provider: VoiceProviderType,
    pub voice_id: String,
    pub settings: VoiceSettings,
    pub metadata: ProfileMetadata,
    pub is_preset: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileStats {
    pub total_user_profiles: usize,
    pub total_presets: usize,
    pub linked_npcs: usize,
    pub profiles_by_provider: std::collections::HashMap<String, usize>,
    pub profiles_by_gender: std::collections::HashMap<String, usize>,
}

pub async fn list_voice_presets() -> Result<Vec<VoiceProfile>, String> {
    invoke_no_args("list_voice_presets").await
}

pub async fn list_voice_presets_by_tag(tag: String) -> Result<Vec<VoiceProfile>, String> {
    #[derive(Serialize)]
    struct Args {
        tag: String,
    }
    invoke("list_voice_presets_by_tag", &Args { tag }).await
}

pub async fn get_voice_preset(preset_id: String) -> Result<Option<VoiceProfile>, String> {
    #[derive(Serialize)]
    struct Args {
        preset_id: String,
    }
    invoke("get_voice_preset", &Args { preset_id }).await
}

pub async fn create_voice_profile(
    name: String,
    provider: String,
    voice_id: String,
    metadata: Option<ProfileMetadata>,
) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        name: String,
        provider: String,
        voice_id: String,
        metadata: Option<ProfileMetadata>,
    }
    invoke("create_voice_profile", &Args { name, provider, voice_id, metadata }).await
}

pub async fn link_voice_profile_to_npc(profile_id: String, npc_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        profile_id: String,
        npc_id: String,
    }
    invoke_void("link_voice_profile_to_npc", &Args { profile_id, npc_id }).await
}

pub async fn get_npc_voice_profile(npc_id: String) -> Result<Option<String>, String> {
    #[derive(Serialize)]
    struct Args {
        npc_id: String,
    }
    invoke("get_npc_voice_profile", &Args { npc_id }).await
}

pub async fn search_voice_profiles(query: String) -> Result<Vec<VoiceProfile>, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
    }
    invoke("search_voice_profiles", &Args { query }).await
}

pub async fn get_voice_profiles_by_gender(gender: String) -> Result<Vec<VoiceProfile>, String> {
    #[derive(Serialize)]
    struct Args {
        gender: String,
    }
    invoke("get_voice_profiles_by_gender", &Args { gender }).await
}

pub async fn get_voice_profiles_by_age(age_range: String) -> Result<Vec<VoiceProfile>, String> {
    #[derive(Serialize)]
    struct Args {
        age_range: String,
    }
    invoke("get_voice_profiles_by_age", &Args { age_range }).await
}

// ============================================================================
// Audio Cache
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub entry_count: usize,
    pub current_size_bytes: u64,
    pub max_size_bytes: u64,
    pub entries_by_format: std::collections::HashMap<String, usize>,
    pub hit_rate: f64,
    pub oldest_entry_age_secs: i64,
    pub avg_entry_size_bytes: u64,
}

pub async fn get_audio_cache_stats() -> Result<VoiceCacheStats, String> {
    invoke_no_args("get_audio_cache_stats").await
}

pub async fn clear_audio_cache_by_tag(tag: String) -> Result<usize, String> {
    #[derive(Serialize)]
    struct Args {
        tag: String,
    }
    invoke("clear_audio_cache_by_tag", &Args { tag }).await
}

pub async fn clear_audio_cache() -> Result<(), String> {
    invoke_void_no_args("clear_audio_cache").await
}

pub async fn prune_audio_cache(max_age_seconds: i64) -> Result<usize, String> {
    #[derive(Serialize)]
    struct Args {
        max_age_seconds: i64,
    }
    invoke("prune_audio_cache", &Args { max_age_seconds }).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioCacheEntry {
    pub key: String,
    pub path: String,
    pub size: u64,
    pub created_at: String,
    pub last_accessed: String,
    pub access_count: u32,
    pub tags: Vec<String>,
    pub profile_id: Option<String>,
    pub duration_ms: Option<u64>,
    pub format: String,
}

pub async fn list_audio_cache_entries() -> Result<Vec<AudioCacheEntry>, String> {
    invoke_no_args("list_audio_cache_entries").await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioCacheSizeInfo {
    pub current_size_bytes: u64,
    pub max_size_bytes: u64,
    pub entry_count: usize,
    pub usage_percent: f64,
}

pub async fn get_audio_cache_size() -> Result<AudioCacheSizeInfo, String> {
    invoke_no_args("get_audio_cache_size").await
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
