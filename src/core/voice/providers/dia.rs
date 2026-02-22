//! Dia Provider (Nari Labs)
//!
//! Dialogue-focused TTS, excellent for podcasts and multi-speaker content.
//! Apache 2.0 licensed.
//! GitHub: https://github.com/nari-labs/dia

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::VoiceProvider;
use crate::core::voice::types::{
    DiaConfig, Result, SynthesisRequest, UsageInfo, Voice, VoiceError,
};

pub struct DiaProvider {
    client: Client,
    config: DiaConfig,
}

#[derive(Serialize)]
struct TtsRequest {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    voice_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dialogue_mode: Option<bool>,
}

#[derive(Deserialize)]
struct VoiceInfo {
    id: String,
    name: String,
    #[serde(default)]
    description: Option<String>,
}

impl DiaProvider {
    pub fn new(config: DiaConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }
}

#[async_trait]
impl VoiceProvider for DiaProvider {
    fn id(&self) -> &'static str {
        "dia"
    }

    async fn synthesize(&self, request: &SynthesisRequest) -> Result<Vec<u8>> {
        let url = format!("{}/api/tts", self.config.base_url);

        let tts_request = TtsRequest {
            text: request.text.clone(),
            voice_id: self.config.voice_id.clone().or_else(|| {
                if request.voice_id != "default" {
                    Some(request.voice_id.clone())
                } else {
                    None
                }
            }),
            dialogue_mode: self.config.dialogue_mode,
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
                "Dia error {}: {}",
                status, error_text
            )));
        }

        Ok(response.bytes().await?.to_vec())
    }

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        // Try to get voices from API
        let url = format!("{}/api/voices", self.config.base_url);

        if let Ok(response) = self.client.get(&url).send().await {
            if response.status().is_success() {
                if let Ok(voices) = response.json::<Vec<VoiceInfo>>().await {
                    return Ok(voices
                        .into_iter()
                        .map(|v| Voice {
                            id: v.id,
                            name: v.name,
                            provider: "dia".to_string(),
                            description: v.description,
                            preview_url: None,
                            labels: vec!["dialogue".to_string()],
                        })
                        .collect());
                }
            }
        }

        // Default voices for Dia
        Ok(vec![
            Voice {
                id: "default".to_string(),
                name: "Default Voice".to_string(),
                provider: "dia".to_string(),
                description: Some("Standard Dia voice".to_string()),
                preview_url: None,
                labels: vec!["dialogue".to_string()],
            },
            Voice {
                id: "podcast".to_string(),
                name: "Podcast Host".to_string(),
                provider: "dia".to_string(),
                description: Some("Optimized for podcast-style content".to_string()),
                preview_url: None,
                labels: vec!["dialogue".to_string(), "podcast".to_string()],
            },
        ])
    }

    async fn check_usage(&self) -> Result<UsageInfo> {
        Ok(UsageInfo::default())
    }
}
