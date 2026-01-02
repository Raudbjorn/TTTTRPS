use async_trait::async_trait;
use std::sync::Mutex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tracing::{debug, info, warn};

use super::super::types::{Result, SynthesisRequest, Voice, UsageInfo, CoquiConfig, VoiceError};
use super::VoiceProvider;

pub struct CoquiProvider {
    client: Client,
    port: u16,
    server_process: Mutex<Option<Child>>,
    is_available: AtomicBool,
}

#[derive(Debug, Serialize)]
struct TtsRequest {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    speaker_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ServerInfo {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    speakers: Vec<String>,
    #[serde(default)]
    languages: Vec<String>,
}

impl CoquiProvider {
    pub fn new(config: CoquiConfig) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("Failed to create HTTP client"),
            port: config.port,
            server_process: Mutex::new(None),
            is_available: AtomicBool::new(false),
        }
    }

    fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub async fn check_available(&self) -> bool {
        let result = self
            .client
            .get(format!("{}/api/tts", self.base_url()))
            .send()
            .await;

        let available = result.is_ok();
        self.is_available.store(available, Ordering::SeqCst);
        available
    }

    pub fn is_available(&self) -> bool {
        self.is_available.load(Ordering::SeqCst)
    }

    pub async fn start_server(&self, model: Option<&str>) -> Result<()> {
        if self.check_available().await {
            info!("Coqui TTS server already running");
            return Ok(());
        }

        info!(port = self.port, "Starting Coqui TTS server");

        let mut cmd = Command::new("tts-server");
        cmd.arg("--port")
            .arg(self.port.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if let Some(m) = model {
            cmd.arg("--model_name").arg(m);
        }

        let child = cmd.spawn().map_err(|e| {
            VoiceError::NotConfigured(format!(
                "Failed to start Coqui TTS server. Is coqui-tts installed? Error: {}",
                e
            ))
        })?;

        {
            let mut guard = self.server_process.lock().unwrap();
            *guard = Some(child);
        }

        // Wait for server to become available
        for i in 0..30 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if self.check_available().await {
                info!(attempts = i + 1, "Coqui TTS server is ready");
                return Ok(());
            }
        }

        Err(VoiceError::ApiError(
            "Coqui TTS server failed to start within 15 seconds".to_string(),
        ))
    }

    pub fn stop_server(&self) {
        let mut guard = self.server_process.lock().unwrap();
        if let Some(mut child) = guard.take() {
            info!("Stopping Coqui TTS server");
            let _ = child.kill();
            let _ = child.wait();
        }
        self.is_available.store(false, Ordering::SeqCst);
    }

    async fn fetch_server_info(&self) -> Result<ServerInfo> {
        let response = self
            .client
            .get(format!("{}/api/tts", self.base_url()))
            .send()
            .await
            .map_err(|e| VoiceError::NetworkError(e))?;

        if response.status().is_success() {
            Ok(ServerInfo {
                model: None,
                speakers: vec![],
                languages: vec!["en".to_string()],
            })
        } else {
            Err(VoiceError::ApiError("Failed to fetch server info".to_string()))
        }
    }

    fn default_voice_list(&self) -> Vec<Voice> {
        vec![
            Voice {
                id: "tts_models/en/ljspeech/tacotron2-DDC".to_string(),
                name: "LJSpeech Tacotron2".to_string(),
                provider: "coqui".to_string(),
                description: Some("High quality English TTS".to_string()),
                preview_url: None,
                labels: vec!["en".to_string(), "tacotron2".to_string()],
            },
            Voice {
                id: "tts_models/en/ljspeech/vits".to_string(),
                name: "LJSpeech VITS".to_string(),
                provider: "coqui".to_string(),
                description: Some("Fast VITS model".to_string()),
                preview_url: None,
                labels: vec!["en".to_string(), "vits".to_string()],
            },
            Voice {
                id: "tts_models/multilingual/multi-dataset/xtts_v2".to_string(),
                name: "XTTS v2 (Multilingual)".to_string(),
                provider: "coqui".to_string(),
                description: Some("High quality multilingual TTS with voice cloning".to_string()),
                preview_url: None,
                labels: vec!["multilingual".to_string(), "voice-clone".to_string()],
            },
            Voice {
                id: "tts_models/de/thorsten/vits".to_string(),
                name: "Thorsten VITS".to_string(),
                provider: "coqui".to_string(),
                description: Some("German VITS model".to_string()),
                preview_url: None,
                labels: vec!["de".to_string(), "vits".to_string()],
            },
        ]
    }
}

impl Drop for CoquiProvider {
    fn drop(&mut self) {
        self.stop_server();
    }
}

#[async_trait]
impl VoiceProvider for CoquiProvider {
    fn id(&self) -> &'static str {
        "coqui"
    }

    async fn synthesize(&self, request: &SynthesisRequest) -> Result<Vec<u8>> {
        if !self.is_available() {
            self.start_server(Some(&request.voice_id)).await?;
        }

        debug!(text_len = request.text.len(), voice = %request.voice_id, "Synthesizing with Coqui");

        let url = format!("{}/api/tts", self.base_url());
        let params = vec![("text", request.text.clone())];

        let response = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| VoiceError::NetworkError(e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(VoiceError::ApiError(format!(
                "Server returned {}: {}",
                status, body
            )));
        }

        let audio_data = response
            .bytes()
            .await
            .map_err(|e| VoiceError::NetworkError(e))?;

        Ok(audio_data.to_vec())
    }

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        if !self.is_available() {
            return Ok(self.default_voice_list());
        }

        match self.fetch_server_info().await {
            Ok(info) => {
                let voice = Voice {
                    id: info.model.clone().unwrap_or_else(|| "default".to_string()),
                    name: info
                        .model
                        .clone()
                        .unwrap_or_else(|| "Current Model".to_string()),
                    provider: "coqui".to_string(),
                    description: None,
                    preview_url: None,
                    labels: info.languages.clone(),
                };
                Ok(vec![voice])
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch Coqui server info");
                Ok(self.default_voice_list())
            }
        }
    }

    async fn check_usage(&self) -> Result<UsageInfo> {
        Ok(UsageInfo::default())
    }
}
