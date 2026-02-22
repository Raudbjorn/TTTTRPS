//! Secure Credential Storage
//!
//! Uses the system keychain (Keyring) for secure storage of API keys
//! and other sensitive credentials.

use keyring::Entry;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const SERVICE_NAME: &str = "ttrpg-assistant";

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum CredentialError {
    #[error("Keyring error: {0}")]
    KeyringError(#[from] keyring::Error),

    #[error("Credential not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid credential format")]
    InvalidFormat,
}

pub type Result<T> = std::result::Result<T, CredentialError>;

// ============================================================================
// Credential Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMCredential {
    pub provider: String,
    pub api_key: Option<String>,
    pub host: Option<String>,
    pub model: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCredential {
    pub provider: String,
    pub api_key: String,
    pub voice_id: Option<String>,
}

// ============================================================================
// Credential Manager
// ============================================================================

pub struct CredentialManager {
    service: String,
}

impl Default for CredentialManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialManager {
    pub fn new() -> Self {
        Self {
            service: SERVICE_NAME.to_string(),
        }
    }

    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    // ========================================================================
    // Raw Key Operations
    // ========================================================================

    /// Store a raw string secret
    pub fn store_secret(&self, key: &str, value: &str) -> Result<()> {
        let entry = Entry::new(&self.service, key)?;
        entry.set_password(value)?;
        log::info!("Stored secret for key: {}", key);
        Ok(())
    }

    /// Retrieve a raw string secret
    pub fn get_secret(&self, key: &str) -> Result<String> {
        let entry = Entry::new(&self.service, key)?;
        match entry.get_password() {
            Ok(value) => Ok(value),
            Err(keyring::Error::NoEntry) => Err(CredentialError::NotFound(key.to_string())),
            Err(e) => Err(CredentialError::KeyringError(e)),
        }
    }

    /// Delete a secret
    pub fn delete_secret(&self, key: &str) -> Result<()> {
        let entry = Entry::new(&self.service, key)?;
        match entry.delete_password() {
            Ok(()) => {
                log::info!("Deleted secret for key: {}", key);
                Ok(())
            }
            Err(keyring::Error::NoEntry) => Ok(()), // Already deleted
            Err(e) => Err(CredentialError::KeyringError(e)),
        }
    }

    /// Check if a secret exists
    pub fn has_secret(&self, key: &str) -> bool {
        self.get_secret(key).is_ok()
    }

    // ========================================================================
    // LLM Credential Operations
    // ========================================================================

    /// Store an LLM provider credential
    pub fn store_llm_credential(&self, credential: &LLMCredential) -> Result<()> {
        let key = format!("llm_{}", credential.provider);
        let json = serde_json::to_string(credential)?;
        self.store_secret(&key, &json)
    }

    /// Get an LLM provider credential
    pub fn get_llm_credential(&self, provider: &str) -> Result<LLMCredential> {
        let key = format!("llm_{}", provider);
        let json = self.get_secret(&key)?;
        let credential: LLMCredential = serde_json::from_str(&json)?;
        Ok(credential)
    }

    /// Delete an LLM provider credential
    pub fn delete_llm_credential(&self, provider: &str) -> Result<()> {
        let key = format!("llm_{}", provider);
        self.delete_secret(&key)
    }

    /// List all stored LLM providers (by checking known providers)
    pub fn list_llm_providers(&self) -> Vec<String> {
        let known_providers = ["ollama", "claude", "gemini", "openai"];
        known_providers
            .iter()
            .filter(|p| self.has_secret(&format!("llm_{}", p)))
            .map(|s| s.to_string())
            .collect()
    }

    // ========================================================================
    // Voice Credential Operations
    // ========================================================================

    /// Store a voice provider credential
    pub fn store_voice_credential(&self, credential: &VoiceCredential) -> Result<()> {
        let key = format!("voice_{}", credential.provider);
        let json = serde_json::to_string(credential)?;
        self.store_secret(&key, &json)
    }

    /// Get a voice provider credential
    pub fn get_voice_credential(&self, provider: &str) -> Result<VoiceCredential> {
        let key = format!("voice_{}", provider);
        let json = self.get_secret(&key)?;
        let credential: VoiceCredential = serde_json::from_str(&json)?;
        Ok(credential)
    }

    /// Delete a voice provider credential
    pub fn delete_voice_credential(&self, provider: &str) -> Result<()> {
        let key = format!("voice_{}", provider);
        self.delete_secret(&key)
    }

    // ========================================================================
    // Utility Functions
    // ========================================================================

    /// Clear all credentials for this service
    pub fn clear_all(&self) -> Result<()> {
        // Clear LLM credentials
        for provider in ["ollama", "claude", "gemini", "openai"] {
            let _ = self.delete_llm_credential(provider);
        }

        // Clear voice credentials
        for provider in ["elevenlabs", "fishaudio", "ollama_tts"] {
            let _ = self.delete_voice_credential(provider);
        }

        log::info!("Cleared all credentials");
        Ok(())
    }

    /// Export credentials as encrypted JSON (for backup)
    /// Note: This returns the raw JSON - encryption should be handled by caller
    pub fn export_credentials(&self) -> Result<String> {
        let mut export = serde_json::Map::new();

        // Export LLM credentials
        let llm_providers = self.list_llm_providers();
        let mut llm_creds = serde_json::Map::new();
        for provider in llm_providers {
            if let Ok(cred) = self.get_llm_credential(&provider) {
                llm_creds.insert(provider, serde_json::to_value(cred)?);
            }
        }
        export.insert("llm".to_string(), serde_json::Value::Object(llm_creds));

        Ok(serde_json::to_string_pretty(&export)?)
    }

    /// Import credentials from JSON
    pub fn import_credentials(&self, json: &str) -> Result<()> {
        let data: serde_json::Value = serde_json::from_str(json)?;

        // Import LLM credentials
        if let Some(llm_creds) = data.get("llm").and_then(|v| v.as_object()) {
            for (_, cred_value) in llm_creds {
                if let Ok(cred) = serde_json::from_value::<LLMCredential>(cred_value.clone()) {
                    self.store_llm_credential(&cred)?;
                }
            }
        }

        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Mask an API key for display (show first 4 and last 4 chars)
pub fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "********".to_string();
    }
    format!("{}...{}", &key[..4], &key[key.len()-4..])
}

/// Validate an API key format
pub fn validate_api_key(provider: &str, key: &str) -> bool {
    match provider {
        "claude" => key.starts_with("sk-ant-"),
        "gemini" => key.starts_with("AIza"),
        "openai" => key.starts_with("sk-"),
        "elevenlabs" => key.len() == 32, // ElevenLabs keys are 32 chars
        _ => !key.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key() {
        assert_eq!(mask_api_key("sk-ant-api03-abcdefghijklmnop"), "sk-a...mnop");
        assert_eq!(mask_api_key("short"), "********");
    }

    #[test]
    fn test_validate_api_key() {
        assert!(validate_api_key("claude", "sk-ant-api03-test"));
        assert!(!validate_api_key("claude", "invalid-key"));
        assert!(validate_api_key("gemini", "AIzaSyTest123"));
        assert!(!validate_api_key("gemini", "invalid"));
    }
}
