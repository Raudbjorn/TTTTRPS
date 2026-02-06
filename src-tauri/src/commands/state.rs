//! Application State Definition
//!
//! Contains the central AppState struct used across all Tauri commands.
//! This module is extracted from the original commands.rs for better organization.

use std::sync::{Arc, RwLock};
use std::path::PathBuf;
use tokio::sync::RwLock as AsyncRwLock;

// Core imports
use crate::core::voice::{VoiceManager, VoiceConfig};
use crate::core::llm::{LLMConfig, LLMClient};
use crate::core::llm::router::{LLMRouter, RouterConfig};
use crate::core::campaign_manager::CampaignManager;
use crate::core::session_manager::SessionManager;
use crate::core::npc_gen::NPCStore;
use crate::core::credentials::CredentialManager;
use crate::core::search::EmbeddedSearch;
use crate::core::meilisearch_pipeline::MeilisearchPipeline;
use crate::core::campaign::versioning::VersionManager;
use crate::core::campaign::world_state::WorldStateManager;
use crate::core::campaign::relationships::RelationshipManager;
use crate::core::personality::{
    PersonalityStore, PersonalityApplicationManager,
    SettingTemplateStore, BlendRuleStore, PersonalityBlender,
    ContextualPersonalityManager, PersonalityIndexManager,
};
use crate::core::archetype::{ArchetypeRegistry, VocabularyBankManager, SettingPackLoader};
use crate::core::preprocess::{QueryPipeline, PreprocessConfig, DictionaryRebuildService};
use crate::core::storage::SurrealStorage;
use crate::database::Database;

// Re-export OAuth state types
pub use super::oauth::{
    ClaudeState, ClaudeStorageBackend,
    GeminiState, GeminiStorageBackend,
    CopilotState, CopilotStorageBackend,
};

// ============================================================================
// Application State
// ============================================================================

pub struct AppState {
    pub llm_client: RwLock<Option<LLMClient>>,
    pub llm_config: RwLock<Option<LLMConfig>>,
    pub llm_router: AsyncRwLock<LLMRouter>,
    pub llm_manager: Arc<AsyncRwLock<crate::core::llm::LLMManager>>,
    pub campaign_manager: CampaignManager,
    pub session_manager: SessionManager,
    pub npc_store: NPCStore,
    pub credentials: CredentialManager,
    pub voice_manager: Arc<AsyncRwLock<VoiceManager>>,
    pub embedded_search: Arc<EmbeddedSearch>,
    pub personality_store: Arc<PersonalityStore>,
    pub personality_manager: Arc<PersonalityApplicationManager>,
    pub ingestion_pipeline: Arc<MeilisearchPipeline>,
    pub database: Database,
    // Campaign management modules
    pub version_manager: VersionManager,
    pub world_state_manager: WorldStateManager,
    pub relationship_manager: RelationshipManager,
    pub location_manager: crate::core::location_manager::LocationManager,
    // Document extraction settings
    pub extraction_settings: AsyncRwLock<crate::ingestion::ExtractionSettings>,
    // OAuth Gate clients
    pub claude: Arc<ClaudeState>,
    pub gemini: Arc<GeminiState>,
    pub copilot: Arc<CopilotState>,
    // Archetype Registry for unified character archetype management
    pub archetype_registry: AsyncRwLock<Option<Arc<ArchetypeRegistry>>>,
    // Vocabulary Bank Manager for NPC dialogue phrase management
    pub vocabulary_manager: AsyncRwLock<Option<Arc<VocabularyBankManager>>>,
    // Setting Pack Loader for campaign setting customization
    pub setting_pack_loader: Arc<SettingPackLoader>,
    // Personality Extensions
    pub template_store: Arc<SettingTemplateStore>,
    pub blend_rule_store: Arc<BlendRuleStore>,
    pub personality_blender: Arc<PersonalityBlender>,
    pub contextual_personality_manager: Arc<ContextualPersonalityManager>,
    // SurrealDB Storage (migration from Meilisearch)
    // Optional during migration period - will be required after Phase 7
    pub surreal_storage: Option<Arc<SurrealStorage>>,
    // Query preprocessing pipeline (typo correction + synonym expansion)
    // Uses AsyncRwLock to allow dictionary reloading after ingestion
    pub query_pipeline: Option<Arc<AsyncRwLock<QueryPipeline>>>,
    // Dictionary rebuild service for post-ingestion dictionary generation
    pub dictionary_rebuild_service: Arc<DictionaryRebuildService>,
}

impl AppState {
    /// Initialize all default state components
    ///
    /// NOTE: This function does NOT initialize `embedded_search` - that requires async
    /// initialization and must be done in main.rs. This function is kept for backward
    /// compatibility but the embedded_search field must be passed separately when
    /// constructing AppState.
    #[allow(clippy::type_complexity)]
    pub fn init_defaults() -> (
        CampaignManager,
        SessionManager,
        NPCStore,
        CredentialManager,
        Arc<AsyncRwLock<VoiceManager>>,
        Arc<PersonalityStore>,
        Arc<PersonalityApplicationManager>,
        Arc<MeilisearchPipeline>,
        AsyncRwLock<LLMRouter>,
        VersionManager,
        WorldStateManager,
        RelationshipManager,
        crate::core::location_manager::LocationManager,
        Arc<AsyncRwLock<crate::core::llm::LLMManager>>,
        Arc<ClaudeState>,
        Arc<GeminiState>,
        Arc<CopilotState>,
        Arc<SettingPackLoader>,
        Arc<SettingTemplateStore>,
        Arc<BlendRuleStore>,
        Arc<PersonalityBlender>,
        Arc<ContextualPersonalityManager>,
        Arc<AsyncRwLock<QueryPipeline>>,
        Arc<DictionaryRebuildService>,
    ) {
        let personality_store = Arc::new(PersonalityStore::new());
        let personality_manager = Arc::new(PersonalityApplicationManager::new(personality_store.clone()));

        // Initialize Claude Client
        let claude = match ClaudeState::with_defaults() {
            Ok(state) => Arc::new(state),
            Err(e) => {
                log::warn!("Failed to initialize Claude with default storage: {}. Using file storage.", e);
                Arc::new(ClaudeState::new(ClaudeStorageBackend::File)
                    .expect("Failed to initialize Claude with file storage"))
            }
        };

        // Initialize Gemini client
        let gemini = match GeminiState::with_defaults() {
            Ok(state) => Arc::new(state),
            Err(e) => {
                log::warn!("Failed to initialize Gemini with default storage: {}. Using file storage.", e);
                Arc::new(GeminiState::new(GeminiStorageBackend::File)
                    .expect("Failed to initialize Gemini with file storage"))
            }
        };

        // Initialize Copilot Gate client
        let copilot = match CopilotState::with_defaults() {
            Ok(state) => Arc::new(state),
            Err(e) => {
                log::warn!("Failed to initialize Copilot Gate with default storage: {}. Using file storage.", e);
                Arc::new(CopilotState::new(CopilotStorageBackend::File)
                    .expect("Failed to initialize Copilot Gate with file storage"))
            }
        };

        // Personality Extension components
        // TODO: PersonalityIndexManager needs to be updated to work with EmbeddedSearch.
        // Currently using placeholder values. Once EmbeddedSearch exposes an HTTP endpoint
        // or we refactor PersonalityIndexManager to use the embedded client directly,
        // this should be updated. For now, these stores will not have search capabilities
        // until the embedded_search is properly integrated.
        //
        // Options for future integration:
        // 1. Pass EmbeddedSearch to PersonalityIndexManager and use its internal client
        // 2. If MeilisearchLib exposes an HTTP server, use that URL
        // 3. Refactor PersonalityIndexManager to accept Arc<MeilisearchLib> directly
        let personality_index_manager = Arc::new(PersonalityIndexManager::new(
            "http://placeholder:7700", // Placeholder - will be updated when EmbeddedSearch integration is complete
            None,
        ));

        let template_store = Arc::new(SettingTemplateStore::from_manager(
            personality_index_manager.clone()
        ));

        let blend_rule_store = Arc::new(BlendRuleStore::new(
            personality_index_manager.clone()
        ));

        let personality_blender = Arc::new(PersonalityBlender::new());
        let contextual_personality_manager = Arc::new(ContextualPersonalityManager::new(
            personality_blender.clone(),
            blend_rule_store.clone(),
            personality_store.clone(),
        ));

        // Initialize query preprocessing pipeline
        // Attempt to load with full dictionaries, fall back to minimal if unavailable
        // Uses AsyncRwLock to allow dictionary reloading after ingestion
        let query_pipeline = match QueryPipeline::new(PreprocessConfig::default()) {
            Ok(pipeline) => {
                log::info!("Query preprocessing pipeline initialized with full dictionaries");
                Arc::new(AsyncRwLock::new(pipeline))
            }
            Err(e) => {
                log::warn!(
                    "Failed to initialize query pipeline with dictionaries: {}. Using minimal pipeline.",
                    e
                );
                Arc::new(AsyncRwLock::new(QueryPipeline::new_minimal()))
            }
        };

        // Initialize dictionary rebuild service for post-ingestion dictionary regeneration
        let dictionary_rebuild_service = Arc::new(DictionaryRebuildService::new());

        (
            CampaignManager::new(),
            SessionManager::new(),
            NPCStore::new(),
            CredentialManager::with_service("ttrpg-assistant"),
            Arc::new(AsyncRwLock::new(VoiceManager::new(VoiceConfig {
                cache_dir: Some(PathBuf::from("./voice_cache")),
                ..Default::default()
            }))),
            personality_store,
            personality_manager,
            Arc::new(MeilisearchPipeline::with_defaults()),
            AsyncRwLock::new(LLMRouter::new(RouterConfig::default())),
            VersionManager::default(),
            WorldStateManager::default(),
            RelationshipManager::default(),
            crate::core::location_manager::LocationManager::new(),
            Arc::new(AsyncRwLock::new(crate::core::llm::LLMManager::new())),
            claude,
            gemini,
            copilot,
            Arc::new(SettingPackLoader::new()),
            template_store,
            blend_rule_store,
            personality_blender,
            contextual_personality_manager,
            query_pipeline,
            dictionary_rebuild_service,
        )
    }
}
