//! Meilisearch Configuration Commands
//!
//! Commands for Meilisearch health checks, reindexing, and chat configuration.

use tauri::State;

use crate::commands::AppState;
use crate::core::meilisearch_chat::{
    ChatProviderConfig, ChatProviderInfo, ChatPrompts, ChatWorkspaceSettings,
    list_chat_providers as get_chat_providers,
};
use super::types::MeilisearchStatus;

// ============================================================================
// Meilisearch Health and Indexing Commands
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
        for idx in crate::core::search::SearchClient::all_indexes() {
            let _ = state.search_client.clear_index(idx).await;
        }
        Ok("Cleared all indexes".to_string())
    }
}

// ============================================================================
// Meilisearch Chat Provider Commands
// ============================================================================

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
        "gemini" => ChatProviderConfig::Google {
            api_key: api_key.ok_or("Gemini requires an API key")?,
            model,
        },
        "ollama" => ChatProviderConfig::Ollama {
            host: host.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model: model.unwrap_or_else(|| "llama3.2".to_string()),
        },
        "openrouter" => ChatProviderConfig::OpenRouter {
            api_key: api_key.ok_or("OpenRouter requires an API key")?,
            model: model.unwrap_or_else(|| "openai/gpt-4o".to_string()),
        },
        "groq" => ChatProviderConfig::Groq {
            api_key: api_key.ok_or("Groq requires an API key")?,
            model: model.unwrap_or_else(|| "llama-3.1-70b-versatile".to_string()),
        },
        "together" => ChatProviderConfig::Together {
            api_key: api_key.ok_or("Together AI requires an API key")?,
            model: model.unwrap_or_else(|| "meta-llama/Llama-3-70b-chat-hf".to_string()),
        },
        "cohere" => ChatProviderConfig::Cohere {
            api_key: api_key.ok_or("Cohere requires an API key")?,
            model: model.unwrap_or_else(|| "command-r-plus".to_string()),
        },
        "deepseek" => ChatProviderConfig::DeepSeek {
            api_key: api_key.ok_or("DeepSeek requires an API key")?,
            model: model.unwrap_or_else(|| "deepseek-chat".to_string()),
        },
        other => return Err(format!("Unknown provider: {}. Supported: openai, claude, mistral, gemini, ollama, openrouter, groq, together, cohere, deepseek", other)),
    };

    // Build custom prompts if system prompt provided
    let custom_prompts = custom_system_prompt.map(|prompt| ChatPrompts {
        system: Some(prompt),
        ..Default::default()
    });

    // Get the unified LLM manager from state
    let manager = state.llm_manager.clone();

    // Configure with Meilisearch host from search client
    {
        let manager_guard = manager.write().await;
        manager_guard.set_chat_client(state.search_client.host(), Some(&state.sidecar_manager.config().master_key)).await;
    }

    // Configure the workspace
    let manager_guard = manager.read().await;
    manager_guard
        .configure_chat_workspace("dm-assistant", provider_config, custom_prompts)
        .await?;

    log::info!("Meilisearch chat configured with provider: {}", provider);
    Ok(())
}
