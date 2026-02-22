//! GPT-SoVITS Provider
//!
//! Zero-shot voice cloning TTS with only 5 seconds of reference audio.
//! Supports Chinese, English, Japanese, Korean, Cantonese.
//! GitHub: https://github.com/RVC-Boss/GPT-SoVITS

use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;

use super::VoiceProvider;
use crate::core::voice::types::{
    GptSoVitsConfig, Result, SynthesisRequest, UsageInfo, Voice, VoiceError,
};

pub struct GptSoVitsProvider {
    client: Client,
    config: GptSoVitsConfig,
}

#[derive(Serialize)]
struct TtsRequest {
    text: String,
    text_lang: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ref_audio_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_lang: Option<String>,
}

impl GptSoVitsProvider {
    pub fn new(config: GptSoVitsConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }
}

#[async_trait]
impl VoiceProvider for GptSoVitsProvider {
    fn id(&self) -> &'static str {
        "gpt_sovits"
    }

    async fn synthesize(&self, request: &SynthesisRequest) -> Result<Vec<u8>> {
        let url = format!("{}/tts", self.config.base_url);

        let lang = self.config.language.clone().unwrap_or_else(|| "en".to_string());

        let tts_request = TtsRequest {
            text: request.text.clone(),
            text_lang: lang.clone(),
            ref_audio_path: self.config.reference_audio.clone(),
            prompt_text: self.config.reference_text.clone(),
            prompt_lang: Some(lang),
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
                "GPT-SoVITS error {}: {}",
                status, error_text
            )));
        }

        Ok(response.bytes().await?.to_vec())
    }

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        // GPT-SoVITS uses reference audio for voice cloning
        // Could potentially list speakers if API supports it
        Ok(vec![
            Voice {
                id: "default".to_string(),
                name: "Default Voice".to_string(),
                provider: "gpt_sovits".to_string(),
                description: Some("Configure reference audio for custom voice".to_string()),
                preview_url: None,
                labels: vec!["voice-clone".to_string(), "multilingual".to_string()],
            },
        ])
    }

    async fn check_usage(&self) -> Result<UsageInfo> {
        Ok(UsageInfo::default())
    }
}
