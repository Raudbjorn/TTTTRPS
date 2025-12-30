use async_trait::async_trait;
use super::types::{Result, SynthesisRequest, Voice, UsageInfo};

// Cloud providers
pub mod elevenlabs;
pub mod fish_audio;
pub mod ollama;
pub mod openai;

// Self-hosted providers
pub mod chatterbox;
pub mod gpt_sovits;
pub mod xtts_v2;
pub mod fish_speech;
pub mod dia;

// Re-exports
pub use chatterbox::ChatterboxProvider;
pub use gpt_sovits::GptSoVitsProvider;
pub use xtts_v2::XttsV2Provider;
pub use fish_speech::FishSpeechProvider;
pub use dia::DiaProvider;

#[async_trait]
pub trait VoiceProvider: Send + Sync {
    /// Unique identifier for the provider (e.g., "elevenlabs")
    fn id(&self) -> &'static str;

    /// Synthesize speech from text
    async fn synthesize(&self, request: &SynthesisRequest) -> Result<Vec<u8>>;

    /// List available voices from this provider
    async fn list_voices(&self) -> Result<Vec<Voice>>;

    /// Check usage quotas
    async fn check_usage(&self) -> Result<UsageInfo>;
}
