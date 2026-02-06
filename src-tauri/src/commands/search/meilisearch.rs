//! Meilisearch Configuration Commands
//!
//! Commands for Meilisearch health checks, reindexing, and chat configuration.
//!
//! TODO: Phase 3 Migration - These commands need to be updated to use EmbeddedSearch/MeilisearchLib.
//! The embedded search is always healthy since there's no network layer.

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
#[allow(unused_variables)]
pub async fn check_meilisearch_health(
    state: State<'_, AppState>,
) -> Result<MeilisearchStatus, String> {
    // With embedded Meilisearch, it's always healthy since there's no network layer
    let meili = state.embedded_search.inner();
    let health = meili.health();
    let healthy = health.status == "available";

    // TODO: Get actual stats from MeilisearchLib when implemented
    // For now, return None for document_counts
    Ok(MeilisearchStatus {
        healthy,
        host: "embedded".to_string(),
        document_counts: None,
    })
}

/// Reindex all documents (clear and re-ingest)
///
/// TODO: Phase 3 Migration - Update to use MeilisearchLib delete_all_documents
#[tauri::command]
#[allow(unused_variables)]
pub async fn reindex_library(
    index_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // TODO: Migrate to embedded MeilisearchLib
    // The old SearchClient had clear_index() method.
    // The new MeilisearchLib uses:
    //   meili.delete_all_documents(uid) to clear an index
    //
    // Access via: state.embedded_search.inner()
    let _meili = state.embedded_search.inner();

    if let Some(name) = &index_name {
        log::warn!(
            "reindex_library() called but not yet migrated to embedded MeilisearchLib. Index: {}",
            name
        );
    } else {
        log::warn!("reindex_library() called but not yet migrated to embedded MeilisearchLib. All indexes.");
    }

    // Return error for now - full migration in Phase 3 Task 5
    Err("Reindexing not yet available - migration in progress".to_string())
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
///
/// TODO: Phase 4 Migration - Update to use MeilisearchLib chat configuration
/// With embedded Meilisearch, we can use MeilisearchLib's chat config API directly
#[tauri::command]
#[allow(unused_variables)]
pub async fn configure_chat_workspace(
    workspace_id: String,
    provider: ChatProviderConfig,
    custom_prompts: Option<ChatPrompts>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // TODO: Migrate to embedded MeilisearchLib
    // The old implementation used SearchClient and SidecarManager.
    // The new MeilisearchLib has chat configuration via:
    //   meili.set_chat_config(ChatConfig) for LLM provider setup
    //
    // Access via: state.embedded_search.inner()
    let _meili = state.embedded_search.inner();

    log::warn!(
        "configure_chat_workspace() called but not yet migrated to embedded MeilisearchLib. Workspace: {}",
        workspace_id
    );

    // Return error for now - full migration in Phase 4
    Err(format!(
        "Chat workspace configuration not yet available - migration in progress. Workspace: {}",
        workspace_id
    ))
}

/// Get the current settings for a Meilisearch chat workspace.
///
/// TODO: Phase 4 Migration - Update to use MeilisearchLib chat configuration
#[tauri::command]
#[allow(unused_variables)]
pub async fn get_chat_workspace_settings(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<Option<ChatWorkspaceSettings>, String> {
    // TODO: Migrate to embedded MeilisearchLib
    // The old implementation used MeilisearchChatClient with HTTP.
    // The new MeilisearchLib has:
    //   meili.get_chat_config() for retrieving chat configuration
    //
    // Access via: state.embedded_search.inner()
    let _meili = state.embedded_search.inner();

    log::warn!(
        "get_chat_workspace_settings() called but not yet migrated to embedded MeilisearchLib. Workspace: {}",
        workspace_id
    );

    // Return None for now - full migration in Phase 4
    Ok(None)
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
///
/// TODO: Phase 4 Migration - Update to use MeilisearchLib chat configuration
#[tauri::command]
#[allow(unused_variables)]
pub async fn configure_meilisearch_chat(
    provider: String,
    api_key: Option<String>,
    model: Option<String>,
    custom_system_prompt: Option<String>,
    host: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // TODO: Migrate to embedded MeilisearchLib
    // The old implementation used SearchClient and SidecarManager.
    // The new MeilisearchLib has chat configuration via:
    //   meili.set_chat_config(ChatConfig) for LLM provider setup
    //
    // The provider config building logic is still valid, but needs to be
    // translated to MeilisearchLib's ChatConfig format.
    //
    // Access via: state.embedded_search.inner()
    let _meili = state.embedded_search.inner();

    log::warn!(
        "configure_meilisearch_chat() called but not yet migrated to embedded MeilisearchLib. Provider: {}",
        provider
    );

    // Return error for now - full migration in Phase 4
    Err(format!(
        "Meilisearch chat configuration not yet available - migration in progress. Provider: {}",
        provider
    ))
}
