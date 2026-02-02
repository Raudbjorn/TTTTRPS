//! Streaming Chat Commands
//!
//! Commands for streaming chat with LLM providers.
//! Uses tokio::spawn for async execution and Tauri events for chunk emission.
//!
//! CRITICAL: This module preserves the exact streaming pattern:
//! - tokio::spawn for async execution
//! - app_handle.emit() for Tauri events
//! - Stream ID tracking for cancellation

use tauri::State;
use tauri::Emitter;

use crate::commands::state::AppState;
use crate::core::llm::{ChatMessage, ChatChunk};

// ============================================================================
// Commands
// ============================================================================

/// Stream chat response - emits 'chat-chunk' events as chunks arrive
///
/// This command uses a fire-and-forget pattern:
/// 1. Validates configuration
/// 2. Creates stream ID
/// 3. Spawns async task with tokio::spawn
/// 4. Returns stream ID immediately
/// 5. Task emits "chat-chunk" events as chunks arrive
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
    log::info!("[stream_chat] Starting with {} messages, system_prompt: {}",
        messages.len(),
        system_prompt.as_ref().map(|s| {
            let chars: Vec<char> = s.chars().take(51).collect();
            if chars.len() > 50 {
                format!("{}...", chars.into_iter().take(50).collect::<String>())
            } else {
                chars.into_iter().collect::<String>()
            }
        }).unwrap_or_else(|| "None".to_string())
    );

    // Build final message list, prepending system prompt if provided
    let final_messages = if let Some(prompt) = system_prompt {
        let mut msgs = vec![ChatMessage::system(prompt)];
        msgs.extend(messages);
        msgs
    } else {
        messages
    };

    let config = state.llm_config.read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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
    let mut rx = manager_guard.chat_stream(final_messages, &model, temperature, max_tokens).await
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
