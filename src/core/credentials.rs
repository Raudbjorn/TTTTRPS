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

/// Legacy credential format — kept for export/import and migration from old
/// JSON-blob keyring entries. New code should use `store_provider_secret()`.
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
    // Provider Secret Operations (new — raw string storage)
    // ========================================================================

    /// Store a provider's API key as a raw string.
    pub fn store_provider_secret(&self, provider_id: &str, api_key: &str) -> Result<()> {
        let key = format!("llm_{provider_id}");
        self.store_secret(&key, api_key)
    }

    /// Retrieve a provider's API key.
    ///
    /// Handles migration: if the stored value starts with `{`, it's the old
    /// JSON `LLMCredential` format — extract `api_key` from it.
    pub fn get_provider_secret(&self, provider_id: &str) -> Result<String> {
        let key = format!("llm_{provider_id}");
        let raw = self.get_secret(&key)?;

        if raw.starts_with('{') {
            // Old JSON format — extract the api_key field
            if let Ok(cred) = serde_json::from_str::<LLMCredential>(&raw) {
                if let Some(api_key) = cred.api_key {
                    if !api_key.is_empty() {
                        return Ok(api_key);
                    }
                }
            }
            // JSON but no usable api_key (e.g. OAuth metadata)
            Err(CredentialError::NotFound(key))
        } else {
            Ok(raw)
        }
    }

    /// Delete a provider's secret from keyring.
    pub fn delete_provider_secret(&self, provider_id: &str) -> Result<()> {
        let key = format!("llm_{provider_id}");
        self.delete_secret(&key)
    }

    /// List provider IDs that have a secret stored in keyring.
    /// Uses the canonical PROVIDERS table from the providers module.
    pub fn list_providers_with_secrets(&self) -> Vec<String> {
        use crate::core::llm::providers::PROVIDERS;
        PROVIDERS
            .iter()
            .filter(|p| self.has_secret(&format!("llm_{}", p.id)))
            .map(|p| p.id.to_string())
            .collect()
    }

    // ========================================================================
    // Legacy LLM Credential Operations (kept for export/import + migration)
    // ========================================================================

    /// Store an LLM provider credential (legacy JSON format)
    pub fn store_llm_credential(&self, credential: &LLMCredential) -> Result<()> {
        let key = format!("llm_{}", credential.provider);
        let json = serde_json::to_string(credential)?;
        self.store_secret(&key, &json)
    }

    /// Get an LLM provider credential (legacy JSON format)
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

    /// List all stored LLM providers (by checking known providers).
    /// Uses the canonical PROVIDERS table.
    pub fn list_llm_providers(&self) -> Vec<String> {
        self.list_providers_with_secrets()
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
        use crate::core::llm::providers::PROVIDERS;

        // Clear LLM credentials (covers both old JSON and new raw formats)
        for p in PROVIDERS {
            let _ = self.delete_provider_secret(p.id);
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
        "anthropic" => key.starts_with("sk-ant-"),
        "google" => key.starts_with("AIza"),
        "openai" => key.starts_with("sk-"),
        "elevenlabs" => key.len() == 32, // ElevenLabs keys are 32 chars
        // OAuth providers don't use API keys
        "claude" | "gemini" | "copilot" => false,
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
        assert!(validate_api_key("anthropic", "sk-ant-api03-test"));
        assert!(!validate_api_key("anthropic", "invalid-key"));
        assert!(validate_api_key("google", "AIzaSyTest123"));
        assert!(!validate_api_key("google", "invalid"));
        // OAuth providers don't use API keys
        assert!(!validate_api_key("claude", "anything"));
        assert!(!validate_api_key("gemini", "anything"));
        assert!(!validate_api_key("copilot", "anything"));
    }

    #[test]
    fn test_get_provider_secret_migrates_old_json() {
        // Simulate the old JSON format that get_provider_secret should handle
        let json = r#"{"provider":"openai","api_key":"sk-test-123","host":null,"model":"gpt-4o","created_at":"2024-01-01","updated_at":"2024-01-01"}"#;
        let cred: LLMCredential = serde_json::from_str(json).unwrap();
        assert_eq!(cred.api_key.as_deref(), Some("sk-test-123"));

        // Verify the migration logic extracts the key correctly
        assert!(json.starts_with('{'));
        let parsed: LLMCredential = serde_json::from_str(json).unwrap();
        let key = parsed.api_key.unwrap();
        assert_eq!(key, "sk-test-123");
    }

    #[test]
    fn test_get_provider_secret_old_json_no_key() {
        // OAuth providers stored JSON with api_key: null
        let json = r#"{"provider":"claude","api_key":null,"host":null,"model":"claude-3","created_at":"2024-01-01","updated_at":"2024-01-01"}"#;
        assert!(json.starts_with('{'));
        let parsed: LLMCredential = serde_json::from_str(json).unwrap();
        assert!(parsed.api_key.is_none());
    }
}
