use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};

use crate::config::AppConfig;
use crate::core::archetype::InMemoryArchetypeRegistry;
use crate::core::budget::BudgetEnforcer;
use crate::core::search::embeddings::EmbeddingProvider;
use crate::core::campaign_manager::CampaignManager;
use crate::core::cost_predictor::CostPredictor;
use crate::core::credentials::CredentialManager;
use crate::core::llm::providers::{AuthMethod, ProviderConfig};
use crate::core::llm::router::LLMRouter;
use crate::core::location_gen::LocationGenerator;
use crate::core::npc_gen::{InMemoryNpcIndexes, NPCGenerator};
use crate::core::personality::application::PersonalityApplicationManager;
use crate::core::personality_base::PersonalityStore;
use crate::core::plot_manager::PlotManager;
use crate::core::preprocess::pipeline::QueryPipeline;
use crate::core::session_manager::SessionManager;
use crate::core::session_summary::SessionSummarizer;
use crate::core::storage::surrealdb::SurrealStorage;
use crate::core::transcription::TranscriptionManager;
use crate::core::voice::manager::VoiceManager;
use crate::core::voice::queue::events::QueueEventEmitter;
use crate::core::voice::queue::SynthesisQueue;
use crate::database::Database;

use super::audio::AudioPlayer;

use super::events::AppEvent;

/// Centralized handle to all backend services.
///
/// Created once at startup, then passed (by ref or clone) to views
/// that need backend access. Clone-able fields are cloned directly;
/// non-Clone fields are wrapped in Arc.
pub struct Services {
    // ---- Original services ----
    pub llm: LLMRouter,
    pub storage: SurrealStorage,
    pub database: Database,
    pub session: Arc<SessionManager>,
    pub personality: Arc<PersonalityApplicationManager>,
    pub voice: Arc<SynthesisQueue>,
    pub voice_manager: Arc<RwLock<VoiceManager>>,
    pub audio: AudioPlayer,
    pub credentials: CredentialManager,
    pub event_tx: mpsc::UnboundedSender<AppEvent>,

    // ---- Phase 3 additions ----
    pub archetype_registry: Arc<InMemoryArchetypeRegistry>,
    pub npc_indexes: Arc<InMemoryNpcIndexes>,
    pub query_pipeline: Arc<RwLock<QueryPipeline>>,
    pub budget: Arc<BudgetEnforcer>,
    pub cost_predictor: Arc<CostPredictor>,
    pub transcription: Arc<TranscriptionManager>,
    pub session_summarizer: Arc<SessionSummarizer>,
    pub npc_generator: Arc<NPCGenerator>,
    pub campaign_manager: Arc<CampaignManager>,
    pub plot_manager: Arc<PlotManager>,
    pub location_generator: Arc<LocationGenerator>,

    // ---- Phase 4 additions ----
    pub embedding_provider: Option<Arc<dyn EmbeddingProvider>>,

    // ---- Phase 7 additions ----
    pub input_validator: Arc<crate::core::input_validator::InputValidator>,
    pub search_analytics: Arc<crate::core::search_analytics::SearchAnalytics>,
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

        // ================================================================
        // Storage layer
        // ================================================================

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

        // ================================================================
        // Asset-backed registries (Phase 1 + 2)
        // ================================================================

        let archetype_registry = Arc::new(InMemoryArchetypeRegistry::new().await);
        log::info!(
            "Archetype registry initialized: {} archetypes",
            archetype_registry.count().await
        );

        let npc_indexes = Arc::new(InMemoryNpcIndexes::new());
        log::info!("NPC indexes initialized (empty — populated on demand)");

        // ================================================================
        // Query preprocessing (Phase 3)
        // ================================================================

        // Load synonyms + preprocessing config from bundled assets
        let query_pipeline = match crate::core::assets::AssetLoader::load_preprocessing_config() {
            Ok(preprocess_config) => {
                match QueryPipeline::new(preprocess_config) {
                    Ok(pipeline) => {
                        log::info!("Query pipeline initialized with typo correction + synonym expansion");
                        Arc::new(RwLock::new(pipeline))
                    }
                    Err(e) => {
                        log::warn!("Query pipeline init failed (using minimal): {e}");
                        Arc::new(RwLock::new(QueryPipeline::new_minimal()))
                    }
                }
            }
            Err(e) => {
                log::warn!("Preprocessing config load failed (using minimal): {e}");
                Arc::new(RwLock::new(QueryPipeline::new_minimal()))
            }
        };

        // ================================================================
        // LLM providers
        // ================================================================

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

        // ================================================================
        // Core gameplay services
        // ================================================================

        let session = Arc::new(SessionManager::new());
        let session_summarizer = Arc::new(SessionSummarizer::new());
        let campaign_manager = Arc::new(CampaignManager::with_data_dir(&data_dir));
        let plot_manager = Arc::new(PlotManager::new());
        let npc_generator = Arc::new(NPCGenerator::new());
        let location_generator = Arc::new(LocationGenerator::new());

        // ================================================================
        // Budget and cost tracking
        // ================================================================

        let budget = Arc::new(BudgetEnforcer::new());
        let cost_predictor = Arc::new(CostPredictor::new());

        // ================================================================
        // Personality system
        // ================================================================

        let personality_store = Arc::new(PersonalityStore::new());
        let personality = Arc::new(PersonalityApplicationManager::new(personality_store));

        // ================================================================
        // Voice and transcription
        // ================================================================

        let voice = Arc::new(SynthesisQueue::with_defaults());
        let voice_manager = Arc::new(RwLock::new(VoiceManager::new(config.voice.clone())));
        let transcription = Arc::new(TranscriptionManager::new());

        // Audio player (dedicated playback thread)
        let audio = AudioPlayer::new(event_tx.clone());

        // ================================================================
        // Embedding provider (Phase 4)
        // ================================================================

        let embedding_provider: Option<Arc<dyn EmbeddingProvider>> = {
            use crate::core::search::providers::ollama::OllamaEmbeddings;
            let ollama = OllamaEmbeddings::new(
                "http://localhost:11434",
                "nomic-embed-text",
                Some(768),
            );
            if ollama.health_check().await {
                log::info!("Embedding provider: Ollama (nomic-embed-text, 768d)");
                Some(Arc::new(ollama))
            } else {
                log::warn!("Ollama not available — embeddings disabled (chunks stored without vectors)");
                None
            }
        };

        log::info!("All services initialized");

        Ok(Self {
            llm,
            storage,
            database,
            session,
            personality,
            voice,
            voice_manager,
            audio,
            credentials,
            event_tx,
            archetype_registry,
            npc_indexes,
            query_pipeline,
            budget,
            cost_predictor,
            transcription,
            session_summarizer,
            npc_generator,
            campaign_manager,
            plot_manager,
            location_generator,
            embedding_provider,
            input_validator: Arc::new(crate::core::input_validator::InputValidator::new()),
            search_analytics: Arc::new(crate::core::search_analytics::SearchAnalytics::new()),
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
