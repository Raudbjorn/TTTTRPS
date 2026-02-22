use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use crate::core::voice::types::{Result, SynthesisRequest, Voice, UsageInfo, VoiceError, OpenAIVoiceConfig};
use crate::core::voice::providers::VoiceProvider;

pub struct OpenAIVoiceProvider {
    client: Client,
    config: OpenAIVoiceConfig,
}

impl OpenAIVoiceProvider {
    pub fn new(config: OpenAIVoiceConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }
}

/// OpenAI TTS voice info
#[derive(Clone)]
pub struct OpenAIVoice {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

/// Available OpenAI TTS voices
pub const OPENAI_VOICES: &[OpenAIVoice] = &[
    OpenAIVoice { id: "alloy", name: "Alloy", description: "Neutral and balanced" },
    OpenAIVoice { id: "echo", name: "Echo", description: "Warm and clear" },
    OpenAIVoice { id: "fable", name: "Fable", description: "British accent, expressive" },
    OpenAIVoice { id: "onyx", name: "Onyx", description: "Deep and authoritative" },
    OpenAIVoice { id: "nova", name: "Nova", description: "Friendly and upbeat" },
    OpenAIVoice { id: "shimmer", name: "Shimmer", description: "Warm and pleasant" },
];

/// Available OpenAI TTS models
pub const OPENAI_TTS_MODELS: &[(&str, &str)] = &[
    ("tts-1", "TTS-1 (Fast)"),
    ("tts-1-hd", "TTS-1 HD (High Quality)"),
];

#[async_trait]
impl VoiceProvider for OpenAIVoiceProvider {
    fn id(&self) -> &'static str {
        "openai"
    }

    async fn synthesize(&self, request: &SynthesisRequest) -> Result<Vec<u8>> {
        let url = "https://api.openai.com/v1/audio/speech";

        // Use voice_id from request, or fall back to config
        let voice = if request.voice_id == "default" {
            &self.config.voice
        } else {
            &request.voice_id
        };

        let response_format = match request.output_format {
            crate::core::voice::types::OutputFormat::Mp3 => "mp3",
            crate::core::voice::types::OutputFormat::Wav => "wav",
            crate::core::voice::types::OutputFormat::Ogg => "opus",
            crate::core::voice::types::OutputFormat::Pcm => "pcm",
        };

        let body = json!({
            "model": self.config.model,
            "input": request.text,
            "voice": voice,
            "response_format": response_format
        });

        let response = self.client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
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
                "OpenAI TTS API error: {}", error_text
            )));
        }

        Ok(response.bytes().await?.to_vec())
    }

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        // OpenAI voices are fixed, return static list
        Ok(OPENAI_VOICES.iter().map(|v| Voice {
            id: v.id.to_string(),
            name: v.name.to_string(),
            provider: "openai".to_string(),
            description: Some(v.description.to_string()),
            preview_url: None,
            labels: vec![],
        }).collect())
    }

    async fn check_usage(&self) -> Result<UsageInfo> {
        // OpenAI doesn't have a direct TTS usage endpoint
        // Would need to check billing API which requires different auth
        Ok(UsageInfo::default())
    }
}

/// Get list of OpenAI TTS voices (no API call needed)
pub fn get_openai_voices() -> Vec<Voice> {
    OPENAI_VOICES.iter().map(|v| Voice {
        id: v.id.to_string(),
        name: v.name.to_string(),
        provider: "openai".to_string(),
        description: Some(v.description.to_string()),
        preview_url: None,
        labels: vec![],
    }).collect()
}

/// Get list of OpenAI TTS models
pub fn get_openai_tts_models() -> Vec<(String, String)> {
    OPENAI_TTS_MODELS.iter()
        .map(|(id, name)| (id.to_string(), name.to_string()))
        .collect()
}
