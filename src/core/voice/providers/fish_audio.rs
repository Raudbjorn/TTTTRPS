use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use crate::core::voice::types::{Result, SynthesisRequest, Voice, UsageInfo, VoiceError, FishAudioConfig};
use crate::core::voice::providers::VoiceProvider;

pub struct FishAudioProvider {
    client: Client,
    config: FishAudioConfig,
}

impl FishAudioProvider {
    pub fn new(config: FishAudioConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }
}

#[async_trait]
impl VoiceProvider for FishAudioProvider {
    fn id(&self) -> &'static str {
        "fish_audio"
    }

    async fn synthesize(&self, request: &SynthesisRequest) -> Result<Vec<u8>> {
        let base_url = self.config.base_url.as_deref().unwrap_or("https://api.fish.audio/v1");
        let url = format!("{}/tts", base_url);

        // Fish Audio API structure (hypothetical/standardized based on common TTS APIs)
        // Adjust payload based on actual Fish Audio API docs if available
        let body = json!({
            "text": request.text,
            "reference_id": request.voice_id, // Fish Audio uses reference IDs or voice IDs
            "format": request.output_format.extension(),
            "normalize": true
        });

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(VoiceError::ApiError(format!(
                "Fish Audio API error: {}", error_text
            )));
        }

        Ok(response.bytes().await?.to_vec())
    }

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        // Placeholder implementation for listing voices
        // Depending on API capability
        Ok(vec![])
    }

    async fn check_usage(&self) -> Result<UsageInfo> {
        // Fish Audio might not expose usage endpoint same way
        Ok(UsageInfo::default())
    }
}
