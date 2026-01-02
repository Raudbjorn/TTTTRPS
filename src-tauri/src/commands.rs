//! Tauri Commands
//!
//! All Tauri IPC commands exposed to the frontend.

use tauri::State;
use crate::core::voice::{
    VoiceManager, VoiceConfig, VoiceProviderType, ElevenLabsConfig,
    OllamaConfig, SynthesisRequest, OutputFormat, VoiceProviderDetection,
    detect_providers,
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
use crate::core::llm::{LLMConfig, LLMClient, ChatMessage, ChatRequest, MessageRole};
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
use crate::core::character_gen::backstory::{
    BackstoryGenerator, BackstoryRequest, GeneratedBackstory,
    BackstoryStyle, RegenerationOptions, EditResult, BackstoryNPC,
};
use crate::core::character_gen::BackstoryLength;
use crate::core::npc_gen::{NPCGenerator, NPCGenerationOptions, NPC, NPCStore};
use crate::core::location_gen::{LocationGenerator, LocationGenerationOptions, Location, LocationType};
use crate::core::personality::{
    PersonalityApplicationManager, ActivePersonalityContext, SceneMood,
    PersonalityApplicationOptions, ContentType, StyledContent, PersonalityPreview,
    NPCDialogueStyler, NarrationStyleManager, NarrationType, PersonalitySettings,
    ExtendedPersonalityPreview, PreviewResponse, NarrativeTone, VocabularyLevel,
    NarrativeStyle, VerbosityLevel, GenreConvention, PersonalityStore,
};
use crate::core::credentials::CredentialManager;
use crate::core::audio::AudioVolumes;
use crate::core::sidecar_manager::{SidecarManager, MeilisearchConfig};
use crate::core::search_client::SearchClient;
use crate::core::meilisearch_pipeline::MeilisearchPipeline;
use crate::core::meilisearch_chat::{DMChatManager, ChatMessage as MeiliChatMessage};
use crate::core::campaign::versioning::VersionManager;
use crate::core::campaign::world_state::WorldStateManager;
use crate::core::campaign::relationships::RelationshipManager;
use crate::core::session::notes::{
    NoteCategory, EntityType as NoteEntityType, EntityLink, NotesManager,
    SessionNote as NoteSessionNote, CategorizationRequest, CategorizationResponse,
    build_categorization_prompt, parse_categorization_response,
};

fn serialize_enum_to_string<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_default()
}

// ============================================================================
// Application State
// ============================================================================

pub struct AppState {
    // pub database: Option<Database>,
    pub llm_client: RwLock<Option<LLMClient>>,
    pub llm_config: RwLock<Option<LLMConfig>>,
    pub llm_router: AsyncRwLock<LLMRouter>,
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
    ) {
        let sidecar_config = MeilisearchConfig::default();
        let search_client = SearchClient::new(
            &sidecar_config.url(),
            Some(&sidecar_config.master_key),
        );
        let personality_store = Arc::new(PersonalityStore::new());
        let personality_manager = Arc::new(PersonalityApplicationManager::new(personality_store.clone()));

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
        )
    }
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

#[tauri::command]
pub async fn configure_llm(
    settings: LLMSettings,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let config = match settings.provider.as_str() {
        "ollama" => LLMConfig::Ollama {
            host: settings.host.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model: settings.model,
        },
        "claude" => LLMConfig::Claude {
            api_key: settings.api_key.clone().ok_or("Claude requires an API key")?,
            model: settings.model,
            max_tokens: 4096,
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

    // RAG Mode: Route through Meilisearch Chat API
    if payload.use_rag {
        let sidecar_config = state.sidecar_manager.config();
        let dm_chat = DMChatManager::new(
            &sidecar_config.url(),
            Some(&sidecar_config.master_key),
        );

        // Get API key for the configured LLM provider
        let api_key = match &config {
            LLMConfig::OpenAI { api_key, .. } => api_key.clone(),
            LLMConfig::Claude { api_key, .. } => api_key.clone(),
            LLMConfig::Gemini { api_key, .. } => api_key.clone(),
            LLMConfig::OpenRouter { api_key, .. } => api_key.clone(),
            LLMConfig::Mistral { api_key, .. } => api_key.clone(),
            LLMConfig::Groq { api_key, .. } => api_key.clone(),
            LLMConfig::Together { api_key, .. } => api_key.clone(),
            LLMConfig::Cohere { api_key, .. } => api_key.clone(),
            LLMConfig::DeepSeek { api_key, .. } => api_key.clone(),
            LLMConfig::Ollama { .. } => String::new(),
        };

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
        };

        // Initialize the DM chat workspace (idempotent)
        if !api_key.is_empty() {
            dm_chat.initialize(&api_key, Some(&model), Some(&system_prompt)).await
                .map_err(|e| format!("Failed to initialize RAG: {}", e))?;
        }

        // Build conversation history
        let mut meili_messages = vec![];
        if let Some(context) = &payload.context {
            for ctx in context {
                meili_messages.push(MeiliChatMessage::user(ctx));
            }
        }
        meili_messages.push(MeiliChatMessage::user(&payload.message));

        // Send to Meilisearch Chat (with automatic RAG)
        let response = dm_chat.chat_with_history(meili_messages, &model).await
            .map_err(|e| format!("RAG chat failed: {}", e))?;

        return Ok(ChatResponsePayload {
            content: response,
            model,
            input_tokens: None, // Meilisearch doesn't report token usage
            output_tokens: None,
        });
    }

    // Standard Mode: Router call
    let mut messages = vec![];

    if let Some(context) = &payload.context {
        for ctx in context {
            messages.push(ChatMessage {
                role: MessageRole::User,
                content: ctx.clone(),
            });
        }
    }

    messages.push(ChatMessage {
        role: MessageRole::User,
        content: payload.message,
    });

    let request = ChatRequest {
        messages,
        system_prompt: Some(system_prompt),
        temperature: Some(0.7),
        max_tokens: Some(2048),
        provider: None,
    };

    let router = (*state.llm_router.read().await).clone();
    let response = router.chat(request).await.map_err(|e| e.to_string())?;

    Ok(ChatResponsePayload {
        content: response.content,
        model: response.model,
        input_tokens: response.usage.as_ref().map(|u| u.input_tokens),
        output_tokens: response.usage.as_ref().map(|u| u.output_tokens),
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
    state: State<'_, AppState>,
) -> Result<String, String> {
    use tauri::Emitter;

    let request = ChatRequest {
        messages,
        system_prompt,
        temperature,
        max_tokens,
        provider: None,
    };

    let router = state.llm_router.read().await;
    let router_clone = router.clone();
    drop(router);

    let mut rx = router_clone.stream_chat(request).await
        .map_err(|e| e.to_string())?;

    let mut stream_id = String::new();
    let mut full_content = String::new();

    // Process chunks and emit events
    while let Some(chunk_result) = rx.recv().await {
        match chunk_result {
            Ok(chunk) => {
                if stream_id.is_empty() {
                    stream_id = chunk.stream_id.clone();
                }
                full_content.push_str(&chunk.content);

                // Emit the chunk event
                let _ = app_handle.emit("chat-chunk", &chunk);

                if chunk.is_final {
                    break;
                }
            }
            Err(e) => {
                // Emit error event
                let error_chunk = ChatChunk {
                    stream_id: stream_id.clone(),
                    content: String::new(),
                    provider: String::new(),
                    model: String::new(),
                    is_final: true,
                    finish_reason: Some("error".to_string()),
                    usage: None,
                    index: 0,
                };
                let _ = app_handle.emit("chat-chunk", &error_chunk);
                return Err(e.to_string());
            }
        }
    }

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

/// Ingest a document with progress reporting via Tauri events
#[tauri::command]
pub async fn ingest_document_with_progress(
    app: tauri::AppHandle,
    path: String,
    source_type: Option<String>,
    state: State<'_, AppState>,
) -> Result<IngestResult, String> {
    use tauri::Emitter;

    let path_buf = std::path::Path::new(&path);
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

    // Get file size for rough progress estimation
    let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let estimated_pages = (file_size / 50_000).max(1) as usize; // Rough estimate: 50KB per page

    // Parse based on file type
    let extension = path_buf.extension().and_then(|e| e.to_str()).unwrap_or("");
    let format_name = match extension.to_lowercase().as_str() {
        "pdf" => "PDF",
        "epub" => "EPUB",
        "mobi" | "azw" | "azw3" => "MOBI",
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

    match extension.to_lowercase().as_str() {
        "pdf" => {
            use crate::ingestion::pdf_parser::PDFParser;
            let pages = PDFParser::extract_text_with_pages(path_buf)
                .map_err(|e| format!("PDF parsing failed: {}", e))?;
            page_count = pages.len();
            text_content = pages.into_iter().map(|(_, text)| text).collect::<Vec<_>>().join("\n\n");

            let _ = app.emit("ingest-progress", IngestProgress {
                stage: "parsing".to_string(),
                progress: 0.4,
                message: format!("Parsed {} pages", page_count),
                source_name: source_name.clone(),
            });
        }
        "epub" => {
            use crate::ingestion::epub_parser::EPUBParser;
            let extracted = EPUBParser::extract_structured(path_buf)
                .map_err(|e| format!("EPUB parsing failed: {}", e))?;
            page_count = extracted.chapter_count;
            text_content = extracted.chapters.into_iter().map(|c| c.text).collect::<Vec<_>>().join("\n\n");

            let _ = app.emit("ingest-progress", IngestProgress {
                stage: "parsing".to_string(),
                progress: 0.4,
                message: format!("Parsed {} chapters", page_count),
                source_name: source_name.clone(),
            });
        }
        "mobi" | "azw" | "azw3" => {
            use crate::ingestion::mobi_parser::MOBIParser;
            let extracted = MOBIParser::extract_structured(path_buf)
                .map_err(|e| format!("MOBI parsing failed: {}", e))?;
            page_count = extracted.section_count;
            text_content = extracted.sections.into_iter().map(|s| s.text).collect::<Vec<_>>().join("\n\n");

            let _ = app.emit("ingest-progress", IngestProgress {
                stage: "parsing".to_string(),
                progress: 0.4,
                message: format!("Parsed {} sections", page_count),
                source_name: source_name.clone(),
            });
        }
        "docx" => {
            use crate::ingestion::docx_parser::DOCXParser;
            let extracted = DOCXParser::extract_structured(path_buf)
                .map_err(|e| format!("DOCX parsing failed: {}", e))?;
            page_count = extracted.paragraphs.len();
            text_content = extracted.text;

            let _ = app.emit("ingest-progress", IngestProgress {
                stage: "parsing".to_string(),
                progress: 0.4,
                message: format!("Parsed {} paragraphs", page_count),
                source_name: source_name.clone(),
            });
        }
        "txt" | "md" | "markdown" => {
            text_content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read file: {}", e))?;
            page_count = text_content.lines().count() / 50; // Rough page estimate

            let _ = app.emit("ingest-progress", IngestProgress {
                stage: "parsing".to_string(),
                progress: 0.4,
                message: format!("Loaded {} characters", text_content.len()),
                source_name: source_name.clone(),
            });
        }
        _ => {
            // Try to read as text for unknown formats
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

    // Use the pipeline to process and index
    let result = state.ingestion_pipeline
        .process_file(
            &state.search_client,
            path_buf,
            &source_type,
            None
        )
        .await
        .map_err(|e| e.to_string())?;

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

#[tauri::command]
pub async fn speak(text: String, state: State<'_, AppState>) -> Result<(), String> {
    // 1. Determine config
    let config = {
        let config_guard = state.llm_config.read().map_err(|e| e.to_string())?;

        if let Some(c) = config_guard.as_ref() {
            match c {
                LLMConfig::Ollama { host, .. } => VoiceConfig {
                    provider: VoiceProviderType::Ollama,
                    ollama: Some(OllamaConfig {
                        base_url: host.clone(),
                        model: "bark".to_string(), // Default placeholder
                    }),
                    ..Default::default()
                },
                LLMConfig::Claude { api_key, .. } => VoiceConfig {
                    provider: VoiceProviderType::ElevenLabs,
                    elevenlabs: Some(ElevenLabsConfig {
                        api_key: api_key.clone(),
                        model_id: None,
                    }),
                    ..Default::default()
                },
                LLMConfig::Gemini { .. } => VoiceConfig::default(),
                LLMConfig::OpenAI { api_key, .. } => VoiceConfig {
                    provider: VoiceProviderType::OpenAI,
                    openai: Some(crate::core::voice::OpenAIVoiceConfig {
                        api_key: api_key.clone(),
                        model: "tts-1".to_string(),
                        voice: "alloy".to_string(),
                    }),
                    ..Default::default()
                },
                // Other providers don't have native TTS - use default (disabled)
                LLMConfig::OpenRouter { .. } |
                LLMConfig::Mistral { .. } |
                LLMConfig::Groq { .. } |
                LLMConfig::Together { .. } |
                LLMConfig::Cohere { .. } |
                LLMConfig::DeepSeek { .. } => VoiceConfig::default(),
            }
        } else {
             VoiceConfig::default()
        }
    };

    let manager = VoiceManager::new(config);

    // 2. Synthesize (async)
    let request = SynthesisRequest {
        text,
        voice_id: "default".to_string(),
        settings: None,
        output_format: OutputFormat::Mp3,
    };

    if let Ok(result) = manager.synthesize(request).await {
         // Read bytes from file (or implementation could return bytes directly if we changed it, but manager returns result with path)
         let bytes = std::fs::read(&result.audio_path).map_err(|e| e.to_string())?;

         // 3. Play
         tauri::async_runtime::spawn_blocking(move || {
             if let Err(e) = manager.play_audio(bytes) {
                 log::error!("Playback failed: {}", e);
             }
         }).await.map_err(|e| e.to_string())?;

         Ok(())
    } else {
        log::info!("Speak request received (synthesis skipped/failed)");
        Ok(())
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
    TimelineEvent, TimelineEventType, EventSeverity, EntityRef,
    SessionTimeline, TimelineSummary,
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
    AdvancedCondition, ConditionDuration, ConditionEffect, StackingRule,
    ConditionTracker, ConditionTemplates,
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
        }],
        system_prompt: Some("You are a TTRPG session note analyzer. Respond only with valid JSON.".to_string()),
        temperature: Some(0.3),
        max_tokens: Some(500),
        provider: None,
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
    Atmosphere, NotableFeature, Inhabitant, Secret, Encounter,
    LocationConnection, LootPotential, MapReference, Difficulty,
    Disposition, TreasureLevel, LocationSize,
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
