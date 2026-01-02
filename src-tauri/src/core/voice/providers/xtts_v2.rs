//! XTTS-v2 Provider (Coqui TTS)
//!
//! Multilingual TTS supporting 17 languages with voice cloning.
//! Uses 6-second reference audio for best results.
//! GitHub: https://github.com/coqui-ai/TTS

use async_trait::async_trait;
use reqwest::{multipart, Client};
use serde::{Deserialize, Serialize};

use super::VoiceProvider;
use crate::core::voice::types::{
    Result, SynthesisRequest, UsageInfo, Voice, VoiceError, XttsV2Config,
};

pub struct XttsV2Provider {
    client: Client,
    config: XttsV2Config,
}

#[derive(Serialize)]
struct TtsRequest {
    text: String,
    language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    speaker_wav: Option<String>,
}

#[derive(Deserialize)]
struct SpeakerInfo {
    name: String,
    #[serde(default)]
    language: Option<String>,
}

impl XttsV2Provider {
    pub fn new(config: XttsV2Config) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }
}

#[async_trait]
impl VoiceProvider for XttsV2Provider {
    fn id(&self) -> &'static str {
        "xtts_v2"
    }

    async fn synthesize(&self, request: &SynthesisRequest) -> Result<Vec<u8>> {
        let lang = self.config.language.clone().unwrap_or_else(|| "en".to_string());

        // XTTS server can accept either JSON or multipart depending on setup
        // Try the TTS API endpoint
        let url = format!("{}/api/tts", self.config.base_url);

        // If we have a speaker WAV, use multipart
        if let Some(ref speaker_path) = self.config.speaker_wav {
            let audio_bytes = tokio::fs::read(speaker_path).await?;

            let form = multipart::Form::new()
                .text("text", request.text.clone())
                .text("language", lang)
                .part(
                    "speaker_wav",
                    multipart::Part::bytes(audio_bytes)
                        .file_name("speaker.wav")
                        .mime_str("audio/wav")
                        .map_err(|e| VoiceError::ApiError(e.to_string()))?,
                );

            let response = self.client
                .post(&url)
                .multipart(form)
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                return Err(VoiceError::ApiError(format!(
                    "XTTS-v2 error {}: {}",
                    status, error_text
                )));
            }

            return Ok(response.bytes().await?.to_vec());
        }

        // No speaker WAV - use default voice
        let tts_request = TtsRequest {
            text: request.text.clone(),
            language: lang,
            speaker_wav: None,
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
                "XTTS-v2 error {}: {}",
                status, error_text
            )));
        }

        Ok(response.bytes().await?.to_vec())
    }

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        // Try to get speakers from API
        let url = format!("{}/api/speakers", self.config.base_url);

        if let Ok(response) = self.client.get(&url).send().await {
            if response.status().is_success() {
                if let Ok(speakers) = response.json::<Vec<SpeakerInfo>>().await {
                    return Ok(speakers
                        .into_iter()
                        .map(|s| Voice {
                            id: s.name.clone(),
                            name: s.name,
                            provider: "xtts_v2".to_string(),
                            description: s.language.map(|l| format!("Language: {}", l)),
                            preview_url: None,
                            labels: vec!["built-in".to_string()],
                        })
                        .collect());
                }
            }
        }

        // Default response if API doesn't expose speakers
        Ok(vec![
            Voice {
                id: "default".to_string(),
                name: "Default / Custom".to_string(),
                provider: "xtts_v2".to_string(),
                description: Some("Use speaker_wav for voice cloning".to_string()),
                preview_url: None,
                labels: vec!["voice-clone".to_string(), "17-languages".to_string()],
            },
        ])
    }

    async fn check_usage(&self) -> Result<UsageInfo> {
        Ok(UsageInfo::default())
    }
}
