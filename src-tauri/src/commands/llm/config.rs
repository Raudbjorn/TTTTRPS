//! LLM Configuration Commands
//!
//! Commands for configuring LLM providers and persisting settings.

use std::path::PathBuf;
use tauri::State;
use tauri::Manager;

use crate::commands::state::AppState;
use crate::core::llm::{LLMConfig, LLMClient};
use crate::core::meilisearch_chat::ChatProviderConfig;
use crate::core::voice::VoiceConfig;

use super::types::{LLMSettings, HealthStatus};

// ============================================================================
// Constants
// ============================================================================

/// Providers that can auto-detect or have default models, so model selection is optional
const PROVIDERS_WITH_OPTIONAL_MODEL: &[&str] = &[];

// ============================================================================
// Helper Functions
// ============================================================================

fn get_config_path(app_handle: &tauri::AppHandle) -> PathBuf {
    // Ensure app data dir exists
    let dir = app_handle.path().app_data_dir().unwrap_or(PathBuf::from("."));
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    dir.join("llm_config.json")
}

/// Load LLM configuration from disk
pub fn load_llm_config_disk(app_handle: &tauri::AppHandle) -> Option<LLMConfig> {
    let path = get_config_path(app_handle);
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(path) {
            return serde_json::from_str(&content).ok();
        }
    }
    None
}

/// Save LLM configuration to disk
pub fn save_llm_config_disk(app_handle: &tauri::AppHandle, config: &LLMConfig) {
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

/// Load voice configuration from disk
pub fn load_voice_config_disk(app_handle: &tauri::AppHandle) -> Option<VoiceConfig> {
    let path = get_voice_config_path(app_handle);
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            return serde_json::from_str(&content).ok();
        }
    }
    None
}

// ============================================================================
// Commands
// ============================================================================

/// Configure LLM provider settings
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
        "google" => LLMConfig::Google {
            api_key: settings.api_key.clone().ok_or("Google requires an API key")?,
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
        "claude" => {
            // Use the actual resolved storage backend from ClaudeState
            // This ensures the LLM provider uses the same backend as the OAuth state
            let storage_backend = state.claude.storage_backend_name().await;
            LLMConfig::Claude {
                storage_backend,
                model: settings.model,
                max_tokens: 8192, // Default max tokens
            }
        }
        "gemini" => {
            // Use the actual resolved storage backend from GeminiState
            // This ensures the LLM provider uses the same backend as the OAuth state
            let storage_backend = state.gemini.storage_backend_name().await;
            LLMConfig::Gemini {
                storage_backend,
                model: settings.model,
                max_tokens: 8192, // Default max tokens
            }
        }
        "copilot" => {
            // Use the actual resolved storage backend from CopilotState
            // This ensures the LLM provider uses the same backend as the OAuth state
            let storage_backend = state.copilot.storage_backend_name().await;
            LLMConfig::Copilot {
                storage_backend,
                model: settings.model,
                max_tokens: 8192, // Default max tokens
            }
        }
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
    let prev_provider = state.llm_config.read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
        .map(|c| LLMClient::new(c.clone()).provider_name().to_string());

    *state.llm_config.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(config.clone());

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

    // Sync Meilisearch chat workspace with the new provider
    // LLMConfig is a type alias for ProviderConfig, so we can convert directly
    if let Ok(chat_provider) = ChatProviderConfig::try_from(&config) {
        // Clone needed values before acquiring lock to minimize lock duration
        let search_host = state.search_client.host().to_string();
        let master_key = state.sidecar_manager.config().master_key.clone();

        // Use write lock since set_chat_client and configure_chat_workspace modify internal state
        let manager = state.llm_manager.clone();
        let manager_guard = manager.write().await;

        // Ensure chat client is configured
        manager_guard.set_chat_client(&search_host, Some(&master_key)).await;

        // Configure the dm-assistant workspace with the new provider
        if let Err(e) = manager_guard.configure_chat_workspace("dm-assistant", chat_provider, None).await {
            log::warn!("Failed to sync Meilisearch chat workspace: {}", e);
            // Don't fail the whole operation, just log the warning
        } else {
            log::info!("Synced Meilisearch chat workspace with {} provider", provider_name);
        }
    }

    Ok(format!("Configured {} provider successfully", provider_name))
}

/// Get current LLM configuration
#[tauri::command]
pub fn get_llm_config(state: State<'_, AppState>) -> Result<Option<LLMSettings>, String> {
    let config = state.llm_config.read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    Ok(config.as_ref().map(LLMSettings::from))
}

/// Check LLM provider health
#[tauri::command]
pub async fn check_llm_health(state: State<'_, AppState>) -> Result<HealthStatus, String> {
    println!("DEBUG: check_llm_health called");
    let config_opt = state.llm_config.read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();

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
