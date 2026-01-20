//! Tauri Commands
//!
//! All Tauri IPC commands exposed to the frontend.

use tauri::{State, Manager};
use crate::core::voice::{
    VoiceManager, VoiceConfig, VoiceProviderType,
    SynthesisRequest, OutputFormat, VoiceProviderDetection,
    detect_providers, ProviderInstaller, InstallStatus,
    AvailablePiperVoice, get_recommended_piper_voices,
    types::{QueuedVoice, VoiceStatus}
};
use crate::core::models::Campaign;
use std::sync::{Arc, RwLock};
use tokio::sync::RwLock as AsyncRwLock;
use std::path::Path;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::database::{Database, NpcConversation, ConversationMessage};

// Core modules
// use crate::core::database::Database;
use crate::core::llm::{LLMConfig, LLMClient, ChatMessage, MessageRole};
use crate::core::llm::{model_selector, ModelSelection, TaskComplexity};
use crate::core::llm::router::{LLMRouter, RouterConfig, ProviderStats};
use crate::core::campaign_manager::{
    CampaignManager, SessionNote, SnapshotSummary, ThemeWeights
};
use crate::core::theme;
use crate::core::session_manager::{
    SessionManager, GameSession, SessionSummary, CombatState, Combatant,
    CombatantType, create_common_condition
};
use crate::core::character_gen::{CharacterGenerator, GenerationOptions, Character, SystemInfo};
use crate::core::npc_gen::{NPCGenerator, NPCGenerationOptions, NPC, NPCStore};
// NPC Extensions - Vocabulary, Names, and Dialects
use crate::core::npc_gen::{
    VocabularyBank as NpcVocabularyBank, Formality, PhraseEntry,
    CulturalNamingRules, NameStructure,
    DialectDefinition, DialectTransformer, DialectTransformResult, Intensity,
    load_yaml_file, get_vocabulary_dir, get_names_dir, get_dialects_dir,
    NpcIndexStats, ensure_npc_indexes, get_npc_index_stats,
};
use crate::core::location_gen::{LocationGenerator, LocationGenerationOptions, Location};
use crate::core::personality::{
    PersonalityApplicationManager, ActivePersonalityContext, SceneMood, ContentType, StyledContent, PersonalityPreview,
    NPCDialogueStyler, NarrationStyleManager, NarrationType, PersonalitySettings,
    ExtendedPersonalityPreview, PreviewResponse, NarrativeTone, VocabularyLevel,
    NarrativeStyle, VerbosityLevel, GenreConvention, PersonalityStore,
    // Phase 4: Personality Extensions
    SettingTemplate, SettingTemplateStore, TemplateLoader, GameplayContext,
    BlendRule, BlendRuleStore, PersonalityBlender, GameplayContextDetector,
    SessionStateSnapshot, ContextualPersonalityManager, ContextualPersonalityResult,
    ContextDetectionResult, PersonalityId, BlendRuleId, TemplateId, BlendComponent,
    PersonalityIndexManager, ContextualConfig, BlenderCacheStats, RuleCacheStats,
};
use crate::core::credentials::CredentialManager;
use crate::core::audio::AudioVolumes;
// Claude Gate OAuth client
use crate::claude_gate::{ClaudeClient, FileTokenStorage, TokenInfo};
#[cfg(feature = "keyring")]
use crate::claude_gate::KeyringTokenStorage;
use crate::core::sidecar_manager::{SidecarManager, MeilisearchConfig};
use crate::core::search_client::SearchClient;
use crate::core::meilisearch_pipeline::MeilisearchPipeline;
// Note: DMChatManager used for meilisearch chat operations - temporarily unused
use crate::core::campaign::versioning::VersionManager;
use crate::core::campaign::world_state::WorldStateManager;
use crate::core::campaign::relationships::RelationshipManager;
use crate::core::session::notes::{
    NoteCategory, EntityType as NoteEntityType,
    SessionNote as NoteSessionNote, CategorizationRequest, CategorizationResponse,
    build_categorization_prompt, parse_categorization_response,
};

// Archetype Registry imports
use crate::core::archetype::{
    // Core types
    Archetype, ArchetypeCategory, ArchetypeSummary,
    // Registry
    ArchetypeRegistry,
    // Resolution
    ResolutionQuery, ResolvedArchetype,
    // Setting packs
    SettingPackSummary,
    // Vocabulary
    VocabularyBank, VocabularyBankManager, VocabularyBankSummary,
    PhraseFilterOptions, BankListFilter,
    // Component types
    PersonalityAffinity, NpcRoleMapping, NamingCultureWeight, StatTendencies,
    // Setting pack loader
    SettingPackLoader,
};

fn serialize_enum_to_string<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_default()
}

// ============================================================================
// Claude Gate State (OAuth Client)
// ============================================================================

/// Storage backend type for Claude Gate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClaudeGateStorageBackend {
    /// File-based storage (~/.config/cld/auth.json)
    File,
    /// System keyring storage
    Keyring,
    /// Auto-select (keyring if available, else file)
    Auto,
}

impl Default for ClaudeGateStorageBackend {
    fn default() -> Self {
        Self::Auto
    }
}

impl std::fmt::Display for ClaudeGateStorageBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File => write!(f, "file"),
            Self::Keyring => write!(f, "keyring"),
            Self::Auto => write!(f, "auto"),
        }
    }
}

impl std::str::FromStr for ClaudeGateStorageBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "file" => Ok(Self::File),
            "keyring" => Ok(Self::Keyring),
            "auto" => Ok(Self::Auto),
            _ => Err(format!("Unknown storage backend: {}. Valid options: file, keyring, auto", s)),
        }
    }
}

// ============================================================================
// Claude Gate Client Trait (for type-erased storage backend support)
// ============================================================================

/// Trait for Claude Gate client operations, allowing type-erased storage backends.
#[async_trait::async_trait]
trait ClaudeGateClientOps: Send + Sync {
    async fn is_authenticated(&self) -> Result<bool, String>;
    async fn get_token_info(&self) -> Result<Option<TokenInfo>, String>;
    async fn start_oauth_flow_with_state(&self) -> Result<(String, crate::claude_gate::OAuthFlowState), String>;
    async fn complete_oauth_flow(&self, code: &str, state: Option<&str>) -> Result<TokenInfo, String>;
    async fn logout(&self) -> Result<(), String>;
    async fn list_models(&self) -> Result<Vec<crate::claude_gate::ApiModel>, String>;
    fn storage_name(&self) -> &'static str;
}

/// File storage client wrapper
struct FileStorageClientWrapper {
    client: ClaudeClient<FileTokenStorage>,
}

#[async_trait::async_trait]
impl ClaudeGateClientOps for FileStorageClientWrapper {
    async fn is_authenticated(&self) -> Result<bool, String> {
        self.client.is_authenticated().await.map_err(|e| e.to_string())
    }
    async fn get_token_info(&self) -> Result<Option<TokenInfo>, String> {
        self.client.get_token_info().await.map_err(|e| e.to_string())
    }
    async fn start_oauth_flow_with_state(&self) -> Result<(String, crate::claude_gate::OAuthFlowState), String> {
        self.client.start_oauth_flow_with_state().await.map_err(|e| e.to_string())
    }
    async fn complete_oauth_flow(&self, code: &str, state: Option<&str>) -> Result<TokenInfo, String> {
        self.client.complete_oauth_flow(code, state).await.map_err(|e| e.to_string())
    }
    async fn logout(&self) -> Result<(), String> {
        self.client.logout().await.map_err(|e| e.to_string())
    }
    async fn list_models(&self) -> Result<Vec<crate::claude_gate::ApiModel>, String> {
        self.client.list_models().await.map_err(|e| e.to_string())
    }
    fn storage_name(&self) -> &'static str {
        "file"
    }
}

/// Keyring storage client wrapper
#[cfg(feature = "keyring")]
struct KeyringStorageClientWrapper {
    client: ClaudeClient<KeyringTokenStorage>,
}

#[cfg(feature = "keyring")]
#[async_trait::async_trait]
impl ClaudeGateClientOps for KeyringStorageClientWrapper {
    async fn is_authenticated(&self) -> Result<bool, String> {
        self.client.is_authenticated().await.map_err(|e| e.to_string())
    }
    async fn get_token_info(&self) -> Result<Option<TokenInfo>, String> {
        self.client.get_token_info().await.map_err(|e| e.to_string())
    }
    async fn start_oauth_flow_with_state(&self) -> Result<(String, crate::claude_gate::OAuthFlowState), String> {
        self.client.start_oauth_flow_with_state().await.map_err(|e| e.to_string())
    }
    async fn complete_oauth_flow(&self, code: &str, state: Option<&str>) -> Result<TokenInfo, String> {
        self.client.complete_oauth_flow(code, state).await.map_err(|e| e.to_string())
    }
    async fn logout(&self) -> Result<(), String> {
        self.client.logout().await.map_err(|e| e.to_string())
    }
    async fn list_models(&self) -> Result<Vec<crate::claude_gate::ApiModel>, String> {
        self.client.list_models().await.map_err(|e| e.to_string())
    }
    fn storage_name(&self) -> &'static str {
        "keyring"
    }
}

/// Type-erased Claude Gate client wrapper.
/// This allows storing the client in AppState regardless of storage backend
/// and supports runtime backend switching.
pub struct ClaudeGateState {
    /// The active client (type-erased)
    client: AsyncRwLock<Option<Box<dyn ClaudeGateClientOps>>>,
    /// In-memory flow state for OAuth (needed for state verification)
    pending_oauth_state: AsyncRwLock<Option<String>>,
    /// Current storage backend
    storage_backend: AsyncRwLock<ClaudeGateStorageBackend>,
}

impl ClaudeGateState {
    /// Create a client for the specified backend
    fn create_client(backend: ClaudeGateStorageBackend) -> Result<Box<dyn ClaudeGateClientOps>, String> {
        match backend {
            ClaudeGateStorageBackend::File => {
                let storage = FileTokenStorage::default_path()
                    .map_err(|e| format!("Failed to create file storage: {}", e))?;
                let client = ClaudeClient::builder()
                    .with_storage(storage)
                    .build()
                    .map_err(|e| format!("Failed to create Claude client: {}", e))?;
                Ok(Box::new(FileStorageClientWrapper { client }))
            }
            #[cfg(feature = "keyring")]
            ClaudeGateStorageBackend::Keyring => {
                let storage = KeyringTokenStorage::new();
                let client = ClaudeClient::builder()
                    .with_storage(storage)
                    .build()
                    .map_err(|e| format!("Failed to create Claude client with keyring: {}", e))?;
                Ok(Box::new(KeyringStorageClientWrapper { client }))
            }
            #[cfg(not(feature = "keyring"))]
            ClaudeGateStorageBackend::Keyring => {
                Err("Keyring storage is not available (keyring feature disabled)".to_string())
            }
            ClaudeGateStorageBackend::Auto => {
                // Try keyring first, fall back to file
                #[cfg(feature = "keyring")]
                {
                    match Self::create_client(ClaudeGateStorageBackend::Keyring) {
                        Ok(client) => {
                            log::info!("Auto-selected keyring storage backend");
                            return Ok(client);
                        }
                        Err(e) => {
                            log::warn!("Keyring storage failed, falling back to file: {}", e);
                        }
                    }
                }
                log::info!("Using file storage backend");
                Self::create_client(ClaudeGateStorageBackend::File)
            }
        }
    }

    /// Create a new ClaudeGateState with the specified backend.
    pub fn new(backend: ClaudeGateStorageBackend) -> Result<Self, String> {
        let client = Self::create_client(backend)?;
        Ok(Self {
            client: AsyncRwLock::new(Some(client)),
            pending_oauth_state: AsyncRwLock::new(None),
            storage_backend: AsyncRwLock::new(backend),
        })
    }

    /// Create with default (Auto) backend
    pub fn with_defaults() -> Result<Self, String> {
        Self::new(ClaudeGateStorageBackend::Auto)
    }

    /// Switch to a different storage backend.
    /// This recreates the client with the new backend.
    /// Note: Any existing tokens will not be migrated.
    pub async fn switch_backend(&self, new_backend: ClaudeGateStorageBackend) -> Result<String, String> {
        let new_client = Self::create_client(new_backend)?;
        let backend_name = new_client.storage_name();

        // Replace the client
        {
            let mut client_lock = self.client.write().await;
            *client_lock = Some(new_client);
        }

        // Update the backend setting
        {
            let mut backend_lock = self.storage_backend.write().await;
            *backend_lock = new_backend;
        }

        // Clear any pending OAuth state
        {
            let mut state_lock = self.pending_oauth_state.write().await;
            *state_lock = None;
        }

        log::info!("Switched Claude Gate storage backend to: {}", backend_name);
        Ok(backend_name.to_string())
    }

    /// Check if authenticated
    pub async fn is_authenticated(&self) -> Result<bool, String> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or("Claude Gate client not initialized")?;
        client.is_authenticated().await
    }

    /// Get token info
    pub async fn get_token_info(&self) -> Result<Option<TokenInfo>, String> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or("Claude Gate client not initialized")?;
        client.get_token_info().await
    }

    /// Start OAuth flow
    pub async fn start_oauth_flow(&self) -> Result<(String, String), String> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or("Claude Gate client not initialized")?;
        let (url, state) = client.start_oauth_flow_with_state().await?;

        // Store the state for verification
        *self.pending_oauth_state.write().await = Some(state.state.clone());

        Ok((url, state.state))
    }

    /// Complete OAuth flow
    pub async fn complete_oauth_flow(&self, code: &str, state: Option<&str>) -> Result<TokenInfo, String> {
        // Verify state if provided
        if let Some(received_state) = state {
            let pending = self.pending_oauth_state.read().await;
            if let Some(expected_state) = pending.as_ref() {
                if received_state != expected_state {
                    return Err(format!(
                        "State mismatch: expected {}, got {}",
                        expected_state, received_state
                    ));
                }
            }
        }

        let client = self.client.read().await;
        let client = client.as_ref().ok_or("Claude Gate client not initialized")?;
        let token = client.complete_oauth_flow(code, state).await?;

        // Clear pending state
        *self.pending_oauth_state.write().await = None;

        Ok(token)
    }

    /// Logout
    pub async fn logout(&self) -> Result<(), String> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or("Claude Gate client not initialized")?;
        client.logout().await
    }

    /// Get current storage backend name
    pub async fn storage_backend_name(&self) -> String {
        let client = self.client.read().await;
        if let Some(c) = client.as_ref() {
            c.storage_name().to_string()
        } else {
            self.storage_backend.read().await.to_string()
        }
    }

    /// List available models from Claude API
    pub async fn list_models(&self) -> Result<Vec<crate::claude_gate::ApiModel>, String> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or("Claude Gate client not initialized")?;
        client.list_models().await
    }
}

// ============================================================================
// Application State
// ============================================================================

pub struct AppState {
    // pub database: Option<Database>,
    pub llm_client: RwLock<Option<LLMClient>>,
    pub llm_config: RwLock<Option<LLMConfig>>,
    pub llm_router: AsyncRwLock<LLMRouter>,
    pub llm_manager: Arc<AsyncRwLock<crate::core::llm::LLMManager>>, // TASK-026: Unified LLM Manager
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
    // Campaign management modules (TASK-006, TASK-007, TASK-009)
    pub version_manager: VersionManager,
    pub world_state_manager: WorldStateManager,
    pub relationship_manager: RelationshipManager,
    pub location_manager: crate::core::location_manager::LocationManager,
    // Document extraction settings
    pub extraction_settings: AsyncRwLock<crate::ingestion::ExtractionSettings>,
    // Claude Gate OAuth client
    pub claude_gate: Arc<ClaudeGateState>,
    // Archetype Registry for unified character archetype management
    // Wrapped in AsyncRwLock<Option<_>> for lazy initialization after Meilisearch starts
    pub archetype_registry: AsyncRwLock<Option<Arc<ArchetypeRegistry>>>,
    // Vocabulary Bank Manager for NPC dialogue phrase management
    // Wrapped in AsyncRwLock<Option<_>> for lazy initialization after Meilisearch starts
    pub vocabulary_manager: AsyncRwLock<Option<Arc<VocabularyBankManager>>>,
    // Setting Pack Loader for campaign setting customization
    pub setting_pack_loader: Arc<SettingPackLoader>,
    // Phase 4: Personality Extensions
    pub template_store: Arc<SettingTemplateStore>,
    pub blend_rule_store: Arc<BlendRuleStore>,
    pub personality_blender: Arc<PersonalityBlender>,
    pub contextual_personality_manager: Arc<ContextualPersonalityManager>,
}

// Helper init for default state components
impl AppState {
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
        Arc<ClaudeGateState>,
        Arc<SettingPackLoader>,
        // Phase 4: Personality Extensions
        Arc<SettingTemplateStore>,
        Arc<BlendRuleStore>,
        Arc<PersonalityBlender>,
        Arc<ContextualPersonalityManager>,
    ) {
        let sidecar_config = MeilisearchConfig::default();
        let search_client = SearchClient::new(
            &sidecar_config.url(),
            Some(&sidecar_config.master_key),
        );
        let personality_store = Arc::new(PersonalityStore::new());
        let personality_manager = Arc::new(PersonalityApplicationManager::new(personality_store.clone()));

        // Initialize Claude Gate client (fallback to file storage if keyring unavailable)
        let claude_gate = match ClaudeGateState::with_defaults() {
            Ok(state) => Arc::new(state),
            Err(e) => {
                log::warn!("Failed to initialize Claude Gate with default storage: {}. Using file storage.", e);
                Arc::new(ClaudeGateState::new(ClaudeGateStorageBackend::File)
                    .expect("Failed to initialize Claude Gate with file storage"))
            }
        };

        // Phase 4: Initialize Personality Extension components
        let meilisearch_url = sidecar_config.url();
        let meilisearch_key = sidecar_config.master_key.clone();

        // Create shared index manager for personality extensions
        let personality_index_manager = Arc::new(PersonalityIndexManager::new(
            &meilisearch_url,
            Some(&meilisearch_key),
        ));

        // Create template store using from_manager (synchronous)
        let template_store = Arc::new(SettingTemplateStore::from_manager(
            personality_index_manager.clone()
        ));

        // Create blend rule store (reuses shared index manager)
        let blend_rule_store = Arc::new(BlendRuleStore::new(
            personality_index_manager.clone()
        ));

        // Create personality blender and contextual manager
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
            claude_gate,
            Arc::new(SettingPackLoader::new()),
            // Phase 4: Personality Extensions
            template_store,
            blend_rule_store,
            personality_blender,
            contextual_personality_manager,
        )
    }
}

// ============================================================================
// Database Helper Macro
// ============================================================================

/// Helper macro to reduce boilerplate for database access in Tauri commands.
///
/// Usage:
/// ```ignore
/// with_db!(db, |db| db.some_method(&arg1, &arg2))
/// ```
macro_rules! with_db {
    ($db_state:expr, |$db:ident| $body:expr) => {{
        let db_guard = $db_state.read().await;
        let $db = db_guard.as_ref().ok_or("Database not initialized")?;
        $body.await.map_err(|e| e.to_string())
    }};
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequestPayload {
    pub message: String,
    pub system_prompt: Option<String>,
    pub personality_id: Option<String>,
    pub context: Option<Vec<String>>,
    /// Enable RAG mode to route through Meilisearch Chat
    #[serde(default)]
    pub use_rag: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponsePayload {
    pub content: String,
    pub model: String,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LLMSettings {
    pub provider: String,
    pub api_key: Option<String>,
    pub host: Option<String>,
    pub model: String,
    pub embedding_model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub provider: String,
    pub healthy: bool,
    pub message: String,
}

// ============================================================================
// LLM Commands
// ============================================================================

/// Providers that can auto-detect or have default models, so model selection is optional
const PROVIDERS_WITH_OPTIONAL_MODEL: &[&str] = &[];


fn get_config_path(app_handle: &tauri::AppHandle) -> PathBuf {
    // Ensure app data dir exists
    let dir = app_handle.path().app_data_dir().unwrap_or(PathBuf::from("."));
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    dir.join("llm_config.json")
}

pub fn load_llm_config_disk(app_handle: &tauri::AppHandle) -> Option<LLMConfig> {
    let path = get_config_path(app_handle);
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(path) {
            return serde_json::from_str(&content).ok();
        }
    }
    None
}

fn save_llm_config_disk(app_handle: &tauri::AppHandle, config: &LLMConfig) {
    let path = get_config_path(app_handle);
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(path, json);
    }
}

// Voice config persistence
fn get_voice_config_path(app_handle: &tauri::AppHandle) -> PathBuf {
    let dir = app_handle.path().app_data_dir().unwrap_or(PathBuf::from("."));
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    dir.join("voice_config.json")
}

pub fn load_voice_config_disk(app_handle: &tauri::AppHandle) -> Option<VoiceConfig> {
    let path = get_voice_config_path(app_handle);
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            return serde_json::from_str(&content).ok();
        }
    }
    None
}

fn save_voice_config_disk(app_handle: &tauri::AppHandle, config: &VoiceConfig) {
    let path = get_voice_config_path(app_handle);
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(path, json);
    }
}

#[tauri::command]
pub async fn configure_llm(
    settings: LLMSettings,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // Validate model is not empty (except for providers that support auto-detection)
    let model_optional = PROVIDERS_WITH_OPTIONAL_MODEL.contains(&settings.provider.as_str());
    if settings.model.trim().is_empty() && !model_optional {
        return Err("Model name is required. Please select a model.".to_string());
    }

    let config = match settings.provider.as_str() {
        "ollama" => LLMConfig::Ollama {
            host: settings.host.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model: settings.model,
        },
        "gemini" => LLMConfig::Gemini {
            api_key: settings.api_key.clone().ok_or("Gemini requires an API key")?,
            model: settings.model,
        },
        "openai" => LLMConfig::OpenAI {
            api_key: settings.api_key.clone().ok_or("OpenAI requires an API key")?,
            model: settings.model,
            max_tokens: 4096,
            organization_id: None,
            base_url: Some("https://api.openai.com/v1".to_string()),
        },
        "openrouter" => LLMConfig::OpenRouter {
            api_key: settings.api_key.clone().ok_or("OpenRouter requires an API key")?,
            model: settings.model,
        },
        "mistral" => LLMConfig::Mistral {
            api_key: settings.api_key.clone().ok_or("Mistral requires an API key")?,
            model: settings.model,
        },
        "groq" => LLMConfig::Groq {
            api_key: settings.api_key.clone().ok_or("Groq requires an API key")?,
            model: settings.model,
        },
        "together" => LLMConfig::Together {
            api_key: settings.api_key.clone().ok_or("Together requires an API key")?,
            model: settings.model,
        },
        "cohere" => LLMConfig::Cohere {
            api_key: settings.api_key.clone().ok_or("Cohere requires an API key")?,
            model: settings.model,
        },
        "deepseek" => LLMConfig::DeepSeek {
            api_key: settings.api_key.clone().ok_or("DeepSeek requires an API key")?,
            model: settings.model,
        },
        "claude" => LLMConfig::Claude {
            storage_backend: "auto".to_string(), // Will use configured backend from AppState
            model: settings.model,
            max_tokens: 8192, // Default max tokens
        },
        _ => return Err(format!("Unknown provider: {}", settings.provider)),
    };

    // Store API key securely if provided
    if let Some(api_key) = &settings.api_key {
        let key_name = format!("{}_api_key", settings.provider);
        let _ = state.credentials.store_secret(&key_name, api_key);
    }

    let client = LLMClient::new(config.clone());
    let provider_name = client.provider_name().to_string();

    // Get the previous provider name before overwriting config
    let prev_provider = state.llm_config.read().unwrap()
        .as_ref()
        .map(|c| LLMClient::new(c.clone()).provider_name().to_string());

    *state.llm_config.write().unwrap() = Some(config.clone());

    // Persist to disk
    save_llm_config_disk(&app_handle, &config);

    // Update Router: remove old provider if different, then add new one
    {
        let mut router = state.llm_router.write().await;
        if let Some(ref prev) = prev_provider {
            if prev != &provider_name {
                router.remove_provider(prev).await;
            }
        }
        router.remove_provider(&provider_name).await;

        let provider = config.create_provider();
        router.add_provider(provider).await;
    }


    Ok(format!("Configured {} provider successfully", provider_name))
}

#[tauri::command]
pub async fn get_router_stats(state: State<'_, AppState>) -> Result<HashMap<String, ProviderStats>, String> {
    Ok(state.llm_router.read().await.get_all_stats().await)
}

#[tauri::command]
pub async fn chat(
    payload: ChatRequestPayload,
    state: State<'_, AppState>,
) -> Result<ChatResponsePayload, String> {
    // Get configuration
    let config = state.llm_config.read().unwrap()
        .clone()
        .ok_or("LLM not configured. Please configure in Settings.")?;

    // Determine effective system prompt
    let system_prompt = if let Some(pid) = &payload.personality_id {
        match state.personality_store.get(pid) {
            Ok(profile) => profile.to_system_prompt(),
            Err(_) => payload.system_prompt.clone().unwrap_or_else(|| {
                "You are a helpful TTRPG Game Master assistant.".to_string()
            })
        }
    } else {
        payload.system_prompt.clone().unwrap_or_else(|| {
            "You are a helpful TTRPG Game Master assistant. Help the user with their tabletop RPG questions, \
             provide rules clarifications, generate content, and assist with running their campaign.".to_string()
        })
    };

    // Use unified LLM Manager using Meilisearch Chat (RAG-enabled)
    let manager = state.llm_manager.clone();

    // Ensure chat client is configured
    {
        let manager_guard = manager.write().await;
        manager_guard.set_chat_client(state.search_client.host(), Some(&state.sidecar_manager.config().master_key)).await;
    }

    // Prepare messages
    let mut messages = vec![];
    if let Some(context) = &payload.context {
        for ctx in context {
            messages.push(ChatMessage {
                role: MessageRole::User,
                content: ctx.clone(),
                images: None,
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }
    }
    messages.push(ChatMessage {
        role: MessageRole::User,
        content: payload.message,
        images: None,
        name: None,
        tool_calls: None,
        tool_call_id: None,
    });

    // Determine model name
    let model = match &config {
        LLMConfig::OpenAI { model, .. } => model.clone(),
        LLMConfig::Claude { model, .. } => model.clone(),
        LLMConfig::Gemini { model, .. } => model.clone(),
        LLMConfig::OpenRouter { model, .. } => model.clone(),
        LLMConfig::Mistral { model, .. } => model.clone(),
        LLMConfig::Groq { model, .. } => model.clone(),
        LLMConfig::Together { model, .. } => model.clone(),
        LLMConfig::Cohere { model, .. } => model.clone(),
        LLMConfig::DeepSeek { model, .. } => model.clone(),
        LLMConfig::Ollama { model, .. } => model.clone(),
        LLMConfig::Claude { model, .. } => model.clone(),
        LLMConfig::Meilisearch { model, .. } => model.clone(),
    };

    // Send chat request
    let manager_guard = manager.read().await;
    let content = manager_guard.chat(messages, &model).await
        .map_err(|e| format!("Chat failed: {}", e))?;

    Ok(ChatResponsePayload {
        content,
        model,
        input_tokens: None, // Meilisearch usage stats passed through would be nice but optional
        output_tokens: None,
    })
}

#[tauri::command]
pub async fn check_llm_health(state: State<'_, AppState>) -> Result<HealthStatus, String> {
    println!("DEBUG: check_llm_health called");
    let config_opt = state.llm_config.read().unwrap().clone();

    match config_opt {
        Some(config) => {
            let client = LLMClient::new(config);
            let provider = client.provider_name().to_string();

            match client.health_check().await {
                Ok(healthy) => Ok(HealthStatus {
                    provider: provider.clone(),
                    healthy,
                    message: if healthy {
                        format!("{} is available", provider)
                    } else {
                        format!("{} is not responding", provider)
                    },
                }),
                Err(e) => Ok(HealthStatus {
                    provider,
                    healthy: false,
                    message: e.to_string(),
                }),
            }
        }
        None => Ok(HealthStatus {
            provider: "none".to_string(),
            healthy: false,
            message: "No LLM configured".to_string(),
        }),
    }
}

#[tauri::command]
pub fn get_llm_config(state: State<'_, AppState>) -> Result<Option<LLMSettings>, String> {
    let config = state.llm_config.read().unwrap();

    Ok(config.as_ref().map(|c| match c {
        LLMConfig::Ollama { host, model } => LLMSettings {
            provider: "ollama".to_string(),
            api_key: None,
            host: Some(host.clone()),
            model: model.clone(),
            embedding_model: None,
        },
        LLMConfig::Claude { model, .. } => LLMSettings {
            provider: "claude".to_string(),
            api_key: Some("********".to_string()),
            host: None,
            model: model.clone(),
            embedding_model: None,
        },
        LLMConfig::Gemini { model, .. } => LLMSettings {
            provider: "gemini".to_string(),
            api_key: Some("********".to_string()),
            host: None,
            model: model.clone(),
            embedding_model: None,
        },
        LLMConfig::OpenAI { model, .. } => LLMSettings {
            provider: "openai".to_string(),
            api_key: Some("********".to_string()),
            host: None,
            model: model.clone(),
            embedding_model: None,
        },
        LLMConfig::OpenRouter { model, .. } => LLMSettings {
            provider: "openrouter".to_string(),
            api_key: Some("********".to_string()),
            host: None,
            model: model.clone(),
            embedding_model: None,
        },
        LLMConfig::Mistral { model, .. } => LLMSettings {
            provider: "mistral".to_string(),
            api_key: Some("********".to_string()),
            host: None,
            model: model.clone(),
            embedding_model: None,
        },
        LLMConfig::Groq { model, .. } => LLMSettings {
            provider: "groq".to_string(),
            api_key: Some("********".to_string()),
            host: None,
            model: model.clone(),
            embedding_model: None,
        },
        LLMConfig::Together { model, .. } => LLMSettings {
            provider: "together".to_string(),
            api_key: Some("********".to_string()),
            host: None,
            model: model.clone(),
            embedding_model: None,
        },
        LLMConfig::Cohere { model, .. } => LLMSettings {
            provider: "cohere".to_string(),
            api_key: Some("********".to_string()),
            host: None,
            model: model.clone(),
            embedding_model: None,
        },
        LLMConfig::DeepSeek { model, .. } => LLMSettings {
            provider: "deepseek".to_string(),
            api_key: Some("********".to_string()),
            host: None,
            model: model.clone(),
            embedding_model: None,
        },
        LLMConfig::Claude { model, .. } => LLMSettings {
            provider: "claude".to_string(),
            api_key: None, // No API key needed - uses OAuth
            host: None,
            model: model.clone(),
            embedding_model: None,
        },
        LLMConfig::Meilisearch { host, model, .. } => LLMSettings {
            provider: "meilisearch".to_string(),
            api_key: None,
            host: Some(host.clone()),
            model: model.clone(),
            embedding_model: None,
        },
    }))
}

/// List available models from an Ollama instance
#[tauri::command]
pub async fn list_ollama_models(host: String) -> Result<Vec<crate::core::llm::OllamaModel>, String> {
    crate::core::llm::LLMClient::list_ollama_models(&host)
        .await
        .map_err(|e| e.to_string())
}

/// List available Claude models (with fallback)
#[tauri::command]
pub async fn list_claude_models(api_key: Option<String>) -> Result<Vec<crate::core::llm::ModelInfo>, String> {
    if let Some(key) = api_key {
        if !key.is_empty() && !key.starts_with("*") {
            match crate::core::llm::LLMClient::list_claude_models(&key).await {
                Ok(models) if !models.is_empty() => return Ok(models),
                _ => {} // Fall through to fallback
            }
        }
    }
    Ok(crate::core::llm::get_fallback_models("claude"))
}

/// List available OpenAI models (with fallback)
#[tauri::command]
pub async fn list_openai_models(api_key: Option<String>) -> Result<Vec<crate::core::llm::ModelInfo>, String> {
    // First try OpenAI API if we have a valid key
    if let Some(key) = api_key {
        if !key.is_empty() && !key.starts_with("*") {
            match crate::core::llm::LLMClient::list_openai_models(&key, None).await {
                Ok(models) if !models.is_empty() => return Ok(models),
                _ => {} // Fall through to GitHub fallback
            }
        }
    }

    // Second try: fetch from GitHub community list
    match crate::core::llm::LLMClient::fetch_openai_models_from_github().await {
        Ok(models) if !models.is_empty() => return Ok(models),
        _ => {} // Fall through to hardcoded fallback
    }

    // Final fallback: hardcoded list
    Ok(crate::core::llm::get_fallback_models("openai"))
}

/// List available Gemini models (with fallback)
#[tauri::command]
pub async fn list_gemini_models(api_key: Option<String>) -> Result<Vec<crate::core::llm::ModelInfo>, String> {
    if let Some(key) = api_key {
        if !key.is_empty() && !key.starts_with("*") {
            match crate::core::llm::LLMClient::list_gemini_models(&key).await {
                Ok(models) if !models.is_empty() => return Ok(models),
                _ => {} // Fall through to fallback
            }
        }
    }
    Ok(crate::core::llm::get_fallback_models("gemini"))
}

/// List available OpenRouter models (no auth required - uses public API)
#[tauri::command]
pub async fn list_openrouter_models() -> Result<Vec<crate::core::llm::ModelInfo>, String> {
    // OpenRouter has a public models endpoint
    match crate::core::llm::fetch_openrouter_models().await {
        Ok(models) => Ok(models.into_iter().map(|m| m.into()).collect()),
        Err(_) => Ok(crate::core::llm::get_extended_fallback_models("openrouter")),
    }
}

/// List available models for any provider via LiteLLM catalog
#[tauri::command]
pub async fn list_provider_models(provider: String) -> Result<Vec<crate::core::llm::ModelInfo>, String> {
    // First try LiteLLM catalog (comprehensive, no auth)
    match crate::core::llm::fetch_litellm_models_for_provider(&provider).await {
        Ok(models) if !models.is_empty() => return Ok(models),
        _ => {} // Fall through
    }
    // Fallback to extended hardcoded list
    Ok(crate::core::llm::get_extended_fallback_models(&provider))
}

// ============================================================================
// LLM Router Commands
// ============================================================================

use crate::core::llm::{
    ChatChunk, CostSummary, ProviderHealth, RoutingStrategy,
};

/// Get health status of all providers
#[tauri::command]
pub async fn get_router_health(
    state: State<'_, AppState>,
) -> Result<HashMap<String, ProviderHealth>, String> {
    let router = state.llm_router.read().await.clone();
    Ok(router.get_all_health().await)
}

/// Get cost summary for the router
#[tauri::command]
pub async fn get_router_costs(
    state: State<'_, AppState>,
) -> Result<CostSummary, String> {
    let router = state.llm_router.read().await.clone();
    Ok(router.get_cost_summary().await)
}

/// Estimate cost for a request
#[tauri::command]
pub async fn estimate_request_cost(
    provider: String,
    model: String,
    input_tokens: u32,
    output_tokens: u32,
    state: State<'_, AppState>,
) -> Result<f64, String> {
    let router = state.llm_router.read().await.clone();
    Ok(router.estimate_cost(&provider, &model, input_tokens, output_tokens).await)
}

/// Get list of healthy providers
#[tauri::command]
pub async fn get_healthy_providers(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let router = state.llm_router.read().await.clone();
    Ok(router.healthy_providers().await)
}

/// Set the routing strategy
#[tauri::command]
pub async fn set_routing_strategy(
    strategy: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let strategy = match strategy.to_lowercase().as_str() {
        "priority" => RoutingStrategy::Priority,
        "cost" | "cost_optimized" | "costoptimized" => RoutingStrategy::CostOptimized,
        "latency" | "latency_optimized" | "latencyoptimized" => RoutingStrategy::LatencyOptimized,
        "round_robin" | "roundrobin" => RoutingStrategy::RoundRobin,
        _ => return Err(format!("Unknown routing strategy: {}", strategy)),
    };

    let mut router = state.llm_router.write().await;
    router.set_routing_strategy(strategy);
    Ok(())
}

/// Run health checks on all providers
#[tauri::command]
pub async fn run_provider_health_checks(
    state: State<'_, AppState>,
) -> Result<HashMap<String, bool>, String> {
    let router = state.llm_router.read().await;
    let router_clone = router.clone();
    drop(router); // Release the lock before async operation

    Ok(router_clone.health_check_all().await)
}

/// Stream chat response - emits 'chat-chunk' events as chunks arrive
#[tauri::command]
pub async fn stream_chat(
    app_handle: tauri::AppHandle,
    messages: Vec<ChatMessage>,
    system_prompt: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    provided_stream_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    use tauri::Emitter;

    log::info!("[stream_chat] Starting with {} messages", messages.len());

    let config = state.llm_config.read().unwrap()
        .clone()
        .ok_or("LLM not configured. Please configure in Settings.")?;

    // Determine model name from config (same logic as chat command)
    let model = config.model_name();

    // Use provided stream ID or generate a new one
    let stream_id = provided_stream_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let stream_id_clone = stream_id.clone();
    log::info!("[stream_chat] Using stream_id: {}", stream_id);

    // Get the Meilisearch chat manager
    let manager = state.llm_manager.clone();

    // Ensure properly configured for this provider (Just like chat command)
    {
        let manager_guard = manager.write().await;
        // Ensure chat client is configured (uses Meilisearch host from search_client)
        manager_guard.set_chat_client(state.search_client.host(), Some(&state.sidecar_manager.config().master_key)).await;
    }

    let manager_guard = manager.read().await;

    // Initiate the stream via Meilisearch manager (enables RAG)
    let mut rx = manager_guard.chat_stream(messages, &model, temperature, max_tokens).await
        .map_err(|e| e.to_string())?;

    // Spawn a task to handle the stream asynchronously
    tokio::spawn(async move {
        log::info!("[stream_chat:{}] Receiver task started", stream_id_clone);
        let mut chunk_count = 0;
        let mut total_bytes = 0;

        // Process chunks and emit events
        while let Some(chunk_result) = rx.recv().await {
            match chunk_result {
                Ok(content) => {
                     // Check for "[DONE]" marker if it wasn't handled by the client
                    if content == "[DONE]" {
                        log::info!("[stream_chat:{}] Received [DONE], stream finished. Total chunks: {}, Total bytes: {}", stream_id_clone, chunk_count, total_bytes);
                        break;
                    }

                    chunk_count += 1;
                    total_bytes += content.len();

                    let chunk = ChatChunk {
                        stream_id: stream_id_clone.clone(),
                        content,
                        provider: String::new(),
                        model: String::new(),
                        is_final: false,
                        finish_reason: None,
                        usage: None,
                        index: chunk_count,
                    };

                    // Emit the chunk event
                    if let Err(e) = app_handle.emit("chat-chunk", &chunk) {
                        log::error!("[stream_chat:{}] Failed to emit chunk: {}", stream_id_clone, e);
                        break;
                    }
                }
                Err(e) => {
                    let error_message = format!("Error: {}", e);
                    log::error!("[stream_chat:{}] Stream error: {}", stream_id_clone, error_message);

                    // Emit error event
                    let error_chunk = ChatChunk {
                        stream_id: stream_id_clone.clone(),
                        content: error_message,
                        provider: String::new(),
                        model: String::new(),
                        is_final: true,
                        finish_reason: Some("error".to_string()),
                        usage: None,
                        index: chunk_count + 1,
                    };
                    let _ = app_handle.emit("chat-chunk", &error_chunk);
                    break;
                }
            }
        }
        log::info!("[stream_chat:{}] Receiver task exiting", stream_id_clone);

        // Emit final chunk to signal completion
        let final_chunk = ChatChunk {
            stream_id: stream_id_clone.clone(),
            content: String::new(),
            provider: String::new(),
            model: String::new(),
            is_final: true,
            finish_reason: Some("stop".to_string()),
            usage: None, // Usage not available from simple stream yet
            index: 0,
        };
        let _ = app_handle.emit("chat-chunk", &final_chunk);
    });


    Ok(stream_id)
}

/// Cancel an active stream
#[tauri::command]
pub async fn cancel_stream(
    stream_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let router = state.llm_router.read().await.clone();
    Ok(router.cancel_stream(&stream_id).await)
}

/// Get list of active stream IDs
#[tauri::command]
pub async fn get_active_streams(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let router = state.llm_router.read().await.clone();
    Ok(router.active_stream_ids().await)
}

// ============================================================================
// Document Ingestion Commands
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct IngestOptions {
    /// Source type: "rules", "fiction", "document", etc.
    #[serde(default = "default_source_type")]
    pub source_type: String,
    /// Campaign ID to associate with
    pub campaign_id: Option<String>,
}

fn default_source_type() -> String {
    "document".to_string()
}

#[tauri::command]
pub async fn ingest_document(
    path: String,
    options: Option<IngestOptions>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let path_obj = Path::new(&path);
    if !path_obj.exists() {
        return Err(format!("File not found: {}", path));
    }

    let opts = options.unwrap_or(IngestOptions {
        source_type: "document".to_string(),
        campaign_id: None,
    });

    // Use Meilisearch pipeline for ingestion
    let result = state.ingestion_pipeline
        .process_file(
            &state.search_client,
            path_obj,
            &opts.source_type,
            opts.campaign_id.as_deref(),
        )
        .await
        .map_err(|e| format!("Ingestion failed: {}", e))?;

    Ok(format!(
        "Ingested '{}': {} chunks into '{}' index",
        result.source, result.stored_chunks, result.index_used
    ))
}

// ============================================================================
// Two-Phase Ingestion (Per-Document Indexes)
// ============================================================================

/// Result of two-phase document ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoPhaseIngestResult {
    /// Generated slug for this source (used as index name base)
    pub slug: String,
    /// Human-readable source name
    pub source_name: String,
    /// Index containing raw pages
    pub raw_index: String,
    /// Index containing semantic chunks
    pub chunks_index: String,
    /// Number of pages extracted
    pub page_count: usize,
    /// Number of semantic chunks created
    pub chunk_count: usize,
    /// Total characters extracted
    pub total_chars: usize,
    /// Detected game system (if any)
    pub game_system: Option<String>,
    /// Detected content category
    pub content_category: Option<String>,
}

/// Ingest a document using two-phase pipeline with per-document indexes.
///
/// Phase 1: Extract pages to `<slug>-raw` index (one doc per page)
/// Phase 2: Create semantic chunks in `<slug>` index with provenance tracking
///
/// This enables page number attribution in search results by tracking
/// which raw pages each chunk was derived from.
#[tauri::command]
pub async fn ingest_document_two_phase(
    app: tauri::AppHandle,
    path: String,
    title_override: Option<String>,
    state: State<'_, AppState>,
) -> Result<TwoPhaseIngestResult, String> {
    use tauri::Emitter;
    use crate::core::meilisearch_pipeline::{MeilisearchPipeline, generate_source_slug};

    let path_buf = std::path::PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("File not found: {}", path));
    }

    let source_name = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Generate slug for progress messages
    let slug = generate_source_slug(&path_buf, title_override.as_deref());

    log::info!("Starting two-phase ingestion for '{}' (slug: {})", source_name, slug);

    // Emit initial progress
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "starting".to_string(),
        progress: 0.0,
        message: format!("Starting two-phase ingestion for {}...", source_name),
        source_name: source_name.clone(),
    });

    let pipeline = MeilisearchPipeline::with_defaults();

    // Phase 1: Extract to raw pages
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "extracting".to_string(),
        progress: 0.1,
        message: format!("Phase 1: Extracting pages from {}...", source_name),
        source_name: source_name.clone(),
    });

    let extraction = pipeline
        .extract_to_raw(&state.search_client, &path_buf, title_override.as_deref())
        .await
        .map_err(|e| format!("Extraction failed: {}", e))?;

    log::info!(
        "Phase 1 complete: {} pages extracted to '{}' (system: {:?})",
        extraction.page_count,
        extraction.raw_index,
        extraction.ttrpg_metadata.game_system
    );

    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "extracted".to_string(),
        progress: 0.5,
        message: format!("Extracted {} pages, creating semantic chunks...", extraction.page_count),
        source_name: source_name.clone(),
    });

    // Phase 2: Create semantic chunks
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "chunking".to_string(),
        progress: 0.6,
        message: format!("Phase 2: Creating semantic chunks for {}...", source_name),
        source_name: source_name.clone(),
    });

    let chunking = pipeline
        .chunk_from_raw(&state.search_client, &extraction)
        .await
        .map_err(|e| format!("Chunking failed: {}", e))?;

    log::info!(
        "Phase 2 complete: {} chunks created in '{}' from {} pages",
        chunking.chunk_count,
        chunking.chunks_index,
        chunking.pages_consumed
    );

    // Done!
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "complete".to_string(),
        progress: 1.0,
        message: format!(
            "Ingested {} pages → {} chunks (indexes: {}, {})",
            extraction.page_count,
            chunking.chunk_count,
            extraction.raw_index,
            chunking.chunks_index
        ),
        source_name: source_name.clone(),
    });

    Ok(TwoPhaseIngestResult {
        slug: extraction.slug,
        source_name: extraction.source_name,
        raw_index: extraction.raw_index,
        chunks_index: chunking.chunks_index,
        page_count: extraction.page_count,
        chunk_count: chunking.chunk_count,
        total_chars: extraction.total_chars,
        game_system: extraction.ttrpg_metadata.game_system,
        content_category: extraction.ttrpg_metadata.content_category,
    })
}

// ============================================================================
// Search Commands
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Maximum results to return
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Source type filter
    pub source_type: Option<String>,
    /// Campaign ID filter
    pub campaign_id: Option<String>,
    /// Search specific index only
    pub index: Option<String>,
}

fn default_limit() -> usize {
    10
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResultPayload {
    pub content: String,
    pub source: String,
    pub source_type: String,
    pub page_number: Option<u32>,
    pub score: f32,
    pub index: String,
}

#[tauri::command]
pub async fn search(
    query: String,
    options: Option<SearchOptions>,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResultPayload>, String> {
    let opts = options.unwrap_or(SearchOptions {
        limit: 10,
        source_type: None,
        campaign_id: None,
        index: None,
    });

    // Build filter if needed
    let filter = match (&opts.source_type, &opts.campaign_id) {
        (Some(st), Some(cid)) => Some(format!("source_type = '{}' AND campaign_id = '{}'", st, cid)),
        (Some(st), None) => Some(format!("source_type = '{}'", st)),
        (None, Some(cid)) => Some(format!("campaign_id = '{}'", cid)),
        (None, None) => None,
    };

    let results = if let Some(index_name) = &opts.index {
        // Search specific index
        state.search_client
            .search(index_name, &query, opts.limit, filter.as_deref())
            .await
            .map_err(|e| format!("Search failed: {}", e))?
    } else {
        // Federated search across all content indexes
        let federated = state.search_client
            .search_all(&query, opts.limit)
            .await
            .map_err(|e| format!("Search failed: {}", e))?;
        federated.results
    };

    // Format results
    let formatted: Vec<SearchResultPayload> = results
        .into_iter()
        .map(|r| SearchResultPayload {
            content: r.document.content,
            source: r.document.source,
            source_type: r.document.source_type,
            page_number: r.document.page_number,
            score: r.score,
            index: r.index,
        })
        .collect();

    Ok(formatted)
}

// ============================================================================
// Hybrid Search Commands
// ============================================================================

use crate::core::search::{
    HybridSearchEngine, HybridConfig,
    hybrid::HybridSearchOptions as CoreHybridSearchOptions,
};

/// Options for hybrid search
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct HybridSearchOptions {
    /// Maximum results to return
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Source type filter
    pub source_type: Option<String>,
    /// Campaign ID filter
    pub campaign_id: Option<String>,
    /// Index to search (None = federated search)
    pub index: Option<String>,
    /// Override semantic weight (0.0 - 1.0)
    pub semantic_weight: Option<f32>,
    /// Override keyword weight (0.0 - 1.0)
    pub keyword_weight: Option<f32>,
    /// Fusion strategy preset: "balanced", "keyword_heavy", "semantic_heavy", etc.
    pub fusion_strategy: Option<String>,
    /// Enable/disable query expansion (default: true)
    pub query_expansion: Option<bool>,
    /// Enable/disable spell correction (default: true)
    pub spell_correction: Option<bool>,
}

/// Hybrid search result for frontend
#[derive(Debug, Serialize, Deserialize)]
pub struct HybridSearchResultPayload {
    pub content: String,
    pub source: String,
    pub source_type: String,
    pub page_number: Option<u32>,
    pub score: f32,
    pub index: String,
    pub keyword_rank: Option<usize>,
    pub semantic_rank: Option<usize>,
    /// Number of search methods that found this result (1 = single, 2 = both)
    pub overlap_count: Option<usize>,
}

/// Hybrid search response for frontend
#[derive(Debug, Serialize, Deserialize)]
pub struct HybridSearchResponsePayload {
    pub results: Vec<HybridSearchResultPayload>,
    pub total_hits: usize,
    pub original_query: String,
    pub expanded_query: Option<String>,
    pub corrected_query: Option<String>,
    pub processing_time_ms: u64,
    pub hints: Vec<String>,
    /// Whether performance target was met (<500ms)
    pub within_target: bool,
}

/// Perform hybrid search with RRF fusion
///
/// Combines keyword (Meilisearch BM25) and semantic (vector similarity) search
/// using Reciprocal Rank Fusion (RRF) for optimal ranking.
///
/// # Arguments
/// * `query` - The search query string
/// * `options` - Optional search configuration
/// * `state` - Application state containing search client
///
/// # Returns
/// Search results with RRF-fused scores, timing, and query enhancement info
#[tauri::command]
pub async fn hybrid_search(
    query: String,
    options: Option<HybridSearchOptions>,
    state: State<'_, AppState>,
) -> Result<HybridSearchResponsePayload, String> {
    let opts = options.unwrap_or_default();

    // Build hybrid config from options
    let mut config = HybridConfig::default();

    // Apply fusion strategy if specified
    if let Some(strategy) = &opts.fusion_strategy {
        config.fusion_strategy = Some(strategy.clone());
    }

    // Apply query expansion setting
    if let Some(expand) = opts.query_expansion {
        config.query_expansion = expand;
    }

    // Apply spell correction setting
    if let Some(correct) = opts.spell_correction {
        config.spell_correction = correct;
    }

    // Create hybrid search engine with configured options
    let engine = HybridSearchEngine::new(
        state.search_client.clone(),
        None, // Embedding provider - use Meilisearch's built-in for now
        config,
    );

    // Convert options to core search options
    let search_options = CoreHybridSearchOptions {
        limit: opts.limit,
        source_type: opts.source_type,
        campaign_id: opts.campaign_id,
        index: opts.index,
        semantic_weight: opts.semantic_weight,
        keyword_weight: opts.keyword_weight,
    };

    // Perform search
    let response = engine
        .search(&query, search_options)
        .await
        .map_err(|e| format!("Hybrid search failed: {}", e))?;

    // Determine overlap count for each result
    let results: Vec<HybridSearchResultPayload> = response
        .results
        .into_iter()
        .map(|r| {
            let overlap_count = match (r.keyword_rank.is_some(), r.semantic_rank.is_some()) {
                (true, true) => Some(2),
                (true, false) | (false, true) => Some(1),
                (false, false) => None,
            };

            HybridSearchResultPayload {
                content: r.document.content,
                source: r.document.source,
                source_type: r.document.source_type,
                page_number: r.document.page_number,
                score: r.score,
                index: r.index,
                keyword_rank: r.keyword_rank,
                semantic_rank: r.semantic_rank,
                overlap_count,
            }
        })
        .collect();

    // Check if within performance target
    let within_target = response.processing_time_ms < 500;

    Ok(HybridSearchResponsePayload {
        results,
        total_hits: response.total_hits,
        original_query: response.original_query,
        expanded_query: response.expanded_query,
        corrected_query: response.corrected_query,
        processing_time_ms: response.processing_time_ms,
        hints: response.hints,
        within_target,
    })
}

/// Get search suggestions for autocomplete
#[tauri::command]
pub fn get_search_suggestions(
    partial: String,
    state: State<'_, AppState>,
) -> Vec<String> {
    let engine = HybridSearchEngine::with_defaults(state.search_client.clone());
    engine.suggest(&partial)
}

/// Get search hints for a query
#[tauri::command]
pub fn get_search_hints(
    query: String,
    state: State<'_, AppState>,
) -> Vec<String> {
    let engine = HybridSearchEngine::with_defaults(state.search_client.clone());
    engine.get_hints(&query)
}

/// Expand a query with TTRPG synonyms
#[tauri::command]
pub fn expand_query(query: String) -> crate::core::search::synonyms::QueryExpansionResult {
    let synonyms = crate::core::search::TTRPGSynonyms::new();
    synonyms.expand_query(&query)
}

/// Correct spelling in a query
#[tauri::command]
pub fn correct_query(query: String) -> crate::core::spell_correction::CorrectionResult {
    let corrector = crate::core::spell_correction::SpellCorrector::new();
    corrector.correct(&query)
}

// ============================================================================
// Voice Configuration Commands
// ============================================================================

#[tauri::command]
pub async fn configure_voice(
    config: VoiceConfig,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // 1. If API keys are provided in config, save them securely and mask them in config
    if let Some(elevenlabs) = config.elevenlabs.clone() {
        if !elevenlabs.api_key.is_empty() && elevenlabs.api_key != "********" {
            state.credentials.store_secret("elevenlabs_api_key", &elevenlabs.api_key)
                .map_err(|e| e.to_string())?;
        }
    }

    let mut effective_config = config.clone();

    // Restore secrets from credential manager if masked
    if let Some(ref mut elevenlabs) = effective_config.elevenlabs {
        if elevenlabs.api_key.is_empty() || elevenlabs.api_key == "********" {
             if let Ok(secret) = state.credentials.get_secret("elevenlabs_api_key") {
                 elevenlabs.api_key = secret;
             }
        }
    }

    // Save config to disk (with secrets restored for persistence)
    save_voice_config_disk(&app_handle, &effective_config);

    let new_manager = VoiceManager::new(effective_config);

    // Update state
    let mut manager = state.voice_manager.write().await;
    *manager = new_manager;
    Ok("Voice configuration updated successfully".to_string())
}

#[tauri::command]
pub async fn get_voice_config(state: State<'_, AppState>) -> Result<VoiceConfig, String> {
    let manager = state.voice_manager.read().await;
    let mut config = manager.get_config().clone();
    // Mask secrets
    if let Some(ref mut elevenlabs) = config.elevenlabs {
        if !elevenlabs.api_key.is_empty() {
            elevenlabs.api_key = "********".to_string();
        }
    }
    Ok(config)
}

/// Detect available voice providers on the system
/// Returns status for each local TTS service (running/not running)
#[tauri::command]
pub async fn detect_voice_providers() -> Result<VoiceProviderDetection, String> {
    Ok(detect_providers().await)
}

// ============================================================================
// Voice Provider Installation Commands
// ============================================================================

/// Check installation status for all local voice providers
#[tauri::command]
pub async fn check_voice_provider_installations() -> Result<Vec<InstallStatus>, String> {
    let models_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("ttrpg-assistant/voice/piper");

    let installer = ProviderInstaller::new(models_dir);
    Ok(installer.check_all_local().await)
}

/// Check installation status for a specific provider
#[tauri::command]
pub async fn check_voice_provider_status(provider: VoiceProviderType) -> Result<InstallStatus, String> {
    let models_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("ttrpg-assistant/voice/piper");

    let installer = ProviderInstaller::new(models_dir);
    Ok(installer.check_status(&provider).await)
}

/// Install a voice provider (Piper or Coqui)
#[tauri::command]
pub async fn install_voice_provider(provider: VoiceProviderType) -> Result<InstallStatus, String> {
    let models_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("ttrpg-assistant/voice/piper");

    let installer = ProviderInstaller::new(models_dir);
    installer.install(&provider).await.map_err(|e| e.to_string())
}

/// List available Piper voices for download from Hugging Face
#[tauri::command]
pub async fn list_downloadable_piper_voices() -> Result<Vec<AvailablePiperVoice>, String> {
    let models_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("ttrpg-assistant/voice/piper");

    let installer = ProviderInstaller::new(models_dir);
    installer.list_available_piper_voices().await.map_err(|e| e.to_string())
}

/// Get recommended/popular Piper voices (quick, no network call)
#[tauri::command]
pub fn get_popular_piper_voices() -> Vec<(String, String, String)> {
    get_recommended_piper_voices()
        .into_iter()
        .map(|(k, n, d)| (k.to_string(), n.to_string(), d.to_string()))
        .collect()
}

/// Download a Piper voice from Hugging Face
#[tauri::command]
pub async fn download_piper_voice(voice_key: String, quality: Option<String>) -> Result<String, String> {
    let models_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("ttrpg-assistant/voice/piper");

    let installer = ProviderInstaller::new(models_dir);
    let path = installer
        .download_piper_voice(&voice_key, quality.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn play_tts(
    text: String,
    voice_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Synthesize audio first, keeping the lock scope minimal.
    let audio_path = {
        let manager = state.voice_manager.read().await;
        let request = SynthesisRequest {
            text,
            voice_id,
            settings: None,
            output_format: OutputFormat::Wav,
        };
        let result = manager.synthesize(request).await.map_err(|e| e.to_string())?;
        result.audio_path
    }; // Read lock is released here.

    // Read audio data in a blocking task to avoid blocking async runtime
    let audio_data = tokio::task::spawn_blocking(move || std::fs::read(&audio_path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    // Play audio in a blocking task to avoid blocking async runtime
    tokio::task::spawn_blocking(move || {
        use rodio::{Decoder, OutputStream, Sink};
        use std::io::Cursor;

        let (_stream, stream_handle) = OutputStream::try_default().map_err(|e| e.to_string())?;
        let sink = Sink::try_new(&stream_handle).map_err(|e| e.to_string())?;
        let cursor = Cursor::new(audio_data);
        let source = Decoder::new(cursor).map_err(|e| e.to_string())?;

        sink.append(source);
        sink.sleep_until_end();
        Ok::<(), String>(())
    }).await.map_err(|e| e.to_string())??;

    Ok(())
}

#[tauri::command]
pub async fn list_all_voices(state: State<'_, AppState>) -> Result<Vec<Voice>, String> {
    state.voice_manager.read().await.list_voices().await.map_err(|e| e.to_string())
}

// ============================================================================
// Meilisearch Commands
// ============================================================================

/// Get Meilisearch health status
#[tauri::command]
pub async fn check_meilisearch_health(
    state: State<'_, AppState>,
) -> Result<MeilisearchStatus, String> {
    let healthy = state.search_client.health_check().await;
    let stats = if healthy {
        state.search_client.get_all_stats().await.ok()
    } else {
        None
    };

    Ok(MeilisearchStatus {
        healthy,
        host: state.search_client.host().to_string(),
        document_counts: stats,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeilisearchStatus {
    pub healthy: bool,
    pub host: String,
    pub document_counts: Option<HashMap<String, u64>>,
}

/// Reindex all documents (clear and re-ingest)
#[tauri::command]
pub async fn reindex_library(
    index_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if let Some(name) = index_name {
        state.search_client
            .clear_index(&name)
            .await
            .map_err(|e| format!("Failed to clear index: {}", e))?;
        Ok(format!("Cleared index '{}'", name))
    } else {
        // Clear all indexes
        for idx in crate::core::search_client::SearchClient::all_indexes() {
            let _ = state.search_client.clear_index(idx).await;
        }
        Ok("Cleared all indexes".to_string())
    }
}
// ============================================================================
// Character Generation Commands
// ============================================================================

#[tauri::command]
pub fn generate_character(
    system: String,
    level: u32,
    genre: Option<String>,
) -> Result<Character, String> {
    let options = GenerationOptions {
        system: Some(system),
        level: Some(level),
        theme: genre,
        ..Default::default()
    };
    let character = CharacterGenerator::generate(&options).map_err(|e| e.to_string())?;
    Ok(character)
}

// ============================================================================

// Campaign Commands
// ============================================================================

#[tauri::command]
pub fn list_campaigns(state: State<'_, AppState>) -> Result<Vec<Campaign>, String> {
    Ok(state.campaign_manager.list_campaigns())
}

#[tauri::command]
pub fn create_campaign(
    name: String,
    system: String,
    state: State<'_, AppState>,
) -> Result<Campaign, String> {
    Ok(state.campaign_manager.create_campaign(&name, &system))
}

#[tauri::command]
pub fn get_campaign(id: String, state: State<'_, AppState>) -> Result<Option<Campaign>, String> {
    Ok(state.campaign_manager.get_campaign(&id))
}

#[tauri::command]
pub fn update_campaign(
    campaign: Campaign,
    auto_snapshot: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.campaign_manager.update_campaign(campaign, auto_snapshot)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_campaign(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.campaign_manager.delete_campaign(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_campaign_theme(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<ThemeWeights, String> {
    state.campaign_manager
        .get_campaign(&campaign_id)
        .map(|c| c.settings.theme_weights)
        .ok_or_else(|| "Campaign not found".to_string())
}

#[tauri::command]
pub async fn set_campaign_theme(
    campaign_id: String,
    weights: ThemeWeights,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut campaign = state.campaign_manager
        .get_campaign(&campaign_id)
        .ok_or_else(|| "Campaign not found".to_string())?;

    campaign.settings.theme_weights = weights;
    state.campaign_manager.update_campaign(campaign, false)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_theme_preset(system: String) -> Result<ThemeWeights, String> {
    Ok(theme::get_theme_preset(&system))
}

#[tauri::command]
pub fn create_snapshot(
    campaign_id: String,
    description: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    state.campaign_manager.create_snapshot(&campaign_id, &description)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_snapshots(campaign_id: String, state: State<'_, AppState>) -> Result<Vec<SnapshotSummary>, String> {
    Ok(state.campaign_manager.list_snapshots(&campaign_id))
}

#[tauri::command]
pub fn restore_snapshot(
    campaign_id: String,
    snapshot_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.campaign_manager.restore_snapshot(&campaign_id, &snapshot_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_campaign(campaign_id: String, state: State<'_, AppState>) -> Result<String, String> {
    state.campaign_manager.export_to_json(&campaign_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_campaign(
    json: String,
    new_id: bool,
    state: State<'_, AppState>,
) -> Result<String, String> {
    state.campaign_manager.import_from_json(&json, new_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_campaign_stats(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<crate::core::campaign_manager::CampaignStats, String> {
    // 1. Get Session Stats
    let sessions = state.session_manager.list_sessions(&campaign_id);
    let session_count = sessions.len();
    let total_playtime_minutes: i64 = sessions.iter()
        .filter_map(|s| s.duration_minutes)
        .sum();

    // Find last played (most recent active/ended session)
    let last_played = sessions.iter()
        .filter(|s| s.status != crate::core::session_manager::SessionStatus::Planned)
        .map(|s| s.started_at) // Approximate default to started_at for sort
        .max();

    // 2. Get NPC Count
    // Helper to get count from DB/Store
    let npc_count = {
        let npcs = state.database.list_npcs(Some(&campaign_id)).await.unwrap_or_default();
        npcs.len()
    };

    Ok(crate::core::campaign_manager::CampaignStats {
        session_count,
        npc_count,
        total_playtime_minutes,
        last_played,
    })
}

// ============================================================================
// Session Notes Commands
// ============================================================================

#[tauri::command]
pub fn add_campaign_note(
    campaign_id: String,
    content: String,
    tags: Vec<String>,
    session_number: Option<u32>,
    state: State<'_, AppState>,
) -> Result<SessionNote, String> {
    Ok(state.campaign_manager.add_note(&campaign_id, &content, tags, session_number))
}

#[tauri::command]
pub fn get_campaign_notes(campaign_id: String, state: State<'_, AppState>) -> Result<Vec<SessionNote>, String> {
    Ok(state.campaign_manager.get_notes(&campaign_id))
}

#[tauri::command]
pub fn search_campaign_notes(
    campaign_id: String,
    query: String,
    tags: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> Result<Vec<SessionNote>, String> {
    let tags_ref = tags.as_deref();
    Ok(state.campaign_manager.search_notes(&campaign_id, &query, tags_ref))
}

#[tauri::command]
pub fn generate_campaign_cover(
    campaign_id: String,
    title: String,
) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use base64::Engine;

    // Deterministic colors based on ID
    let mut hasher = DefaultHasher::new();
    campaign_id.hash(&mut hasher);
    let h1 = hasher.finish();
    let h2 = !h1;

    let c1 = format!("#{:06x}", h1 & 0xFFFFFF);
    let c2 = format!("#{:06x}", h2 & 0xFFFFFF);

    // Initials
    let initials: String = title.split_whitespace()
        .take(2)
        .filter_map(|w| w.chars().next())
        .collect::<String>()
        .to_uppercase();

    // SVG
    let svg = format!(
        r#"<svg width="400" height="200" viewBox="0 0 400 200" xmlns="http://www.w3.org/2000/svg">
            <defs>
                <linearGradient id="g" x1="0%" y1="0%" x2="100%" y2="100%">
                    <stop offset="0%" style="stop-color:{};stop-opacity:1" />
                    <stop offset="100%" style="stop-color:{};stop-opacity:1" />
                </linearGradient>
            </defs>
            <rect width="100%" height="100%" fill="url(#g)" />
            <text x="50%" y="50%" dominant-baseline="middle" text-anchor="middle" font-family="Arial, sans-serif" font-size="80" fill="rgba(255,255,255,0.8)" font-weight="bold">{}</text>
        </svg>"#,
        c1, c2, initials
    );

    let b64 = base64::engine::general_purpose::STANDARD.encode(svg);
    format!("data:image/svg+xml;base64,{}", b64)
}

#[tauri::command]
pub fn delete_campaign_note(
    campaign_id: String,
    note_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.campaign_manager.delete_note(&campaign_id, &note_id)
        .map_err(|e| e.to_string())
}

// ============================================================================
// Session Commands
// ============================================================================

#[tauri::command]
pub fn start_session(
    campaign_id: String,
    session_number: u32,
    state: State<'_, AppState>,
) -> Result<GameSession, String> {
    Ok(state.session_manager.start_session(&campaign_id, session_number))
}

#[tauri::command]
pub fn get_session(session_id: String, state: State<'_, AppState>) -> Result<Option<GameSession>, String> {
    Ok(state.session_manager.get_session(&session_id))
}

#[tauri::command]
pub fn get_active_session(campaign_id: String, state: State<'_, AppState>) -> Result<Option<GameSession>, String> {
    Ok(state.session_manager.get_active_session(&campaign_id))
}

#[tauri::command]
pub fn list_sessions(campaign_id: String, state: State<'_, AppState>) -> Result<Vec<SessionSummary>, String> {
    Ok(state.session_manager.list_sessions(&campaign_id))
}

#[tauri::command]
pub fn end_session(session_id: String, state: State<'_, AppState>) -> Result<SessionSummary, String> {
    state.session_manager.end_session(&session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_planned_session(
    campaign_id: String,
    title: Option<String>,
    state: State<'_, AppState>,
) -> Result<GameSession, String> {
    Ok(state.session_manager.create_planned_session(&campaign_id, title))
}

#[tauri::command]
pub fn start_planned_session(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<GameSession, String> {
    state.session_manager.start_planned_session(&session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_session(
    session_id: String,
    new_order: i32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.session_manager.reorder_session(&session_id, new_order)
        .map_err(|e| e.to_string())
}

// ============================================================================
// Global Chat Session Commands (Persistent LLM Chat History)
// ============================================================================

/// Get or create the active global chat session
#[tauri::command]
pub async fn get_or_create_chat_session(
    state: State<'_, AppState>,
) -> Result<crate::database::GlobalChatSessionRecord, String> {
    state.database.get_or_create_active_chat_session()
        .await
        .map_err(|e| e.to_string())
}

/// Get the current active chat session
#[tauri::command]
pub async fn get_active_chat_session(
    state: State<'_, AppState>,
) -> Result<Option<crate::database::GlobalChatSessionRecord>, String> {
    state.database.get_active_chat_session()
        .await
        .map_err(|e| e.to_string())
}

/// Get messages for a chat session
#[tauri::command]
pub async fn get_chat_messages(
    session_id: String,
    limit: Option<i32>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::database::ChatMessageRecord>, String> {
    state.database.get_chat_messages(&session_id, limit.unwrap_or(100))
        .await
        .map_err(|e| e.to_string())
}

/// Add a message to the chat session
#[tauri::command]
pub async fn add_chat_message(
    session_id: String,
    role: String,
    content: String,
    tokens: Option<(i32, i32)>,
    state: State<'_, AppState>,
) -> Result<crate::database::ChatMessageRecord, String> {
    let mut message = crate::database::ChatMessageRecord::new(session_id, role, content);
    if let Some((input, output)) = tokens {
        message = message.with_tokens(input, output);
    }
    state.database.add_chat_message(&message)
        .await
        .map_err(|e| e.to_string())?;
    Ok(message)
}

/// Update a chat message (e.g., after streaming completes)
/// Fetches existing record and merges fields to preserve existing tokens/metadata
#[tauri::command]
pub async fn update_chat_message(
    message_id: String,
    content: String,
    tokens: Option<(i32, i32)>,
    is_streaming: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Fetch existing message to preserve fields not being updated
    let mut message = state.database.get_chat_message(&message_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Message not found: {}", message_id))?;

    // Update only the fields that are being changed
    message.content = content;
    message.is_streaming = if is_streaming { 1 } else { 0 };

    // Only update tokens if provided, otherwise preserve existing
    if let Some((input, output)) = tokens {
        message.tokens_input = Some(input);
        message.tokens_output = Some(output);
    }

    state.database.update_chat_message(&message)
        .await
        .map_err(|e| e.to_string())
}

/// Link the current chat session to a game session
#[tauri::command]
pub async fn link_chat_to_game_session(
    chat_session_id: String,
    game_session_id: String,
    campaign_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.database.link_chat_session_to_game(
        &chat_session_id,
        &game_session_id,
        campaign_id.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

/// Archive the current chat session and create a new one
/// Used when ending a game session
///
/// Note: Archives first due to unique index constraint (only one active session allowed).
/// If new session creation fails after archiving, call get_or_create_chat_session
/// which handles the race-condition-safe creation.
#[tauri::command]
pub async fn end_chat_session_and_spawn_new(
    chat_session_id: String,
    state: State<'_, AppState>,
) -> Result<crate::database::GlobalChatSessionRecord, String> {
    // Archive current session first (removes the 'active' constraint)
    state.database.archive_chat_session(&chat_session_id)
        .await
        .map_err(|e| e.to_string())?;

    // Now create new session (only one active session allowed by unique index)
    let new_session = crate::database::GlobalChatSessionRecord::new();
    state.database.create_chat_session(&new_session)
        .await
        .map_err(|e| e.to_string())?;

    Ok(new_session)
}

/// Clear all messages in a chat session
#[tauri::command]
pub async fn clear_chat_messages(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    state.database.clear_chat_messages(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// List recent chat sessions (all statuses, ordered by most recent)
#[tauri::command]
pub async fn list_chat_sessions(
    limit: Option<i32>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::database::GlobalChatSessionRecord>, String> {
    state.database.list_chat_sessions(limit.unwrap_or(50))
        .await
        .map_err(|e| e.to_string())
}

/// Get chat sessions linked to a specific game session
#[tauri::command]
pub async fn get_chat_sessions_for_game(
    game_session_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::database::GlobalChatSessionRecord>, String> {
    state.database.get_chat_sessions_by_game_session(&game_session_id)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Combat Commands
// ============================================================================

#[tauri::command]
pub fn start_combat(session_id: String, state: State<'_, AppState>) -> Result<CombatState, String> {
    state.session_manager.start_combat(&session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn end_combat(session_id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.session_manager.end_combat(&session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_combat(session_id: String, state: State<'_, AppState>) -> Result<Option<CombatState>, String> {
    Ok(state.session_manager.get_combat(&session_id))
}

#[tauri::command]
pub fn add_combatant(
    session_id: String,
    name: String,
    initiative: i32,
    combatant_type: String,
    hp_current: Option<i32>,
    hp_max: Option<i32>,
    armor_class: Option<i32>,
    state: State<'_, AppState>,
) -> Result<Combatant, String> {
    use crate::core::session::ConditionTracker;

    let ctype = match combatant_type.as_str() {
        "player" => CombatantType::Player,
        "npc" => CombatantType::NPC,
        "monster" => CombatantType::Monster,
        "ally" => CombatantType::Ally,
        _ => CombatantType::Monster,
    };

    // Create full combatant with optional HP/AC
    let combatant = Combatant {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        initiative,
        initiative_modifier: 0,
        combatant_type: ctype,
        current_hp: hp_current.or(hp_max),
        max_hp: hp_max,
        temp_hp: None,
        armor_class,
        conditions: vec![],
        condition_tracker: ConditionTracker::new(),
        condition_immunities: vec![],
        is_active: true,
        notes: String::new(),
    };

    state.session_manager.add_combatant(&session_id, combatant.clone())
        .map_err(|e| e.to_string())?;

    Ok(combatant)
}

#[tauri::command]
pub fn remove_combatant(
    session_id: String,
    combatant_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.session_manager.remove_combatant(&session_id, &combatant_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn next_turn(session_id: String, state: State<'_, AppState>) -> Result<Option<Combatant>, String> {
    state.session_manager.next_turn(&session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_current_combatant(session_id: String, state: State<'_, AppState>) -> Result<Option<Combatant>, String> {
    Ok(state.session_manager.get_current_combatant(&session_id))
}

#[tauri::command]
pub fn damage_combatant(
    session_id: String,
    combatant_id: String,
    amount: i32,
    state: State<'_, AppState>,
) -> Result<i32, String> {
    state.session_manager.damage_combatant(&session_id, &combatant_id, amount)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn heal_combatant(
    session_id: String,
    combatant_id: String,
    amount: i32,
    state: State<'_, AppState>,
) -> Result<i32, String> {
    state.session_manager.heal_combatant(&session_id, &combatant_id, amount)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_condition(
    session_id: String,
    combatant_id: String,
    condition_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let condition = create_common_condition(&condition_name)
        .ok_or_else(|| format!("Unknown condition: {}", condition_name))?;

    state.session_manager.add_condition(&session_id, &combatant_id, condition)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_condition(
    session_id: String,
    combatant_id: String,
    condition_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.session_manager.remove_condition(&session_id, &combatant_id, &condition_name)
        .map_err(|e| e.to_string())
}

// ============================================================================
// Advanced Condition Commands (TASK-015)
// ============================================================================

/// Request payload for adding a condition with full options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddConditionRequest {
    pub session_id: String,
    pub combatant_id: String,
    pub condition_name: String,
    pub duration_type: Option<String>,
    pub duration_value: Option<u32>,
    pub source_id: Option<String>,
    pub source_name: Option<String>,
    pub save_type: Option<String>,
    pub save_dc: Option<u32>,
}

/// Parse duration from request
fn parse_condition_duration(
    duration_type: Option<String>,
    duration_value: Option<u32>,
    save_type: Option<String>,
    save_dc: Option<u32>,
) -> Option<crate::core::session::conditions::ConditionDuration> {
    use crate::core::session::conditions::{ConditionDuration, SaveTiming};

    let duration_type = duration_type?;
    match duration_type.as_str() {
        "turns" => Some(ConditionDuration::Turns(duration_value.unwrap_or(1))),
        "rounds" => Some(ConditionDuration::Rounds(duration_value.unwrap_or(1))),
        "minutes" => Some(ConditionDuration::Minutes(duration_value.unwrap_or(1))),
        "hours" => Some(ConditionDuration::Hours(duration_value.unwrap_or(1))),
        "end_of_next_turn" => Some(ConditionDuration::EndOfNextTurn),
        "start_of_next_turn" => Some(ConditionDuration::StartOfNextTurn),
        "end_of_source_turn" => Some(ConditionDuration::EndOfSourceTurn),
        "until_save" => Some(ConditionDuration::UntilSave {
            save_type: save_type.unwrap_or_else(|| "CON".to_string()),
            dc: save_dc.unwrap_or(10),
            timing: SaveTiming::EndOfTurn,
        }),
        "until_removed" => Some(ConditionDuration::UntilRemoved),
        "permanent" => Some(ConditionDuration::Permanent),
        _ => None,
    }
}

#[tauri::command]
pub fn add_condition_advanced(
    request: AddConditionRequest,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use crate::core::session::conditions::{AdvancedCondition, ConditionTemplates};

    let duration = parse_condition_duration(
        request.duration_type,
        request.duration_value,
        request.save_type,
        request.save_dc,
    );

    // Try to get a standard condition template, or create a custom one
    let mut condition = ConditionTemplates::by_name(&request.condition_name)
        .unwrap_or_else(|| {
            use crate::core::session::conditions::ConditionDuration;
            AdvancedCondition::new(
                &request.condition_name,
                format!("Custom condition: {}", request.condition_name),
                duration.clone().unwrap_or(ConditionDuration::UntilRemoved),
            )
        });

    // Override duration if specified
    if let Some(dur) = duration {
        condition.duration = dur.clone();
        condition.remaining = match &dur {
            crate::core::session::conditions::ConditionDuration::Turns(n) => Some(*n),
            crate::core::session::conditions::ConditionDuration::Rounds(n) => Some(*n),
            crate::core::session::conditions::ConditionDuration::Minutes(n) => Some(*n),
            crate::core::session::conditions::ConditionDuration::Hours(n) => Some(*n),
            _ => None,
        };
    }

    // Set source if provided
    if let (Some(src_id), Some(src_name)) = (request.source_id, request.source_name) {
        condition.source_id = Some(src_id);
        condition.source_name = Some(src_name);
    }

    state.session_manager.apply_advanced_condition(
        &request.session_id,
        &request.combatant_id,
        condition,
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_condition_by_id(
    session_id: String,
    combatant_id: String,
    condition_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.session_manager.remove_advanced_condition(&session_id, &combatant_id, &condition_id)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// Duplicates removed


// Duplicate removed


// ============================================================================
// Character Generation Commands (Enhanced for TASK-018)
// ============================================================================

#[tauri::command]
pub fn get_supported_systems() -> Vec<String> {
    CharacterGenerator::supported_systems()
}

#[tauri::command]
pub fn list_system_info() -> Vec<SystemInfo> {
    CharacterGenerator::list_system_info()
}

#[tauri::command]
pub fn get_system_info(system: String) -> Option<SystemInfo> {
    CharacterGenerator::get_system_info(&system)
}

#[tauri::command]
pub fn generate_character_advanced(options: GenerationOptions) -> Result<Character, String> {
    CharacterGenerator::generate(&options).map_err(|e| e.to_string())
}

// ============================================================================
// NPC Commands
// ============================================================================

#[tauri::command]
pub async fn generate_npc(
    options: NPCGenerationOptions,
    campaign_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<NPC, String> {
    let generator = NPCGenerator::new();
    let npc = generator.generate_quick(&options);

    // Save to memory store
    state.npc_store.add(npc.clone(), campaign_id.as_deref());

    // Save to Database
    let personality_json = serde_json::to_string(&npc.personality).map_err(|e| e.to_string())?;
    let stats_json = npc.stats.as_ref().map(|s| serde_json::to_string(s).unwrap_or_default());
    let role_str = serialize_enum_to_string(&npc.role);
    let data_json = serde_json::to_string(&npc).map_err(|e| e.to_string())?;

    let record = crate::database::NpcRecord {
        id: npc.id.clone(),
        campaign_id: campaign_id.clone(),
        name: npc.name.clone(),
        role: role_str,
        personality_id: None,
        personality_json,
        data_json: Some(data_json),
        stats_json,
        notes: Some(npc.notes.clone()),
        location_id: None,
        voice_profile_id: None,
        quest_hooks: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    state.database.save_npc(&record).await.map_err(|e| e.to_string())?;

    Ok(npc)
}

#[tauri::command]
pub async fn get_npc(id: String, state: State<'_, AppState>) -> Result<Option<NPC>, String> {
    if let Some(npc) = state.npc_store.get(&id) {
        return Ok(Some(npc));
    }

    if let Some(record) = state.database.get_npc(&id).await.map_err(|e| e.to_string())? {
        if let Some(json) = record.data_json {
             let npc: NPC = serde_json::from_str(&json).map_err(|e| e.to_string())?;
             state.npc_store.add(npc.clone(), record.campaign_id.as_deref());
             return Ok(Some(npc));
        }
    }
    Ok(None)
}

#[tauri::command]
pub async fn list_npcs(campaign_id: Option<String>, state: State<'_, AppState>) -> Result<Vec<NPC>, String> {
    let records = state.database.list_npcs(campaign_id.as_deref()).await.map_err(|e| e.to_string())?;
    let mut npcs = Vec::new();

    for r in records {
        if let Some(json) = r.data_json {
             if let Ok(npc) = serde_json::from_str::<NPC>(&json) {
                 npcs.push(npc);
             }
        }
    }

    if npcs.is_empty() {
        let mem_npcs = state.npc_store.list(campaign_id.as_deref());
        if !mem_npcs.is_empty() {
            return Ok(mem_npcs);
        }
    }

    Ok(npcs)
}

#[tauri::command]
pub async fn update_npc(npc: NPC, state: State<'_, AppState>) -> Result<(), String> {
    state.npc_store.update(npc.clone());

    let personality_json = serde_json::to_string(&npc.personality).map_err(|e| e.to_string())?;
    let stats_json = npc.stats.as_ref().map(|s| serde_json::to_string(s).unwrap_or_default());
    let role_str = serialize_enum_to_string(&npc.role);
    let data_json = serde_json::to_string(&npc).map_err(|e| e.to_string())?;

    let created_at = if let Some(old) = state.database.get_npc(&npc.id).await.map_err(|e| e.to_string())? {
        old.created_at
    } else {
        chrono::Utc::now().to_rfc3339()
    };

    let (campaign_id, location_id, voice_profile_id, quest_hooks) = if let Some(old) = state.database.get_npc(&npc.id).await.map_err(|e| e.to_string())? {
        (old.campaign_id, old.location_id, old.voice_profile_id, old.quest_hooks)
    } else {
        (None, None, None, None)
    };

    let record = crate::database::NpcRecord {
        id: npc.id.clone(),
        campaign_id,
        name: npc.name.clone(),
        role: role_str,
        personality_id: None,
        personality_json,
        data_json: Some(data_json),
        stats_json,
        notes: Some(npc.notes.clone()),
        location_id,
        voice_profile_id,
        quest_hooks,
        created_at,
    };

    state.database.save_npc(&record).await.map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn delete_npc(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.npc_store.delete(&id);
    state.database.delete_npc(&id).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn search_npcs(
    query: String,
    campaign_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<NPC>, String> {
    Ok(state.npc_store.search(&query, campaign_id.as_deref()))
}

// ============================================================================
// NPC Extensions - Vocabulary, Names, and Dialects Commands
// ============================================================================

/// Load a vocabulary bank from YAML file (legacy NPC format)
#[tauri::command]
pub async fn load_vocabulary_bank(path: String) -> Result<NpcVocabularyBank, String> {
    load_yaml_file(&std::path::PathBuf::from(path))
        .await
        .map_err(|e| e.to_string())
}

/// Get the vocabulary directory path
#[tauri::command]
pub fn get_vocabulary_directory() -> String {
    get_vocabulary_dir().to_string_lossy().to_string()
}

/// Get a random phrase from a vocabulary bank (legacy NPC format)
#[tauri::command]
pub fn get_vocabulary_phrase(
    bank: NpcVocabularyBank,
    category: String,
    formality: String,
) -> Result<Option<PhraseEntry>, String> {
    let formality = Formality::from_str(&formality);
    let mut rng = rand::thread_rng();

    let phrase = match category.as_str() {
        "greeting" | "greetings" => bank.get_greeting(formality, &mut rng),
        "farewell" | "farewells" => bank.get_farewell(formality, &mut rng),
        "exclamation" | "exclamations" => bank.get_exclamation(&mut rng),
        "negotiation" => bank.get_negotiation_phrase(&mut rng),
        "combat" => bank.get_combat_phrase(&mut rng),
        _ => None,
    };

    Ok(phrase.cloned())
}

/// Load cultural naming rules from YAML file
#[tauri::command]
pub async fn load_naming_rules(path: String) -> Result<CulturalNamingRules, String> {
    load_yaml_file(&std::path::PathBuf::from(path))
        .await
        .map_err(|e| e.to_string())
}

/// Get the names directory path
#[tauri::command]
pub fn get_names_directory() -> String {
    get_names_dir().to_string_lossy().to_string()
}

/// Get a random structure from cultural naming rules
#[tauri::command]
pub fn get_random_name_structure(
    rules: CulturalNamingRules,
) -> NameStructure {
    let mut rng = rand::thread_rng();
    rules.random_structure(&mut rng)
}

/// Validate cultural naming rules
#[tauri::command]
pub fn validate_naming_rules(
    rules: CulturalNamingRules,
) -> Result<(), String> {
    rules.validate().map_err(|e| e.to_string())
}

/// Load a dialect definition from YAML file
#[tauri::command]
pub async fn load_dialect(path: String) -> Result<DialectDefinition, String> {
    load_yaml_file(&std::path::PathBuf::from(path))
        .await
        .map_err(|e| e.to_string())
}

/// Get the dialects directory path
#[tauri::command]
pub fn get_dialects_directory() -> String {
    get_dialects_dir().to_string_lossy().to_string()
}

/// Transform text using a dialect
#[tauri::command]
pub fn apply_dialect(
    dialect: DialectDefinition,
    text: String,
    intensity: String,
) -> Result<DialectTransformResult, String> {
    let intensity = Intensity::from_str(&intensity);
    let mut rng = rand::thread_rng();
    let transformer = DialectTransformer::new(dialect).with_intensity(intensity);
    Ok(transformer.transform(&text, &mut rng))
}

/// Initialize NPC extension indexes in Meilisearch
#[tauri::command]
pub async fn initialize_npc_indexes(
    state: State<'_, AppState>,
) -> Result<(), String> {
    ensure_npc_indexes(state.search_client.get_client())
        .await
        .map_err(|e| e.to_string())
}

/// Get NPC index statistics
#[tauri::command]
pub async fn get_npc_indexes_stats(
    state: State<'_, AppState>,
) -> Result<NpcIndexStats, String> {
    get_npc_index_stats(state.search_client.get_client())
        .await
        .map_err(|e| e.to_string())
}

/// Clear NPC indexes
#[tauri::command]
pub async fn clear_npc_indexes(
    state: State<'_, AppState>,
) -> Result<(), String> {
    crate::core::npc_gen::clear_npc_indexes(state.search_client.get_client())
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Document Ingestion Commands
// ============================================================================

#[tauri::command]
pub async fn ingest_pdf(
    path: String,
    state: State<'_, AppState>,
) -> Result<IngestResult, String> {
    let path_buf = std::path::Path::new(&path);

    // Process using MeilisearchPipeline
    let result = state.ingestion_pipeline
        .process_file(
            &state.search_client,
            path_buf,
            "document",
            None // No campaign ID for generic library ingestion
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(IngestResult {
        page_count: 0, // Simplified pipeline result doesn't return page count yet
        character_count: result.total_chunks * 500, // Approximation if needed, or update IngestResult
        source_name: result.source,
    })
}

#[tauri::command]
pub async fn get_vector_store_status(state: State<'_, AppState>) -> Result<String, String> {
    if state.search_client.health_check().await {
        Ok("Meilisearch Ready".to_string())
    } else {
        Ok("Meilisearch Unhealthy".to_string())
    }
}

/// Configure Meilisearch embedder for semantic search
#[derive(Debug, Serialize, Deserialize)]
pub struct EmbedderConfigRequest {
    /// Embedder name (e.g., "default", "openai", "ollama")
    pub name: String,
    /// Provider type: "openAi", "ollama", or "huggingFace"
    pub provider: String,
    /// API key (for OpenAI)
    pub api_key: Option<String>,
    /// Model name (e.g., "text-embedding-3-small", "nomic-embed-text")
    pub model: Option<String>,
    /// Embedding dimensions (e.g., 1536 for OpenAI)
    pub dimensions: Option<u32>,
    /// Base URL (for Ollama)
    pub url: Option<String>,
}

/// Configure Meilisearch embedder for semantic/vector search
#[tauri::command]
pub async fn configure_meilisearch_embedder(
    index_name: String,
    config: EmbedderConfigRequest,
    state: State<'_, AppState>,
) -> Result<String, String> {
    use crate::core::search_client::EmbedderConfig;

    let embedder_config = match config.provider.as_str() {
        "openAi" | "openai" => {
            let api_key = config.api_key.ok_or("OpenAI API key required")?;
            EmbedderConfig::OpenAI {
                api_key,
                model: config.model,
                dimensions: config.dimensions,
            }
        }
        "ollama" => {
            let url = config.url.unwrap_or_else(|| "http://localhost:11434".to_string());
            let model = config.model.unwrap_or_else(|| "nomic-embed-text".to_string());
            EmbedderConfig::Ollama { url, model }
        }
        "huggingFace" | "huggingface" => {
            let model = config.model.unwrap_or_else(|| "BAAI/bge-base-en-v1.5".to_string());
            EmbedderConfig::HuggingFace { model }
        }
        other => return Err(format!("Unknown provider: {}. Use 'openAi', 'ollama', or 'huggingFace'", other)),
    };

    state.search_client
        .configure_embedder(&index_name, &config.name, &embedder_config)
        .await
        .map_err(|e| format!("Failed to configure embedder: {}", e))?;

    Ok(format!("Configured embedder '{}' for index '{}'", config.name, index_name))
}

/// Setup Ollama embeddings on all content indexes using REST embedder
///
/// This configures Meilisearch to use Ollama for AI-powered semantic search.
/// The embedder is configured as a REST source for maximum compatibility.
#[tauri::command]
pub async fn setup_ollama_embeddings(
    host: String,
    model: String,
    state: State<'_, AppState>,
) -> Result<SetupEmbeddingsResult, String> {
    let configured = state.search_client
        .setup_ollama_embeddings(&host, &model)
        .await
        .map_err(|e| format!("Failed to setup embeddings: {}", e))?;

    let dimensions = crate::core::search_client::ollama_embedding_dimensions(&model);

    Ok(SetupEmbeddingsResult {
        indexes_configured: configured,
        model: model.clone(),
        dimensions,
        host: host.clone(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetupEmbeddingsResult {
    pub indexes_configured: Vec<String>,
    pub model: String,
    pub dimensions: u32,
    pub host: String,
}

/// Get embedder configuration for an index
#[tauri::command]
pub async fn get_embedder_status(
    index_name: String,
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>, String> {
    state.search_client
        .get_embedder_settings(&index_name)
        .await
        .map_err(|e| format!("Failed to get embedder status: {}", e))
}

/// List available Ollama embedding models (filters for embedding-capable models)
#[tauri::command]
pub async fn list_ollama_embedding_models(host: String) -> Result<Vec<OllamaEmbeddingModel>, String> {
    // Get all models from Ollama
    let models = crate::core::llm::LLMClient::list_ollama_models(&host)
        .await
        .map_err(|e| e.to_string())?;

    // Known embedding model patterns
    let embedding_patterns = [
        "nomic-embed",
        "mxbai-embed",
        "all-minilm",
        "bge-",
        "snowflake-arctic-embed",
        "gte-",
        "e5-",
        "embed",
    ];

    let embedding_models: Vec<OllamaEmbeddingModel> = models
        .into_iter()
        .filter(|m| {
            let name_lower = m.name.to_lowercase();
            embedding_patterns.iter().any(|p| name_lower.contains(p))
        })
        .map(|m| {
            let dimensions = crate::core::search_client::ollama_embedding_dimensions(&m.name);
            OllamaEmbeddingModel {
                name: m.name,
                size: m.size,
                dimensions,
            }
        })
        .collect();

    // If no embedding models found, return common defaults that user should pull
    if embedding_models.is_empty() {
        return Ok(vec![
            OllamaEmbeddingModel {
                name: "nomic-embed-text".to_string(),
                size: "274 MB".to_string(),
                dimensions: 768,
            },
            OllamaEmbeddingModel {
                name: "mxbai-embed-large".to_string(),
                size: "669 MB".to_string(),
                dimensions: 1024,
            },
            OllamaEmbeddingModel {
                name: "all-minilm".to_string(),
                size: "46 MB".to_string(),
                dimensions: 384,
            },
        ]);
    }

    Ok(embedding_models)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaEmbeddingModel {
    pub name: String,
    pub size: String,
    pub dimensions: u32,
}

/// Local embedding model info (HuggingFace/ONNX - runs locally via Meilisearch)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalEmbeddingModel {
    pub id: String,
    pub name: String,
    pub dimensions: u32,
    pub description: String,
}

/// List available local embedding models (HuggingFace/ONNX - no external service required)
///
/// These models run locally within Meilisearch using the HuggingFace embedder.
/// No GPU required - uses ONNX runtime for CPU inference.
#[tauri::command]
pub async fn list_local_embedding_models() -> Result<Vec<LocalEmbeddingModel>, String> {
    // Curated list of recommended HuggingFace embedding models
    // These are known to work well with Meilisearch and have reasonable performance
    Ok(vec![
        LocalEmbeddingModel {
            id: "BAAI/bge-base-en-v1.5".to_string(),
            name: "BGE Base (English)".to_string(),
            dimensions: 768,
            description: "Balanced performance and quality. Good for general use.".to_string(),
        },
        LocalEmbeddingModel {
            id: "BAAI/bge-small-en-v1.5".to_string(),
            name: "BGE Small (English)".to_string(),
            dimensions: 384,
            description: "Faster, smaller. Good for limited resources.".to_string(),
        },
        LocalEmbeddingModel {
            id: "BAAI/bge-large-en-v1.5".to_string(),
            name: "BGE Large (English)".to_string(),
            dimensions: 1024,
            description: "Highest quality. Slower, needs more memory.".to_string(),
        },
        LocalEmbeddingModel {
            id: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            name: "MiniLM-L6 (Multilingual)".to_string(),
            dimensions: 384,
            description: "Fast and small. Supports 100+ languages.".to_string(),
        },
        LocalEmbeddingModel {
            id: "sentence-transformers/all-mpnet-base-v2".to_string(),
            name: "MPNet Base".to_string(),
            dimensions: 768,
            description: "High quality general-purpose embeddings.".to_string(),
        },
        LocalEmbeddingModel {
            id: "thenlper/gte-base".to_string(),
            name: "GTE Base".to_string(),
            dimensions: 768,
            description: "Excellent retrieval performance.".to_string(),
        },
        LocalEmbeddingModel {
            id: "thenlper/gte-small".to_string(),
            name: "GTE Small".to_string(),
            dimensions: 384,
            description: "Compact with good retrieval quality.".to_string(),
        },
    ])
}

/// Setup local embeddings on all content indexes using HuggingFace embedder
///
/// This configures Meilisearch to use local ONNX models for AI-powered semantic search.
/// Models are downloaded and cached automatically by Meilisearch.
/// No external service (like Ollama) is required.
#[tauri::command]
pub async fn setup_local_embeddings(
    model: String,
    state: State<'_, AppState>,
) -> Result<SetupEmbeddingsResult, String> {
    use crate::core::search_client::EmbedderConfig;

    // Get dimensions for the model
    let dimensions = huggingface_embedding_dimensions(&model);

    // Configure HuggingFace embedder on all content indexes
    let indexes = vec!["documents", "chat_history", "rules", "campaigns"];
    let mut configured = Vec::new();

    for index_name in indexes {
        let config = EmbedderConfig::HuggingFace {
            model: model.clone(),
        };

        match state.search_client
            .configure_embedder(index_name, "default", &config)
            .await
        {
            Ok(_) => {
                configured.push(index_name.to_string());
                log::info!("Configured HuggingFace embedder on index '{}'", index_name);
            }
            Err(e) => {
                log::warn!("Failed to configure embedder on '{}': {}", index_name, e);
            }
        }
    }

    Ok(SetupEmbeddingsResult {
        indexes_configured: configured,
        model: model.clone(),
        dimensions,
        host: "local".to_string(),
    })
}

/// Get dimensions for HuggingFace embedding models
fn huggingface_embedding_dimensions(model: &str) -> u32 {
    match model.to_lowercase().as_str() {
        m if m.contains("bge-small") => 384,
        m if m.contains("bge-base") => 768,
        m if m.contains("bge-large") => 1024,
        m if m.contains("minilm") => 384,
        m if m.contains("mpnet") => 768,
        m if m.contains("gte-small") => 384,
        m if m.contains("gte-base") => 768,
        m if m.contains("gte-large") => 1024,
        m if m.contains("e5-small") => 384,
        m if m.contains("e5-base") => 768,
        m if m.contains("e5-large") => 1024,
        _ => 768, // Default assumption
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IngestResult {
    pub page_count: usize,
    pub character_count: usize,
    pub source_name: String,
}

/// Progress event for document ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestProgress {
    pub stage: String,
    pub progress: f32,       // 0.0 to 1.0
    pub message: String,
    pub source_name: String,
}

/// Estimate PDF page count using pdfinfo (fast) or file size heuristic
fn estimate_pdf_pages(path: &std::path::Path, file_size: u64) -> usize {
    // For PDFs, try to get actual page count from pdfinfo (very fast)
    if path.extension().and_then(|e| e.to_str()) == Some("pdf") {
        if let Ok(output) = std::process::Command::new("pdfinfo")
            .arg(path)
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("Pages:") {
                    if let Some(count_str) = line.split(':').nth(1) {
                        if let Ok(count) = count_str.trim().parse::<usize>() {
                            return count;
                        }
                    }
                }
            }
        }
    }

    // Fallback: estimate based on file size
    // Use 200KB per page as middle ground between text (50KB) and scanned (500KB+)
    (file_size / 200_000).max(1) as usize
}

/// List all documents from the library (persisted in Meilisearch)
#[tauri::command]
pub async fn list_library_documents(
    state: State<'_, AppState>,
) -> Result<Vec<crate::core::search_client::LibraryDocumentMetadata>, String> {
    state.search_client
        .list_library_documents()
        .await
        .map_err(|e| format!("Failed to list documents: {}", e))
}

/// Delete a document from the library (removes metadata and content chunks)
#[tauri::command]
pub async fn delete_library_document(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.search_client
        .delete_library_document_with_content(&id)
        .await
        .map_err(|e| format!("Failed to delete document: {}", e))
}

/// Update library document TTRPG metadata fields
#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpdateLibraryDocumentRequest {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setting: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
}

/// Update a library document's TTRPG metadata
#[tauri::command]
pub async fn update_library_document(
    request: UpdateLibraryDocumentRequest,
    state: State<'_, AppState>,
) -> Result<crate::core::search_client::LibraryDocumentMetadata, String> {
    // Fetch existing document
    let mut doc = state.search_client
        .get_library_document(&request.id)
        .await
        .map_err(|e| format!("Failed to fetch document: {}", e))?
        .ok_or_else(|| format!("Document not found: {}", request.id))?;

    // Update TTRPG metadata fields
    doc.game_system = request.game_system;
    doc.setting = request.setting;
    doc.content_type = request.content_type;
    doc.publisher = request.publisher;

    // Save updated document
    state.search_client
        .save_library_document(&doc)
        .await
        .map_err(|e| format!("Failed to save document: {}", e))?;

    log::info!("Updated library document metadata: {}", request.id);
    Ok(doc)
}

/// Rebuild library metadata from existing content indices.
///
/// Scans all content indices for unique sources and creates metadata entries
/// for sources that don't already have entries. Useful for migrating legacy data.
#[tauri::command]
pub async fn rebuild_library_metadata(
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let created = state.search_client
        .rebuild_library_metadata()
        .await
        .map_err(|e| format!("Failed to rebuild metadata: {}", e))?;

    Ok(created.len())
}

/// Clear a document's content and re-ingest from the original file.
///
/// Useful when ingestion produced garbage content (e.g., failed font decoding)
/// and you want to try again (possibly with OCR this time).
#[tauri::command]
pub async fn clear_and_reingest_document(
    id: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<IngestResult, String> {
    use tauri::Emitter;

    // Get the document metadata to find the file path
    let doc = state.search_client
        .get_library_document(&id)
        .await
        .map_err(|e| format!("Failed to get document: {}", e))?
        .ok_or_else(|| "Document not found".to_string())?;

    let file_path = doc.file_path
        .ok_or_else(|| "Document has no file path - cannot re-ingest".to_string())?;

    // Verify file still exists
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("Original file no longer exists: {}", file_path));
    }

    log::info!("Clearing and re-ingesting document: {} ({})", doc.name, id);

    // Delete existing content and metadata
    state.search_client
        .delete_library_document_with_content(&id)
        .await
        .map_err(|e| format!("Failed to delete existing content: {}", e))?;

    // Emit progress for clearing
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "clearing".to_string(),
        progress: 0.05,
        message: format!("Cleared old content, re-ingesting {}...", doc.name),
        source_name: doc.name.clone(),
    });

    // Re-ingest using the existing ingest logic
    // We need to call ingest_document_with_progress internally
    let source_type = Some(doc.source_type.clone());

    // Call the internal ingestion logic
    ingest_document_with_progress_internal(
        file_path,
        source_type,
        app,
        state,
    ).await
}

/// Internal ingestion logic shared by ingest_document_with_progress and clear_and_reingest
async fn ingest_document_with_progress_internal(
    path: String,
    source_type: Option<String>,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<IngestResult, String> {
    use tauri::Emitter;
    use crate::core::meilisearch_pipeline::MeilisearchPipeline;

    let path_buf = std::path::PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("File not found: {}", path));
    }

    let source_name = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let source_type = source_type.unwrap_or_else(|| "document".to_string());

    // Stage 1: Parsing (0-40%)
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "parsing".to_string(),
        progress: 0.0,
        message: format!("Loading {}...", source_name),
        source_name: source_name.clone(),
    });

    let extension = path_buf
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let file_size = std::fs::metadata(&path_buf)
        .map(|m| m.len())
        .unwrap_or(0);
    let estimated_pages = estimate_pdf_pages(&path_buf, file_size);

    let format_name = match extension.as_str() {
        "pdf" => "PDF",
        "epub" => "EPUB",
        "mobi" | "azw" | "azw3" => "MOBI/AZW",
        "docx" => "DOCX",
        "txt" => "text",
        "md" | "markdown" => "Markdown",
        _ => "document",
    };

    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "parsing".to_string(),
        progress: 0.1,
        message: format!("Parsing {} (~{} estimated pages)...", format_name, estimated_pages),
        source_name: source_name.clone(),
    });

    let page_count: usize;
    let text_content: String;

    // Use kreuzberg for supported document formats (PDF, EPUB, DOCX, MOBI, etc.)
    if crate::ingestion::DocumentExtractor::is_supported(&path_buf) {
        use crate::ingestion::DocumentExtractor;

        // Use OCR-enabled extractor for scanned documents
        let extractor = DocumentExtractor::with_ocr();

        let _ = app.emit("ingest-progress", IngestProgress {
            stage: "parsing".to_string(),
            progress: 0.2,
            message: format!("Extracting {} with kreuzberg...", format_name),
            source_name: source_name.clone(),
        });

        // Clone for callback
        let app_handle = app.clone();
        let source_name_cb = source_name.clone();

        let progress_callback = move |p: f32, msg: &str| {
            // Map 0.0-1.0 from extractor to 0.2-0.4 in overall progress
            // Actually extractor uses arbitrary 0.2-0.8 range in OCR,
            // so we can map it to 0.2 + (p * 0.2) roughly?
            // Let's trust the extractor's p if it's 0-1 and map it to a sub-range
            // or just log it.
            // But wait, the extractor's OCR loop emits p from 0.2 to 0.8
            // Our overall progress for "parsing" is roughly 0.0 to 0.4.
            // So let's map generic p to 0.2 + (p * 0.2)

            let scaled_progress = 0.2 + (p * 0.2);

            let _ = app_handle.emit("ingest-progress", IngestProgress {
                stage: "parsing".to_string(),
                progress: scaled_progress,
                message: msg.to_string(),
                source_name: source_name_cb.clone(),
            });
        };

        let extracted = extractor.extract(&path_buf, Some(progress_callback))
            .await
            .map_err(|e| format!("Document extraction failed: {}", e))?;

        page_count = extracted.page_count;
        text_content = extracted.content;

        let _ = app.emit("ingest-progress", IngestProgress {
            stage: "parsing".to_string(),
            progress: 0.4,
            message: format!("Extracted {} pages ({} chars)", page_count, text_content.len()),
            source_name: source_name.clone(),
        });
    } else if extension == "txt" || extension == "md" || extension == "markdown" {
        // Plain text files - read directly
        text_content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        page_count = text_content.lines().count() / 50;

        let _ = app.emit("ingest-progress", IngestProgress {
            stage: "parsing".to_string(),
            progress: 0.4,
            message: format!("Loaded {} characters", text_content.len()),
            source_name: source_name.clone(),
        });
    } else {
        // Try to read as text for other formats
        text_content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Unsupported format or failed to read: {}", e))?;
        page_count = 1;

        let _ = app.emit("ingest-progress", IngestProgress {
            stage: "parsing".to_string(),
            progress: 0.4,
            message: "File loaded".to_string(),
            source_name: source_name.clone(),
        });
    }

    // Stage 2: Chunking (40-60%)
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "chunking".to_string(),
        progress: 0.5,
        message: format!("Chunking {} characters...", text_content.len()),
        source_name: source_name.clone(),
    });

    // Stage 3: Indexing (60-100%)
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "indexing".to_string(),
        progress: 0.6,
        message: "Indexing to Meilisearch...".to_string(),
        source_name: source_name.clone(),
    });

    let pipeline = MeilisearchPipeline::default();
    let result = pipeline.ingest_text(
        &state.search_client,
        &text_content,
        &source_name,
        &source_type,
        None,
        None,
    )
    .await
    .map_err(|e| e.to_string())?;

    // Save document metadata
    let library_doc = crate::core::search_client::LibraryDocumentMetadata {
        id: uuid::Uuid::new_v4().to_string(),
        name: source_name.clone(),
        source_type: source_type.clone(),
        file_path: Some(path.clone()),
        page_count: page_count as u32,
        chunk_count: result.total_chunks as u32,
        character_count: text_content.len() as u64,
        content_index: result.index_used.clone(),
        status: "ready".to_string(),
        error_message: None,
        ingested_at: chrono::Utc::now().to_rfc3339(),
        // TTRPG metadata - user-editable, not set during ingestion
        game_system: None,
        setting: None,
        content_type: None,
        publisher: None,
    };

    if let Err(e) = state.search_client.save_library_document(&library_doc).await {
        log::warn!("Failed to save library document metadata: {}. Document indexed but may not persist.", e);
    }

    // Done!
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "complete".to_string(),
        progress: 1.0,
        message: format!("Indexed {} chunks", result.total_chunks),
        source_name: source_name.clone(),
    });

    Ok(IngestResult {
        page_count,
        character_count: text_content.len(),
        source_name: result.source,
    })
}

// ============================================================================
// Voice Synthesis Commands
// ============================================================================

// configure_voice and synthesize_voice removed in favor of speak command
// ============================================================================
// Audio Playback Commands
// ============================================================================

// Note: Audio playback uses rodio which requires the OutputStream to stay
// on the same thread. For Tauri, we handle this by creating the audio player
// on-demand in the main thread context.

#[tauri::command]
pub fn get_audio_volumes() -> AudioVolumes {
    AudioVolumes::default()
}

#[tauri::command]
pub fn get_sfx_categories() -> Vec<String> {
    crate::core::audio::get_sfx_categories()
}

// ============================================================================
// Credential Commands
// ============================================================================

#[tauri::command]
pub fn save_api_key(
    provider: String,
    api_key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let key_name = format!("{}_api_key", provider);
    state.credentials.store_secret(&key_name, &api_key)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_api_key(provider: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    let key_name = format!("{}_api_key", provider);
    match state.credentials.get_secret(&key_name) {
        Ok(key) => Ok(Some(key)),
        Err(crate::core::credentials::CredentialError::NotFound(_)) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn delete_api_key(provider: String, state: State<'_, AppState>) -> Result<(), String> {
    let key_name = format!("{}_api_key", provider);
    state.credentials.delete_secret(&key_name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_stored_providers(state: State<'_, AppState>) -> Vec<String> {
    state.credentials.list_llm_providers()
}

// ============================================================================
// Utility Commands
// ============================================================================

#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub fn get_app_system_info() -> AppSystemInfo {
    AppSystemInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppSystemInfo {
    pub os: String,
    pub arch: String,
    pub version: String,
}

// ============================================================================
// Voice Preset Commands
// ============================================================================

// get_voice_presets removed


// ============================================================================
// Voice Commands
// ============================================================================

use crate::core::voice::Voice;

/// List available OpenAI TTS voices (static list, no API call needed)
#[tauri::command]
pub fn list_openai_voices() -> Vec<Voice> {
    crate::core::voice::providers::openai::get_openai_voices()
}

/// List available OpenAI TTS models
#[tauri::command]
pub fn list_openai_tts_models() -> Vec<(String, String)> {
    crate::core::voice::providers::openai::get_openai_tts_models()
}

/// List available ElevenLabs voices (requires API key)
#[tauri::command]
pub async fn list_elevenlabs_voices(api_key: String) -> Result<Vec<Voice>, String> {
    use crate::core::voice::ElevenLabsConfig;
    use crate::core::voice::providers::elevenlabs::ElevenLabsProvider;
    use crate::core::voice::providers::VoiceProvider;

    let provider = ElevenLabsProvider::new(ElevenLabsConfig {
        api_key,
        model_id: None,
    });

    provider.list_voices().await.map_err(|e| e.to_string())
}

/// List all voices from the currently configured voice provider
#[tauri::command]
pub async fn list_available_voices(state: State<'_, AppState>) -> Result<Vec<Voice>, String> {
    // Clone the config to avoid holding the lock across await
    let config = {
        let manager = state.voice_manager.read().await;
        manager.get_config().clone()
    };

    // Create a new manager with the config for the async call
    let manager = VoiceManager::new(config);
    manager.list_voices().await.map_err(|e| e.to_string())
}

/// Audio data returned from speak command for frontend playback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakResult {
    /// Base64-encoded audio data
    pub audio_data: String,
    /// Audio format (e.g., "wav")
    pub format: String,
}

#[tauri::command]
pub async fn speak(text: String, app_handle: tauri::AppHandle) -> Result<Option<SpeakResult>, String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

    // 1. Load voice config from disk (saved by the settings UI)
    let config = load_voice_config_disk(&app_handle)
        .unwrap_or_else(|| {
            log::warn!("No voice config found on disk, using default");
            VoiceConfig::default()
        });

    // Check if voice is disabled
    if matches!(config.provider, VoiceProviderType::Disabled) {
        log::info!("Voice synthesis disabled, skipping speak request");
        return Ok(None);
    }

    // Get the default voice ID from config
    let voice_id = config.default_voice_id.clone()
        .unwrap_or_else(|| "default".to_string());

    log::info!("Speaking with provider {:?}, voice_id: '{}', piper_config: {:?}",
        config.provider, voice_id, config.piper);

    let manager = VoiceManager::new(config);

    // 2. Synthesize (async)
    let request = SynthesisRequest {
        text,
        voice_id,
        settings: None,
        output_format: OutputFormat::Wav, // Piper outputs WAV natively
    };

    match manager.synthesize(request).await {
        Ok(result) => {
            // Read bytes from file
            let bytes = std::fs::read(&result.audio_path).map_err(|e| e.to_string())?;

            // Return base64-encoded audio for frontend playback
            let audio_data = BASE64.encode(&bytes);
            log::info!("Synthesis complete, returning {} bytes as base64", bytes.len());

            Ok(Some(SpeakResult {
                audio_data,
                format: "wav".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Synthesis failed: {}", e);
            Err(format!("Voice synthesis failed: {}", e))
        }
    }
}

#[tauri::command]
pub async fn transcribe_audio(
    path: String,
    state: State<'_, AppState>,
) -> Result<crate::core::transcription::TranscriptionResult, String> {
    // 1. Check Config for OpenAI API Key
    let api_key = if let Some(config) = state.llm_config.read().unwrap().clone() {
        match config {
            LLMConfig::OpenAI { api_key, .. } => api_key,
            _ => return Err("Transcription requires OpenAI configuration (for now)".to_string()),
        }
    } else {
        return Err("LLM not configured".to_string());
    };

    if api_key.is_empty() || api_key.starts_with('*') {
        // Try getting from credentials if masked/empty
        // (Assuming standard key name 'openai_api_key')
        let creds = state.credentials.get_secret("openai_api_key")
            .map_err(|_| "OpenAI API Key not found/configured".to_string())?;
        if creds.is_empty() {
             return Err("OpenAI API Key is empty".to_string());
        }
    }

    // Unmasking logic is a bit duplicated here, ideally use a helper.
    // For now, let's rely on stored secret if the config one is masked.
    let effective_key = if api_key.starts_with('*') {
         state.credentials.get_secret("openai_api_key")
            .map_err(|_| "Invalid API Key state".to_string())?
    } else {
        api_key
    };

    // 2. Call Service
    let service = crate::core::transcription::TranscriptionService::new();
    service.transcribe_openai(&effective_key, Path::new(&path))
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// NPC Conversation Commands
// ============================================================================

#[tauri::command]
pub async fn list_npc_conversations(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<NpcConversation>, String> {
    state.database.list_npc_conversations(&campaign_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_npc_conversation(
    npc_id: String,
    state: State<'_, AppState>,
) -> Result<NpcConversation, String> {
    match state.database.get_npc_conversation(&npc_id).await.map_err(|e| e.to_string())? {
        Some(c) => Ok(c),
        None => Err(format!("Conversation not found for NPC {}", npc_id)),
    }
}

#[tauri::command]
pub async fn add_npc_message(
    npc_id: String,
    content: String,
    role: String,
    parent_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<ConversationMessage, String> {
    // 1. Get Conversation - strict requirement, must exist
    // (In future we might auto-create, but we need campaign_id)
    let mut conv = match state.database.get_npc_conversation(&npc_id).await.map_err(|e| e.to_string())? {
        Some(c) => c,
        None => return Err("Conversation does not exist.".to_string()),
    };

    // 2. Add Message
    let message = ConversationMessage {
        id: uuid::Uuid::new_v4().to_string(),
        role,
        content,
        parent_message_id: parent_id,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let mut messages: Vec<ConversationMessage> = serde_json::from_str(&conv.messages_json)
        .unwrap_or_default();
    messages.push(message.clone());

    conv.messages_json = serde_json::to_string(&messages).map_err(|e| e.to_string())?;
    conv.last_message_at = message.created_at.clone();
    conv.unread_count += 1;

    // 3. Save
    state.database.save_npc_conversation(&conv).await.map_err(|e| e.to_string())?;

    Ok(message)
}

#[tauri::command]
pub async fn mark_npc_read(
    npc_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Some(mut conv) = state.database.get_npc_conversation(&npc_id).await.map_err(|e| e.to_string())? {
        conv.unread_count = 0;
        state.database.save_npc_conversation(&conv).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NpcSummary {
    pub id: String,
    pub name: String,
    pub role: String,
    pub avatar_url: String,
    pub status: String,
    pub last_message: String,
    pub unread_count: u32,
    pub last_active: String,
}

#[tauri::command]
pub async fn list_npc_summaries(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<NpcSummary>, String> {
    // 1. Get NPCs
    let npcs = state.database.list_npcs(Some(&campaign_id)).await.map_err(|e| e.to_string())?;

    let mut summaries = Vec::new();

    // 2. Build summaries
    for npc in npcs {
        let conv = state.database.get_npc_conversation(&npc.id).await.map_err(|e| e.to_string())?;

        let (last_message, unread_count, last_active) = if let Some(c) = conv {
             let msgs: Vec<ConversationMessage> = serde_json::from_str(&c.messages_json).unwrap_or_default();
             let last_text = msgs.last().map(|m| m.content.clone()).unwrap_or_default();
             // Truncate
             let truncated = if last_text.len() > 50 {
                 format!("{}...", &last_text[0..50])
             } else {
                 last_text
             };
             (truncated, c.unread_count, c.last_message_at)
        } else {
             ("".to_string(), 0, "".to_string())
        };

        summaries.push(NpcSummary {
            id: npc.id,
            name: npc.name.clone(),
            role: npc.role,
            avatar_url: npc.name.chars().next().unwrap_or('?').to_string(),
            status: "online".to_string(), // Placeholder
            last_message,
            unread_count,
            last_active,
        });
    }

    Ok(summaries)
}

#[tauri::command]
pub async fn reply_as_npc(
    npc_id: String,
    state: State<'_, AppState>,
) -> Result<ConversationMessage, String> {
    // 1. Load NPC
    let npc = state.database.get_npc(&npc_id).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "NPC not found".to_string())?;

    // 2. Load Personality
    let system_prompt = if let Some(pid) = &npc.personality_id {
         match state.database.get_personality(pid).await.map_err(|e| e.to_string())? {
             Some(p) => {
                 let profile: crate::core::personality::PersonalityProfile = serde_json::from_str(&p.data_json)
                     .map_err(|e| format!("Invalid personality data: {}", e))?;
                 profile.to_system_prompt()
             },
             None => "You are an NPC. Respond in character.".to_string(),
         }
    } else {
        "You are an NPC. Respond in character.".to_string()
    };

    // 3. Load Conversation History
    let conv = state.database.get_npc_conversation(&npc.id).await.map_err(|e| e.to_string())?
         .ok_or_else(|| "Conversation not found".to_string())?;
    let history: Vec<ConversationMessage> = serde_json::from_str(&conv.messages_json).unwrap_or_default();

    // 4. Construct LLM Request
    let llm_messages: Vec<crate::core::llm::ChatMessage> = history.iter().map(|m| crate::core::llm::ChatMessage {
        role: if m.role == "user" { crate::core::llm::MessageRole::User } else { crate::core::llm::MessageRole::Assistant },
        content: m.content.clone(),
        images: None,
        name: None,
        tool_calls: None,
        tool_call_id: None,
    }).collect();

    if llm_messages.is_empty() {
        return Err("No context to reply to.".to_string());
    }

    // 5. Call LLM
    let config = state.llm_config.read().unwrap().clone().ok_or("LLM not configured")?;
    let client = crate::core::llm::LLMClient::new(config);

    let req = crate::core::llm::ChatRequest {
        messages: llm_messages,
        system_prompt: Some(system_prompt),
        temperature: Some(0.8),
        max_tokens: Some(250),
        provider: None,
        tools: None,
        tool_choice: None,
    };

    let resp = client.chat(req).await.map_err(|e| e.to_string())?;

    // 6. Save Reply
    let message = ConversationMessage {
        id: uuid::Uuid::new_v4().to_string(),
        role: "assistant".to_string(), // standard role
        content: resp.content,
        parent_message_id: history.last().map(|m| m.id.clone()),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let mut conv_update = conv.clone();
    let mut msgs = history;
    msgs.push(message.clone());
    conv_update.messages_json = serde_json::to_string(&msgs).map_err(|e| e.to_string())?;
    conv_update.last_message_at = message.created_at.clone();
    conv_update.unread_count += 1;

    state.database.save_npc_conversation(&conv_update).await.map_err(|e| e.to_string())?;

    Ok(message)
}


// ============================================================================
// Theme Commands
// ============================================================================



// ============================================================================
// Voice Queue Commands
// ============================================================================



#[tauri::command]
pub async fn queue_voice(
    text: String,
    voice_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<QueuedVoice, String> {
    // 1. Determine Voice ID
    let vid = voice_id.unwrap_or_else(|| "default".to_string());

    // 2. Add to Queue
    let item = {
        let mut manager = state.voice_manager.write().await;
        manager.add_to_queue(text, vid)
    };

    // 3. Trigger Processing (Background)
    match process_voice_queue(state).await {
        Ok(_) => {},
        Err(e) => eprintln!("Failed to trigger voice queue processing: {}", e),
    }

    Ok(item)
}

#[tauri::command]
pub async fn get_voice_queue(state: State<'_, AppState>) -> Result<Vec<QueuedVoice>, String> {
    let manager = state.voice_manager.read().await;
    Ok(manager.get_queue())
}

#[tauri::command]
pub async fn cancel_voice(queue_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut manager = state.voice_manager.write().await;
    manager.remove_from_queue(&queue_id);
    Ok(())
}

/// Internal helper to process the queue
async fn process_voice_queue(state: State<'_, AppState>) -> Result<(), String> {
    let vm_clone = state.voice_manager.clone();

    // Spawn a detached task
    tauri::async_runtime::spawn(async move {
        // We loop until queue is empty or processing fails
        loop {
            // 1. Get next pending (Read Lock)
            let next_item = {
                let manager = vm_clone.read().await;
                if manager.is_playing {
                    None
                } else {
                    manager.get_next_pending()
                }
            };

            if let Some(item) = next_item {
                // 2. Mark Processing
                {
                    let mut manager = vm_clone.write().await;
                    manager.update_status(&item.id, VoiceStatus::Processing);
                }

                // 3. Synthesize
                let req = SynthesisRequest {
                    text: item.text.clone(),
                    voice_id: item.voice_id.clone(),
                    settings: None,
                    output_format: OutputFormat::Mp3, // Default
                };

                // Perform synthesis without holding lock
                let result = {
                    let manager = vm_clone.read().await;
                    manager.synthesize(req).await
                };

                match result {
                    Ok(res) => {
                        // 4. Synthesized. Now Play.
                        // Read file
                        if let Ok(audio_data) = tokio::fs::read(&res.audio_path).await {
                             // Mark Playing
                            {
                                let mut manager = vm_clone.write().await;
                                manager.update_status(&item.id, VoiceStatus::Playing);
                                manager.is_playing = true;
                            }

                            // Play (Blocking for now, inside spawn)
                            let vm_for_clos = vm_clone.clone();
                            let play_result = tokio::task::spawn_blocking(move || {
                                let manager = vm_for_clos.blocking_read();
                                manager.play_audio(audio_data)
                            }).await;

                            let play_result = match play_result {
                                Ok(inner) => inner.map_err(|e| e.to_string()),
                                Err(e) => Err(e.to_string()),
                            };

                            // Mark Completed
                            {
                                let mut manager = vm_clone.write().await;
                                manager.is_playing = false;
                                manager.update_status(&item.id, if play_result.is_ok() {
                                    VoiceStatus::Completed
                                } else {
                                    VoiceStatus::Failed("Playback failed".into())
                                });
                            }
                        } else {
                             // File read failed
                            let mut manager = vm_clone.write().await;
                            manager.update_status(&item.id, VoiceStatus::Failed("Could not read audio file".into()));
                        }
                    }
                    Err(e) => {
                        // Synthesis Failed
                        let mut manager = vm_clone.write().await;
                        manager.update_status(&item.id, VoiceStatus::Failed(e.to_string()));
                    }
                }
            } else {
                // No more items
                break;
            }
        }
    });

    Ok(())
}

// ============================================================================
// Campaign Versioning Commands (TASK-006)
// ============================================================================

use crate::core::campaign::versioning::{
    CampaignVersion, VersionType, CampaignDiff, VersionSummary,
};
use crate::core::campaign::world_state::{
    WorldState, WorldEvent, WorldEventType, EventImpact, LocationState,
    LocationCondition, InGameDate, CalendarConfig,
};
use crate::core::campaign::relationships::{
    EntityRelationship, RelationshipType, EntityType, RelationshipStrength,
    EntityGraph, RelationshipSummary,
};

/// Create a new campaign version
#[tauri::command]
pub fn create_campaign_version(
    campaign_id: String,
    description: String,
    version_type: String,
    state: State<'_, AppState>,
) -> Result<VersionSummary, String> {
    // Get current campaign data as JSON
    let campaign = state.campaign_manager.get_campaign(&campaign_id)
        .ok_or_else(|| "Campaign not found".to_string())?;

    let data_snapshot = serde_json::to_string(&campaign)
        .map_err(|e| format!("Failed to serialize campaign: {}", e))?;

    let vtype = match version_type.as_str() {
        "auto" => VersionType::Auto,
        "milestone" => VersionType::Milestone,
        "pre_rollback" => VersionType::PreRollback,
        "import" => VersionType::Import,
        _ => VersionType::Manual,
    };

    let version = state.version_manager.create_version(
        &campaign_id,
        &description,
        vtype,
        &data_snapshot,
    ).map_err(|e| e.to_string())?;

    Ok(VersionSummary::from(&version))
}

/// List all versions for a campaign
#[tauri::command]
pub fn list_campaign_versions(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<VersionSummary>, String> {
    Ok(state.version_manager.list_versions(&campaign_id))
}

/// Get a specific version
#[tauri::command]
pub fn get_campaign_version(
    campaign_id: String,
    version_id: String,
    state: State<'_, AppState>,
) -> Result<CampaignVersion, String> {
    state.version_manager.get_version(&campaign_id, &version_id)
        .ok_or_else(|| "Version not found".to_string())
}

/// Compare two versions
#[tauri::command]
pub fn compare_campaign_versions(
    campaign_id: String,
    from_version_id: String,
    to_version_id: String,
    state: State<'_, AppState>,
) -> Result<CampaignDiff, String> {
    state.version_manager.compare_versions(&campaign_id, &from_version_id, &to_version_id)
        .map_err(|e| e.to_string())
}

/// Rollback a campaign to a previous version
#[tauri::command]
pub fn rollback_campaign(
    campaign_id: String,
    version_id: String,
    state: State<'_, AppState>,
) -> Result<Campaign, String> {
    // Get current campaign data for pre-rollback snapshot
    let current = state.campaign_manager.get_campaign(&campaign_id)
        .ok_or_else(|| "Campaign not found".to_string())?;

    let current_json = serde_json::to_string(&current)
        .map_err(|e| format!("Failed to serialize current state: {}", e))?;

    // Prepare rollback (creates pre-rollback snapshot and returns target data)
    let target_data = state.version_manager.prepare_rollback(&campaign_id, &version_id, &current_json)
        .map_err(|e| e.to_string())?;

    // Deserialize and restore campaign
    let restored: Campaign = serde_json::from_str(&target_data)
        .map_err(|e| format!("Failed to deserialize version data: {}", e))?;

    state.campaign_manager.update_campaign(restored.clone(), false)
        .map_err(|e| e.to_string())?;

    Ok(restored)
}

/// Delete a version
#[tauri::command]
pub fn delete_campaign_version(
    campaign_id: String,
    version_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.version_manager.delete_version(&campaign_id, &version_id)
        .map_err(|e| e.to_string())
}

/// Add a tag to a version
#[tauri::command]
pub fn add_version_tag(
    campaign_id: String,
    version_id: String,
    tag: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.version_manager.add_tag(&campaign_id, &version_id, &tag)
        .map_err(|e| e.to_string())
}

/// Mark a version as a milestone
#[tauri::command]
pub fn mark_version_milestone(
    campaign_id: String,
    version_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.version_manager.mark_as_milestone(&campaign_id, &version_id)
        .map_err(|e| e.to_string())
}

// ============================================================================
// World State Commands (TASK-007)
// ============================================================================

/// Get world state for a campaign
#[tauri::command]
pub fn get_world_state(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<WorldState, String> {
    Ok(state.world_state_manager.get_or_create(&campaign_id))
}

/// Update world state
#[tauri::command]
pub fn update_world_state(
    world_state: WorldState,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.world_state_manager.update_state(world_state)
        .map_err(|e| e.to_string())
}

/// Set current in-game date
#[tauri::command]
pub fn set_in_game_date(
    campaign_id: String,
    date: InGameDate,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.world_state_manager.set_current_date(&campaign_id, date)
        .map_err(|e| e.to_string())
}

/// Advance in-game date by days
#[tauri::command]
pub fn advance_in_game_date(
    campaign_id: String,
    days: i32,
    state: State<'_, AppState>,
) -> Result<InGameDate, String> {
    state.world_state_manager.advance_date(&campaign_id, days)
        .map_err(|e| e.to_string())
}

/// Get current in-game date
#[tauri::command]
pub fn get_in_game_date(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<InGameDate, String> {
    state.world_state_manager.get_current_date(&campaign_id)
        .map_err(|e| e.to_string())
}

/// Add a world event
#[tauri::command]
pub fn add_world_event(
    campaign_id: String,
    title: String,
    description: String,
    date: InGameDate,
    event_type: String,
    impact: String,
    state: State<'_, AppState>,
) -> Result<WorldEvent, String> {
    let etype = match event_type.as_str() {
        "combat" => WorldEventType::Combat,
        "political" => WorldEventType::Political,
        "natural" => WorldEventType::Natural,
        "economic" => WorldEventType::Economic,
        "religious" => WorldEventType::Religious,
        "magical" => WorldEventType::Magical,
        "social" => WorldEventType::Social,
        "personal" => WorldEventType::Personal,
        "discovery" => WorldEventType::Discovery,
        "session" => WorldEventType::Session,
        _ => WorldEventType::Custom(event_type),
    };

    let eimpact = match impact.as_str() {
        "personal" => EventImpact::Personal,
        "local" => EventImpact::Local,
        "regional" => EventImpact::Regional,
        "national" => EventImpact::National,
        "global" => EventImpact::Global,
        "cosmic" => EventImpact::Cosmic,
        _ => EventImpact::Local,
    };

    let event = WorldEvent::new(&campaign_id, &title, &description, date)
        .with_type(etype)
        .with_impact(eimpact);

    state.world_state_manager.add_event(&campaign_id, event)
        .map_err(|e| e.to_string())
}

/// List world events
#[tauri::command]
pub fn list_world_events(
    campaign_id: String,
    event_type: Option<String>,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<WorldEvent>, String> {
    let etype = event_type.map(|et| match et.as_str() {
        "combat" => WorldEventType::Combat,
        "political" => WorldEventType::Political,
        "natural" => WorldEventType::Natural,
        "economic" => WorldEventType::Economic,
        "religious" => WorldEventType::Religious,
        "magical" => WorldEventType::Magical,
        "social" => WorldEventType::Social,
        "personal" => WorldEventType::Personal,
        "discovery" => WorldEventType::Discovery,
        "session" => WorldEventType::Session,
        _ => WorldEventType::Custom(et),
    });

    Ok(state.world_state_manager.list_events(&campaign_id, etype, limit))
}

/// Delete a world event
#[tauri::command]
pub fn delete_world_event(
    campaign_id: String,
    event_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.world_state_manager.delete_event(&campaign_id, &event_id)
        .map_err(|e| e.to_string())
}

/// Set location state
#[tauri::command]
pub fn set_location_state(
    campaign_id: String,
    location: LocationState,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.world_state_manager.set_location_state(&campaign_id, location)
        .map_err(|e| e.to_string())
}

/// Get location state
#[tauri::command]
pub fn get_location_state(
    campaign_id: String,
    location_id: String,
    state: State<'_, AppState>,
) -> Result<Option<LocationState>, String> {
    Ok(state.world_state_manager.get_location_state(&campaign_id, &location_id))
}

/// List all locations
#[tauri::command]
pub fn list_locations(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<LocationState>, String> {
    Ok(state.world_state_manager.list_locations(&campaign_id))
}

/// Update location condition
#[tauri::command]
pub fn update_location_condition(
    campaign_id: String,
    location_id: String,
    condition: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cond = match condition.as_str() {
        "pristine" => LocationCondition::Pristine,
        "normal" => LocationCondition::Normal,
        "damaged" => LocationCondition::Damaged,
        "ruined" => LocationCondition::Ruined,
        "destroyed" => LocationCondition::Destroyed,
        "occupied" => LocationCondition::Occupied,
        "abandoned" => LocationCondition::Abandoned,
        "under_siege" => LocationCondition::UnderSiege,
        "cursed" => LocationCondition::Cursed,
        "blessed" => LocationCondition::Blessed,
        _ => LocationCondition::Custom(condition),
    };

    state.world_state_manager.update_location_condition(&campaign_id, &location_id, cond)
        .map_err(|e| e.to_string())
}

/// Set a custom field on world state
#[tauri::command]
pub fn set_world_custom_field(
    campaign_id: String,
    key: String,
    value: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.world_state_manager.set_custom_field(&campaign_id, &key, value)
        .map_err(|e| e.to_string())
}

/// Get a custom field from world state
#[tauri::command]
pub fn get_world_custom_field(
    campaign_id: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>, String> {
    Ok(state.world_state_manager.get_custom_field(&campaign_id, &key))
}

/// Get all custom fields
#[tauri::command]
pub fn list_world_custom_fields(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<HashMap<String, serde_json::Value>, String> {
    Ok(state.world_state_manager.list_custom_fields(&campaign_id))
}

/// Set calendar configuration
#[tauri::command]
pub fn set_calendar_config(
    campaign_id: String,
    config: CalendarConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.world_state_manager.set_calendar_config(&campaign_id, config)
        .map_err(|e| e.to_string())
}

/// Get calendar configuration
#[tauri::command]
pub fn get_calendar_config(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<Option<CalendarConfig>, String> {
    Ok(state.world_state_manager.get_calendar_config(&campaign_id))
}

// ============================================================================
// Entity Relationship Commands (TASK-009)
// ============================================================================

/// Create an entity relationship
#[tauri::command]
pub fn create_entity_relationship(
    campaign_id: String,
    source_id: String,
    source_type: String,
    source_name: String,
    target_id: String,
    target_type: String,
    target_name: String,
    relationship_type: String,
    strength: Option<String>,
    description: Option<String>,
    state: State<'_, AppState>,
) -> Result<EntityRelationship, String> {
    let src_type = parse_entity_type(&source_type);
    let tgt_type = parse_entity_type(&target_type);
    let rel_type = parse_relationship_type(&relationship_type);
    let str_level = strength.map(|s| parse_relationship_strength(&s)).unwrap_or_default();

    let mut relationship = EntityRelationship::new(
        &campaign_id,
        &source_id,
        src_type,
        &source_name,
        &target_id,
        tgt_type,
        &target_name,
        rel_type,
    ).with_strength(str_level);

    if let Some(desc) = description {
        relationship = relationship.with_description(&desc);
    }

    state.relationship_manager.create_relationship(relationship)
        .map_err(|e| e.to_string())
}

/// Get a relationship by ID
#[tauri::command]
pub fn get_entity_relationship(
    campaign_id: String,
    relationship_id: String,
    state: State<'_, AppState>,
) -> Result<Option<EntityRelationship>, String> {
    Ok(state.relationship_manager.get_relationship(&campaign_id, &relationship_id))
}

/// Update a relationship
#[tauri::command]
pub fn update_entity_relationship(
    relationship: EntityRelationship,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.relationship_manager.update_relationship(relationship)
        .map_err(|e| e.to_string())
}

/// Delete a relationship
#[tauri::command]
pub fn delete_entity_relationship(
    campaign_id: String,
    relationship_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.relationship_manager.delete_relationship(&campaign_id, &relationship_id)
        .map_err(|e| e.to_string())
}

/// List all relationships for a campaign
#[tauri::command]
pub fn list_entity_relationships(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<RelationshipSummary>, String> {
    Ok(state.relationship_manager.list_relationships(&campaign_id))
}

/// Get relationships for a specific entity
#[tauri::command]
pub fn get_relationships_for_entity(
    campaign_id: String,
    entity_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<EntityRelationship>, String> {
    Ok(state.relationship_manager.get_entity_relationships(&campaign_id, &entity_id))
}

/// Get relationships between two entities
#[tauri::command]
pub fn get_relationships_between_entities(
    campaign_id: String,
    entity_a: String,
    entity_b: String,
    state: State<'_, AppState>,
) -> Result<Vec<EntityRelationship>, String> {
    Ok(state.relationship_manager.get_relationships_between(&campaign_id, &entity_a, &entity_b))
}

/// Get the full entity graph for visualization
#[tauri::command]
pub fn get_entity_graph(
    campaign_id: String,
    include_inactive: Option<bool>,
    state: State<'_, AppState>,
) -> Result<EntityGraph, String> {
    Ok(state.relationship_manager.get_entity_graph(&campaign_id, include_inactive.unwrap_or(false)))
}

/// Get ego graph centered on an entity
#[tauri::command]
pub fn get_ego_graph(
    campaign_id: String,
    entity_id: String,
    depth: Option<usize>,
    state: State<'_, AppState>,
) -> Result<EntityGraph, String> {
    Ok(state.relationship_manager.get_ego_graph(&campaign_id, &entity_id, depth.unwrap_or(2)))
}

// Helper functions for parsing enum types from strings
fn parse_entity_type(s: &str) -> EntityType {
    match s.to_lowercase().as_str() {
        "pc" | "player" => EntityType::PC,
        "npc" => EntityType::NPC,
        "location" => EntityType::Location,
        "faction" => EntityType::Faction,
        "item" => EntityType::Item,
        "event" => EntityType::Event,
        "quest" => EntityType::Quest,
        "deity" => EntityType::Deity,
        "creature" => EntityType::Creature,
        _ => EntityType::Custom(s.to_string()),
    }
}

fn parse_relationship_type(s: &str) -> RelationshipType {
    match s.to_lowercase().as_str() {
        "ally" => RelationshipType::Ally,
        "enemy" => RelationshipType::Enemy,
        "romantic" => RelationshipType::Romantic,
        "family" => RelationshipType::Family,
        "mentor" => RelationshipType::Mentor,
        "acquaintance" => RelationshipType::Acquaintance,
        "employee" => RelationshipType::Employee,
        "business_partner" => RelationshipType::BusinessPartner,
        "patron" => RelationshipType::Patron,
        "teacher" => RelationshipType::Teacher,
        "protector" => RelationshipType::Protector,
        "member_of" => RelationshipType::MemberOf,
        "leader_of" => RelationshipType::LeaderOf,
        "allied_with" => RelationshipType::AlliedWith,
        "at_war_with" => RelationshipType::AtWarWith,
        "vassal_of" => RelationshipType::VassalOf,
        "located_at" => RelationshipType::LocatedAt,
        "connected_to" => RelationshipType::ConnectedTo,
        "part_of" => RelationshipType::PartOf,
        "controls" => RelationshipType::Controls,
        "owns" => RelationshipType::Owns,
        "seeks" => RelationshipType::Seeks,
        "created" => RelationshipType::Created,
        "destroyed" => RelationshipType::Destroyed,
        "quest_giver" => RelationshipType::QuestGiver,
        "quest_target" => RelationshipType::QuestTarget,
        "related_to" => RelationshipType::RelatedTo,
        "worships" => RelationshipType::Worships,
        "blessed_by" => RelationshipType::BlessedBy,
        "cursed_by" => RelationshipType::CursedBy,
        _ => RelationshipType::Custom(s.to_string()),
    }
}

fn parse_relationship_strength(s: &str) -> RelationshipStrength {
    match s.to_lowercase().as_str() {
        "weak" => RelationshipStrength::Weak,
        "moderate" => RelationshipStrength::Moderate,
        "strong" => RelationshipStrength::Strong,
        "unbreakable" => RelationshipStrength::Unbreakable,
        _ => {
            if let Ok(v) = s.parse::<u8>() {
                RelationshipStrength::Custom(v.min(100))
            } else {
                RelationshipStrength::Moderate
            }
        }
    }
}

// ============================================================================
// TASK-022: Usage Tracking Commands
// ============================================================================

use crate::core::usage::{
    UsageTracker, UsageStats, CostBreakdown, BudgetLimit, BudgetStatus,
    ProviderUsage,
};

/// Get total usage statistics
#[tauri::command]
pub fn get_usage_stats(state: State<'_, UsageTrackerState>) -> UsageStats {
    state.tracker.get_total_stats()
}

/// Get usage statistics for a time period (in hours)
#[tauri::command]
pub fn get_usage_by_period(hours: i64, state: State<'_, UsageTrackerState>) -> UsageStats {
    state.tracker.get_stats_by_period(hours)
}

/// Get detailed cost breakdown
#[tauri::command]
pub fn get_cost_breakdown(hours: Option<i64>, state: State<'_, UsageTrackerState>) -> CostBreakdown {
    state.tracker.get_cost_breakdown(hours)
}

/// Get current budget status for all configured limits
#[tauri::command]
pub fn get_budget_status(state: State<'_, UsageTrackerState>) -> Vec<BudgetStatus> {
    state.tracker.check_budget_status()
}

/// Set a budget limit
#[tauri::command]
pub fn set_budget_limit(limit: BudgetLimit, state: State<'_, UsageTrackerState>) -> Result<(), String> {
    state.tracker.set_budget_limit(limit);
    Ok(())
}

/// Get usage for a specific provider
#[tauri::command]
pub fn get_provider_usage(provider: String, state: State<'_, UsageTrackerState>) -> ProviderUsage {
    state.tracker.get_provider_stats(&provider)
}

/// Reset usage tracking session
#[tauri::command]
pub fn reset_usage_session(state: State<'_, UsageTrackerState>) {
    state.tracker.reset_session();
}

// ============================================================================
// TASK-023: Search Analytics Commands
// ============================================================================

use crate::core::search_analytics::{
    SearchAnalytics, AnalyticsSummary, PopularQuery, CacheStats,
    ResultSelection, SearchRecord, DbSearchAnalytics,
};

// --- In-Memory Analytics (Fast, Session-Only) ---

/// Get search analytics summary for a time period (in-memory)
#[tauri::command]
pub fn get_search_analytics(hours: i64, state: State<'_, SearchAnalyticsState>) -> AnalyticsSummary {
    state.analytics.get_summary(hours)
}

/// Get popular queries with detailed stats (in-memory)
#[tauri::command]
pub fn get_popular_queries(limit: usize, state: State<'_, SearchAnalyticsState>) -> Vec<PopularQuery> {
    state.analytics.get_popular_queries_detailed(limit)
}

/// Get cache statistics (in-memory)
#[tauri::command]
pub fn get_cache_stats(state: State<'_, SearchAnalyticsState>) -> CacheStats {
    state.analytics.get_cache_stats()
}

/// Get trending queries (in-memory)
#[tauri::command]
pub fn get_trending_queries(limit: usize, state: State<'_, SearchAnalyticsState>) -> Vec<String> {
    state.analytics.get_trending_queries(limit)
}

/// Get queries with zero results (in-memory)
#[tauri::command]
pub fn get_zero_result_queries(hours: i64, state: State<'_, SearchAnalyticsState>) -> Vec<String> {
    state.analytics.get_zero_result_queries(hours)
}

/// Get click position distribution
#[tauri::command]
pub fn get_click_distribution(state: State<'_, SearchAnalyticsState>) -> std::collections::HashMap<usize, u32> {
    state.analytics.get_click_position_distribution()
}

/// Record a search result selection (in-memory)
#[tauri::command]
pub fn record_search_selection(
    search_id: String,
    query: String,
    result_index: usize,
    source: String,
    selection_delay_ms: u64,
    state: State<'_, SearchAnalyticsState>,
) {
    state.analytics.record_selection(ResultSelection {
        search_id,
        query,
        result_index,
        source,
        was_helpful: None,
        selection_delay_ms,
        timestamp: chrono::Utc::now(),
    });
}

// --- Database-Backed Analytics (Persistent, Full History) ---

/// Get search analytics summary from database
#[tauri::command]
pub async fn get_search_analytics_db(
    hours: i64,
    app_state: State<'_, AppState>,
) -> Result<AnalyticsSummary, String> {
    let db = Arc::new(app_state.database.clone());
    let db_analytics = DbSearchAnalytics::new(db);
    db_analytics.get_summary(hours).await
}

/// Get popular queries from database
#[tauri::command]
pub async fn get_popular_queries_db(
    limit: usize,
    app_state: State<'_, AppState>,
) -> Result<Vec<PopularQuery>, String> {
    let db = Arc::new(app_state.database.clone());
    let db_analytics = DbSearchAnalytics::new(db);
    db_analytics.get_popular_queries_detailed(limit).await
}

/// Get cache statistics from database
#[tauri::command]
pub async fn get_cache_stats_db(
    app_state: State<'_, AppState>,
) -> Result<CacheStats, String> {
    let db = Arc::new(app_state.database.clone());
    let db_analytics = DbSearchAnalytics::new(db);
    db_analytics.get_cache_stats().await
}

/// Get trending queries from database
#[tauri::command]
pub async fn get_trending_queries_db(
    limit: usize,
    app_state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let db = Arc::new(app_state.database.clone());
    let db_analytics = DbSearchAnalytics::new(db);
    db_analytics.get_trending_queries(limit).await
}

/// Get queries with zero results from database
#[tauri::command]
pub async fn get_zero_result_queries_db(
    hours: i64,
    app_state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let db = Arc::new(app_state.database.clone());
    let db_analytics = DbSearchAnalytics::new(db);
    db_analytics.get_zero_result_queries(hours).await
}

/// Get click position distribution from database
#[tauri::command]
pub async fn get_click_distribution_db(
    app_state: State<'_, AppState>,
) -> Result<std::collections::HashMap<usize, u32>, String> {
    let db = Arc::new(app_state.database.clone());
    let db_analytics = DbSearchAnalytics::new(db);
    db_analytics.get_click_position_distribution().await
}

/// Record a search event (to both in-memory and database)
#[tauri::command]
pub async fn record_search_event(
    query: String,
    result_count: usize,
    execution_time_ms: u64,
    search_type: String,
    from_cache: bool,
    source_filter: Option<String>,
    campaign_id: Option<String>,
    state: State<'_, SearchAnalyticsState>,
    app_state: State<'_, AppState>,
) -> Result<String, String> {
    // Create search record
    let mut record = SearchRecord::new(query, result_count, execution_time_ms, search_type);
    record.from_cache = from_cache;
    record.source_filter = source_filter;
    record.campaign_id = campaign_id;
    let search_id = record.id.clone();

    // Record to in-memory analytics
    state.analytics.record(record.clone());

    // Record to database
    let db = Arc::new(app_state.database.clone());
    let db_analytics = DbSearchAnalytics::new(db);
    db_analytics.record(record).await?;

    Ok(search_id)
}

/// Record a result selection (to both in-memory and database)
#[tauri::command]
pub async fn record_search_selection_db(
    search_id: String,
    query: String,
    result_index: usize,
    source: String,
    selection_delay_ms: u64,
    was_helpful: Option<bool>,
    state: State<'_, SearchAnalyticsState>,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    // Create selection record
    let selection = ResultSelection {
        search_id: search_id.clone(),
        query: query.clone(),
        result_index,
        source: source.clone(),
        was_helpful,
        selection_delay_ms,
        timestamp: chrono::Utc::now(),
    };

    // Record to in-memory analytics
    state.analytics.record_selection(selection.clone());

    // Record to database
    let db = Arc::new(app_state.database.clone());
    let db_analytics = DbSearchAnalytics::new(db);
    db_analytics.record_selection(selection).await
}

/// Clean up old search analytics records
#[tauri::command]
pub async fn cleanup_search_analytics(
    days: i64,
    app_state: State<'_, AppState>,
) -> Result<u64, String> {
    let db = Arc::new(app_state.database.clone());
    let db_analytics = DbSearchAnalytics::new(db);
    db_analytics.cleanup(days).await
}

// ============================================================================
// TASK-024: Security Audit Logging Commands
// ============================================================================

use crate::core::security::{
    SecurityAuditLogger, SecurityAuditEvent, AuditLogQuery, AuditSeverity, ExportFormat,
};

/// Get recent audit events
#[tauri::command]
pub fn get_audit_logs(
    count: Option<usize>,
    min_severity: Option<String>,
    state: State<'_, AuditLoggerState>,
) -> Vec<SecurityAuditEvent> {
    let count = count.unwrap_or(100);

    if let Some(severity_str) = min_severity {
        let severity = match severity_str.to_lowercase().as_str() {
            "debug" => AuditSeverity::Debug,
            "info" => AuditSeverity::Info,
            "warning" => AuditSeverity::Warning,
            "security" => AuditSeverity::Security,
            "critical" => AuditSeverity::Critical,
            _ => AuditSeverity::Info,
        };
        state.logger.get_by_severity(severity).into_iter().take(count).collect()
    } else {
        state.logger.get_recent(count)
    }
}

/// Query audit logs with filters
#[tauri::command]
pub fn query_audit_logs(
    from_hours: Option<i64>,
    min_severity: Option<String>,
    event_types: Option<Vec<String>>,
    search_text: Option<String>,
    limit: Option<usize>,
    state: State<'_, AuditLoggerState>,
) -> Vec<SecurityAuditEvent> {
    let from = from_hours.map(|h| chrono::Utc::now() - chrono::Duration::hours(h));
    let min_sev = min_severity.map(|s| match s.to_lowercase().as_str() {
        "debug" => AuditSeverity::Debug,
        "info" => AuditSeverity::Info,
        "warning" => AuditSeverity::Warning,
        "security" => AuditSeverity::Security,
        "critical" => AuditSeverity::Critical,
        _ => AuditSeverity::Info,
    });

    state.logger.query(AuditLogQuery {
        from,
        to: None,
        min_severity: min_sev,
        event_types,
        search_text,
        limit,
        offset: None,
    })
}

/// Export audit logs
#[tauri::command]
pub fn export_audit_logs(
    format: String,
    from_hours: Option<i64>,
    state: State<'_, AuditLoggerState>,
) -> Result<String, String> {
    let export_format = match format.to_lowercase().as_str() {
        "json" => ExportFormat::Json,
        "csv" => ExportFormat::Csv,
        "jsonl" => ExportFormat::Jsonl,
        _ => return Err(format!("Unsupported format: {}", format)),
    };

    let query = AuditLogQuery {
        from: from_hours.map(|h| chrono::Utc::now() - chrono::Duration::hours(h)),
        ..Default::default()
    };

    state.logger.export(query, export_format)
}

/// Clear old audit logs (older than specified days)
#[tauri::command]
pub fn clear_old_logs(days: i64, state: State<'_, AuditLoggerState>) -> usize {
    state.logger.cleanup(days)
}

/// Get security event counts by severity
#[tauri::command]
pub fn get_audit_summary(state: State<'_, AuditLoggerState>) -> std::collections::HashMap<String, usize> {
    state.logger.count_by_severity()
}

/// Get recent security-level events (last 24 hours)
#[tauri::command]
pub fn get_security_events(state: State<'_, AuditLoggerState>) -> Vec<SecurityAuditEvent> {
    state.logger.get_security_events()
}

// ============================================================================
// State Types for Analytics Modules
// ============================================================================

/// State wrapper for usage tracking
pub struct UsageTrackerState {
    pub tracker: UsageTracker,
}

impl Default for UsageTrackerState {
    fn default() -> Self {
        Self {
            tracker: UsageTracker::new(),
        }
    }
}

/// State wrapper for search analytics
pub struct SearchAnalyticsState {
    pub analytics: SearchAnalytics,
}

impl Default for SearchAnalyticsState {
    fn default() -> Self {
        Self {
            analytics: SearchAnalytics::new(),
        }
    }
}

/// State wrapper for audit logging
pub struct AuditLoggerState {
    pub logger: SecurityAuditLogger,
}

impl Default for AuditLoggerState {
    fn default() -> Self {
        // Initialize with file logging to the app data directory
        let log_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("ai-rpg")
            .join("logs");

        Self {
            logger: SecurityAuditLogger::with_file_logging(log_dir),
        }
    }
}

impl AuditLoggerState {
    /// Create with custom log directory
    pub fn with_log_dir(log_dir: std::path::PathBuf) -> Self {
        Self {
            logger: SecurityAuditLogger::with_file_logging(log_dir),
        }
    }
}

// ============================================================================
// Voice Profile Commands (TASK-004)
// ============================================================================

use crate::core::voice::{
    VoiceProfile, ProfileMetadata, AgeRange, Gender,
    CacheStats as VoiceCacheStats, get_dm_presets,
    SynthesisJob, JobPriority, JobStatus, JobProgress,
    QueueStats as VoiceQueueStats,
};

/// List all voice profile presets (built-in DM personas)
#[tauri::command]
pub fn list_voice_presets() -> Vec<VoiceProfile> {
    get_dm_presets()
}

/// List voice presets filtered by tag
#[tauri::command]
pub fn list_voice_presets_by_tag(tag: String) -> Vec<VoiceProfile> {
    crate::core::voice::get_presets_by_tag(&tag)
}

/// Get a specific voice preset by ID
#[tauri::command]
pub fn get_voice_preset(preset_id: String) -> Option<VoiceProfile> {
    crate::core::voice::get_preset_by_id(&preset_id)
}

/// Create a new voice profile
#[tauri::command]
pub async fn create_voice_profile(
    name: String,
    provider: String,
    voice_id: String,
    metadata: Option<ProfileMetadata>,
    _state: State<'_, AppState>,
) -> Result<String, String> {
    let provider_type = match provider.as_str() {
        "elevenlabs" => VoiceProviderType::ElevenLabs,
        "openai" => VoiceProviderType::OpenAI,
        "fish_audio" => VoiceProviderType::FishAudio,
        "piper" => VoiceProviderType::Piper,
        "ollama" => VoiceProviderType::Ollama,
        "chatterbox" => VoiceProviderType::Chatterbox,
        "gpt_sovits" => VoiceProviderType::GptSoVits,
        "xtts_v2" => VoiceProviderType::XttsV2,
        "fish_speech" => VoiceProviderType::FishSpeech,
        "dia" => VoiceProviderType::Dia,
        _ => return Err(format!("Unknown provider: {}", provider)),
    };

    let mut profile = VoiceProfile::new(&name, provider_type, &voice_id);
    if let Some(meta) = metadata {
        profile = profile.with_metadata(meta);
    }

    Ok(profile.id)
}

/// Link a voice profile to an NPC
#[tauri::command]
pub async fn link_voice_profile_to_npc(
    profile_id: String,
    npc_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Some(mut record) = state.database.get_npc(&npc_id).await.map_err(|e| e.to_string())? {
        if let Some(json) = &record.data_json {
            let mut npc: serde_json::Value = serde_json::from_str(json)
                .map_err(|e| e.to_string())?;
            npc["voice_profile_id"] = serde_json::json!(profile_id);
            record.data_json = Some(serde_json::to_string(&npc).map_err(|e| e.to_string())?);
            state.database.save_npc(&record).await.map_err(|e| e.to_string())?;
        }
    } else {
        return Err(format!("NPC not found: {}", npc_id));
    }
    Ok(())
}

/// Get the voice profile linked to an NPC
#[tauri::command]
pub async fn get_npc_voice_profile(
    npc_id: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    if let Some(record) = state.database.get_npc(&npc_id).await.map_err(|e| e.to_string())? {
        if let Some(json) = &record.data_json {
            let npc: serde_json::Value = serde_json::from_str(json)
                .map_err(|e| e.to_string())?;
            if let Some(profile_id) = npc.get("voice_profile_id").and_then(|v| v.as_str()) {
                return Ok(Some(profile_id.to_string()));
            }
        }
    }
    Ok(None)
}

/// Search voice profiles by query
#[tauri::command]
pub fn search_voice_profiles(query: String) -> Vec<VoiceProfile> {
    let query_lower = query.to_lowercase();
    get_dm_presets()
        .into_iter()
        .filter(|p| {
            p.name.to_lowercase().contains(&query_lower)
                || p.metadata.personality_traits.iter().any(|t| t.to_lowercase().contains(&query_lower))
                || p.metadata.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
                || p.metadata.description.as_ref().map_or(false, |d| d.to_lowercase().contains(&query_lower))
        })
        .collect()
}

/// Get voice profiles by gender
#[tauri::command]
pub fn get_voice_profiles_by_gender(gender: String) -> Vec<VoiceProfile> {
    let target_gender = match gender.to_lowercase().as_str() {
        "male" => Gender::Male,
        "female" => Gender::Female,
        "neutral" => Gender::Neutral,
        "nonbinary" | "non-binary" => Gender::NonBinary,
        _ => return Vec::new(),
    };
    get_dm_presets()
        .into_iter()
        .filter(|p| p.metadata.gender == target_gender)
        .collect()
}

/// Get voice profiles by age range
#[tauri::command]
pub fn get_voice_profiles_by_age(age_range: String) -> Vec<VoiceProfile> {
    let target_age = match age_range.to_lowercase().as_str() {
        "child" => AgeRange::Child,
        "young_adult" | "youngadult" => AgeRange::YoungAdult,
        "adult" => AgeRange::Adult,
        "middle_aged" | "middleaged" => AgeRange::MiddleAged,
        "elderly" => AgeRange::Elderly,
        _ => return Vec::new(),
    };
    get_dm_presets()
        .into_iter()
        .filter(|p| p.metadata.age_range == target_age)
        .collect()
}

// ============================================================================
// Audio Cache Commands (TASK-005)
// ============================================================================

/// Get audio cache statistics
///
/// Returns comprehensive cache statistics including:
/// - Hit/miss counts and rate
/// - Current and max cache size
/// - Entry counts by format
/// - Average entry size
/// - Oldest entry age
#[tauri::command]
pub async fn get_audio_cache_stats(state: State<'_, AppState>) -> Result<VoiceCacheStats, String> {
    let voice_manager = state.voice_manager.read().await;
    voice_manager.get_cache_stats().await.map_err(|e| e.to_string())
}

/// Clear audio cache entries by tag
///
/// Removes all cached audio entries that have the specified tag.
/// Tags can be used to group entries by session_id, npc_id, campaign_id, etc.
///
/// # Arguments
/// * `tag` - The tag to filter by (e.g., "session:abc123", "npc:wizard_01")
///
/// # Returns
/// The number of entries removed
#[tauri::command]
pub async fn clear_audio_cache_by_tag(
    tag: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let voice_manager = state.voice_manager.read().await;
    voice_manager.clear_cache_by_tag(&tag).await.map_err(|e| e.to_string())
}

/// Clear all audio cache entries
///
/// Removes all cached audio files and resets cache statistics.
/// Use with caution as this will force re-synthesis of all audio.
#[tauri::command]
pub async fn clear_audio_cache(state: State<'_, AppState>) -> Result<(), String> {
    let voice_manager = state.voice_manager.read().await;
    voice_manager.clear_cache().await.map_err(|e| e.to_string())
}

/// Prune old audio cache entries
///
/// Removes cache entries older than the specified age.
/// Useful for automatic cleanup of stale audio files.
///
/// # Arguments
/// * `max_age_seconds` - Maximum age in seconds; entries older than this will be removed
///
/// # Returns
/// The number of entries removed
#[tauri::command]
pub async fn prune_audio_cache(
    max_age_seconds: i64,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let voice_manager = state.voice_manager.read().await;
    voice_manager.prune_cache(max_age_seconds).await.map_err(|e| e.to_string())
}

/// List cached audio entries
///
/// Returns all cache entries with metadata including:
/// - File path and size
/// - Creation and last access times
/// - Access count
/// - Associated tags
/// - Audio format and duration
#[tauri::command]
pub async fn list_audio_cache_entries(state: State<'_, AppState>) -> Result<Vec<crate::core::voice::CacheEntry>, String> {
    let cache_dir = state.voice_manager.read().await.get_config()
        .cache_dir.clone().unwrap_or_else(|| std::path::PathBuf::from("./voice_cache"));

    // For listing entries, we still need direct cache access since VoiceManager doesn't expose list_entries
    match crate::core::voice::AudioCache::with_defaults(cache_dir).await {
        Ok(cache) => Ok(cache.list_entries().await),
        Err(e) => Err(format!("Failed to access cache: {}", e)),
    }
}

/// Get cache size information
///
/// Returns the current cache size and maximum allowed size in bytes.
#[tauri::command]
pub async fn get_audio_cache_size(state: State<'_, AppState>) -> Result<AudioCacheSizeInfo, String> {
    let voice_manager = state.voice_manager.read().await;
    let stats = voice_manager.get_cache_stats().await.map_err(|e| e.to_string())?;

    Ok(AudioCacheSizeInfo {
        current_size_bytes: stats.current_size_bytes,
        max_size_bytes: stats.max_size_bytes,
        entry_count: stats.entry_count,
        usage_percent: if stats.max_size_bytes > 0 {
            (stats.current_size_bytes as f64 / stats.max_size_bytes as f64) * 100.0
        } else {
            0.0
        },
    })
}

/// Audio cache size information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioCacheSizeInfo {
    pub current_size_bytes: u64,
    pub max_size_bytes: u64,
    pub entry_count: usize,
    pub usage_percent: f64,
}

// ============================================================================
// Voice Synthesis Queue Commands (TASK-025)
// ============================================================================

use crate::core::voice::{SynthesisQueue, QueueConfig};

/// State wrapper for the synthesis queue
pub struct SynthesisQueueState {
    pub queue: Arc<SynthesisQueue>,
}

impl Default for SynthesisQueueState {
    fn default() -> Self {
        Self {
            queue: Arc::new(SynthesisQueue::with_defaults()),
        }
    }
}

impl SynthesisQueueState {
    /// Create with custom configuration
    pub fn with_config(config: QueueConfig) -> Self {
        Self {
            queue: Arc::new(SynthesisQueue::new(config)),
        }
    }
}

/// Helper to parse provider string to VoiceProviderType for queue commands
fn parse_queue_provider(provider: &str) -> Result<VoiceProviderType, String> {
    match provider {
        "elevenlabs" => Ok(VoiceProviderType::ElevenLabs),
        "openai" => Ok(VoiceProviderType::OpenAI),
        "fish_audio" => Ok(VoiceProviderType::FishAudio),
        "piper" => Ok(VoiceProviderType::Piper),
        "ollama" => Ok(VoiceProviderType::Ollama),
        "chatterbox" => Ok(VoiceProviderType::Chatterbox),
        "gpt_sovits" => Ok(VoiceProviderType::GptSoVits),
        "xtts_v2" => Ok(VoiceProviderType::XttsV2),
        "fish_speech" => Ok(VoiceProviderType::FishSpeech),
        "dia" => Ok(VoiceProviderType::Dia),
        _ => Err(format!("Unknown provider: {}", provider)),
    }
}

/// Helper to parse priority string to JobPriority
fn parse_queue_priority(priority: Option<&str>) -> Result<JobPriority, String> {
    match priority {
        Some("immediate") => Ok(JobPriority::Immediate),
        Some("high") => Ok(JobPriority::High),
        Some("normal") | None => Ok(JobPriority::Normal),
        Some("low") => Ok(JobPriority::Low),
        Some("batch") => Ok(JobPriority::Batch),
        Some(p) => Err(format!("Unknown priority: {}", p)),
    }
}

/// Request type for batch job submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisJobRequest {
    pub text: String,
    pub profile_id: String,
    pub voice_id: String,
    pub provider: String,
    pub priority: Option<String>,
    pub session_id: Option<String>,
    pub npc_id: Option<String>,
    pub campaign_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Submit a voice synthesis job to the queue
#[tauri::command]
pub async fn submit_synthesis_job(
    app_handle: tauri::AppHandle,
    text: String,
    profile_id: String,
    voice_id: String,
    provider: String,
    priority: Option<String>,
    session_id: Option<String>,
    npc_id: Option<String>,
    campaign_id: Option<String>,
    tags: Option<Vec<String>>,
    state: State<'_, SynthesisQueueState>,
) -> Result<SynthesisJob, String> {
    let provider_type = parse_queue_provider(&provider)?;
    let job_priority = parse_queue_priority(priority.as_deref())?;

    let mut job = SynthesisJob::new(&text, &profile_id, provider_type, &voice_id)
        .with_priority(job_priority);

    if let Some(sid) = session_id {
        job = job.for_session(&sid);
    }
    if let Some(nid) = npc_id {
        job = job.for_npc(&nid);
    }
    if let Some(cid) = campaign_id {
        job = job.for_campaign(&cid);
    }
    if let Some(t) = tags {
        job = job.with_tags(t);
    }

    let job_id = state.queue.submit(job, Some(&app_handle))
        .await
        .map_err(|e| e.to_string())?;

    let submitted_job = state.queue.get_job(&job_id).await
        .ok_or_else(|| "Job not found after submission".to_string())?;

    Ok(submitted_job)
}

/// Get a synthesis job by ID
#[tauri::command]
pub async fn get_synthesis_job(
    job_id: String,
    state: State<'_, SynthesisQueueState>,
) -> Result<Option<SynthesisJob>, String> {
    Ok(state.queue.get_job(&job_id).await)
}

/// Get status of a synthesis job
#[tauri::command]
pub async fn get_synthesis_job_status(
    job_id: String,
    state: State<'_, SynthesisQueueState>,
) -> Result<Option<JobStatus>, String> {
    Ok(state.queue.get_status(&job_id).await)
}

/// Get progress of a synthesis job
#[tauri::command]
pub async fn get_synthesis_job_progress(
    job_id: String,
    state: State<'_, SynthesisQueueState>,
) -> Result<Option<JobProgress>, String> {
    Ok(state.queue.get_progress(&job_id).await)
}

/// Cancel a synthesis job
#[tauri::command]
pub async fn cancel_synthesis_job(
    app_handle: tauri::AppHandle,
    job_id: String,
    state: State<'_, SynthesisQueueState>,
) -> Result<(), String> {
    state.queue.cancel(&job_id, Some(&app_handle))
        .await
        .map_err(|e| e.to_string())
}

/// Cancel all synthesis jobs
#[tauri::command]
pub async fn cancel_all_synthesis_jobs(
    app_handle: tauri::AppHandle,
    state: State<'_, SynthesisQueueState>,
) -> Result<usize, String> {
    state.queue.cancel_all(Some(&app_handle))
        .await
        .map_err(|e| e.to_string())
}

/// Pre-generate voice audio for a session (batch queue)
#[tauri::command]
pub async fn pregen_session_voices(
    app_handle: tauri::AppHandle,
    session_id: String,
    texts: Vec<(String, String, String)>,
    provider: String,
    state: State<'_, SynthesisQueueState>,
) -> Result<Vec<String>, String> {
    let provider_type = parse_queue_provider(&provider)?;

    state.queue.pregen_session(&session_id, texts, provider_type, Some(&app_handle))
        .await
        .map_err(|e| e.to_string())
}

/// Submit a batch of synthesis jobs
#[tauri::command]
pub async fn submit_synthesis_batch(
    app_handle: tauri::AppHandle,
    jobs: Vec<SynthesisJobRequest>,
    state: State<'_, SynthesisQueueState>,
) -> Result<Vec<String>, String> {
    let mut synthesis_jobs = Vec::with_capacity(jobs.len());

    for req in jobs {
        let provider_type = parse_queue_provider(&req.provider)?;
        let priority = parse_queue_priority(req.priority.as_deref())?;

        let mut job = SynthesisJob::new(&req.text, &req.profile_id, provider_type, &req.voice_id)
            .with_priority(priority);

        if let Some(sid) = req.session_id {
            job = job.for_session(&sid);
        }
        if let Some(nid) = req.npc_id {
            job = job.for_npc(&nid);
        }
        if let Some(cid) = req.campaign_id {
            job = job.for_campaign(&cid);
        }
        if let Some(t) = req.tags {
            job = job.with_tags(t);
        }

        synthesis_jobs.push(job);
    }

    state.queue.submit_batch(synthesis_jobs, Some(&app_handle))
        .await
        .map_err(|e| e.to_string())
}

/// Get synthesis queue statistics
#[tauri::command]
pub async fn get_synthesis_queue_stats(
    state: State<'_, SynthesisQueueState>,
) -> Result<VoiceQueueStats, String> {
    Ok(state.queue.stats().await)
}

/// Pause the synthesis queue
#[tauri::command]
pub async fn pause_synthesis_queue(
    app_handle: tauri::AppHandle,
    state: State<'_, SynthesisQueueState>,
) -> Result<(), String> {
    state.queue.pause(Some(&app_handle)).await;
    Ok(())
}

/// Resume the synthesis queue
#[tauri::command]
pub async fn resume_synthesis_queue(
    app_handle: tauri::AppHandle,
    state: State<'_, SynthesisQueueState>,
) -> Result<(), String> {
    state.queue.resume(Some(&app_handle)).await;
    Ok(())
}

/// Check if synthesis queue is paused
#[tauri::command]
pub async fn is_synthesis_queue_paused(
    state: State<'_, SynthesisQueueState>,
) -> Result<bool, String> {
    Ok(state.queue.is_paused().await)
}

/// List pending synthesis jobs
#[tauri::command]
pub async fn list_pending_synthesis_jobs(
    state: State<'_, SynthesisQueueState>,
) -> Result<Vec<SynthesisJob>, String> {
    Ok(state.queue.list_pending().await)
}

/// List processing synthesis jobs
#[tauri::command]
pub async fn list_processing_synthesis_jobs(
    state: State<'_, SynthesisQueueState>,
) -> Result<Vec<SynthesisJob>, String> {
    Ok(state.queue.list_processing().await)
}

/// List synthesis job history (completed/failed/cancelled)
#[tauri::command]
pub async fn list_synthesis_job_history(
    limit: Option<usize>,
    state: State<'_, SynthesisQueueState>,
) -> Result<Vec<SynthesisJob>, String> {
    Ok(state.queue.list_history(limit).await)
}

/// List synthesis jobs by session
#[tauri::command]
pub async fn list_synthesis_jobs_by_session(
    session_id: String,
    state: State<'_, SynthesisQueueState>,
) -> Result<Vec<SynthesisJob>, String> {
    Ok(state.queue.list_by_session(&session_id).await)
}

/// List synthesis jobs by NPC
#[tauri::command]
pub async fn list_synthesis_jobs_by_npc(
    npc_id: String,
    state: State<'_, SynthesisQueueState>,
) -> Result<Vec<SynthesisJob>, String> {
    Ok(state.queue.list_by_npc(&npc_id).await)
}

/// List synthesis jobs by tag
#[tauri::command]
pub async fn list_synthesis_jobs_by_tag(
    tag: String,
    state: State<'_, SynthesisQueueState>,
) -> Result<Vec<SynthesisJob>, String> {
    Ok(state.queue.list_by_tag(&tag).await)
}

/// Clear synthesis job history
#[tauri::command]
pub async fn clear_synthesis_job_history(
    state: State<'_, SynthesisQueueState>,
) -> Result<(), String> {
    state.queue.clear_history().await;
    Ok(())
}

/// Get total active jobs (pending + processing)
#[tauri::command]
pub async fn get_synthesis_queue_length(
    state: State<'_, SynthesisQueueState>,
) -> Result<usize, String> {
    Ok(state.queue.total_active().await)
}


// ============================================================================
// TASK-014: Session Timeline Commands
// ============================================================================

use crate::core::session::timeline::{
    TimelineEvent, TimelineEventType, EventSeverity, EntityRef, TimelineSummary,
};

/// Add a timeline event to a session
#[tauri::command]
pub fn add_timeline_event(
    session_id: String,
    event_type: String,
    title: String,
    description: String,
    severity: Option<String>,
    entity_refs: Option<Vec<EntityRef>>,
    tags: Option<Vec<String>>,
    metadata: Option<HashMap<String, serde_json::Value>>,
    state: State<'_, AppState>,
) -> Result<TimelineEvent, String> {
    let etype = match event_type.as_str() {
        "session_start" => TimelineEventType::SessionStart,
        "session_end" => TimelineEventType::SessionEnd,
        "combat_start" => TimelineEventType::CombatStart,
        "combat_end" => TimelineEventType::CombatEnd,
        "combat_round_start" => TimelineEventType::CombatRoundStart,
        "combat_turn_start" => TimelineEventType::CombatTurnStart,
        "combat_damage" => TimelineEventType::CombatDamage,
        "combat_healing" => TimelineEventType::CombatHealing,
        "combat_death" => TimelineEventType::CombatDeath,
        "note_added" => TimelineEventType::NoteAdded,
        "npc_interaction" => TimelineEventType::NPCInteraction,
        "location_change" => TimelineEventType::LocationChange,
        "player_action" => TimelineEventType::PlayerAction,
        "condition_applied" => TimelineEventType::ConditionApplied,
        "condition_removed" => TimelineEventType::ConditionRemoved,
        "item_acquired" => TimelineEventType::ItemAcquired,
        _ => TimelineEventType::Custom(event_type),
    };

    let eseverity = severity.map(|s| match s.as_str() {
        "trace" => EventSeverity::Trace,
        "info" => EventSeverity::Info,
        "notable" => EventSeverity::Notable,
        "important" => EventSeverity::Important,
        "critical" => EventSeverity::Critical,
        _ => EventSeverity::Info,
    }).unwrap_or(EventSeverity::Info);

    let mut event = TimelineEvent::new(&session_id, etype, &title, &description)
        .with_severity(eseverity);

    if let Some(refs) = entity_refs {
        for r in refs {
            event.entity_refs.push(r);
        }
    }

    if let Some(t) = tags {
        event.tags = t;
    }

    if let Some(m) = metadata {
        event.metadata = m;
    }

    // Store in session manager's timeline
    state.session_manager.add_timeline_event(&session_id, event.clone())
        .map_err(|e| e.to_string())?;

    Ok(event)
}

/// Get the timeline for a session
#[tauri::command]
pub fn get_session_timeline(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<TimelineEvent>, String> {
    Ok(state.session_manager.get_timeline_events(&session_id))
}

/// Get timeline summary for a session
#[tauri::command]
pub fn get_timeline_summary(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<TimelineSummary, String> {
    state.session_manager.get_timeline_summary(&session_id)
        .map_err(|e| e.to_string())
}

/// Get timeline events by type
#[tauri::command]
pub fn get_timeline_events_by_type(
    session_id: String,
    event_type: String,
    state: State<'_, AppState>,
) -> Result<Vec<TimelineEvent>, String> {
    let etype = match event_type.as_str() {
        "session_start" => TimelineEventType::SessionStart,
        "session_end" => TimelineEventType::SessionEnd,
        "combat_start" => TimelineEventType::CombatStart,
        "combat_end" => TimelineEventType::CombatEnd,
        "note_added" => TimelineEventType::NoteAdded,
        "npc_interaction" => TimelineEventType::NPCInteraction,
        "location_change" => TimelineEventType::LocationChange,
        _ => TimelineEventType::Custom(event_type),
    };

    Ok(state.session_manager.get_timeline_events_by_type(&session_id, &etype))
}

// ============================================================================
// TASK-015: Advanced Condition Commands
// ============================================================================

use crate::core::session::conditions::{
    AdvancedCondition, ConditionDuration, ConditionTemplates,
};

/// Apply an advanced condition to a combatant
#[tauri::command]
pub fn apply_advanced_condition(
    session_id: String,
    combatant_id: String,
    condition_name: String,
    duration_type: Option<String>,
    duration_value: Option<u32>,
    source_id: Option<String>,
    source_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<AdvancedCondition, String> {
    // Try to get a template condition first
    let mut condition = ConditionTemplates::by_name(&condition_name)
        .unwrap_or_else(|| {
            // Create a custom condition
            let duration = match duration_type.as_deref() {
                Some("turns") => ConditionDuration::Turns(duration_value.unwrap_or(1)),
                Some("rounds") => ConditionDuration::Rounds(duration_value.unwrap_or(1)),
                Some("minutes") => ConditionDuration::Minutes(duration_value.unwrap_or(1)),
                Some("hours") => ConditionDuration::Hours(duration_value.unwrap_or(1)),
                Some("end_of_turn") => ConditionDuration::EndOfNextTurn,
                Some("start_of_turn") => ConditionDuration::StartOfNextTurn,
                _ => ConditionDuration::UntilRemoved,
            };
            AdvancedCondition::new(&condition_name, "Custom condition", duration)
        });

    // Set source if provided
    if let (Some(sid), Some(sname)) = (source_id, source_name) {
        condition = condition.from_source(sid, sname);
    }

    // Apply to combatant
    state.session_manager.apply_advanced_condition(&session_id, &combatant_id, condition.clone())
        .map_err(|e| e.to_string())?;

    Ok(condition)
}

/// Remove an advanced condition from a combatant
#[tauri::command]
pub fn remove_advanced_condition(
    session_id: String,
    combatant_id: String,
    condition_id: String,
    state: State<'_, AppState>,
) -> Result<Option<AdvancedCondition>, String> {
    state.session_manager.remove_advanced_condition(&session_id, &combatant_id, &condition_id)
        .map_err(|e| e.to_string())
}

/// Get all conditions for a combatant
#[tauri::command]
pub fn get_combatant_conditions(
    session_id: String,
    combatant_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<AdvancedCondition>, String> {
    state.session_manager.get_combatant_conditions(&session_id, &combatant_id)
        .map_err(|e| e.to_string())
}

/// Tick conditions at end of turn
#[tauri::command]
pub fn tick_conditions_end_of_turn(
    session_id: String,
    combatant_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    state.session_manager.tick_conditions_end_of_turn(&session_id, &combatant_id)
        .map_err(|e| e.to_string())
}

/// Tick conditions at start of turn
#[tauri::command]
pub fn tick_conditions_start_of_turn(
    session_id: String,
    combatant_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    state.session_manager.tick_conditions_start_of_turn(&session_id, &combatant_id)
        .map_err(|e| e.to_string())
}

/// List available condition templates
#[tauri::command]
pub fn list_condition_templates() -> Vec<String> {
    ConditionTemplates::list_names().iter().map(|s| s.to_string()).collect()
}

// ============================================================================
// TASK-017: Session Notes Commands
// ============================================================================

/// Create a new session note
#[tauri::command]
pub fn create_session_note(
    session_id: String,
    campaign_id: String,
    title: String,
    content: String,
    category: Option<String>,
    tags: Option<Vec<String>>,
    is_pinned: Option<bool>,
    is_private: Option<bool>,
    state: State<'_, AppState>,
) -> Result<NoteSessionNote, String> {
    let note_category = category.map(|c| match c.as_str() {
        "general" => NoteCategory::General,
        "combat" => NoteCategory::Combat,
        "character" => NoteCategory::Character,
        "location" => NoteCategory::Location,
        "plot" => NoteCategory::Plot,
        "quest" => NoteCategory::Quest,
        "loot" => NoteCategory::Loot,
        "rules" => NoteCategory::Rules,
        "meta" => NoteCategory::Meta,
        "worldbuilding" => NoteCategory::Worldbuilding,
        "dialogue" => NoteCategory::Dialogue,
        "secret" => NoteCategory::Secret,
        _ => NoteCategory::Custom(c),
    }).unwrap_or(NoteCategory::General);

    let mut note = NoteSessionNote::new(&session_id, &campaign_id, &title, &content)
        .with_category(note_category);

    if let Some(t) = tags {
        note = note.with_tags(t);
    }

    if is_pinned.unwrap_or(false) {
        note = note.pinned();
    }

    if is_private.unwrap_or(false) {
        note = note.private();
    }

    state.session_manager.create_note(note.clone())
        .map_err(|e| e.to_string())?;

    Ok(note)
}

/// Get a session note by ID
#[tauri::command]
pub fn get_session_note(
    note_id: String,
    state: State<'_, AppState>,
) -> Result<Option<NoteSessionNote>, String> {
    Ok(state.session_manager.get_note(&note_id))
}

/// Update a session note
#[tauri::command]
pub fn update_session_note(
    note: NoteSessionNote,
    state: State<'_, AppState>,
) -> Result<NoteSessionNote, String> {
    state.session_manager.update_note(note)
        .map_err(|e| e.to_string())
}

/// Delete a session note
#[tauri::command]
pub fn delete_session_note(
    note_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.session_manager.delete_note(&note_id)
        .map_err(|e| e.to_string())
}

/// List notes for a session
#[tauri::command]
pub fn list_session_notes(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<NoteSessionNote>, String> {
    Ok(state.session_manager.list_notes_for_session(&session_id))
}

/// Search notes
#[tauri::command]
pub fn search_session_notes(
    query: String,
    session_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<NoteSessionNote>, String> {
    Ok(state.session_manager.search_notes(&query, session_id.as_deref()))
}

/// Get notes by category
#[tauri::command]
pub fn get_notes_by_category(
    category: String,
    session_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<NoteSessionNote>, String> {
    let note_category = match category.as_str() {
        "general" => NoteCategory::General,
        "combat" => NoteCategory::Combat,
        "character" => NoteCategory::Character,
        "location" => NoteCategory::Location,
        "plot" => NoteCategory::Plot,
        "quest" => NoteCategory::Quest,
        "loot" => NoteCategory::Loot,
        "rules" => NoteCategory::Rules,
        "meta" => NoteCategory::Meta,
        "worldbuilding" => NoteCategory::Worldbuilding,
        "dialogue" => NoteCategory::Dialogue,
        "secret" => NoteCategory::Secret,
        _ => NoteCategory::Custom(category),
    };

    Ok(state.session_manager.get_notes_by_category(&note_category, session_id.as_deref()))
}

/// Get notes with a specific tag
#[tauri::command]
pub fn get_notes_by_tag(
    tag: String,
    state: State<'_, AppState>,
) -> Result<Vec<NoteSessionNote>, String> {
    Ok(state.session_manager.get_notes_by_tag(&tag))
}

/// AI categorize a note
#[tauri::command]
pub async fn categorize_note_ai(
    title: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<CategorizationResponse, String> {
    // Build the categorization prompt
    let request = CategorizationRequest {
        title,
        content,
        available_categories: vec![
            "General".to_string(),
            "Combat".to_string(),
            "Character".to_string(),
            "Location".to_string(),
            "Plot".to_string(),
            "Quest".to_string(),
            "Loot".to_string(),
            "Rules".to_string(),
            "Worldbuilding".to_string(),
            "Dialogue".to_string(),
            "Secret".to_string(),
        ],
    };

    let prompt = build_categorization_prompt(&request);

    // Call LLM
    let config = state.llm_config.read().unwrap().clone()
        .ok_or("LLM not configured")?;
    let client = crate::core::llm::LLMClient::new(config);

    let llm_request = crate::core::llm::ChatRequest {
        messages: vec![crate::core::llm::ChatMessage {
            role: crate::core::llm::MessageRole::User,
            content: prompt,
            images: None,
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }],
        system_prompt: Some("You are a TTRPG session note analyzer. Respond only with valid JSON.".to_string()),
        temperature: Some(0.3),
        max_tokens: Some(500),
        provider: None,
        tools: None,
        tool_choice: None,
    };

    let response = client.chat(llm_request).await
        .map_err(|e| e.to_string())?;

    // Parse the response
    parse_categorization_response(&response.content)
}

/// Link an entity to a note
#[tauri::command]
pub fn link_entity_to_note(
    note_id: String,
    entity_type: String,
    entity_id: String,
    entity_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let etype = match entity_type.as_str() {
        "npc" => NoteEntityType::NPC,
        "player" => NoteEntityType::Player,
        "location" => NoteEntityType::Location,
        "item" => NoteEntityType::Item,
        "quest" => NoteEntityType::Quest,
        "session" => NoteEntityType::Session,
        "campaign" => NoteEntityType::Campaign,
        "combat" => NoteEntityType::Combat,
        _ => NoteEntityType::Custom(entity_type),
    };

    state.session_manager.link_entity_to_note(&note_id, etype, &entity_id, &entity_name)
        .map_err(|e| e.to_string())
}

/// Unlink an entity from a note
#[tauri::command]
pub fn unlink_entity_from_note(
    note_id: String,
    entity_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.session_manager.unlink_entity_from_note(&note_id, &entity_id)
        .map_err(|e| e.to_string())
}

// ============================================================================
// Backstory Generation Commands (TASK-019)
// ============================================================================



/// Build NPC system prompt with personality - stub for compatibility
/// (Actual implementation provided by personality_manager commands below)
#[tauri::command]
pub fn build_npc_system_prompt_stub(
    npc_id: String,
    campaign_id: String,
    additional_context: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let manager = Arc::new(crate::core::personality::PersonalityApplicationManager::new(state.personality_store.clone()));
    let styler = crate::core::personality::NPCDialogueStyler::new(manager);
    styler.build_npc_system_prompt(&npc_id, &campaign_id, additional_context.as_deref())
        .map_err(|e| e.to_string())
}

// Duplicate removed


// ============================================================================
// Personality Application Commands (TASK-021)
// ============================================================================

/// Request payload for setting active personality
#[derive(Debug, Serialize, Deserialize)]
pub struct SetActivePersonalityRequest {
    pub session_id: String,
    pub personality_id: Option<String>,
    pub campaign_id: String,
}

/// Request payload for personality settings update
#[derive(Debug, Serialize, Deserialize)]
pub struct PersonalitySettingsRequest {
    pub campaign_id: String,
    pub tone: Option<String>,
    pub vocabulary: Option<String>,
    pub narrative_style: Option<String>,
    pub verbosity: Option<String>,
    pub genre: Option<String>,
    pub custom_patterns: Option<Vec<String>>,
    pub use_dialect: Option<bool>,
    pub dialect: Option<String>,
}

/// Set the active personality for a session
#[tauri::command]
pub fn set_active_personality(
    request: SetActivePersonalityRequest,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.personality_manager.set_active_personality(
        &request.session_id,
        request.personality_id,
        &request.campaign_id,
    );
    Ok(())
}

/// Get the active personality ID for a session
#[tauri::command]
pub fn get_active_personality(
    session_id: String,
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    Ok(state.personality_manager.get_active_personality_id(&session_id, &campaign_id))
}

/// Get the system prompt for a personality
#[tauri::command]
pub fn get_personality_prompt(
    personality_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    state.personality_manager.get_personality_prompt(&personality_id)
        .map_err(|e| e.to_string())
}

/// Apply personality styling to text using LLM transformation
#[tauri::command]
pub async fn apply_personality_to_text(
    text: String,
    personality_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let config = state.llm_config.read().unwrap()
        .clone()
        .ok_or("LLM not configured")?;
    let client = LLMClient::new(config);

    state.personality_manager.apply_personality_to_text(&text, &personality_id, &client)
        .await
        .map_err(|e| e.to_string())
}

/// Get personality context for a campaign
#[tauri::command]
pub fn get_personality_context(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<ActivePersonalityContext, String> {
    Ok(state.personality_manager.get_context(&campaign_id))
}

/// Get personality context for a session
#[tauri::command]
pub fn get_session_personality_context(
    session_id: String,
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<ActivePersonalityContext, String> {
    Ok(state.personality_manager.get_session_context(&session_id, &campaign_id))
}

/// Update personality context for a campaign
#[tauri::command]
pub fn set_personality_context(
    context: ActivePersonalityContext,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.personality_manager.set_context(context);
    Ok(())
}

/// Set the narrator personality for a campaign
#[tauri::command]
pub fn set_narrator_personality(
    campaign_id: String,
    personality_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.personality_manager.set_narrator_personality(&campaign_id, personality_id);
    Ok(())
}

/// Assign a personality to an NPC
#[tauri::command]
pub fn assign_npc_personality(
    campaign_id: String,
    npc_id: String,
    personality_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.personality_manager.assign_npc_personality(&campaign_id, &npc_id, &personality_id);
    Ok(())
}

/// Unassign personality from an NPC
#[tauri::command]
pub fn unassign_npc_personality(
    campaign_id: String,
    npc_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.personality_manager.unassign_npc_personality(&campaign_id, &npc_id);
    Ok(())
}

/// Set scene mood for a campaign
#[tauri::command]
pub fn set_scene_mood(
    campaign_id: String,
    mood: Option<SceneMood>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.personality_manager.set_scene_mood(&campaign_id, mood);
    Ok(())
}

/// Update personality settings for a campaign
#[tauri::command]
pub fn set_personality_settings(
    request: PersonalitySettingsRequest,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let settings = PersonalitySettings {
        tone: request.tone.map(|t| NarrativeTone::from_str(&t)).unwrap_or_default(),
        vocabulary: request.vocabulary.map(|v| VocabularyLevel::from_str(&v)).unwrap_or_default(),
        narrative_style: request.narrative_style.map(|n| NarrativeStyle::from_str(&n)).unwrap_or_default(),
        verbosity: request.verbosity.map(|v| VerbosityLevel::from_str(&v)).unwrap_or_default(),
        genre: request.genre.map(|g| GenreConvention::from_str(&g)).unwrap_or_default(),
        custom_patterns: request.custom_patterns.unwrap_or_default(),
        use_dialect: request.use_dialect.unwrap_or(false),
        dialect: request.dialect,
    };

    state.personality_manager.set_personality_settings(&request.campaign_id, settings);
    Ok(())
}

/// Toggle personality application on/off
#[tauri::command]
pub fn set_personality_active(
    campaign_id: String,
    active: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.personality_manager.set_personality_active(&campaign_id, active);
    Ok(())
}

/// Preview a personality
#[tauri::command]
pub fn preview_personality(
    personality_id: String,
    state: State<'_, AppState>,
) -> Result<PersonalityPreview, String> {
    state.personality_manager.preview_personality(&personality_id)
        .map_err(|e| e.to_string())
}

/// Get extended personality preview with full details
#[tauri::command]
pub fn preview_personality_extended(
    personality_id: String,
    state: State<'_, AppState>,
) -> Result<ExtendedPersonalityPreview, String> {
    state.personality_manager.preview_personality_extended(&personality_id)
        .map_err(|e| e.to_string())
}

/// Generate a preview response for personality selection UI
#[tauri::command]
pub async fn generate_personality_preview(
    personality_id: String,
    state: State<'_, AppState>,
) -> Result<PreviewResponse, String> {
    let config = state.llm_config.read().unwrap()
        .clone()
        .ok_or("LLM not configured")?;
    let client = LLMClient::new(config);

    state.personality_manager.generate_preview_response(&personality_id, &client)
        .await
        .map_err(|e| e.to_string())
}

/// Test a personality by generating a response
#[tauri::command]
pub async fn test_personality(
    personality_id: String,
    test_prompt: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let config = state.llm_config.read().unwrap()
        .clone()
        .ok_or("LLM not configured")?;
    let client = LLMClient::new(config);

    state.personality_manager.test_personality(&personality_id, &test_prompt, &client)
        .await
        .map_err(|e| e.to_string())
}

/// Get the session system prompt with personality applied
#[tauri::command]
pub fn get_session_system_prompt(
    session_id: String,
    campaign_id: String,
    content_type: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let ct = match content_type.as_str() {
        "dialogue" => ContentType::Dialogue,
        "narration" => ContentType::Narration,
        "internal_thought" => ContentType::InternalThought,
        "description" => ContentType::Description,
        "action" => ContentType::Action,
        _ => ContentType::Narration,
    };

    state.personality_manager.get_session_system_prompt(&session_id, &campaign_id, ct)
        .map_err(|e| e.to_string())
}

/// Style NPC dialogue with personality
#[tauri::command]
pub fn style_npc_dialogue(
    npc_id: String,
    campaign_id: String,
    raw_dialogue: String,
    state: State<'_, AppState>,
) -> Result<StyledContent, String> {
    let styler = NPCDialogueStyler::new(state.personality_manager.clone());
    styler.style_npc_dialogue(&npc_id, &campaign_id, &raw_dialogue)
        .map_err(|e| e.to_string())
}

/// Build NPC system prompt with personality
#[tauri::command]
pub fn build_npc_system_prompt(
    npc_id: String,
    campaign_id: String,
    additional_context: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let styler = NPCDialogueStyler::new(state.personality_manager.clone());
    styler.build_npc_system_prompt(&npc_id, &campaign_id, additional_context.as_deref())
        .map_err(|e| e.to_string())
}

/// Build narration prompt with personality
#[tauri::command]
pub fn build_narration_prompt(
    campaign_id: String,
    narration_type: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let nt = match narration_type.as_str() {
        "scene_description" => NarrationType::SceneDescription,
        "action" => NarrationType::Action,
        "transition" => NarrationType::Transition,
        "atmosphere" => NarrationType::Atmosphere,
        _ => NarrationType::SceneDescription,
    };

    let manager = NarrationStyleManager::new(state.personality_manager.clone());
    manager.build_narration_prompt(&campaign_id, nt)
        .map_err(|e| e.to_string())
}

/// List all available personalities from the store
#[tauri::command]
pub fn list_personalities(
    state: State<'_, AppState>,
) -> Result<Vec<PersonalityPreview>, String> {
    let personalities = state.personality_store.list();
    let previews: Vec<PersonalityPreview> = personalities
        .iter()
        .filter_map(|p| state.personality_manager.preview_personality(&p.id).ok())
        .collect();
    Ok(previews)
}

/// Clear session-specific personality context
#[tauri::command]
pub fn clear_session_personality_context(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.personality_manager.clear_session_context(&session_id);
    Ok(())
}

// ============================================================================
// TASK-020: Location Generation Commands
// ============================================================================

use crate::core::location_gen::{
    Inhabitant, Secret, Encounter,
    LocationConnection, MapReference, Difficulty,
};

/// Generate a new location using procedural templates or AI
#[tauri::command]
pub async fn generate_location(
    location_type: String,
    campaign_id: Option<String>,
    options: Option<LocationGenerationOptions>,
    state: State<'_, AppState>,
) -> Result<Location, String> {
    let mut gen_options = options.unwrap_or_default();
    gen_options.location_type = Some(location_type);
    gen_options.campaign_id = campaign_id;

    if gen_options.use_ai {
        // Use AI-enhanced generation if LLM is configured
        let llm_config = state.llm_config.read()
            .map_err(|e| e.to_string())?
            .clone();

        if let Some(config) = llm_config {
            let generator = LocationGenerator::with_llm(config);
            generator.generate_detailed(&gen_options).await
                .map_err(|e| e.to_string())
        } else {
            // Fall back to quick generation if no LLM configured
            let generator = LocationGenerator::new();
            Ok(generator.generate_quick(&gen_options))
        }
    } else {
        // Use quick procedural generation
        let generator = LocationGenerator::new();
        Ok(generator.generate_quick(&gen_options))
    }
}

/// Generate a location quickly using procedural templates only
#[tauri::command]
pub fn generate_location_quick(
    location_type: String,
    campaign_id: Option<String>,
    name: Option<String>,
    theme: Option<String>,
    include_inhabitants: Option<bool>,
    include_secrets: Option<bool>,
    include_encounters: Option<bool>,
    include_loot: Option<bool>,
    danger_level: Option<String>,
) -> Location {
    let options = LocationGenerationOptions {
        location_type: Some(location_type),
        name,
        campaign_id,
        theme,
        include_inhabitants: include_inhabitants.unwrap_or(true),
        include_secrets: include_secrets.unwrap_or(true),
        include_encounters: include_encounters.unwrap_or(true),
        include_loot: include_loot.unwrap_or(true),
        danger_level: danger_level.map(|d| parse_difficulty(&d)),
        ..Default::default()
    };

    let generator = LocationGenerator::new();
    generator.generate_quick(&options)
}

/// Get all available location types
#[tauri::command]
pub fn get_location_types() -> Vec<LocationTypeInfo> {
    vec![
        LocationTypeInfo { id: "city".to_string(), name: "City".to_string(), category: "Urban".to_string(), description: "A large settlement with walls, markets, and political intrigue".to_string() },
        LocationTypeInfo { id: "town".to_string(), name: "Town".to_string(), category: "Urban".to_string(), description: "A medium-sized settlement with basic amenities".to_string() },
        LocationTypeInfo { id: "village".to_string(), name: "Village".to_string(), category: "Urban".to_string(), description: "A small rural community".to_string() },
        LocationTypeInfo { id: "tavern".to_string(), name: "Tavern".to_string(), category: "Buildings".to_string(), description: "A place for drinking, dining, and gathering information".to_string() },
        LocationTypeInfo { id: "inn".to_string(), name: "Inn".to_string(), category: "Buildings".to_string(), description: "Lodging for weary travelers".to_string() },
        LocationTypeInfo { id: "shop".to_string(), name: "Shop".to_string(), category: "Buildings".to_string(), description: "A merchant's establishment".to_string() },
        LocationTypeInfo { id: "market".to_string(), name: "Market".to_string(), category: "Buildings".to_string(), description: "An open marketplace with many vendors".to_string() },
        LocationTypeInfo { id: "temple".to_string(), name: "Temple".to_string(), category: "Buildings".to_string(), description: "A place of worship and divine power".to_string() },
        LocationTypeInfo { id: "shrine".to_string(), name: "Shrine".to_string(), category: "Buildings".to_string(), description: "A small sacred site".to_string() },
        LocationTypeInfo { id: "guild".to_string(), name: "Guild Hall".to_string(), category: "Buildings".to_string(), description: "Headquarters of a professional organization".to_string() },
        LocationTypeInfo { id: "castle".to_string(), name: "Castle".to_string(), category: "Fortifications".to_string(), description: "A noble's fortified residence".to_string() },
        LocationTypeInfo { id: "stronghold".to_string(), name: "Stronghold".to_string(), category: "Fortifications".to_string(), description: "A military fortress".to_string() },
        LocationTypeInfo { id: "manor".to_string(), name: "Manor".to_string(), category: "Fortifications".to_string(), description: "A wealthy estate".to_string() },
        LocationTypeInfo { id: "tower".to_string(), name: "Tower".to_string(), category: "Fortifications".to_string(), description: "A wizard's tower or watchtower".to_string() },
        LocationTypeInfo { id: "dungeon".to_string(), name: "Dungeon".to_string(), category: "Adventure Sites".to_string(), description: "An underground complex of danger and treasure".to_string() },
        LocationTypeInfo { id: "cave".to_string(), name: "Cave".to_string(), category: "Adventure Sites".to_string(), description: "A natural underground cavern".to_string() },
        LocationTypeInfo { id: "ruins".to_string(), name: "Ruins".to_string(), category: "Adventure Sites".to_string(), description: "The remains of an ancient civilization".to_string() },
        LocationTypeInfo { id: "tomb".to_string(), name: "Tomb".to_string(), category: "Adventure Sites".to_string(), description: "A burial place for the dead".to_string() },
        LocationTypeInfo { id: "mine".to_string(), name: "Mine".to_string(), category: "Adventure Sites".to_string(), description: "An excavation for precious resources".to_string() },
        LocationTypeInfo { id: "lair".to_string(), name: "Monster Lair".to_string(), category: "Adventure Sites".to_string(), description: "The den of a dangerous creature".to_string() },
        LocationTypeInfo { id: "forest".to_string(), name: "Forest".to_string(), category: "Wilderness".to_string(), description: "A vast woodland area".to_string() },
        LocationTypeInfo { id: "mountain".to_string(), name: "Mountain".to_string(), category: "Wilderness".to_string(), description: "A towering peak or mountain range".to_string() },
        LocationTypeInfo { id: "swamp".to_string(), name: "Swamp".to_string(), category: "Wilderness".to_string(), description: "A treacherous wetland".to_string() },
        LocationTypeInfo { id: "desert".to_string(), name: "Desert".to_string(), category: "Wilderness".to_string(), description: "An arid wasteland".to_string() },
        LocationTypeInfo { id: "plains".to_string(), name: "Plains".to_string(), category: "Wilderness".to_string(), description: "Open grassland terrain".to_string() },
        LocationTypeInfo { id: "coast".to_string(), name: "Coast".to_string(), category: "Wilderness".to_string(), description: "Shoreline and coastal waters".to_string() },
        LocationTypeInfo { id: "island".to_string(), name: "Island".to_string(), category: "Wilderness".to_string(), description: "An isolated landmass surrounded by water".to_string() },
        LocationTypeInfo { id: "river".to_string(), name: "River".to_string(), category: "Wilderness".to_string(), description: "A major waterway".to_string() },
        LocationTypeInfo { id: "lake".to_string(), name: "Lake".to_string(), category: "Wilderness".to_string(), description: "A body of fresh water".to_string() },
        LocationTypeInfo { id: "portal".to_string(), name: "Portal".to_string(), category: "Magical".to_string(), description: "A gateway to another place or plane".to_string() },
        LocationTypeInfo { id: "planar".to_string(), name: "Planar Location".to_string(), category: "Magical".to_string(), description: "A location on another plane of existence".to_string() },
        LocationTypeInfo { id: "custom".to_string(), name: "Custom".to_string(), category: "Other".to_string(), description: "A unique location type".to_string() },
    ]
}

/// Location type information for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationTypeInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
}

/// Save a generated location to the location manager
#[tauri::command]
pub async fn save_location(
    location: Location,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let location_id = location.id.clone();

    // Save to location manager
    state.location_manager.save_location(location)
        .map_err(|e| e.to_string())?;

    Ok(location_id)
}

/// Get a location by ID
#[tauri::command]
pub fn get_location(
    location_id: String,
    state: State<'_, AppState>,
) -> Result<Option<Location>, String> {
    Ok(state.location_manager.get_location(&location_id))
}

/// List all locations for a campaign
#[tauri::command]
pub fn list_campaign_locations(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Location>, String> {
    Ok(state.location_manager.list_locations_for_campaign(&campaign_id))
}

/// Delete a location
#[tauri::command]
pub fn delete_location(
    location_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.location_manager.delete_location(&location_id)
        .map_err(|e| e.to_string())
}

/// Update a location
#[tauri::command]
pub fn update_location(
    location: Location,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.location_manager.update_location(location)
        .map_err(|e| e.to_string())
}

/// List available location types
#[tauri::command]
pub fn list_location_types() -> Vec<String> {
    vec![
        "Tavern", "Inn", "Shop", "Guild", "Temple", "Castle", "Manor", "Prison", "Slum", "Market", "City", "Town", "Village",
        "Forest", "Mountain", "Swamp", "Desert", "Plains", "Coast", "Island", "River", "Lake", "Cave",
        "Dungeon", "Ruins", "Tower", "Tomb", "Mine", "Stronghold", "Lair", "Camp", "Shrine", "Portal",
        "Planar", "Underwater", "Aerial"
    ].into_iter().map(String::from).collect()
}

/// Add a connection between two locations
#[tauri::command]
pub fn add_location_connection(
    source_location_id: String,
    target_location_id: String,
    connection_type: String,
    description: Option<String>,
    travel_time: Option<String>,
    bidirectional: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let connection = LocationConnection {
        target_id: Some(target_location_id.clone()),
        target_name: "Unknown".to_string(), // Placeholder
        connection_type: crate::core::location_gen::ConnectionType::Path, // Placeholder/Default
        travel_time,
        hazards: vec![],
    };

    state.location_manager.add_connection(&source_location_id, connection.clone())
        .map_err(|e| e.to_string())?;

    // If bidirectional, add reverse connection
    if bidirectional.unwrap_or(true) {
        let reverse = LocationConnection {
            target_id: Some(source_location_id),
            target_name: "Unknown".to_string(),
            connection_type: connection.connection_type.clone(),
            travel_time: connection.travel_time,
            hazards: vec![],
        };
        state.location_manager.add_connection(&target_location_id, reverse)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Remove a connection between locations
#[tauri::command]
pub fn remove_location_connection(
    source_location_id: String,
    target_location_id: String,
    bidirectional: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.location_manager.remove_connection(&source_location_id, &target_location_id)
        .map_err(|e| e.to_string())?;

    if bidirectional.unwrap_or(true) {
        state.location_manager.remove_connection(&target_location_id, &source_location_id)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Search locations by criteria
#[tauri::command]
pub fn search_locations(
    campaign_id: Option<String>,
    location_type: Option<String>,
    tags: Option<Vec<String>>,
    query: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Location>, String> {
    Ok(state.location_manager.search_locations(campaign_id, location_type, tags, query))
}

/// Get locations connected to a specific location
#[tauri::command]
pub fn get_connected_locations(
    location_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Location>, String> {
    Ok(state.location_manager.get_connected_locations(&location_id))
}

/// Add an inhabitant to a location
#[tauri::command]
pub fn add_location_inhabitant(
    location_id: String,
    inhabitant: Inhabitant,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.location_manager.add_inhabitant(&location_id, inhabitant)
        .map_err(|e| e.to_string())
}

/// Remove an inhabitant from a location
#[tauri::command]
pub fn remove_location_inhabitant(
    location_id: String,
    inhabitant_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.location_manager.remove_inhabitant(&location_id, &inhabitant_name)
        .map_err(|e| e.to_string())
}

/// Add a secret to a location
#[tauri::command]
pub fn add_location_secret(
    location_id: String,
    secret: Secret,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.location_manager.add_secret(&location_id, secret)
        .map_err(|e| e.to_string())
}

/// Add an encounter to a location
#[tauri::command]
pub fn add_location_encounter(
    location_id: String,
    encounter: Encounter,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.location_manager.add_encounter(&location_id, encounter)
        .map_err(|e| e.to_string())
}

/// Set map reference for a location
#[tauri::command]
pub fn set_location_map_reference(
    location_id: String,
    map_reference: MapReference,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.location_manager.set_map_reference(&location_id, map_reference)
        .map_err(|e| e.to_string())
}

/// Helper to parse difficulty from string
fn parse_difficulty(s: &str) -> Difficulty {
    match s.to_lowercase().as_str() {
        "easy" => Difficulty::Easy,
        "medium" => Difficulty::Medium,
        "hard" => Difficulty::Hard,
        "very_hard" | "veryhard" => Difficulty::VeryHard,
        "nearly_impossible" | "nearlyimpossible" => Difficulty::NearlyImpossible,
        _ => Difficulty::Medium,
    }
}

// ============================================================================

// ============================================================================
// Claude Code CLI Commands
// ============================================================================

// ============================================================================
// Gemini CLI Status and Extension Commands
// ============================================================================

// ============================================================================
// Meilisearch Chat Provider Commands
// ============================================================================

use crate::core::meilisearch_chat::{
    ChatProviderConfig, ChatProviderInfo, ChatPrompts, ChatWorkspaceSettings,
    list_chat_providers as get_chat_providers,
};

/// List available chat providers with their capabilities.
#[tauri::command]
pub fn list_chat_providers() -> Vec<ChatProviderInfo> {
    get_chat_providers()
}

/// Configure a Meilisearch chat workspace with a specific LLM provider.
///
/// This command:
/// 1. Starts the LLM proxy if needed (for non-native providers)
/// 2. Registers the provider with the proxy
/// 3. Configures the Meilisearch chat workspace
#[tauri::command]
pub async fn configure_chat_workspace(
    workspace_id: String,
    provider: ChatProviderConfig,
    custom_prompts: Option<ChatPrompts>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Get the unified LLM manager from state
    let manager = state.llm_manager.clone();

    // Ensure Meilisearch client is configured
    {
        let _manager_guard = manager.read().await;
        // We can't access chat_client easily to check if it's set without lock,
        // but set_chat_client handles it.
    }

    // Configure with Meilisearch host from search client
    // TODO: Get API key from credentials if needed
    {
        let manager_guard = manager.write().await;
        // Re-configure chat client to ensure it has latest host/key
        manager_guard.set_chat_client(state.search_client.host(), Some(&state.sidecar_manager.config().master_key)).await;
    }

    // Configure the workspace
    let manager_guard = manager.read().await;
    manager_guard
        .configure_chat_workspace(&workspace_id, provider, custom_prompts)
        .await
}

/// Get the current settings for a Meilisearch chat workspace.
#[tauri::command]
pub async fn get_chat_workspace_settings(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<Option<ChatWorkspaceSettings>, String> {
    use crate::core::meilisearch_chat::MeilisearchChatClient;

    let client = MeilisearchChatClient::new(state.search_client.host(), Some(&state.sidecar_manager.config().master_key));
    client.get_workspace_settings(&workspace_id).await
}


/// Configure Meilisearch chat workspace with individual parameters.
///
/// This is a convenience command that builds the ChatProviderConfig from
/// individual parameters, making it easier to call from the frontend.
///
/// # Arguments
/// * `provider` - Provider type: "openai", "claude", "mistral", "gemini", "ollama",
///                "openrouter", "groq", "together", "cohere", "deepseek"
/// * `api_key` - API key for the provider (optional for ollama)
/// * `model` - Model to use (optional, uses provider default if not specified)
/// * `custom_system_prompt` - Custom system prompt (optional)
/// * `host` - Host URL for ollama (optional, defaults to localhost:11434)
#[tauri::command]
pub async fn configure_meilisearch_chat(
    provider: String,
    api_key: Option<String>,
    model: Option<String>,
    custom_system_prompt: Option<String>,
    host: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Build ChatProviderConfig from individual parameters
    let provider_config = match provider.to_lowercase().as_str() {
        "openai" => ChatProviderConfig::OpenAI {
            api_key: api_key.ok_or("OpenAI requires an API key")?,
            model,
            organization_id: None,
        },
        "claude" => ChatProviderConfig::Claude {
            api_key: api_key.ok_or("Claude requires an API key")?,
            model,
            max_tokens: Some(4096),
        },
        "mistral" => ChatProviderConfig::Mistral {
            api_key: api_key.ok_or("Mistral requires an API key")?,
            model,
        },
        "gemini" => ChatProviderConfig::Gemini {
            api_key: api_key.ok_or("Gemini requires an API key")?,
            model,
        },
        "ollama" => ChatProviderConfig::Ollama {
            host: host.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model: model.unwrap_or_else(|| "llama3:latest".to_string()),
        },
        "openrouter" => ChatProviderConfig::OpenRouter {
            api_key: api_key.ok_or("OpenRouter requires an API key")?,
            model: model.ok_or("OpenRouter requires a model")?,
        },
        "groq" => ChatProviderConfig::Groq {
            api_key: api_key.ok_or("Groq requires an API key")?,
            model: model.ok_or("Groq requires a model")?,
        },
        "together" => ChatProviderConfig::Together {
            api_key: api_key.ok_or("Together requires an API key")?,
            model: model.ok_or("Together requires a model")?,
        },
        "cohere" => ChatProviderConfig::Cohere {
            api_key: api_key.ok_or("Cohere requires an API key")?,
            model: model.ok_or("Cohere requires a model")?,
        },
        "deepseek" => ChatProviderConfig::DeepSeek {
            api_key: api_key.ok_or("DeepSeek requires an API key")?,
            model: model.ok_or("DeepSeek requires a model")?,
        },
        "grok" => ChatProviderConfig::Grok {
            api_key: api_key.ok_or("Grok requires an API key")?,
            model,
        },
        _ => {
            let valid_providers = crate::core::meilisearch_chat::list_chat_providers()
                .into_iter()
                .map(|p| p.id)
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!("Unknown provider: {}. Valid providers: {}", provider, valid_providers));
        }
    };

    // Build custom prompts if system prompt provided
    let custom_prompts = custom_system_prompt.map(|prompt| ChatPrompts {
        system: Some(prompt),
        ..Default::default()
    });

    // Configure Meilisearch chat client and workspace under single write lock
    let manager_guard = state.llm_manager.write().await;
    manager_guard.set_chat_client(state.search_client.host(), Some(&state.sidecar_manager.config().master_key)).await;
    manager_guard
        .configure_chat_workspace("dm-assistant", provider_config, custom_prompts)
        .await?;

    log::info!("Meilisearch chat configured with provider: {}", provider);
    Ok(())
}


// ============================================================================
// Model Selection Commands
// ============================================================================

/// Get the recommended model selection based on subscription plan and usage.
///
/// Returns a ModelSelection with the recommended model, plan info, and selection reason.
/// Uses medium task complexity as the default.
#[tauri::command]
pub async fn get_model_selection() -> Result<ModelSelection, String> {
    let selector = model_selector();
    selector.get_selection(TaskComplexity::Medium).await
}

/// Get the recommended model selection with complexity auto-detected from the prompt.
///
/// Analyzes the prompt for keywords that indicate task complexity (light/medium/heavy)
/// and returns the appropriate model recommendation.
#[tauri::command]
pub async fn get_model_selection_for_prompt(prompt: String) -> Result<ModelSelection, String> {
    let selector = model_selector();
    selector.get_selection_for_prompt(&prompt).await
}

/// Set a manual model override that bypasses automatic selection.
///
/// Pass `None` to clear the override and return to automatic selection.
/// Pass `Some("claude-opus-4-20250514")` or similar to force a specific model.
#[tauri::command]
pub async fn set_model_override(model: Option<String>) -> Result<(), String> {
    let selector = model_selector();
    selector.set_override(model).await;
    Ok(())
}

// ============================================================================
// TTRPG Document Commands
// ============================================================================

/// List TTRPG documents by source document ID
#[tauri::command]
pub async fn list_ttrpg_documents_by_source(
    source_document_id: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGDocumentRecord>, String> {
    with_db!(db, |db| db.list_ttrpg_documents_by_source(&source_document_id))
}

/// List TTRPG documents by element type
#[tauri::command]
pub async fn list_ttrpg_documents_by_type(
    element_type: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGDocumentRecord>, String> {
    with_db!(db, |db| db.list_ttrpg_documents_by_type(&element_type))
}

/// List TTRPG documents by game system
#[tauri::command]
pub async fn list_ttrpg_documents_by_system(
    game_system: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGDocumentRecord>, String> {
    with_db!(db, |db| db.list_ttrpg_documents_by_system(&game_system))
}

/// Search TTRPG documents by name pattern
#[tauri::command]
pub async fn search_ttrpg_documents_by_name(
    name_pattern: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGDocumentRecord>, String> {
    with_db!(db, |db| db.search_ttrpg_documents_by_name(&name_pattern))
}

/// List TTRPG documents by challenge rating range
#[tauri::command]
pub async fn list_ttrpg_documents_by_cr(
    min_cr: f64,
    max_cr: f64,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGDocumentRecord>, String> {
    with_db!(db, |db| db.list_ttrpg_documents_by_cr(min_cr, max_cr))
}

/// Get a specific TTRPG document by ID
#[tauri::command]
pub async fn get_ttrpg_document(
    id: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Option<crate::database::TTRPGDocumentRecord>, String> {
    with_db!(db, |db| db.get_ttrpg_document(&id))
}

/// Get attributes for a TTRPG document
#[tauri::command]
pub async fn get_ttrpg_document_attributes(
    document_id: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGDocumentAttribute>, String> {
    with_db!(db, |db| db.get_ttrpg_document_attributes(&document_id))
}

/// Find TTRPG documents by attribute
#[tauri::command]
pub async fn find_ttrpg_documents_by_attribute(
    attribute_type: String,
    attribute_value: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGDocumentRecord>, String> {
    with_db!(db, |db| db.find_ttrpg_documents_by_attribute(&attribute_type, &attribute_value))
}

/// Delete a TTRPG document
#[tauri::command]
pub async fn delete_ttrpg_document(
    id: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<(), String> {
    with_db!(db, |db| db.delete_ttrpg_document(&id))
}

/// Get TTRPG document statistics
#[tauri::command]
pub async fn get_ttrpg_document_stats(
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<crate::database::TTRPGDocumentStats, String> {
    with_db!(db, |db| db.get_ttrpg_document_stats())
}

/// Count TTRPG documents grouped by type
#[tauri::command]
pub async fn count_ttrpg_documents_by_type(
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<(String, i64)>, String> {
    with_db!(db, |db| db.count_ttrpg_documents_by_type())
}

/// Get TTRPG ingestion job status
#[tauri::command]
pub async fn get_ttrpg_ingestion_job(
    job_id: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Option<crate::database::TTRPGIngestionJob>, String> {
    with_db!(db, |db| db.get_ttrpg_ingestion_job(&job_id))
}

/// Get TTRPG ingestion job for a document
#[tauri::command]
pub async fn get_ttrpg_ingestion_job_by_document(
    document_id: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Option<crate::database::TTRPGIngestionJob>, String> {
    with_db!(db, |db| db.get_ttrpg_ingestion_job_by_document(&document_id))
}

/// List pending TTRPG ingestion jobs
#[tauri::command]
pub async fn list_pending_ttrpg_ingestion_jobs(
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGIngestionJob>, String> {
    with_db!(db, |db| db.list_pending_ttrpg_ingestion_jobs())
}

/// List active TTRPG ingestion jobs
#[tauri::command]
pub async fn list_active_ttrpg_ingestion_jobs(
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGIngestionJob>, String> {
    with_db!(db, |db| db.list_active_ttrpg_ingestion_jobs())
}

// ============================================================================
// Extraction Settings Commands
// ============================================================================

use crate::ingestion::{ExtractionSettings, SupportedFormats};

/// Get current extraction settings
#[tauri::command]
pub async fn get_extraction_settings(
    state: tauri::State<'_, AppState>,
) -> Result<ExtractionSettings, String> {
    // Try to load from state or return defaults
    let settings_guard = state.extraction_settings.read().await;
    Ok(settings_guard.clone())
}

/// Save extraction settings
#[tauri::command]
pub async fn save_extraction_settings(
    settings: ExtractionSettings,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Validate settings
    settings.validate()?;

    // Save to state
    let mut settings_guard = state.extraction_settings.write().await;
    *settings_guard = settings;

    log::info!("Extraction settings saved");
    Ok(())
}

/// Get supported file formats for extraction
#[tauri::command]
pub fn get_supported_formats() -> SupportedFormats {
    SupportedFormats::get_all()
}

/// Get extraction settings presets
#[tauri::command]
pub fn get_extraction_presets() -> Vec<ExtractionPreset> {
    vec![
        ExtractionPreset {
            name: "Default".to_string(),
            description: "Balanced settings for most documents".to_string(),
            settings: ExtractionSettings::default(),
        },
        ExtractionPreset {
            name: "TTRPG Rulebooks".to_string(),
            description: "Optimized for tabletop RPG rulebooks and sourcebooks".to_string(),
            settings: ExtractionSettings::for_rulebooks(),
        },
        ExtractionPreset {
            name: "Scanned Documents".to_string(),
            description: "For scanned PDFs requiring OCR processing".to_string(),
            settings: ExtractionSettings::for_scanned_documents(),
        },
        ExtractionPreset {
            name: "Quick Extract".to_string(),
            description: "Fast extraction with minimal processing".to_string(),
            settings: ExtractionSettings::quick(),
        },
    ]
}

/// Extraction settings preset
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractionPreset {
    pub name: String,
    pub description: String,
    pub settings: ExtractionSettings,
}

/// Check if OCR is available on the system
#[tauri::command]
pub async fn check_ocr_availability() -> OcrAvailability {
    use tokio::process::Command;

    let tesseract_available = Command::new("tesseract")
        .arg("--version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    let pdftoppm_available = Command::new("pdftoppm")
        .arg("-v")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    // Get installed tesseract languages
    let languages = if tesseract_available {
        Command::new("tesseract")
            .arg("--list-langs")
            .output()
            .await
            .ok()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .skip(1) // Skip header line
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    OcrAvailability {
        tesseract_installed: tesseract_available,
        pdftoppm_installed: pdftoppm_available,
        available_languages: languages,
        external_ocr_ready: tesseract_available && pdftoppm_available,
    }
}

/// OCR availability status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OcrAvailability {
    pub tesseract_installed: bool,
    pub pdftoppm_installed: bool,
    pub available_languages: Vec<String>,
    pub external_ocr_ready: bool,
}

// ============================================================================
// Claude Gate OAuth Commands
// ============================================================================

/// Response for claude_gate_get_status command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeGateStatusResponse {
    /// Whether the user is authenticated with valid tokens
    pub authenticated: bool,
    /// Current storage backend being used (file, keyring, auto)
    pub storage_backend: String,
    /// Unix timestamp when token expires, if authenticated
    pub token_expires_at: Option<i64>,
    /// Whether keyring (secret service) is available on this system
    pub keyring_available: bool,
}

/// Get Claude Gate OAuth status
///
/// Returns authentication status, storage backend, token expiration, and keyring availability.
#[tauri::command]
pub async fn claude_gate_get_status(
    state: State<'_, AppState>,
) -> Result<ClaudeGateStatusResponse, String> {
    let authenticated = state.claude_gate.is_authenticated().await?;
    let storage_backend = state.claude_gate.storage_backend_name().await;

    let token_expires_at = if authenticated {
        state.claude_gate.get_token_info().await?
            .map(|t| t.expires_at)
    } else {
        None
    };

    // Check if keyring is available on this system
    #[cfg(feature = "keyring")]
    let keyring_available = crate::claude_gate::KeyringTokenStorage::is_available();
    #[cfg(not(feature = "keyring"))]
    let keyring_available = false;

    Ok(ClaudeGateStatusResponse {
        authenticated,
        storage_backend,
        token_expires_at,
        keyring_available,
    })
}

/// Response for claude_gate_start_oauth command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeGateOAuthStartResponse {
    /// URL to open in user's browser for OAuth authorization
    pub auth_url: String,
    /// State parameter for CSRF protection (pass back to complete_oauth)
    pub state: String,
}

/// Start Claude Gate OAuth flow
///
/// Returns the authorization URL that the user should open in their browser,
/// along with a state parameter for CSRF verification.
#[tauri::command]
pub async fn claude_gate_start_oauth(
    state: State<'_, AppState>,
) -> Result<ClaudeGateOAuthStartResponse, String> {
    let (auth_url, oauth_state) = state.claude_gate.start_oauth_flow().await?;

    log::info!("Claude Gate OAuth flow started");

    Ok(ClaudeGateOAuthStartResponse {
        auth_url,
        state: oauth_state,
    })
}

/// Response for claude_gate_complete_oauth command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeGateOAuthCompleteResponse {
    /// Whether the OAuth flow completed successfully
    pub success: bool,
    /// Error message if the flow failed
    pub error: Option<String>,
}

/// Complete Claude Gate OAuth flow
///
/// Exchange the authorization code for tokens and store them.
///
/// # Arguments
/// * `code` - The authorization code from the OAuth callback. May also be in
///   `code#state` format where the state is embedded after a `#` character.
/// * `oauth_state` - Optional state parameter for CSRF verification (if not embedded in code)
#[tauri::command]
pub async fn claude_gate_complete_oauth(
    code: String,
    oauth_state: Option<String>,
    state: State<'_, AppState>,
) -> Result<ClaudeGateOAuthCompleteResponse, String> {
    // Parse code#state format if present
    let (actual_code, embedded_state) = if let Some(hash_pos) = code.find('#') {
        let (c, s) = code.split_at(hash_pos);
// Only treat as embedded state if there is content after the '#' character
        let embedded = if s.len() > 1 {
            Some(s[1..].to_string())
        } else {
            None
        };
        (c.to_string(), embedded)
    } else {
        (code, None)
    };

    // Use embedded state if present, otherwise use the provided oauth_state
    let final_state = embedded_state.or(oauth_state);

    log::debug!(
        "OAuth complete: code_len={}, state_provided={}",
        actual_code.len(),
        final_state.is_some()
    );

    match state.claude_gate.complete_oauth_flow(&actual_code, final_state.as_deref()).await {
        Ok(_token) => {
            log::info!("Claude Gate OAuth flow completed successfully");
            Ok(ClaudeGateOAuthCompleteResponse {
                success: true,
                error: None,
            })
        }
        Err(e) => {
            log::error!("Claude Gate OAuth flow failed: {}", e);
            Ok(ClaudeGateOAuthCompleteResponse {
                success: false,
                error: Some(e),
            })
        }
    }
}

/// Response for claude_gate_logout command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeGateLogoutResponse {
    /// Whether logout was successful
    pub success: bool,
}

/// Logout from Claude Gate and remove stored tokens
#[tauri::command]
pub async fn claude_gate_logout(
    state: State<'_, AppState>,
) -> Result<ClaudeGateLogoutResponse, String> {
    state.claude_gate.logout().await?;
    log::info!("Claude Gate logout completed");

    Ok(ClaudeGateLogoutResponse {
        success: true,
    })
}

/// Response for claude_gate_set_storage_backend command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeGateSetStorageResponse {
    /// Whether the storage backend was changed successfully
    pub success: bool,
    /// The currently active storage backend after the change
    pub active_backend: String,
}

/// Change Claude Gate storage backend
///
/// Note: Changing the storage backend requires re-authentication as tokens
/// are not automatically migrated between backends.
///
/// # Arguments
/// * `backend` - Storage backend to use: "file", "keyring", or "auto"
#[tauri::command]
pub async fn claude_gate_set_storage_backend(
    backend: String,
    state: State<'_, AppState>,
) -> Result<ClaudeGateSetStorageResponse, String> {
    // Parse and validate the backend string
    let new_backend: ClaudeGateStorageBackend = backend.parse()?;

    // Switch to the new backend - this recreates the client
    let active = state.claude_gate.switch_backend(new_backend).await?;
    log::info!("Claude Gate storage backend switched to: {}", active);

    Ok(ClaudeGateSetStorageResponse {
        success: true,
        active_backend: active,
    })
}

/// Model info returned from Claude Gate API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeGateModelInfo {
    /// Model ID (e.g., "claude-sonnet-4-20250514")
    pub id: String,
    /// Display name (may be same as ID if not provided)
    pub name: String,
}

/// List available models from Claude Gate API
///
/// Requires authentication. Returns list of models the user can access.
#[tauri::command]
pub async fn claude_gate_list_models(
    state: State<'_, AppState>,
) -> Result<Vec<ClaudeGateModelInfo>, String> {
    // Check if authenticated
    if !state.claude_gate.is_authenticated().await? {
        return Err("Not authenticated. Please log in first.".to_string());
    }

    // Get models from API
    let models = state.claude_gate.list_models().await?;

    // Convert to response format
    let model_infos: Vec<ClaudeGateModelInfo> = models
        .into_iter()
        .map(|m| ClaudeGateModelInfo {
            id: m.id.clone(),
            name: if m.display_name.is_empty() {
                m.id
            } else {
                m.display_name
            },
        })
        .collect();

    log::info!("Claude Gate: Listed {} models", model_infos.len());
    Ok(model_infos)
}

// ============================================================================
// Phase 4: Personality Extension Commands (TASK-PERS-014, TASK-PERS-015, TASK-PERS-016, TASK-PERS-017)
// ============================================================================

// ----------------------------------------------------------------------------
// Request/Response Types for Personality Extensions
// ----------------------------------------------------------------------------

/// Request for applying a template to a campaign
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyTemplateRequest {
    /// Template ID to apply
    pub template_id: String,
    /// Campaign ID to apply the template to
    pub campaign_id: String,
    /// Optional session ID for immediate application
    #[serde(default)]
    pub session_id: Option<String>,
}

/// Request for creating a template from an existing personality
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTemplateFromPersonalityRequest {
    /// Personality ID to create template from
    pub personality_id: String,
    /// Name for the new template
    pub name: String,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
    /// Optional game system
    #[serde(default)]
    pub game_system: Option<String>,
    /// Optional setting name
    #[serde(default)]
    pub setting_name: Option<String>,
}

/// Request for setting a blend rule
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBlendRuleRequest {
    /// Rule name
    pub name: String,
    /// Context this rule applies to (e.g., "combat_encounter")
    pub context: String,
    /// Campaign ID (None for global rules)
    #[serde(default)]
    pub campaign_id: Option<String>,
    /// Blend components as [(personality_id, weight)]
    pub components: Vec<BlendComponentInput>,
    /// Priority (higher = evaluated first)
    #[serde(default)]
    pub priority: i32,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Input for a blend component
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlendComponentInput {
    /// Personality ID
    pub personality_id: String,
    /// Weight (0.0-1.0)
    pub weight: f32,
}

/// Request for context detection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectContextRequest {
    /// User input text to analyze
    pub user_input: String,
    /// Optional session state for enhanced detection
    #[serde(default)]
    pub session_state: Option<SessionStateSnapshot>,
}

/// Request for contextual personality lookup
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContextualPersonalityRequest {
    /// Campaign ID
    pub campaign_id: String,
    /// User input text
    pub user_input: String,
    /// Optional session state
    #[serde(default)]
    pub session_state: Option<SessionStateSnapshot>,
}

/// Response for template preview
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplatePreviewResponse {
    /// Template ID
    pub id: String,
    /// Template name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Base profile ID
    pub base_profile: String,
    /// Game system
    pub game_system: Option<String>,
    /// Setting name
    pub setting_name: Option<String>,
    /// Whether it's a built-in template
    pub is_builtin: bool,
    /// Tags
    pub tags: Vec<String>,
    /// Number of vocabulary entries
    pub vocabulary_count: usize,
    /// Number of common phrases
    pub phrase_count: usize,
}

impl From<SettingTemplate> for TemplatePreviewResponse {
    fn from(template: SettingTemplate) -> Self {
        Self {
            id: template.id.to_string(),
            name: template.name.clone(),
            description: template.description.clone(),
            base_profile: template.base_profile.to_string(),
            game_system: template.game_system.clone(),
            setting_name: template.setting_name.clone(),
            is_builtin: template.is_builtin,
            tags: template.tags.clone(),
            vocabulary_count: template.vocabulary.len(),
            phrase_count: template.common_phrases.len(),
        }
    }
}

/// Response for blend rule
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlendRuleResponse {
    /// Rule ID
    pub id: String,
    /// Rule name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Context
    pub context: String,
    /// Priority
    pub priority: i32,
    /// Whether the rule is enabled
    pub enabled: bool,
    /// Whether it's a built-in rule
    pub is_builtin: bool,
    /// Campaign ID
    pub campaign_id: Option<String>,
    /// Blend weights
    pub blend_weights: Vec<BlendComponentInput>,
    /// Tags
    pub tags: Vec<String>,
}

impl From<BlendRule> for BlendRuleResponse {
    fn from(rule: BlendRule) -> Self {
        Self {
            id: rule.id.to_string(),
            name: rule.name,
            description: rule.description,
            context: rule.context,
            priority: rule.priority,
            enabled: rule.enabled,
            is_builtin: rule.is_builtin,
            campaign_id: rule.campaign_id,
            blend_weights: rule
                .blend_weights
                .into_iter()
                .map(|(id, weight)| BlendComponentInput {
                    personality_id: id.to_string(),
                    weight,
                })
                .collect(),
            tags: rule.tags,
        }
    }
}

// ----------------------------------------------------------------------------
// TASK-PERS-014: Template Tauri Commands
// ----------------------------------------------------------------------------

/// List all personality templates
#[tauri::command]
pub async fn list_personality_templates(
    state: State<'_, AppState>,
) -> Result<Vec<TemplatePreviewResponse>, String> {
    let templates = state.template_store.list_with_limit(1000).await.map_err(|e| e.to_string())?;
    Ok(templates.into_iter().map(TemplatePreviewResponse::from).collect())
}

/// Filter templates by game system
#[tauri::command]
pub async fn filter_templates_by_game_system(
    game_system: String,
    state: State<'_, AppState>,
) -> Result<Vec<TemplatePreviewResponse>, String> {
    let templates = state.template_store.filter_by_game_system(&game_system).await.map_err(|e| e.to_string())?;
    Ok(templates.into_iter().map(TemplatePreviewResponse::from).collect())
}

/// Filter templates by setting name
#[tauri::command]
pub async fn filter_templates_by_setting(
    setting_name: String,
    state: State<'_, AppState>,
) -> Result<Vec<TemplatePreviewResponse>, String> {
    let templates = state.template_store.filter_by_setting(&setting_name).await.map_err(|e| e.to_string())?;
    Ok(templates.into_iter().map(TemplatePreviewResponse::from).collect())
}

/// Search personality templates by keyword
#[tauri::command]
pub async fn search_personality_templates(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<TemplatePreviewResponse>, String> {
    let templates = state.template_store.search_with_limit(&query, 100).await.map_err(|e| e.to_string())?;
    Ok(templates.into_iter().map(TemplatePreviewResponse::from).collect())
}

/// Get template preview by ID
#[tauri::command]
pub async fn get_template_preview(
    template_id: String,
    state: State<'_, AppState>,
) -> Result<Option<TemplatePreviewResponse>, String> {
    let id = TemplateId::new(template_id);
    let template = state.template_store.get(&id).await.map_err(|e| e.to_string())?;
    Ok(template.map(TemplatePreviewResponse::from))
}

/// Apply a template to a campaign
#[tauri::command]
pub async fn apply_template_to_campaign(
    request: ApplyTemplateRequest,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let id = TemplateId::new(&request.template_id);

    // Get the template
    let template = state.template_store.get(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Template not found: {}", request.template_id))?;

    // Get the base profile that the template extends
    let base_profile = state.personality_store.get(template.base_profile.as_str())
        .map_err(|e| format!("Base profile not found: {}", e))?;

    // Convert template to profile by applying overrides to base
    let profile = template.to_personality_profile(&base_profile);
    let profile_id = profile.id.clone();

    // Store the generated profile
    state.personality_store.create(profile)
        .map_err(|e| format!("Failed to store profile: {}", e))?;

    log::info!(
        "Applied template '{}' to campaign '{}', created profile '{}'",
        template.name,
        request.campaign_id,
        profile_id
    );

    Ok(profile_id)
}

/// Create a template from an existing personality profile
#[tauri::command]
pub async fn create_template_from_personality(
    request: CreateTemplateFromPersonalityRequest,
    state: State<'_, AppState>,
) -> Result<TemplatePreviewResponse, String> {
    // Get the source personality
    let profile = state.personality_store.get(&request.personality_id)
        .map_err(|e| format!("Personality not found: {}", e))?;

    // Create template from profile using the builder
    let mut template = SettingTemplate::new(&request.name, PersonalityId::new(&request.personality_id));
    template.description = request.description;
    template.game_system = request.game_system;
    template.setting_name = request.setting_name;

    // Copy common phrases from the profile
    template.common_phrases = profile.speech_patterns.common_phrases.clone();

    // Copy tags
    template.tags = profile.tags.clone();

    // Save template
    state.template_store.save(&template).await.map_err(|e| e.to_string())?;

    log::info!("Created template '{}' from personality '{}'", template.name, request.personality_id);

    Ok(TemplatePreviewResponse::from(template))
}

/// Export a personality template to YAML
#[tauri::command]
pub async fn export_personality_template(
    template_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let id = TemplateId::new(template_id);

    let template = state.template_store.get(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Template not found".to_string())?;

    // Convert to YAML
    serde_yaml_ng::to_string(&template).map_err(|e| format!("YAML serialization failed: {}", e))
}

/// Import a personality template from YAML
#[tauri::command]
pub async fn import_personality_template(
    yaml_content: String,
    state: State<'_, AppState>,
) -> Result<TemplatePreviewResponse, String> {
    // Parse YAML
    let template: SettingTemplate = serde_yaml_ng::from_str(&yaml_content)
        .map_err(|e| format!("YAML parse failed: {}", e))?;

    // Save template
    state.template_store.save(&template).await.map_err(|e| e.to_string())?;

    log::info!("Imported template '{}'", template.name);

    Ok(TemplatePreviewResponse::from(template))
}

// ----------------------------------------------------------------------------
// TASK-PERS-015: Blend Rule Tauri Commands
// ----------------------------------------------------------------------------

/// Set (create or update) a blend rule
#[tauri::command]
pub async fn set_blend_rule(
    request: SetBlendRuleRequest,
    state: State<'_, AppState>,
) -> Result<BlendRuleResponse, String> {
    // Build the blend rule
    let mut rule = BlendRule::new(&request.name, &request.context);
    rule.campaign_id = request.campaign_id;
    rule.priority = request.priority;
    rule.description = request.description;
    rule.tags = request.tags;

    // Add components
    for comp in request.components {
        rule = rule.with_component(PersonalityId::new(comp.personality_id), comp.weight);
    }

    // Normalize weights
    rule.normalize_weights();

    // Save rule
    let saved = state.blend_rule_store.set_rule(rule).await.map_err(|e| e.to_string())?;

    log::info!("Set blend rule '{}' for context '{}'", saved.name, saved.context);

    Ok(BlendRuleResponse::from(saved))
}

/// Get a blend rule by campaign and context
#[tauri::command]
pub async fn get_blend_rule(
    campaign_id: Option<String>,
    context: String,
    state: State<'_, AppState>,
) -> Result<Option<BlendRuleResponse>, String> {
    let ctx: GameplayContext = match context.parse() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to parse gameplay context '{}': {}. Defaulting to Unknown.", context, e);
            GameplayContext::Unknown
        }
    };
    let rule = state.blend_rule_store
        .get_rule_for_context(campaign_id.as_deref(), &ctx)
        .await
        .map_err(|e| e.to_string())?;

    Ok(rule.map(BlendRuleResponse::from))
}

/// List blend rules for a campaign
#[tauri::command]
pub async fn list_blend_rules(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<BlendRuleResponse>, String> {
    let rules = state.blend_rule_store
        .list_by_campaign(&campaign_id, 1000)
        .await
        .map_err(|e| e.to_string())?;

    Ok(rules.into_iter().map(BlendRuleResponse::from).collect())
}

/// Delete a blend rule
#[tauri::command]
pub async fn delete_blend_rule(
    rule_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let id = BlendRuleId::new(rule_id);
    state.blend_rule_store.delete_rule(&id).await.map_err(|e| e.to_string())?;

    log::info!("Deleted blend rule '{}'", id);

    Ok(())
}

// ----------------------------------------------------------------------------
// TASK-PERS-016: Context Detection Commands
// ----------------------------------------------------------------------------

/// Detect gameplay context from input
#[tauri::command]
pub async fn detect_gameplay_context(
    request: DetectContextRequest,
    state: State<'_, AppState>,
) -> Result<ContextDetectionResult, String> {
    let result = state.contextual_personality_manager
        .detect_context(&request.user_input, request.session_state.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(result)
}

// ----------------------------------------------------------------------------
// TASK-PERS-017: Contextual Personality Commands
// ----------------------------------------------------------------------------

/// Get contextual personality for a session
///
/// Combines context detection, blend rule lookup, and personality blending
/// to return the appropriate personality for the current conversation context.
#[tauri::command]
pub async fn get_contextual_personality(
    request: GetContextualPersonalityRequest,
    state: State<'_, AppState>,
) -> Result<ContextualPersonalityResult, String> {
    let result = state.contextual_personality_manager
        .get_contextual_personality(
            &request.campaign_id,
            request.session_state.as_ref(),
            &request.user_input,
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(result)
}

/// Get the current smoothed context without applying blend rules
#[tauri::command]
pub async fn get_current_context(
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let context = state.contextual_personality_manager.current_context().await;
    Ok(context.map(|c| c.as_str().to_string()))
}

/// Clear context detection history for a fresh start
#[tauri::command]
pub async fn clear_context_history(
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.contextual_personality_manager.clear_context_history().await;
    log::info!("Cleared context detection history");
    Ok(())
}

/// Get contextual personality configuration
#[tauri::command]
pub async fn get_contextual_personality_config(
    state: State<'_, AppState>,
) -> Result<ContextualConfig, String> {
    Ok(state.contextual_personality_manager.config().await)
}

/// Update contextual personality configuration
#[tauri::command]
pub async fn set_contextual_personality_config(
    config: ContextualConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.contextual_personality_manager.set_config(config).await;
    log::info!("Updated contextual personality configuration");
    Ok(())
}

/// Get personality blender cache statistics
#[tauri::command]
pub async fn get_blender_cache_stats(
    state: State<'_, AppState>,
) -> Result<BlenderCacheStats, String> {
    Ok(state.personality_blender.cache_stats().await)
}

/// Get blend rule cache statistics
#[tauri::command]
pub async fn get_blend_rule_cache_stats(
    state: State<'_, AppState>,
) -> Result<RuleCacheStats, String> {
    Ok(state.blend_rule_store.cache_stats().await)
}

/// List all gameplay context types
#[tauri::command]
pub fn list_gameplay_contexts() -> Vec<GameplayContextInfo> {
    GameplayContext::all_defined()
        .into_iter()
        .map(|ctx| GameplayContextInfo {
            id: ctx.as_str().to_string(),
            name: ctx.display_name().to_string(),
            description: ctx.description().to_string(),
            is_combat_related: ctx.is_combat_related(),
        })
        .collect()
}

/// Info about a gameplay context for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayContextInfo {
    /// Context ID (e.g., "combat_encounter")
    pub id: String,
    /// Display name (e.g., "Combat Encounter")
    pub name: String,
    /// Description of when this context applies
    pub description: String,
    /// Whether this is a combat-related context
    pub is_combat_related: bool,
}

// ============================================================================
// Utility Commands
// ============================================================================

/// Open a URL in the system's default browser
///
/// Uses Tauri's shell plugin to open URLs properly on all platforms.
#[tauri::command]
pub async fn open_url_in_browser(
    url: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    use tauri_plugin_shell::ShellExt;

    app_handle.shell().open(&url, None)
        .map_err(|e| format!("Failed to open URL: {}", e))
}

// ============================================================================
// Archetype Registry Commands (TASK-ARCH-060 through TASK-ARCH-063)
// ============================================================================

// ============================================================================
// Request/Response Types for Archetype Commands
// ============================================================================

/// Request payload for creating a new archetype.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateArchetypeRequest {
    /// Unique identifier for the archetype (e.g., "dwarf_merchant").
    pub id: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Category: "role", "race", "class", or "setting".
    pub category: String,
    /// Optional parent archetype ID for inheritance.
    pub parent_id: Option<String>,
    /// Optional description text.
    pub description: Option<String>,
    /// Personality trait affinities.
    #[serde(default)]
    pub personality_affinity: Vec<PersonalityAffinityInput>,
    /// NPC role mappings.
    #[serde(default)]
    pub npc_role_mapping: Vec<NpcRoleMappingInput>,
    /// Naming culture weights.
    #[serde(default)]
    pub naming_cultures: Vec<NamingCultureWeightInput>,
    /// Optional vocabulary bank ID reference.
    pub vocabulary_bank_id: Option<String>,
    /// Optional stat tendencies.
    pub stat_tendencies: Option<StatTendenciesInput>,
    /// Tags for categorization and search.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Input type for personality affinity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalityAffinityInput {
    pub trait_id: String,
    pub weight: f32,
}

/// Input type for NPC role mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcRoleMappingInput {
    pub role: String,
    pub weight: f32,
}

/// Input type for naming culture weight.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamingCultureWeightInput {
    pub culture: String,
    pub weight: f32,
}

/// Input type for stat tendencies.
///
/// Uses HashMaps to support arbitrary stat names for different game systems.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatTendenciesInput {
    /// Stat modifiers (e.g., {"strength": 2, "charisma": -1}).
    #[serde(default)]
    pub modifiers: std::collections::HashMap<String, i32>,
    /// Minimum stat values (e.g., {"constitution": 12}).
    #[serde(default)]
    pub minimums: std::collections::HashMap<String, u8>,
    /// Priority order for stat allocation (e.g., ["strength", "constitution"]).
    #[serde(default)]
    pub priority_order: Vec<String>,
}

/// Response for archetype operations that return an archetype.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchetypeResponse {
    pub id: String,
    pub display_name: String,
    pub category: String,
    pub parent_id: Option<String>,
    pub description: Option<String>,
    pub personality_affinity: Vec<PersonalityAffinityInput>,
    pub npc_role_mapping: Vec<NpcRoleMappingInput>,
    pub naming_cultures: Vec<NamingCultureWeightInput>,
    pub vocabulary_bank_id: Option<String>,
    pub stat_tendencies: Option<StatTendenciesInput>,
    pub tags: Vec<String>,
}

impl From<Archetype> for ArchetypeResponse {
    fn from(a: Archetype) -> Self {
        Self {
            id: a.id.to_string(),
            display_name: a.display_name.to_string(),
            category: format!("{:?}", a.category).to_lowercase(),
            parent_id: a.parent_id.map(|p| p.to_string()),
            description: a.description.map(|d| d.to_string()),
            personality_affinity: a.personality_affinity.into_iter()
                .map(|p| PersonalityAffinityInput {
                    trait_id: p.trait_id,
                    weight: p.weight,
                })
                .collect(),
            npc_role_mapping: a.npc_role_mapping.into_iter()
                .map(|m| NpcRoleMappingInput {
                    role: m.role,
                    weight: m.weight,
                })
                .collect(),
            naming_cultures: a.naming_cultures.into_iter()
                .map(|c| NamingCultureWeightInput {
                    culture: c.culture,
                    weight: c.weight,
                })
                .collect(),
            vocabulary_bank_id: a.vocabulary_bank_id,
            stat_tendencies: a.stat_tendencies.map(|s| StatTendenciesInput {
                modifiers: s.modifiers,
                minimums: s.minimums,
                priority_order: s.priority_order,
            }),
            tags: a.tags,
        }
    }
}

/// Response for archetype list operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchetypeSummaryResponse {
    pub id: String,
    pub display_name: String,
    pub category: String,
    pub tags: Vec<String>,
}

impl From<ArchetypeSummary> for ArchetypeSummaryResponse {
    fn from(s: ArchetypeSummary) -> Self {
        Self {
            id: s.id.to_string(),
            display_name: s.display_name.to_string(),
            category: format!("{:?}", s.category).to_lowercase(),
            tags: s.tags,
        }
    }
}

/// Request for resolution query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionQueryRequest {
    /// Direct archetype ID to resolve.
    pub archetype_id: Option<String>,
    /// NPC role for role-based resolution layer.
    pub npc_role: Option<String>,
    /// Race for race-based resolution layer.
    pub race: Option<String>,
    /// Class for class-based resolution layer.
    pub class: Option<String>,
    /// Setting pack ID for setting overrides.
    pub setting: Option<String>,
    /// Campaign ID for campaign-specific setting pack.
    pub campaign_id: Option<String>,
}

/// Response for resolved archetype.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedArchetypeResponse {
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub category: Option<String>,
    pub personality_affinity: Vec<PersonalityAffinityInput>,
    pub npc_role_mapping: Vec<NpcRoleMappingInput>,
    pub naming_cultures: Vec<NamingCultureWeightInput>,
    pub vocabulary_bank_id: Option<String>,
    pub stat_tendencies: Option<StatTendenciesInput>,
    pub tags: Vec<String>,
    pub resolution_metadata: Option<ResolutionMetadataResponse>,
}

/// Metadata about the resolution process.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionMetadataResponse {
    pub layers_checked: Vec<String>,
    pub merge_operations: usize,
    pub resolution_time_ms: Option<u64>,
    pub cache_hit: bool,
}

impl From<ResolvedArchetype> for ResolvedArchetypeResponse {
    fn from(r: ResolvedArchetype) -> Self {
        Self {
            id: r.id.map(|id| id.to_string()),
            display_name: r.display_name.map(|n| n.to_string()),
            category: r.category.map(|c| format!("{:?}", c).to_lowercase()),
            personality_affinity: r.personality_affinity.into_iter()
                .map(|p| PersonalityAffinityInput {
                    trait_id: p.trait_id,
                    weight: p.weight,
                })
                .collect(),
            npc_role_mapping: r.npc_role_mapping.into_iter()
                .map(|m| NpcRoleMappingInput {
                    role: m.role,
                    weight: m.weight,
                })
                .collect(),
            naming_cultures: r.naming_cultures.into_iter()
                .map(|c| NamingCultureWeightInput {
                    culture: c.culture,
                    weight: c.weight,
                })
                .collect(),
            vocabulary_bank_id: r.vocabulary_bank_id,
            stat_tendencies: r.stat_tendencies.map(|s| StatTendenciesInput {
                modifiers: s.modifiers,
                minimums: s.minimums,
                priority_order: s.priority_order,
            }),
            tags: r.tags,
            resolution_metadata: r.resolution_metadata.map(|m| ResolutionMetadataResponse {
                layers_checked: m.layers_checked,
                merge_operations: m.merge_operations,
                resolution_time_ms: m.resolution_time_ms,
                cache_hit: m.cache_hit,
            }),
        }
    }
}

/// Response for setting pack summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingPackSummaryResponse {
    pub id: String,
    pub name: String,
    pub version: String,
    pub game_system: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
}

impl From<SettingPackSummary> for SettingPackSummaryResponse {
    fn from(s: SettingPackSummary) -> Self {
        Self {
            id: s.id,
            name: s.name,
            version: s.version,
            game_system: s.game_system,
            author: s.author,
            tags: s.tags,
        }
    }
}

/// Response for vocabulary bank summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VocabularyBankSummaryResponse {
    pub id: String,
    pub display_name: String,
    pub culture: Option<String>,
    pub role: Option<String>,
    pub is_builtin: bool,
    pub category_count: usize,
    pub phrase_count: usize,
}

impl From<VocabularyBankSummary> for VocabularyBankSummaryResponse {
    fn from(s: VocabularyBankSummary) -> Self {
        Self {
            id: s.id,
            display_name: s.display_name,
            culture: s.culture,
            role: s.role,
            is_builtin: s.is_builtin,
            category_count: s.category_count,
            phrase_count: s.phrase_count,
        }
    }
}

/// Request for creating a vocabulary bank.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVocabularyBankRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub culture: Option<String>,
    pub role: Option<String>,
    #[serde(default)]
    pub phrases: Vec<PhraseInput>,
}

/// Input type for a phrase in vocabulary bank.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhraseInput {
    pub text: String,
    pub category: String,
    #[serde(default = "default_formality")]
    pub formality: u8,
    pub tones: Option<Vec<String>>,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_formality() -> u8 {
    5
}

/// Response for vocabulary bank.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VocabularyBankResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub culture: Option<String>,
    pub role: Option<String>,
    pub phrases: Vec<PhraseOutput>,
}

/// Output type for a phrase.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhraseOutput {
    pub text: String,
    pub category: String,
    pub formality: u8,
    pub tone: Option<String>,
    pub tags: Vec<String>,
}

/// Filter options for listing phrases.
///
/// Note: category is passed as a separate required parameter to get_phrases.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhraseFilterRequest {
    pub formality_min: Option<u8>,
    pub formality_max: Option<u8>,
    pub tone: Option<String>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse category string to ArchetypeCategory enum.
fn parse_category(s: &str) -> Result<ArchetypeCategory, String> {
    match s.to_lowercase().as_str() {
        "role" => Ok(ArchetypeCategory::Role),
        "race" => Ok(ArchetypeCategory::Race),
        "class" => Ok(ArchetypeCategory::Class),
        "setting" => Ok(ArchetypeCategory::Setting),
        _ => Err(format!("Invalid category: {}. Must be 'role', 'race', 'class', or 'setting'", s)),
    }
}

/// Get the archetype registry from state, returning error if not initialized.
async fn get_registry(state: &AppState) -> Result<Arc<ArchetypeRegistry>, String> {
    state.archetype_registry
        .read()
        .await
        .as_ref()
        .cloned()
        .ok_or_else(|| "Archetype registry not initialized. Please wait for Meilisearch to start.".to_string())
}

/// Get the vocabulary manager from state, returning error if not initialized.
async fn get_vocabulary_manager(state: &AppState) -> Result<Arc<VocabularyBankManager>, String> {
    state.vocabulary_manager
        .read()
        .await
        .as_ref()
        .cloned()
        .ok_or_else(|| "Vocabulary manager not initialized. Please wait for Meilisearch to start.".to_string())
}

// ============================================================================
// TASK-ARCH-060: Archetype CRUD Commands
// ============================================================================

/// Create a new archetype.
///
/// # Arguments
/// * `request` - Archetype creation request with all fields
///
/// # Returns
/// The ID of the created archetype.
///
/// # Errors
/// - If archetype ID already exists
/// - If parent_id references non-existent archetype
/// - If validation fails
#[tauri::command]
pub async fn create_archetype(
    request: CreateArchetypeRequest,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let registry = get_registry(&state).await?;

    let category = parse_category(&request.category)?;

    let mut archetype = Archetype::new(request.id.clone(), request.display_name.as_str(), category);

    if let Some(parent) = request.parent_id {
        archetype = archetype.with_parent(parent);
    }

    if let Some(desc) = request.description {
        archetype = archetype.with_description(desc);
    }

    // Add personality affinities
    let affinities: Vec<PersonalityAffinity> = request.personality_affinity
        .into_iter()
        .map(|p| PersonalityAffinity::new(p.trait_id, p.weight))
        .collect();
    if !affinities.is_empty() {
        archetype = archetype.with_personality_affinity(affinities);
    }

    // Add NPC role mappings
    let mappings: Vec<NpcRoleMapping> = request.npc_role_mapping
        .into_iter()
        .map(|m| NpcRoleMapping::new(m.role, m.weight))
        .collect();
    if !mappings.is_empty() {
        archetype = archetype.with_npc_role_mapping(mappings);
    }

    // Add naming cultures
    let cultures: Vec<NamingCultureWeight> = request.naming_cultures
        .into_iter()
        .map(|c| NamingCultureWeight::new(c.culture, c.weight))
        .collect();
    if !cultures.is_empty() {
        archetype = archetype.with_naming_cultures(cultures);
    }

    // Add vocabulary bank reference
    if let Some(vocab_id) = request.vocabulary_bank_id {
        archetype = archetype.with_vocabulary_bank(vocab_id);
    }

    // Add stat tendencies
    if let Some(stats) = request.stat_tendencies {
        let tendencies = StatTendencies {
            modifiers: stats.modifiers,
            minimums: stats.minimums,
            priority_order: stats.priority_order,
        };
        archetype = archetype.with_stat_tendencies(tendencies);
    }

    // Add tags
    archetype = archetype.with_tags(request.tags);

    let id = registry.register(archetype).await
        .map_err(|e| e.to_string())?;

    log::info!("Created archetype: {}", id);
    Ok(id.to_string())
}

/// Get an archetype by ID.
///
/// # Arguments
/// * `id` - The archetype ID
///
/// # Returns
/// The full archetype data.
#[tauri::command]
pub async fn get_archetype(
    id: String,
    state: State<'_, AppState>,
) -> Result<ArchetypeResponse, String> {
    let registry = get_registry(&state).await?;

    let archetype = registry.get(&id).await
        .map_err(|e| e.to_string())?;

    Ok(ArchetypeResponse::from(archetype))
}

/// List all archetypes, optionally filtered by category.
///
/// # Arguments
/// * `category` - Optional category filter: "role", "race", "class", or "setting"
///
/// # Returns
/// List of archetype summaries.
#[tauri::command]
pub async fn list_archetypes(
    category: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ArchetypeSummaryResponse>, String> {
    let registry = get_registry(&state).await?;

    let filter = category
        .map(|c| parse_category(&c))
        .transpose()?;

    let summaries = registry.list(filter).await;

    Ok(summaries.into_iter().map(ArchetypeSummaryResponse::from).collect())
}

/// Update an existing archetype.
///
/// # Arguments
/// * `request` - Archetype update request (must have existing ID)
///
/// # Errors
/// - If archetype doesn't exist
/// - If validation fails
#[tauri::command]
pub async fn update_archetype(
    request: CreateArchetypeRequest,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let registry = get_registry(&state).await?;

    let category = parse_category(&request.category)?;

    let archetype_id = request.id.clone();
    let mut archetype = Archetype::new(request.id, request.display_name.as_str(), category);

    if let Some(parent) = request.parent_id {
        archetype = archetype.with_parent(parent);
    }

    if let Some(desc) = request.description {
        archetype = archetype.with_description(desc);
    }

    let affinities: Vec<PersonalityAffinity> = request.personality_affinity
        .into_iter()
        .map(|p| PersonalityAffinity::new(p.trait_id, p.weight))
        .collect();
    if !affinities.is_empty() {
        archetype = archetype.with_personality_affinity(affinities);
    }

    let mappings: Vec<NpcRoleMapping> = request.npc_role_mapping
        .into_iter()
        .map(|m| NpcRoleMapping::new(m.role, m.weight))
        .collect();
    if !mappings.is_empty() {
        archetype = archetype.with_npc_role_mapping(mappings);
    }

    let cultures: Vec<NamingCultureWeight> = request.naming_cultures
        .into_iter()
        .map(|c| NamingCultureWeight::new(c.culture, c.weight))
        .collect();
    if !cultures.is_empty() {
        archetype = archetype.with_naming_cultures(cultures);
    }

    if let Some(vocab_id) = request.vocabulary_bank_id {
        archetype = archetype.with_vocabulary_bank(vocab_id);
    }

    if let Some(stats) = request.stat_tendencies {
        let tendencies = StatTendencies {
            modifiers: stats.modifiers,
            minimums: stats.minimums,
            priority_order: stats.priority_order,
        };
        archetype = archetype.with_stat_tendencies(tendencies);
    }

    archetype = archetype.with_tags(request.tags);

    registry.update(archetype).await
        .map_err(|e| e.to_string())?;

    log::info!("Updated archetype: {}", archetype_id);
    Ok(())
}

/// Delete an archetype.
///
/// # Arguments
/// * `id` - The archetype ID to delete
/// * `force` - If true, ignore dependent children check
///
/// # Errors
/// - If archetype doesn't exist
/// - If archetype has dependent children (unless force=true)
#[tauri::command]
pub async fn delete_archetype(
    id: String,
    force: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let registry = get_registry(&state).await?;

    // Note: The registry's delete method checks for children automatically.
    // For force deletion, we would need to delete children first.
    // For now, we don't support force deletion - user must delete children first.
    if force.unwrap_or(false) {
        return Err("Force deletion is not yet supported. Please delete child archetypes first.".to_string());
    }

    registry.delete(&id).await
        .map_err(|e| e.to_string())?;

    log::info!("Deleted archetype: {}", id);
    Ok(())
}

/// Check if an archetype exists.
///
/// # Arguments
/// * `id` - The archetype ID to check
///
/// # Returns
/// True if the archetype exists, false otherwise.
#[tauri::command]
pub async fn archetype_exists(
    id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let registry = get_registry(&state).await?;
    Ok(registry.exists(&id).await)
}

/// Get the total count of archetypes.
#[tauri::command]
pub async fn count_archetypes(
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let registry = get_registry(&state).await?;
    Ok(registry.count().await)
}

// ============================================================================
// TASK-ARCH-061: Vocabulary Bank Commands
// ============================================================================

/// Create a new vocabulary bank.
///
/// # Arguments
/// * `request` - Vocabulary bank creation request
///
/// # Returns
/// The ID of the created vocabulary bank.
#[tauri::command]
pub async fn create_vocabulary_bank(
    request: CreateVocabularyBankRequest,
    state: State<'_, AppState>,
) -> Result<String, String> {
    use crate::core::archetype::setting_pack::{VocabularyBankDefinition, PhraseDefinition};

    let manager = get_vocabulary_manager(&state).await?;

    // Build VocabularyBankDefinition
    let mut definition = VocabularyBankDefinition::new(&request.id, &request.name);

    if let Some(desc) = request.description {
        definition.description = Some(desc);
    }

    if let Some(culture) = request.culture {
        definition.culture = Some(culture);
    }

    if let Some(role) = request.role {
        definition.role = Some(role);
    }

    // Group phrases by category and add to definition
    let mut phrase_groups: std::collections::HashMap<String, Vec<PhraseDefinition>> = std::collections::HashMap::new();
    for phrase in request.phrases {
        let mut phrase_def = PhraseDefinition::new(&phrase.text);
        phrase_def.formality = phrase.formality;
        if let Some(tones) = phrase.tones {
            phrase_def.tone_markers = tones;
        }
        phrase_def.context_tags = phrase.tags;

        phrase_groups
            .entry(phrase.category)
            .or_default()
            .push(phrase_def);
    }
    definition.phrases = phrase_groups;

    // Create VocabularyBank from definition
    let bank = VocabularyBank::from_definition(definition);

    let id = manager.register(bank).await
        .map_err(|e| e.to_string())?;

    log::info!("Created vocabulary bank: {}", id);
    Ok(id)
}

/// Get a vocabulary bank by ID.
///
/// # Arguments
/// * `id` - The vocabulary bank ID
///
/// # Returns
/// The full vocabulary bank data.
#[tauri::command]
pub async fn get_vocabulary_bank(
    id: String,
    state: State<'_, AppState>,
) -> Result<VocabularyBankResponse, String> {
    let manager = get_vocabulary_manager(&state).await?;

    let bank = manager.get_bank(&id).await
        .map_err(|e| e.to_string())?;

    // Flatten phrases from HashMap<String, Vec<PhraseDefinition>> to Vec<PhraseOutput>
    let phrases: Vec<PhraseOutput> = bank.definition.phrases
        .iter()
        .flat_map(|(category, phrase_list)| {
            phrase_list.iter().map(move |p| PhraseOutput {
                text: p.text.clone(),
                category: category.clone(),
                formality: p.formality,
                tone: p.tone_markers.first().cloned(),
                tags: p.context_tags.clone(),
            })
        })
        .collect();

    Ok(VocabularyBankResponse {
        id: bank.definition.id.clone(),
        name: bank.definition.display_name.clone(),
        description: bank.definition.description.clone(),
        culture: bank.definition.culture.clone(),
        role: bank.definition.role.clone(),
        phrases,
    })
}

/// List all vocabulary banks with optional filtering.
///
/// # Arguments
/// * `culture` - Optional culture filter
/// * `role` - Optional role filter
///
/// # Returns
/// List of vocabulary bank summaries.
#[tauri::command]
pub async fn list_vocabulary_banks(
    culture: Option<String>,
    role: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<VocabularyBankSummaryResponse>, String> {
    let manager = get_vocabulary_manager(&state).await?;

    let filter = BankListFilter {
        culture,
        role,
        race: None,
        builtin_only: None,
    };

    let summaries = manager.list_banks(Some(filter)).await;

    Ok(summaries.into_iter().map(VocabularyBankSummaryResponse::from).collect())
}

/// Update an existing vocabulary bank.
///
/// # Arguments
/// * `request` - Vocabulary bank update request (must have existing ID)
#[tauri::command]
pub async fn update_vocabulary_bank(
    request: CreateVocabularyBankRequest,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use crate::core::archetype::setting_pack::{VocabularyBankDefinition, PhraseDefinition};

    let manager = get_vocabulary_manager(&state).await?;

    // Build VocabularyBankDefinition
    let mut definition = VocabularyBankDefinition::new(&request.id, &request.name);

    if let Some(desc) = request.description {
        definition.description = Some(desc);
    }

    if let Some(culture) = request.culture {
        definition.culture = Some(culture);
    }

    if let Some(role) = request.role {
        definition.role = Some(role);
    }

    // Group phrases by category and add to definition
    let mut phrase_groups: std::collections::HashMap<String, Vec<PhraseDefinition>> = std::collections::HashMap::new();
    for phrase in request.phrases {
        let mut phrase_def = PhraseDefinition::new(&phrase.text);
        phrase_def.formality = phrase.formality;
        if let Some(tones) = phrase.tones {
            phrase_def.tone_markers = tones;
        }
        phrase_def.context_tags = phrase.tags;

        phrase_groups
            .entry(phrase.category)
            .or_default()
            .push(phrase_def);
    }
    definition.phrases = phrase_groups;

    // Create VocabularyBank from definition
    let bank = VocabularyBank::from_definition(definition);

    manager.update(bank).await
        .map_err(|e| e.to_string())?;

    log::info!("Updated vocabulary bank: {}", request.id);
    Ok(())
}

/// Delete a vocabulary bank.
///
/// # Arguments
/// * `id` - The vocabulary bank ID to delete
///
/// # Errors
/// - If vocabulary bank doesn't exist
/// - If vocabulary bank is in use by archetypes
#[tauri::command]
pub async fn delete_vocabulary_bank(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = get_vocabulary_manager(&state).await?;

    manager.delete_bank(&id).await
        .map_err(|e| e.to_string())?;

    log::info!("Deleted vocabulary bank: {}", id);
    Ok(())
}

/// Get phrases from a vocabulary bank with optional filtering.
///
/// This command returns just the phrase text strings, filtered by category,
/// formality range, and tone. It uses session-based tracking to avoid
/// repeating the same phrase.
///
/// # Arguments
/// * `bank_id` - The vocabulary bank ID
/// * `category` - Required category to filter by (e.g., "greetings")
/// * `filter` - Optional additional filters for formality and tone
/// * `session_id` - Session ID for usage tracking (prevents repeating phrases)
///
/// # Returns
/// List of matching phrase texts.
#[tauri::command]
pub async fn get_phrases(
    bank_id: String,
    category: String,
    filter: Option<PhraseFilterRequest>,
    session_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let manager = get_vocabulary_manager(&state).await?;

    // Build filter options starting with category (required)
    let mut opts = PhraseFilterOptions::for_category(&category);

    if let Some(f) = filter {
        if let (Some(min), Some(max)) = (f.formality_min, f.formality_max) {
            opts = opts.with_formality(min, max);
        }
        if let Some(tone) = f.tone {
            opts = opts.with_tone(&tone);
        }
    }

    // Use provided session_id or generate a temporary one
    let session = session_id.unwrap_or_else(|| "default".to_string());

    let phrases = manager.get_phrases(&bank_id, opts, &session).await
        .map_err(|e| e.to_string())?;

    Ok(phrases)
}

// ============================================================================
// TASK-ARCH-062: Setting Pack Commands
// ============================================================================

/// Load a setting pack from a file path.
///
/// # Arguments
/// * `path` - Path to the YAML or JSON setting pack file
///
/// # Returns
/// The version key of the loaded pack (format: "pack_id@version").
#[tauri::command]
pub async fn load_setting_pack(
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let loader = &state.setting_pack_loader;

    let vkey = loader.load_from_file(&path).await
        .map_err(|e| e.to_string())?;

    log::info!("Loaded setting pack from {}: {}", path, vkey);
    Ok(vkey)
}

/// List all loaded setting packs.
///
/// # Returns
/// List of setting pack summaries (latest version of each).
#[tauri::command]
pub async fn list_setting_packs(
    state: State<'_, AppState>,
) -> Result<Vec<SettingPackSummaryResponse>, String> {
    let loader = &state.setting_pack_loader;

    let summaries = loader.list_packs().await;

    Ok(summaries.into_iter().map(SettingPackSummaryResponse::from).collect())
}

/// Get a setting pack by ID.
///
/// # Arguments
/// * `pack_id` - The setting pack ID
/// * `version` - Optional specific version (uses latest if not specified)
///
/// # Returns
/// The setting pack data.
#[tauri::command]
pub async fn get_setting_pack(
    pack_id: String,
    version: Option<String>,
    state: State<'_, AppState>,
) -> Result<SettingPackSummaryResponse, String> {
    let loader = &state.setting_pack_loader;

    let pack = if let Some(ver) = version {
        loader.get_version(&pack_id, &ver).await
    } else {
        loader.get_latest(&pack_id).await
    }.map_err(|e| e.to_string())?;

    Ok(SettingPackSummaryResponse::from(SettingPackSummary::from(&pack)))
}

/// Activate a setting pack for a campaign.
///
/// # Arguments
/// * `pack_id` - The setting pack ID to activate
/// * `campaign_id` - The campaign ID to activate for
///
/// # Errors
/// - If pack is not loaded
/// - If pack references missing archetypes
#[tauri::command]
pub async fn activate_setting_pack(
    pack_id: String,
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let loader = &state.setting_pack_loader;
    let registry = get_registry(&state).await?;

    // Get existing archetype IDs for validation
    let existing: std::collections::HashSet<String> = registry.list(None).await
        .into_iter()
        .map(|s| s.id.to_string())
        .collect();

    loader.activate(&pack_id, &campaign_id, &existing).await
        .map_err(|e| e.to_string())?;

    log::info!("Activated setting pack '{}' for campaign '{}'", pack_id, campaign_id);
    Ok(())
}

/// Deactivate the setting pack for a campaign.
///
/// # Arguments
/// * `campaign_id` - The campaign ID to deactivate pack for
#[tauri::command]
pub async fn deactivate_setting_pack(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let loader = &state.setting_pack_loader;

    loader.deactivate(&campaign_id).await
        .map_err(|e| e.to_string())?;

    log::info!("Deactivated setting pack for campaign '{}'", campaign_id);
    Ok(())
}

/// Get the active setting pack for a campaign.
///
/// # Arguments
/// * `campaign_id` - The campaign ID
///
/// # Returns
/// The active setting pack summary, or null if none active.
#[tauri::command]
pub async fn get_active_setting_pack(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<Option<SettingPackSummaryResponse>, String> {
    let loader = &state.setting_pack_loader;

    let pack = loader.get_active_pack(&campaign_id).await;

    Ok(pack.map(|p| SettingPackSummaryResponse::from(SettingPackSummary::from(&p))))
}

/// Get all versions of a setting pack.
///
/// # Arguments
/// * `pack_id` - The setting pack ID
///
/// # Returns
/// List of version strings sorted by semver.
#[tauri::command]
pub async fn get_setting_pack_versions(
    pack_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let loader = &state.setting_pack_loader;

    Ok(loader.get_versions(&pack_id).await)
}

// ============================================================================
// TASK-ARCH-063: Resolution Query Commands
// ============================================================================

/// Resolve an archetype using the hierarchical resolution system.
///
/// Resolution applies layers in order: Role -> Race -> Class -> Setting -> Direct ID.
/// Later layers override earlier ones according to merge rules.
///
/// # Arguments
/// * `query` - The resolution query specifying which layers to apply
///
/// # Returns
/// The resolved archetype with merged data from all applicable layers.
#[tauri::command]
pub async fn resolve_archetype(
    query: ResolutionQueryRequest,
    state: State<'_, AppState>,
) -> Result<ResolvedArchetypeResponse, String> {
    let registry = get_registry(&state).await?;

    // Build the resolution query
    let mut resolution_query = if let Some(ref id) = query.archetype_id {
        ResolutionQuery::single(id)
    } else if let Some(ref role) = query.npc_role {
        ResolutionQuery::for_npc(role)
    } else {
        return Err("Either archetype_id or npc_role must be specified".to_string());
    };

    if let Some(ref race) = query.race {
        resolution_query = resolution_query.with_race(race);
    }

    if let Some(ref class) = query.class {
        resolution_query = resolution_query.with_class(class);
    }

    if let Some(ref setting) = query.setting {
        resolution_query = resolution_query.with_setting(setting);
    }

    if let Some(ref campaign) = query.campaign_id {
        resolution_query = resolution_query.with_campaign(campaign);
    }

    // Check cache first
    if let Some(cached) = registry.get_cached(&resolution_query).await {
        let mut response = ResolvedArchetypeResponse::from(cached);
        if let Some(ref mut meta) = response.resolution_metadata {
            meta.cache_hit = true;
        }
        return Ok(response);
    }

    // Create resolver and resolve
    let resolver = crate::core::archetype::ArchetypeResolver::new(
        registry.archetypes(),
        registry.setting_packs(),
        registry.active_packs(),
    );

    let resolved = resolver.resolve(&resolution_query).await
        .map_err(|e| e.to_string())?;

    // Cache the result
    registry.cache_resolved(&resolution_query, resolved.clone()).await;

    Ok(ResolvedArchetypeResponse::from(resolved))
}

/// Convenience command to resolve an archetype for NPC generation.
///
/// This is a shortcut for the common use case of resolving by role, race, and class.
///
/// # Arguments
/// * `role` - The NPC role (e.g., "merchant", "guard")
/// * `race` - Optional race (e.g., "dwarf", "elf")
/// * `class` - Optional class (e.g., "fighter", "wizard")
/// * `setting` - Optional setting pack ID
/// * `campaign_id` - Optional campaign ID for campaign-specific settings
///
/// # Returns
/// The resolved archetype with merged data.
#[tauri::command]
pub async fn resolve_for_npc(
    role: String,
    race: Option<String>,
    class: Option<String>,
    setting: Option<String>,
    campaign_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<ResolvedArchetypeResponse, String> {
    let query = ResolutionQueryRequest {
        archetype_id: None,
        npc_role: Some(role),
        race,
        class,
        setting,
        campaign_id,
    };

    resolve_archetype(query, state).await
}

/// Get cache statistics for the archetype registry.
///
/// # Returns
/// Cache statistics including size and capacity.
#[tauri::command]
pub async fn get_archetype_cache_stats(
    state: State<'_, AppState>,
) -> Result<ArchetypeCacheStatsResponse, String> {
    let registry = get_registry(&state).await?;

    let stats = registry.cache_stats().await;

    Ok(ArchetypeCacheStatsResponse {
        current_size: stats.len,
        capacity: stats.cap,
    })
}

/// Response for cache statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchetypeCacheStatsResponse {
    pub current_size: usize,
    pub capacity: usize,
}

/// Clear the archetype resolution cache.
#[tauri::command]
pub async fn clear_archetype_cache(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let registry = get_registry(&state).await?;

    registry.clear_cache().await;

    log::info!("Cleared archetype resolution cache");
    Ok(())
}

/// Check if the archetype registry is initialized.
///
/// # Returns
/// True if the registry is ready to use, false otherwise.
#[tauri::command]
pub async fn is_archetype_registry_ready(
    state: State<'_, AppState>,
) -> Result<bool, String> {
    Ok(state.archetype_registry.read().await.is_some())
}
