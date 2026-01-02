//! Chatterbox TTS Provider
//!
//! Resemble AI's open-source TTS model - currently #1 on HuggingFace.
//! Requires 5-second audio sample for voice cloning.
//! GitHub: https://github.com/resemble-ai/chatterbox

use async_trait::async_trait;
use reqwest::{multipart, Client};
use serde::Deserialize;

use super::VoiceProvider;
use crate::core::voice::types::{
    ChatterboxConfig, OutputFormat, Result, SynthesisRequest, UsageInfo, Voice, VoiceError,
};

pub struct ChatterboxProvider {
    client: Client,
    config: ChatterboxConfig,
}

#[derive(Deserialize)]
struct ChatterboxResponse {
    audio: Option<String>, // base64 encoded audio
    error: Option<String>,
}

impl ChatterboxProvider {
    pub fn new(config: ChatterboxConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }
}

#[async_trait]
impl VoiceProvider for ChatterboxProvider {
    fn id(&self) -> &'static str {
        "chatterbox"
    }

    async fn synthesize(&self, request: &SynthesisRequest) -> Result<Vec<u8>> {
        let url = format!("{}/generate", self.config.base_url);

        // Build multipart form
        let mut form = multipart::Form::new()
            .text("text", request.text.clone());

        // Add reference audio if provided
        if let Some(ref audio_path) = self.config.reference_audio {
            let audio_bytes = tokio::fs::read(audio_path).await?;

            // Detect mime type and filename from path
            let filename = std::path::Path::new(audio_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("reference.wav")
                .to_string();
            let mime_type = match std::path::Path::new(audio_path)
                .extension()
                .and_then(|e| e.to_str())
            {
                Some("mp3") => "audio/mpeg",
                Some("ogg") => "audio/ogg",
                Some("flac") => "audio/flac",
                Some("m4a") => "audio/mp4",
                _ => "audio/wav",
            };

            let part = multipart::Part::bytes(audio_bytes)
                .file_name(filename)
                .mime_str(mime_type)
                .map_err(|e| VoiceError::ApiError(e.to_string()))?;
            form = form.part("audio", part);
        }

        // Add optional parameters
        if let Some(exag) = self.config.exaggeration {
            form = form.text("exaggeration", exag.to_string());
        }
        if let Some(cfg) = self.config.cfg_weight {
            form = form.text("cfg_weight", cfg.to_string());
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
                "Chatterbox error {}: {}",
                status, error_text
            )));
        }

        // Chatterbox returns audio bytes directly or as base64 depending on setup
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.contains("audio") {
            // Direct audio response
            Ok(response.bytes().await?.to_vec())
        } else {
            // JSON with base64 audio
            let data: ChatterboxResponse = response.json().await
                .map_err(|e| VoiceError::ApiError(format!("Failed to parse response: {}", e)))?;

            if let Some(error) = data.error {
                return Err(VoiceError::ApiError(error));
            }

            data.audio
                .ok_or_else(|| VoiceError::ApiError("No audio in response".to_string()))
                .and_then(|b64| {
                    use base64::{Engine, engine::general_purpose::STANDARD};
                    STANDARD.decode(&b64)
                        .map_err(|e| VoiceError::ApiError(format!("Base64 decode error: {}", e)))
                })
        }
    }

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        // Chatterbox uses reference audio for voice cloning, no predefined voices
        Ok(vec![
            Voice {
                id: "default".to_string(),
                name: "Default Voice".to_string(),
                provider: "chatterbox".to_string(),
                description: Some("Use reference audio for custom voice".to_string()),
                preview_url: None,
                labels: vec!["voice-clone".to_string()],
            },
        ])
    }

    async fn check_usage(&self) -> Result<UsageInfo> {
        // Local provider - no usage limits
        Ok(UsageInfo::default())
    }
}
