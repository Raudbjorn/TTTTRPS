use std::sync::Arc;

use tokio::sync::mpsc;

use crate::config::AppConfig;
use crate::core::credentials::CredentialManager;
use crate::core::llm::providers::{AuthMethod, ProviderConfig};
use crate::core::llm::router::LLMRouter;
use crate::core::personality::application::PersonalityApplicationManager;
use crate::core::personality_base::PersonalityStore;
use crate::core::session_manager::SessionManager;
use crate::core::storage::surrealdb::SurrealStorage;
use crate::core::voice::queue::events::QueueEventEmitter;
use crate::core::voice::queue::SynthesisQueue;
use crate::database::Database;

use super::events::AppEvent;

/// Centralized handle to all backend services.
///
/// Created once at startup, then passed (by ref or clone) to views
/// that need backend access. Clone-able fields are cloned directly;
/// non-Clone fields are wrapped in Arc.
pub struct Services {
    pub llm: LLMRouter,
    pub storage: SurrealStorage,
    pub database: Database,
    pub session: Arc<SessionManager>,
    pub personality: Arc<PersonalityApplicationManager>,
    pub voice: Arc<SynthesisQueue>,
    pub credentials: CredentialManager,
    pub event_tx: mpsc::UnboundedSender<AppEvent>,
}

impl Services {
    /// Initialize all services from config.
    ///
    /// Failures here are fatal — the TUI cannot run without core services.
    pub async fn init(
        config: &AppConfig,
        event_tx: mpsc::UnboundedSender<AppEvent>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = config.data_dir();
        log::info!("Initializing services with data dir: {}", data_dir.display());

        // SurrealDB (embedded RocksDB)
        let surreal_path = data_dir.join("surrealdb");
        let storage = SurrealStorage::new(surreal_path).await?;
        log::info!("SurrealDB storage initialized");

        // Legacy SQLite (still needed during migration)
        let database = Database::new(&data_dir).await?;
        log::info!("SQLite database initialized");

        // Credential manager (keyring)
        let credentials = CredentialManager::new();
        log::info!("Credential manager initialized");

        // LLM router — load saved providers from config + keyring
        let mut llm = LLMRouter::with_defaults();
        for (id, provider_config) in &config.llm.providers {
            log::info!("Loading saved LLM provider: {id}");
            let provider_config = restore_provider_config(id, provider_config, &credentials);
            let provider = provider_config.create_provider();
            llm.add_provider(provider).await;
        }
        log::info!(
            "LLM router initialized with {} providers",
            llm.provider_ids().len()
        );

        // Session manager (in-memory)
        let session = Arc::new(SessionManager::new());

        // Personality system
        let personality_store = Arc::new(PersonalityStore::new());
        let personality = Arc::new(PersonalityApplicationManager::new(personality_store));

        // Voice synthesis queue
        let voice = Arc::new(SynthesisQueue::with_defaults());

        Ok(Self {
            llm,
            storage,
            database,
            session,
            personality,
            voice,
            credentials,
            event_tx,
        })
    }

    // ========================================================================
    // Provider CRUD
    // ========================================================================

    /// Save a provider: store secret in keyring, config in config.toml, add to router.
    pub fn save_provider(
        &self,
        provider_id: &str,
        api_key: &str,
        host: &str,
        model: &str,
    ) -> Result<(), String> {
        let config = ProviderConfig::from_parts(provider_id, api_key, host, model);

        // Store secret in keyring (only for API-key providers with a non-empty key)
        if config.auth_method() == AuthMethod::ApiKey && !api_key.is_empty() {
            self.credentials
                .store_provider_secret(provider_id, api_key)
                .map_err(|e| format!("Failed to store credential: {e}"))?;
        }

        // Persist to config.toml (without secret)
        let mut app_config = AppConfig::load();
        app_config
            .llm
            .providers
            .insert(provider_id.to_string(), config.without_secret());
        app_config.save()?;

        // Add to live router
        let provider = config.create_provider();
        let mut llm = self.llm.clone();
        tokio::spawn(async move {
            llm.add_provider(provider).await;
        });

        log::info!("Saved provider: {provider_id} ({model})");
        Ok(())
    }

    /// Delete a provider: remove from keyring, config.toml, and router.
    pub fn delete_provider(&self, provider_id: &str) {
        // Remove from keyring
        let _ = self.credentials.delete_provider_secret(provider_id);

        // Remove from config.toml
        let mut app_config = AppConfig::load();
        app_config.llm.providers.remove(provider_id);
        if let Err(e) = app_config.save() {
            log::error!("Failed to save config after delete: {e}");
        }

        // Remove from live router
        let mut llm = self.llm.clone();
        let id = provider_id.to_string();
        tokio::spawn(async move {
            llm.remove_provider(&id).await;
        });

        log::info!("Deleted provider: {provider_id}");
    }

    /// Save an OAuth/DeviceCode provider (config.toml only — tokens use their own TokenStorage).
    pub fn save_oauth_provider(&self, provider_id: &str, model: &str) -> Result<(), String> {
        let config = ProviderConfig::from_parts(provider_id, "", "", model);

        let mut app_config = AppConfig::load();
        app_config
            .llm
            .providers
            .insert(provider_id.to_string(), config);
        app_config.save()?;

        log::info!("Saved OAuth provider config: {provider_id} ({model})");
        Ok(())
    }
}

/// Restore a `ProviderConfig` by injecting the API key from keyring.
///
/// The config.toml stores provider settings but not secrets. This function
/// reads the secret from keyring and merges it into the config.
fn restore_provider_config(
    id: &str,
    config: &ProviderConfig,
    credentials: &CredentialManager,
) -> ProviderConfig {
    match credentials.get_provider_secret(id) {
        Ok(key) => config.with_api_key(&key),
        Err(_) => config.clone(),
    }
}

/// Voice queue event emitter that forwards events into the TUI event channel.
pub struct TuiQueueEmitter {
    tx: mpsc::UnboundedSender<AppEvent>,
}

impl TuiQueueEmitter {
    pub fn new(tx: mpsc::UnboundedSender<AppEvent>) -> Self {
        Self { tx }
    }
}

impl QueueEventEmitter for TuiQueueEmitter {
    fn emit_json(&self, channel: &str, payload: serde_json::Value) {
        log::debug!("Voice queue event on {channel}: {payload}");
        let msg = format!("[voice] {channel}");
        let _ = self.tx.send(AppEvent::Notification(
            super::events::Notification {
                id: 0,
                message: msg,
                level: super::events::NotificationLevel::Info,
                ttl_ticks: 60,
            },
        ));
    }
}
