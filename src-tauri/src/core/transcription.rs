use serde::{Deserialize, Serialize};
use reqwest::multipart;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub language: Option<String>,
    pub duration_seconds: Option<f64>,
}

pub struct TranscriptionService {
    client: reqwest::Client,
}

impl Default for TranscriptionService {
    fn default() -> Self {
        Self::new()
    }
}

impl TranscriptionService {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn transcribe_openai(
        &self,
        api_key: &str,
        audio_path: &Path,
    ) -> Result<TranscriptionResult, String> {
        let file_name = audio_path.file_name()
            .ok_or("Invalid path")?
            .to_string_lossy()
            .to_string();

        let file_content = tokio::fs::read(audio_path).await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let part = multipart::Part::bytes(file_content)
            .file_name(file_name);

        let form = multipart::Form::new()
            .part("file", part)
            .text("model", "whisper-1");

        let response = self.client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("OpenAI API error: {}", response.status()));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| format!("Invalid JSON: {}", e))?;

        let text = json["text"].as_str().unwrap_or("").to_string();

        Ok(TranscriptionResult {
            text,
            language: None,
            duration_seconds: None,
        })
    }
}
