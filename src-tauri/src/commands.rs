//! Tauri Commands
//!
//! All Tauri IPC commands exposed to the frontend.

use tauri::State;
use crate::core::llm::{LLMConfig, LLMClient, ChatMessage, ChatRequest};
use crate::ingestion::pdf_parser;
use crate::ingestion::character_gen::{Character, CharacterGenerator, TTRPGGenre};
use crate::core::models::Campaign;
use std::sync::Mutex;
use std::path::Path;
use chrono::Utc;
use serde::{Deserialize, Serialize};

// ============================================================================
// Application State
// ============================================================================

pub struct AppState {
    pub llm_client: Mutex<Option<LLMClient>>,
    pub llm_config: Mutex<Option<LLMConfig>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            llm_client: Mutex::new(None),
            llm_config: Mutex::new(None),
        }
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequestPayload {
    pub message: String,
    pub system_prompt: Option<String>,
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
pub fn configure_llm(
    settings: LLMSettings,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let config = match settings.provider.as_str() {
        "ollama" => LLMConfig::Ollama {
            host: settings.host.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model: settings.model,
            embedding_model: Some("nomic-embed-text".to_string()),
        },
        "claude" => LLMConfig::Claude {
            api_key: settings.api_key.ok_or("Claude requires an API key")?,
            model: settings.model,
            max_tokens: 4096,
        },
        "gemini" => LLMConfig::Gemini {
            api_key: settings.api_key.ok_or("Gemini requires an API key")?,
            model: settings.model,
        },
        _ => return Err(format!("Unknown provider: {}", settings.provider)),
    };

    let client = LLMClient::new(config.clone());
    let provider_name = client.provider_name().to_string();

    // Store both config and client
    {
        let mut config_guard = state.llm_config.lock().map_err(|e| e.to_string())?;
        *config_guard = Some(config);
    }
    {
        let mut client_guard = state.llm_client.lock().map_err(|e| e.to_string())?;
        *client_guard = Some(client);
    }

    Ok(format!("Configured {} provider successfully", provider_name))
}

#[tauri::command]
pub async fn chat(
    payload: ChatRequestPayload,
    state: State<'_, AppState>,
) -> Result<ChatResponsePayload, String> {
    // Get config and create client in a sync block to avoid holding lock across await
    let config = {
        let config_guard = state.llm_config.lock().map_err(|e| e.to_string())?;
        config_guard.clone().ok_or("LLM not configured. Please configure in Settings.")?
    };

    let client = LLMClient::new(config);

    let request = ChatRequest::new(vec![ChatMessage::user(&payload.message)])
        .with_system(payload.system_prompt.unwrap_or_else(|| {
            "You are a helpful TTRPG Game Master assistant. Help the user with their tabletop RPG questions, \
             provide rules clarifications, generate content, and assist with running their campaign.".to_string()
        }));

    let response = client.chat(request).await.map_err(|e| e.to_string())?;

    Ok(ChatResponsePayload {
        content: response.content,
        model: response.model,
        input_tokens: response.usage.as_ref().map(|u| u.input_tokens),
        output_tokens: response.usage.as_ref().map(|u| u.output_tokens),
    })
}

#[tauri::command]
pub async fn check_llm_health(state: State<'_, AppState>) -> Result<HealthStatus, String> {
    // Get config in a sync block to avoid holding lock across await
    let config_opt = {
        let config_guard = state.llm_config.lock().map_err(|e| e.to_string())?;
        config_guard.clone()
    };

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
    let config_guard = state.llm_config.lock().map_err(|e| e.to_string())?;

    Ok(config_guard.as_ref().map(|config| match config {
        LLMConfig::Ollama { host, model, .. } => LLMSettings {
            provider: "ollama".to_string(),
            api_key: None,
            host: Some(host.clone()),
            model: model.clone(),
        },
        LLMConfig::Claude { model, .. } => LLMSettings {
            provider: "claude".to_string(),
            api_key: Some("********".to_string()), // Mask the key
            host: None,
            model: model.clone(),
        },
        LLMConfig::Gemini { model, .. } => LLMSettings {
            provider: "gemini".to_string(),
            api_key: Some("********".to_string()), // Mask the key
            host: None,
            model: model.clone(),
        },
    }))
}

// ============================================================================
// Document Ingestion Commands
// ============================================================================

#[tauri::command]
pub fn ingest_document(path: String) -> Result<String, String> {
    let text = pdf_parser::PDFParser::extract_text(Path::new(&path))
        .map_err(|e| e.to_string())?;
    Ok(format!("Ingested {} characters from document", text.len()))
}

// ============================================================================
// Search Commands
// ============================================================================

#[tauri::command]
pub async fn search(
    query: String,
    _state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    // Placeholder for vector search - will be implemented with LanceDB
    Ok(vec![format!("Search result for: {}", query)])
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
    let genre_enum = match genre.as_deref() {
        Some("scifi") | Some("SciFi") => Some(TTRPGGenre::SciFi),
        Some("cyberpunk") | Some("Cyberpunk") => Some(TTRPGGenre::Cyberpunk),
        Some("horror") | Some("Horror") | Some("CosmicHorror") => Some(TTRPGGenre::CosmicHorror),
        Some("postapoc") | Some("PostApocalyptic") => Some(TTRPGGenre::PostApocalyptic),
        Some("superhero") | Some("Superhero") => Some(TTRPGGenre::Superhero),
        Some("western") | Some("Western") => Some(TTRPGGenre::Western),
        _ => Some(TTRPGGenre::Fantasy),
    };

    let character = CharacterGenerator::generate(&system, level as i32, genre_enum);
    Ok(character)
}

// ============================================================================
// Campaign Commands
// ============================================================================

#[tauri::command]
pub fn list_campaigns() -> Result<Vec<Campaign>, String> {
    // Placeholder - will be implemented with SQLite
    Ok(vec![])
}

#[tauri::command]
pub fn create_campaign(
    name: String,
    system: String,
    description: Option<String>,
) -> Result<Campaign, String> {
    Ok(Campaign {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        system,
        description,
        current_date: "Session 1".to_string(),
        notes: vec![],
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
        settings: Default::default(),
    })
}

#[tauri::command]
pub fn get_campaign(id: String) -> Result<Option<Campaign>, String> {
    // Placeholder - will be implemented with SQLite
    let _ = id;
    Ok(None)
}

#[tauri::command]
pub fn delete_campaign(id: String) -> Result<bool, String> {
    // Placeholder - will be implemented with SQLite
    let _ = id;
    Ok(true)
}
