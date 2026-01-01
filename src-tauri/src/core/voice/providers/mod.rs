use async_trait::async_trait;
use super::types::{Result, SynthesisRequest, Voice, UsageInfo};

pub mod elevenlabs;
pub mod fish_audio;
pub mod ollama;
pub mod openai;
pub mod piper;

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
