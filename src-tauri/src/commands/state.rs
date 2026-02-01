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
use crate::core::sidecar_manager::{SidecarManager, MeilisearchConfig};
use crate::core::search::SearchClient;
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
    pub sidecar_manager: Arc<SidecarManager>,
    pub search_client: Arc<SearchClient>,
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
}

impl AppState {
    /// Initialize all default state components
    pub fn init_defaults() -> (
        CampaignManager,
        SessionManager,
        NPCStore,
        CredentialManager,
        Arc<AsyncRwLock<VoiceManager>>,
        Arc<SidecarManager>,
        Arc<SearchClient>,
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
    ) {
        let sidecar_config = MeilisearchConfig::default();
        let search_client = SearchClient::new(
            &sidecar_config.url(),
            Some(&sidecar_config.master_key),
        ).expect("Failed to create SearchClient");
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
        let meilisearch_url = sidecar_config.url();
        let meilisearch_key = sidecar_config.master_key.clone();

        let personality_index_manager = Arc::new(PersonalityIndexManager::new(
            &meilisearch_url,
            Some(&meilisearch_key),
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

        (
            CampaignManager::new(),
            SessionManager::new(),
            NPCStore::new(),
            CredentialManager::with_service("ttrpg-assistant"),
            Arc::new(AsyncRwLock::new(VoiceManager::new(VoiceConfig {
                cache_dir: Some(PathBuf::from("./voice_cache")),
                ..Default::default()
            }))),
            Arc::new(SidecarManager::with_config(sidecar_config)),
            Arc::new(search_client),
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
        )
    }
}
