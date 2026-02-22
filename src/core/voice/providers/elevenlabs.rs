use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use crate::core::voice::types::{Result, SynthesisRequest, Voice, UsageInfo, VoiceError, ElevenLabsConfig};
use crate::core::voice::providers::VoiceProvider;

pub struct ElevenLabsProvider {
    client: Client,
    config: ElevenLabsConfig,
}

impl ElevenLabsProvider {
    pub fn new(config: ElevenLabsConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }
}

#[async_trait]
impl VoiceProvider for ElevenLabsProvider {
    fn id(&self) -> &'static str {
        "elevenlabs"
    }

    async fn synthesize(&self, request: &SynthesisRequest) -> Result<Vec<u8>> {
        let url = format!(
            "https://api.elevenlabs.io/v1/text-to-speech/{}",
            request.voice_id
        );

        let settings = request.settings.clone().unwrap_or_default();

        let model_id = self.config.model_id.as_deref().unwrap_or("eleven_monolingual_v1");

        let body = json!({
            "text": request.text,
            "model_id": model_id,
            "voice_settings": {
                "stability": settings.stability,
                "similarity_boost": settings.similarity_boost,
                "style": settings.style,
                "use_speaker_boost": settings.use_speaker_boost
            }
        });

        let response = self.client
            .post(&url)
            .header("xi-api-key", &self.config.api_key)
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

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        let response = self.client
            .get("https://api.elevenlabs.io/v1/voices")
            .header("xi-api-key", &self.config.api_key)
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

    async fn check_usage(&self) -> Result<UsageInfo> {
        let response = self.client
            .get("https://api.elevenlabs.io/v1/user/subscription")
            .header("xi-api-key", &self.config.api_key)
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
}
