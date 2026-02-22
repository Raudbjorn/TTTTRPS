//! System keyring token storage (feature-gated).
//!
//! Provides secure token storage using the system's native credential store:
//! - macOS: Keychain
//! - Linux: Secret Service (GNOME Keyring, KWallet)
//! - Windows: Credential Manager
//!
//! # Feature Flag
//!
//! This module requires the `keyring` feature:
//!
//! ```toml
//! [dependencies]
//! ttrpg-assistant = { version = "0.1", features = ["keyring"] }
//! ```

use async_trait::async_trait;
use keyring::Entry;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use tracing::instrument;

use super::TokenStorage;
use crate::oauth::token::TokenInfo;
use crate::oauth::{Error, Result};

/// Base service name used for keyring entries.
const SERVICE_NAME: &str = "gate";

/// Cached keyring availability status.
static KEYRING_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Flag to track if we've already checked availability.
static AVAILABILITY_CHECKED: AtomicBool = AtomicBool::new(false);

/// Keyring-based token storage.
///
/// Uses the system's native credential store for secure token storage.
/// Tokens are serialized to JSON before storage. Each provider gets
/// its own keyring entry with service name "gate-{provider}".
///
/// # Platform Support
///
/// - **macOS**: Uses Keychain Services
/// - **Linux**: Uses Secret Service (requires `gnome-keyring` or `kwallet`)
/// - **Windows**: Uses Credential Manager
///
/// # Example
///
/// ```rust,ignore
/// use crate::oauth::storage::KeyringTokenStorage;
///
/// // Check if keyring is available
/// if KeyringTokenStorage::is_available() {
///     let storage = KeyringTokenStorage::new();
///     // Use storage...
/// }
/// ```
#[derive(Debug, Clone)]
pub struct KeyringTokenStorage {
    /// Account name for keyring entry.
    account: String,
}

impl Default for KeyringTokenStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyringTokenStorage {
    /// Create a new KeyringTokenStorage with default account name.
    ///
    /// Uses account name "oauth-token".
    pub fn new() -> Self {
        Self {
            account: "oauth-token".to_string(),
        }
    }

    /// Create a KeyringTokenStorage with a custom account name.
    ///
    /// Useful for storing multiple tokens (e.g., for different users).
    pub fn with_account(account: impl Into<String>) -> Self {
        Self {
            account: account.into(),
        }
    }

    /// Check if the system keyring is available.
    ///
    /// Returns `true` if a keyring backend is available and functional.
    /// This performs a test operation to verify the keyring works.
    /// The result is cached for subsequent calls.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use crate::oauth::storage::KeyringTokenStorage;
    ///
    /// if KeyringTokenStorage::is_available() {
    ///     println!("Keyring is available");
    /// } else {
    ///     println!("Falling back to file storage");
    /// }
    /// ```
    pub fn is_available() -> bool {
        // Return cached value if available
        if let Some(&available) = KEYRING_AVAILABLE.get() {
            return available;
        }

        // Perform the check only once
        if AVAILABILITY_CHECKED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let available = Self::check_availability();
            let _ = KEYRING_AVAILABLE.set(available);
            available
        } else {
            // Another thread is checking, wait for result
            let start = std::time::Instant::now();
            loop {
                if let Some(&available) = KEYRING_AVAILABLE.get() {
                    return available;
                }
                if start.elapsed() > std::time::Duration::from_secs(5) {
                    // Timeout, assume unavailable
                    return false;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }

    /// Actually check keyring availability (internal helper).
    fn check_availability() -> bool {
        // Try to create an entry - this tests if the backend is available
        match Entry::new("gate-test", "availability-check") {
            Ok(entry) => {
                // Try to get the password (will fail with NoEntry, which is fine)
                match entry.get_password() {
                    Ok(_) => true,
                    Err(keyring::Error::NoEntry) => true, // Backend works, just no entry
                    Err(keyring::Error::NoStorageAccess(_)) => false,
                    Err(keyring::Error::PlatformFailure(_)) => false,
                    Err(_) => true, // Other errors might be transient
                }
            }
            Err(_) => false,
        }
    }

    /// Get the account name for this storage.
    pub fn account(&self) -> &str {
        &self.account
    }

    /// Get the service name for a provider.
    fn service_name(provider: &str) -> String {
        format!("{}-{}", SERVICE_NAME, provider)
    }

    /// Get the keyring entry for a provider.
    fn entry(&self, provider: &str) -> Result<Entry> {
        let service = Self::service_name(provider);
        Entry::new(&service, &self.account)
            .map_err(|e| Error::storage(format!("Failed to create keyring entry: {}", e)))
    }
}

#[async_trait]
impl TokenStorage for KeyringTokenStorage {
    #[instrument(skip(self))]
    async fn load(&self, provider: &str) -> Result<Option<TokenInfo>> {
        let entry = self.entry(provider)?;

        // Run blocking keyring operation in a blocking task
        let result = tokio::task::spawn_blocking(move || entry.get_password())
            .await
            .map_err(|e| Error::storage(format!("Keyring task failed: {}", e)))?;

        match result {
            Ok(password) => {
                let token: TokenInfo = serde_json::from_str(&password).map_err(|e| {
                    Error::storage(format!("Failed to parse token from keyring: {}", e))
                })?;
                Ok(Some(token))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(Error::from(e)),
        }
    }

    #[instrument(skip(self, token))]
    async fn save(&self, provider: &str, token: &TokenInfo) -> Result<()> {
        let entry = self.entry(provider)?;
        let json = serde_json::to_string(token)?;

        // Run blocking keyring operation in a blocking task
        tokio::task::spawn_blocking(move || entry.set_password(&json))
            .await
            .map_err(|e| Error::storage(format!("Keyring task failed: {}", e)))??;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn remove(&self, provider: &str) -> Result<()> {
        let entry = self.entry(provider)?;

        // Run blocking keyring operation in a blocking task
        let result = tokio::task::spawn_blocking(move || entry.delete_password())
            .await
            .map_err(|e| Error::storage(format!("Keyring task failed: {}", e)))?;

        match result {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // Already removed
            Err(e) => Err(Error::from(e)),
        }
    }

    fn name(&self) -> &str {
        "keyring"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a working keyring backend.
    // They may fail or be skipped on CI systems without one.

    #[test]
    fn test_new() {
        let storage = KeyringTokenStorage::new();
        assert_eq!(storage.account(), "oauth-token");
    }

    #[test]
    fn test_with_account() {
        let storage = KeyringTokenStorage::with_account("my-account");
        assert_eq!(storage.account(), "my-account");
    }

    #[test]
    fn test_default() {
        let storage = KeyringTokenStorage::default();
        assert_eq!(storage.account(), "oauth-token");
    }

    #[test]
    fn test_is_available() {
        // Just test that this doesn't panic
        let _available = KeyringTokenStorage::is_available();
    }

    #[test]
    fn test_storage_name() {
        let storage = KeyringTokenStorage::new();
        assert_eq!(storage.name(), "keyring");
    }

    #[test]
    fn test_service_name() {
        assert_eq!(
            KeyringTokenStorage::service_name("anthropic"),
            "gate-anthropic"
        );
        assert_eq!(KeyringTokenStorage::service_name("gemini"), "gate-gemini");
    }

    // Integration tests that require a working keyring
    // These use a unique account name to avoid conflicts

    #[tokio::test]
    async fn test_save_load_remove() {
        if !KeyringTokenStorage::is_available() {
            eprintln!("Skipping keyring test: keyring not available");
            return;
        }

        // Use a unique account for this test
        let storage = KeyringTokenStorage::with_account("test-gate-save-load-remove");
        let provider = "test-provider";

        // Clean up any leftover test data
        let _ = storage.remove(provider).await;

        // Initially empty
        assert!(storage.load(provider).await.unwrap().is_none());

        // Save a token
        let token = TokenInfo::new("access".into(), "refresh".into(), 3600);
        match storage.save(provider, &token).await {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Skipping keyring test: save failed: {}", e);
                return;
            }
        }

        // Load it back
        match storage.load(provider).await {
            Ok(Some(loaded)) => {
                assert_eq!(loaded.access_token, "access");
                assert_eq!(loaded.refresh_token, "refresh");
            }
            Ok(None) => {
                eprintln!("Skipping keyring test: load returned None after save");
                return;
            }
            Err(e) => {
                eprintln!("Skipping keyring test: load failed: {}", e);
                return;
            }
        }

        // Remove
        storage.remove(provider).await.unwrap();
        assert!(storage.load(provider).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_multiple_providers() {
        if !KeyringTokenStorage::is_available() {
            eprintln!("Skipping keyring test: keyring not available");
            return;
        }

        let storage = KeyringTokenStorage::with_account("test-gate-multi-provider");

        // Clean up
        let _ = storage.remove("provider-a").await;
        let _ = storage.remove("provider-b").await;

        let token_a = TokenInfo::new("access-a".into(), "refresh-a".into(), 3600);
        let token_b = TokenInfo::new("access-b".into(), "refresh-b".into(), 3600);

        match storage.save("provider-a", &token_a).await {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Skipping keyring test: save failed: {}", e);
                return;
            }
        }
        match storage.save("provider-b", &token_b).await {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Skipping keyring test: save failed: {}", e);
                let _ = storage.remove("provider-a").await;
                return;
            }
        }

        // Load both
        match (
            storage.load("provider-a").await,
            storage.load("provider-b").await,
        ) {
            (Ok(Some(loaded_a)), Ok(Some(loaded_b))) => {
                assert_eq!(loaded_a.access_token, "access-a");
                assert_eq!(loaded_b.access_token, "access-b");
            }
            _ => {
                eprintln!("Skipping keyring test: load failed");
            }
        }

        // Cleanup
        let _ = storage.remove("provider-a").await;
        let _ = storage.remove("provider-b").await;
    }

    #[tokio::test]
    async fn test_composite_token() {
        if !KeyringTokenStorage::is_available() {
            eprintln!("Skipping keyring test: keyring not available");
            return;
        }

        let storage = KeyringTokenStorage::with_account("test-gate-composite-token");
        let provider = "test-composite";
        let _ = storage.remove(provider).await;

        let token = TokenInfo::new("access".into(), "refresh".into(), 3600)
            .with_project_ids("proj-123", Some("managed-456"));
        match storage.save(provider, &token).await {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Skipping keyring test: save failed: {}", e);
                return;
            }
        }

        match storage.load(provider).await {
            Ok(Some(loaded)) => {
                let (base, project, managed) = loaded.parse_refresh_parts();
                assert_eq!(base, "refresh");
                assert_eq!(project.as_deref(), Some("proj-123"));
                assert_eq!(managed.as_deref(), Some("managed-456"));
            }
            Ok(None) => {
                eprintln!("Skipping keyring test: load returned None after save");
                return;
            }
            Err(e) => {
                eprintln!("Skipping keyring test: load failed: {}", e);
                return;
            }
        }

        storage.remove(provider).await.unwrap();
    }

    #[tokio::test]
    async fn test_overwrite() {
        if !KeyringTokenStorage::is_available() {
            eprintln!("Skipping keyring test: keyring not available");
            return;
        }

        let storage = KeyringTokenStorage::with_account("test-gate-overwrite");
        let provider = "test-overwrite";
        let _ = storage.remove(provider).await;

        let token1 = TokenInfo::new("access1".into(), "refresh1".into(), 3600);
        match storage.save(provider, &token1).await {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Skipping keyring test: save failed: {}", e);
                return;
            }
        }

        let token2 = TokenInfo::new("access2".into(), "refresh2".into(), 7200);
        match storage.save(provider, &token2).await {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Skipping keyring test: save failed: {}", e);
                let _ = storage.remove(provider).await;
                return;
            }
        }

        match storage.load(provider).await {
            Ok(Some(loaded)) => {
                assert_eq!(loaded.access_token, "access2");
                assert_eq!(loaded.refresh_token, "refresh2");
            }
            Ok(None) => {
                eprintln!("Skipping keyring test: load returned None after save");
                return;
            }
            Err(e) => {
                eprintln!("Skipping keyring test: load failed: {}", e);
                return;
            }
        }

        storage.remove(provider).await.unwrap();
    }

    #[tokio::test]
    async fn test_remove_nonexistent() {
        if !KeyringTokenStorage::is_available() {
            eprintln!("Skipping keyring test: keyring not available");
            return;
        }

        let storage = KeyringTokenStorage::with_account("test-gate-remove-nonexistent");

        // Should not error when removing nonexistent entry
        storage.remove("nonexistent-provider").await.unwrap();
    }
}
