//! Transcription Module
//!
//! Multi-provider transcription abstraction supporting audio-to-text conversion
//! via various backends (OpenAI Whisper, future providers).

use async_trait::async_trait;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

/// Default timeout for transcription API requests (2 minutes for large audio files)
const DEFAULT_TRANSCRIPTION_TIMEOUT: Duration = Duration::from_secs(120);

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum TranscriptionError {
    #[error("Provider not available: {0}")]
    ProviderNotAvailable(String),

    #[error("Invalid audio file: {0}")]
    InvalidAudioFile(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, TranscriptionError>;

// ============================================================================
// Types
// ============================================================================

/// Result from transcription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub language: Option<String>,
    pub duration_seconds: Option<f64>,
    pub provider: String,
}

/// Configuration for a transcription provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionConfig {
    pub provider: TranscriptionProviderType,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub language_hint: Option<String>,
}

/// Supported transcription providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptionProviderType {
    #[default]
    OpenAI,
    Groq,
    // Future: Local (Whisper.cpp), AssemblyAI, Deepgram, etc.
}

impl std::str::FromStr for TranscriptionProviderType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" | "whisper" => Ok(Self::OpenAI),
            "groq" => Ok(Self::Groq),
            _ => Err(format!("Unknown transcription provider: {}", s)),
        }
    }
}

impl std::fmt::Display for TranscriptionProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenAI => write!(f, "openai"),
            Self::Groq => write!(f, "groq"),
        }
    }
}

// ============================================================================
// Provider Trait
// ============================================================================

/// Trait for transcription providers
#[async_trait]
pub trait TranscriptionProvider: Send + Sync {
    /// Provider identifier
    fn id(&self) -> &'static str;

    /// Display name
    fn name(&self) -> &'static str;

    /// Check if provider is available (has credentials, etc.)
    fn is_available(&self) -> bool;

    /// Transcribe an audio file
    async fn transcribe(&self, audio_path: &Path) -> Result<TranscriptionResult>;
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Check if an API key is valid (non-empty and not a placeholder)
///
/// Used by transcription providers to determine availability.
fn is_api_key_valid(key: &str) -> bool {
    !key.is_empty() && !key.starts_with('*')
}

// ============================================================================
// OpenAI-Compatible Transcription Helper
// ============================================================================

/// Helper for OpenAI-compatible transcription APIs (OpenAI, Groq, etc.)
/// Extracts common logic to reduce code duplication.
async fn transcribe_openai_compatible(
    client: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    model: &str,
    audio_path: &Path,
    provider_name: &str,
) -> Result<TranscriptionResult> {
    let file_name = audio_path
        .file_name()
        .ok_or_else(|| TranscriptionError::InvalidAudioFile("Invalid path".to_string()))?
        .to_string_lossy()
        .to_string();

    let file_content = tokio::fs::read(audio_path).await?;

    let part = multipart::Part::bytes(file_content).file_name(file_name);

    let form = multipart::Form::new()
        .part("file", part)
        .text("model", model.to_string());

    let response = client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .timeout(DEFAULT_TRANSCRIPTION_TIMEOUT)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                TranscriptionError::NetworkError(format!(
                    "{} request timed out after {:?}",
                    provider_name, DEFAULT_TRANSCRIPTION_TIMEOUT
                ))
            } else {
                TranscriptionError::NetworkError(e.to_string())
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        // Security: Don't include full response body in error message
        // to avoid leaking sensitive data from upstream API
        return Err(TranscriptionError::ApiError(format!(
            "{} API error: HTTP {}",
            provider_name, status.as_u16()
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| TranscriptionError::ApiError(format!("Invalid response format: {}", e)))?;

    let text = json
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            TranscriptionError::ApiError(format!(
                "{} response missing 'text' field or invalid format",
                provider_name
            ))
        })?
        .to_string();

    Ok(TranscriptionResult {
        text,
        language: json["language"].as_str().map(String::from),
        duration_seconds: json["duration"].as_f64(),
        provider: provider_name.to_string(),
    })
}

// ============================================================================
// OpenAI Provider
// ============================================================================

pub struct OpenAITranscriptionProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl OpenAITranscriptionProvider {
    const API_URL: &'static str = "https://api.openai.com/v1/audio/transcriptions";

    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model: "whisper-1".to_string(),
        }
    }

    pub fn with_model(api_key: String, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model: model.to_string(),
        }
    }
}

#[async_trait]
impl TranscriptionProvider for OpenAITranscriptionProvider {
    fn id(&self) -> &'static str {
        "openai"
    }

    fn name(&self) -> &'static str {
        "OpenAI Whisper"
    }

    fn is_available(&self) -> bool {
        is_api_key_valid(&self.api_key)
    }

    async fn transcribe(&self, audio_path: &Path) -> Result<TranscriptionResult> {
        transcribe_openai_compatible(
            &self.client,
            Self::API_URL,
            &self.api_key,
            &self.model,
            audio_path,
            self.name(),
        )
        .await
    }
}

// ============================================================================
// Groq Provider (Whisper via Groq API)
// ============================================================================

pub struct GroqTranscriptionProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl GroqTranscriptionProvider {
    const API_URL: &'static str = "https://api.groq.com/openai/v1/audio/transcriptions";

    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model: "whisper-large-v3".to_string(),
        }
    }

    pub fn with_model(api_key: String, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model: model.to_string(),
        }
    }
}

#[async_trait]
impl TranscriptionProvider for GroqTranscriptionProvider {
    fn id(&self) -> &'static str {
        "groq"
    }

    fn name(&self) -> &'static str {
        "Groq Whisper"
    }

    fn is_available(&self) -> bool {
        is_api_key_valid(&self.api_key)
    }

    async fn transcribe(&self, audio_path: &Path) -> Result<TranscriptionResult> {
        transcribe_openai_compatible(
            &self.client,
            Self::API_URL,
            &self.api_key,
            &self.model,
            audio_path,
            self.name(),
        )
        .await
    }
}

// ============================================================================
// Transcription Manager
// ============================================================================

/// Manages transcription providers with fallback support
pub struct TranscriptionManager {
    providers: Vec<Arc<dyn TranscriptionProvider>>,
    default_provider: Option<TranscriptionProviderType>,
}

impl Default for TranscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TranscriptionManager {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            default_provider: None,
        }
    }

    /// Add a provider to the manager
    pub fn add_provider(&mut self, provider: Arc<dyn TranscriptionProvider>) {
        self.providers.push(provider);
    }

    /// Set the default provider type
    pub fn set_default(&mut self, provider_type: TranscriptionProviderType) {
        self.default_provider = Some(provider_type);
    }

    /// Get all available providers
    pub fn available_providers(&self) -> Vec<&'static str> {
        self.providers
            .iter()
            .filter(|p| p.is_available())
            .map(|p| p.id())
            .collect()
    }

    /// Transcribe using the default or first available provider
    pub async fn transcribe(&self, audio_path: &Path) -> Result<TranscriptionResult> {
        // Try default provider first
        if let Some(default_type) = &self.default_provider {
            let default_id = default_type.to_string();
            if let Some(provider) = self.providers.iter().find(|p| p.id() == default_id) {
                if provider.is_available() {
                    return provider.transcribe(audio_path).await;
                }
            }
        }

        // Fall back to first available
        for provider in &self.providers {
            if provider.is_available() {
                return provider.transcribe(audio_path).await;
            }
        }

        Err(TranscriptionError::ProviderNotAvailable(
            "No transcription providers available".to_string(),
        ))
    }

    /// Transcribe using a specific provider
    pub async fn transcribe_with(
        &self,
        provider_id: &str,
        audio_path: &Path,
    ) -> Result<TranscriptionResult> {
        let provider = self
            .providers
            .iter()
            .find(|p| p.id() == provider_id)
            .ok_or_else(|| {
                TranscriptionError::ProviderNotAvailable(format!(
                    "Provider '{}' not found",
                    provider_id
                ))
            })?;

        if !provider.is_available() {
            return Err(TranscriptionError::ProviderNotAvailable(format!(
                "Provider '{}' is not available (missing credentials?)",
                provider_id
            )));
        }

        provider.transcribe(audio_path).await
    }
}

// ============================================================================
// Builder for TranscriptionManager
// ============================================================================

/// Builder for creating a TranscriptionManager with configured providers
pub struct TranscriptionManagerBuilder {
    manager: TranscriptionManager,
}

impl Default for TranscriptionManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TranscriptionManagerBuilder {
    pub fn new() -> Self {
        Self {
            manager: TranscriptionManager::new(),
        }
    }

    /// Add OpenAI provider
    pub fn with_openai(mut self, api_key: String) -> Self {
        self.manager
            .add_provider(Arc::new(OpenAITranscriptionProvider::new(api_key)));
        self
    }

    /// Add Groq provider
    pub fn with_groq(mut self, api_key: String) -> Self {
        self.manager
            .add_provider(Arc::new(GroqTranscriptionProvider::new(api_key)));
        self
    }

    /// Set default provider
    pub fn default_provider(mut self, provider_type: TranscriptionProviderType) -> Self {
        self.manager.set_default(provider_type);
        self
    }

    /// Build the manager
    pub fn build(self) -> TranscriptionManager {
        self.manager
    }
}

// ============================================================================
// Legacy Compatibility
// ============================================================================

/// Legacy TranscriptionService for backward compatibility
#[deprecated(note = "Use TranscriptionManager instead")]
pub struct TranscriptionService;

#[allow(deprecated)]
impl Default for TranscriptionService {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(deprecated)]
impl TranscriptionService {
    pub fn new() -> Self {
        Self
    }

    pub async fn transcribe_openai(
        &self,
        api_key: &str,
        audio_path: &Path,
    ) -> std::result::Result<TranscriptionResult, String> {
        let provider = OpenAITranscriptionProvider::new(api_key.to_string());
        provider
            .transcribe(audio_path)
            .await
            .map_err(|e| e.to_string())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type_from_str() {
        assert_eq!(
            "openai".parse::<TranscriptionProviderType>().unwrap(),
            TranscriptionProviderType::OpenAI
        );
        assert_eq!(
            "whisper".parse::<TranscriptionProviderType>().unwrap(),
            TranscriptionProviderType::OpenAI
        );
        assert_eq!(
            "groq".parse::<TranscriptionProviderType>().unwrap(),
            TranscriptionProviderType::Groq
        );
        assert!("unknown".parse::<TranscriptionProviderType>().is_err());
    }

    #[test]
    fn test_provider_type_display() {
        assert_eq!(TranscriptionProviderType::OpenAI.to_string(), "openai");
        assert_eq!(TranscriptionProviderType::Groq.to_string(), "groq");
    }

    #[test]
    fn test_manager_builder() {
        let manager = TranscriptionManagerBuilder::new()
            .with_openai("test-key".to_string())
            .default_provider(TranscriptionProviderType::OpenAI)
            .build();

        assert_eq!(manager.available_providers(), vec!["openai"]);
    }

    #[test]
    fn test_empty_key_not_available() {
        let provider = OpenAITranscriptionProvider::new("".to_string());
        assert!(!provider.is_available());

        let provider = OpenAITranscriptionProvider::new("********".to_string());
        assert!(!provider.is_available());
    }
}
