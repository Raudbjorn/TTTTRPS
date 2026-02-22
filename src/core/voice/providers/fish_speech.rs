//! Fish Speech Provider (Self-hosted)
//!
//! Open-source TTS with voice cloning capabilities.
//! Different from Fish Audio cloud service.
//! GitHub: https://github.com/fishaudio/fish-speech

use async_trait::async_trait;
use reqwest::{multipart, Client};
use serde::Serialize;

use super::VoiceProvider;
use crate::core::voice::types::{
    FishSpeechConfig, Result, SynthesisRequest, UsageInfo, Voice, VoiceError,
};

pub struct FishSpeechProvider {
    client: Client,
    config: FishSpeechConfig,
}

#[derive(Serialize)]
struct TtsRequest {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reference_audio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reference_text: Option<String>,
}

impl FishSpeechProvider {
    pub fn new(config: FishSpeechConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }
}

#[async_trait]
impl VoiceProvider for FishSpeechProvider {
    fn id(&self) -> &'static str {
        "fish_speech"
    }

    async fn synthesize(&self, request: &SynthesisRequest) -> Result<Vec<u8>> {
        let url = format!("{}/v1/tts", self.config.base_url);

        // If we have reference audio, use multipart
        if let Some(ref audio_path) = self.config.reference_audio {
            let audio_bytes = tokio::fs::read(audio_path).await?;

            let mut form = multipart::Form::new()
                .text("text", request.text.clone())
                .part(
                    "reference_audio",
                    multipart::Part::bytes(audio_bytes)
                        .file_name("reference.wav")
                        .mime_str("audio/wav")
                        .map_err(|e| VoiceError::ApiError(e.to_string()))?,
                );

            if let Some(ref ref_text) = self.config.reference_text {
                form = form.text("reference_text", ref_text.clone());
            }

            let response = self.client
                .post(&url)
                .multipart(form)
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                return Err(VoiceError::ApiError(format!(
                    "Fish Speech error {}: {}",
                    status, error_text
                )));
            }

            return Ok(response.bytes().await?.to_vec());
        }

        // Simple JSON request without reference
        let tts_request = TtsRequest {
            text: request.text.clone(),
            reference_audio: None,
            reference_text: None,
        };

        let response = self.client
            .post(&url)
            .json(&tts_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VoiceError::ApiError(format!(
                "Fish Speech error {}: {}",
                status, error_text
            )));
        }

        Ok(response.bytes().await?.to_vec())
    }

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        Ok(vec![
            Voice {
                id: "default".to_string(),
                name: "Default Voice".to_string(),
                provider: "fish_speech".to_string(),
                description: Some("Use reference audio for voice cloning".to_string()),
                preview_url: None,
                labels: vec!["voice-clone".to_string()],
            },
        ])
    }

    async fn check_usage(&self) -> Result<UsageInfo> {
        Ok(UsageInfo::default())
    }
}
