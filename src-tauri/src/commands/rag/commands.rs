//! RAG Tauri Commands
//!
//! Tauri command handlers for RAG (Retrieval-Augmented Generation) operations.
//! These commands expose the meilisearch_lib chat API to the frontend.

use futures::StreamExt;
use tauri::{Emitter, State};

use crate::commands::state::AppState;

use super::types::{
    RagChunkPayload, RagConfigPayload, RagMessagePayload, RagResponsePayload, RagSourcePayload,
};

// ============================================================================
// Configuration Commands
// ============================================================================

/// Configure the RAG pipeline with LLM provider settings.
///
/// This sets up the chat configuration for the embedded Meilisearch instance,
/// enabling RAG queries. The configuration includes:
/// - LLM provider (OpenAI, Anthropic, Azure, Mistral, vLLM)
/// - API credentials
/// - Model selection
/// - Prompt templates
/// - Per-index search parameters
///
/// # Arguments
///
/// * `config` - Complete RAG configuration payload
/// * `state` - Application state containing embedded search
///
/// # Errors
///
/// Returns an error if the configuration cannot be applied.
///
/// # Example (Frontend)
///
/// ```typescript
/// await invoke('configure_rag', {
///   config: {
///     source: 'open_ai',
///     apiKey: 'sk-...',
///     model: 'gpt-4',
///     prompts: {
///       system: 'You are a helpful TTRPG assistant.'
///     },
///     indexConfigs: {
///       ttrpg_rules: {
///         description: 'TTRPG rulebook content',
///         template: '{{ content }}',
///         searchParams: { limit: 10, semanticRatio: 0.7 }
///       }
///     }
///   }
/// });
/// ```
#[tauri::command]
pub async fn configure_rag(
    config: RagConfigPayload,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!(
        "[configure_rag] Configuring RAG with provider: {:?}, model: {}",
        config.source,
        config.model
    );

    // Convert to meilisearch_lib::ChatConfig
    let chat_config: meilisearch_lib::ChatConfig = config.into();

    // Apply configuration to embedded search
    let meili = state.embedded_search.inner();
    meili.set_chat_config(Some(chat_config));

    log::info!("[configure_rag] RAG configuration applied successfully");
    Ok(())
}

/// Get the current RAG configuration.
///
/// Returns the current chat configuration with the API key masked for security.
/// Returns `None` if RAG has not been configured.
///
/// # Arguments
///
/// * `state` - Application state containing embedded search
///
/// # Returns
///
/// The current RAG configuration with masked API key, or `None` if not configured.
///
/// # Example (Frontend)
///
/// ```typescript
/// const config = await invoke('get_rag_config');
/// if (config) {
///   console.log(`Provider: ${config.source}, Model: ${config.model}`);
///   // API key is masked: sk-1************7890
/// }
/// ```
#[tauri::command]
pub async fn get_rag_config(
    state: State<'_, AppState>,
) -> Result<Option<RagConfigPayload>, String> {
    log::debug!("[get_rag_config] Retrieving current RAG configuration");

    let meili = state.embedded_search.inner();
    let config = meili.get_chat_config();

    Ok(config.map(|c| RagConfigPayload::from(c).with_masked_api_key()))
}

/// Clear the RAG configuration.
///
/// Removes the current chat configuration, disabling RAG queries until
/// reconfigured.
///
/// # Arguments
///
/// * `state` - Application state containing embedded search
#[tauri::command]
pub async fn clear_rag_config(state: State<'_, AppState>) -> Result<(), String> {
    log::info!("[clear_rag_config] Clearing RAG configuration");

    let meili = state.embedded_search.inner();
    meili.set_chat_config(None);

    Ok(())
}

// ============================================================================
// Query Commands
// ============================================================================

/// Execute a non-streaming RAG query.
///
/// Performs a RAG (Retrieval-Augmented Generation) query that:
/// 1. Extracts the query from the last user message
/// 2. Searches the specified index for relevant context
/// 3. Sends context + messages to the configured LLM
/// 4. Returns the response with source citations
///
/// # Arguments
///
/// * `index_uid` - Index to search for context
/// * `messages` - Conversation messages (user/assistant history)
/// * `state` - Application state containing embedded search
///
/// # Returns
///
/// RAG response containing:
/// - `content`: Generated response text
/// - `sources`: Document IDs used as context
/// - `usage`: Token usage statistics (if provided by LLM)
///
/// # Errors
///
/// Returns an error if:
/// - RAG is not configured (`configure_rag` not called)
/// - The index doesn't exist
/// - The LLM provider returns an error
///
/// # Example (Frontend)
///
/// ```typescript
/// const response = await invoke('rag_query', {
///   indexUid: 'ttrpg_rules',
///   messages: [
///     { role: 'user', content: 'How does combat work in D&D 5e?' }
///   ]
/// });
///
/// console.log(response.content);
/// console.log(`Sources: ${response.sources.map(s => s.id).join(', ')}`);
/// ```
#[tauri::command]
pub async fn rag_query(
    index_uid: String,
    messages: Vec<RagMessagePayload>,
    state: State<'_, AppState>,
) -> Result<RagResponsePayload, String> {
    log::info!(
        "[rag_query] Starting RAG query on index '{}' with {} messages",
        index_uid,
        messages.len()
    );

    // Verify chat is configured
    let meili = state.embedded_search.clone_inner();
    if meili.get_chat_config().is_none() {
        return Err("RAG not configured. Call configure_rag first.".to_string());
    }

    // Convert messages to meilisearch_lib format
    let meili_messages: Vec<meilisearch_lib::Message> =
        messages.into_iter().map(Into::into).collect();

    // Build chat request
    let request = meilisearch_lib::ChatRequest {
        messages: meili_messages,
        index_uid: index_uid.clone(),
        stream: false,
    };

    // Execute chat completion
    let response = meili
        .chat_completion(request)
        .await
        .map_err(|e| format!("RAG query failed: {}", e))?;

    log::info!(
        "[rag_query] Completed with {} sources, {} tokens",
        response.sources.len(),
        response.usage.as_ref().map(|u| u.total_tokens).unwrap_or(0)
    );

    Ok(response.into())
}

/// Execute a streaming RAG query.
///
/// Same as `rag_query` but streams the response in real-time via Tauri events.
/// The frontend receives 'rag-chunk' events as the LLM generates the response.
///
/// # Arguments
///
/// * `app_handle` - Tauri app handle for event emission
/// * `index_uid` - Index to search for context
/// * `messages` - Conversation messages
/// * `stream_id` - Optional stream identifier (generated if not provided)
/// * `state` - Application state containing embedded search
///
/// # Returns
///
/// The stream ID immediately. Use this to match incoming 'rag-chunk' events.
///
/// # Events Emitted
///
/// - `rag-chunk`: Emitted for each chunk with payload:
///   - `streamId`: Stream identifier
///   - `delta`: Incremental content
///   - `done`: Whether this is the final chunk
///   - `sources`: Source citations (first chunk only)
///   - `index`: Chunk index for ordering
///
/// # Errors
///
/// Returns an error if:
/// - RAG is not configured
/// - The stream fails to start
///
/// Note: Errors during streaming are emitted as error chunks rather than
/// returned from this function.
///
/// # Example (Frontend)
///
/// ```typescript
/// import { listen } from '@tauri-apps/api/event';
///
/// // Start listening before the query
/// const unlisten = await listen('rag-chunk', (event) => {
///   const chunk = event.payload;
///   if (chunk.streamId === streamId) {
///     process.stdout.write(chunk.delta);
///     if (chunk.done) {
///       console.log('\nStream complete');
///       unlisten();
///     }
///   }
/// });
///
/// // Start the streaming query
/// const streamId = await invoke('rag_query_stream', {
///   indexUid: 'ttrpg_rules',
///   messages: [{ role: 'user', content: 'Explain initiative order' }]
/// });
/// ```
#[tauri::command]
pub async fn rag_query_stream(
    app_handle: tauri::AppHandle,
    index_uid: String,
    messages: Vec<RagMessagePayload>,
    stream_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Generate or use provided stream ID
    let stream_id = stream_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let stream_id_clone = stream_id.clone();

    log::info!(
        "[rag_query_stream] Starting streaming RAG query on index '{}' with stream_id: {}",
        index_uid,
        stream_id
    );

    // Verify chat is configured
    let meili = state.embedded_search.clone_inner();
    if meili.get_chat_config().is_none() {
        return Err("RAG not configured. Call configure_rag first.".to_string());
    }

    // Convert messages to meilisearch_lib format
    let meili_messages: Vec<meilisearch_lib::Message> =
        messages.into_iter().map(Into::into).collect();

    // Build chat request with streaming enabled
    let request = meilisearch_lib::ChatRequest {
        messages: meili_messages,
        index_uid: index_uid.clone(),
        stream: true,
    };

    // Start the stream
    let mut stream = meili
        .chat_completion_stream(request)
        .await
        .map_err(|e| format!("Failed to start RAG stream: {}", e))?;

    // Spawn task to consume stream and emit events
    tokio::spawn(async move {
        log::debug!("[rag_query_stream:{}] Stream task started", stream_id_clone);

        let mut chunk_index: u32 = 0;
        let mut is_first = true;

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    chunk_index += 1;

                    // Convert sources for first chunk
                    let sources = if is_first && chunk.sources.is_some() {
                        is_first = false;
                        chunk
                            .sources
                            .map(|s| s.into_iter().map(|id| RagSourcePayload { id }).collect())
                    } else {
                        None
                    };

                    let payload = RagChunkPayload {
                        stream_id: stream_id_clone.clone(),
                        delta: chunk.delta,
                        done: chunk.done,
                        sources,
                        index: chunk_index,
                    };

                    // Emit the chunk event
                    if let Err(e) = app_handle.emit("rag-chunk", &payload) {
                        log::error!(
                            "[rag_query_stream:{}] Failed to emit chunk: {}",
                            stream_id_clone,
                            e
                        );
                        break;
                    }

                    if chunk.done {
                        log::info!(
                            "[rag_query_stream:{}] Stream completed with {} chunks",
                            stream_id_clone,
                            chunk_index
                        );
                        break;
                    }
                }
                Err(e) => {
                    log::error!("[rag_query_stream:{}] Stream error: {}", stream_id_clone, e);

                    // Emit error as final chunk
                    let error_payload = RagChunkPayload {
                        stream_id: stream_id_clone.clone(),
                        delta: format!("Error: {}", e),
                        done: true,
                        sources: None,
                        index: chunk_index + 1,
                    };
                    let _ = app_handle.emit("rag-chunk", &error_payload);
                    break;
                }
            }
        }

        log::debug!("[rag_query_stream:{}] Stream task exiting", stream_id_clone);
    });

    Ok(stream_id)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Integration tests require a running Meilisearch instance with
    // chat configuration. Unit tests focus on type conversions and validation.

    #[test]
    fn test_rag_message_conversion() {
        let payload = RagMessagePayload {
            role: super::super::types::RagMessageRole::User,
            content: "Hello".to_string(),
            tool_call_id: None,
        };

        let meili_msg: meilisearch_lib::Message = payload.into();
        assert_eq!(meili_msg.content, "Hello");
        assert_eq!(meili_msg.role, meilisearch_lib::Role::User);
    }

    #[test]
    fn test_rag_config_conversion() {
        let config = RagConfigPayload {
            source: super::super::types::RagProviderSource::Anthropic,
            api_key: "sk-test".to_string(),
            base_url: None,
            model: "claude-3-sonnet-20240229".to_string(),
            org_id: None,
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts: super::super::types::RagPromptsPayload::default(),
            index_configs: std::collections::HashMap::new(),
        };

        let meili_config: meilisearch_lib::ChatConfig = config.into();
        assert_eq!(meili_config.source, meilisearch_lib::ChatSource::Anthropic);
        assert_eq!(meili_config.model, "claude-3-sonnet-20240229");
    }
}
