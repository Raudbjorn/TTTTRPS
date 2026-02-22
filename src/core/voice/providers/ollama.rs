use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use crate::core::voice::types::{Result, SynthesisRequest, Voice, UsageInfo, VoiceError, OllamaConfig};
use crate::core::voice::providers::VoiceProvider;

pub struct OllamaProvider {
    client: Client,
    config: OllamaConfig,
}

impl OllamaProvider {
    pub fn new(config: OllamaConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }
}

#[async_trait]
impl VoiceProvider for OllamaProvider {
    fn id(&self) -> &'static str {
        "ollama"
    }

    async fn synthesize(&self, request: &SynthesisRequest) -> Result<Vec<u8>> {
        let url = format!("{}/api/tts", self.config.base_url.trim_end_matches('/'));

        let body = json!({
            "model": self.config.model,
            "input": request.text,
            "voice": request.voice_id,
        });

        let response = self.client
            .post(&url)
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                Ok(resp.bytes().await?.to_vec())
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                Err(VoiceError::ApiError(format!("Ollama TTS error: {}", error)))
            }
            Err(e) => {
                Err(VoiceError::NotConfigured(format!(
                    "Ollama TTS not available: {}", e
                )))
            }
        }
    }

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        Ok(vec![
            Voice {
                id: "default".to_string(),
                name: "Default".to_string(),
                provider: "ollama".to_string(),
                description: Some("Default Ollama TTS voice".to_string()),
                preview_url: None,
                labels: vec!["local".to_string()],
            },
        ])
    }

    async fn check_usage(&self) -> Result<UsageInfo> {
        Ok(UsageInfo::default())
    }
}
