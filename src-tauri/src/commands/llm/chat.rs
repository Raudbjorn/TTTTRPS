//! Non-streaming Chat Commands
//!
//! Commands for synchronous chat with LLM providers.

use tauri::State;

use crate::commands::state::AppState;
use crate::core::llm::{ChatMessage, MessageRole};

use super::types::{ChatRequestPayload, ChatResponsePayload};

// ============================================================================
// Commands
// ============================================================================

/// Non-streaming chat request
#[tauri::command]
pub async fn chat(
    payload: ChatRequestPayload,
    state: State<'_, AppState>,
) -> Result<ChatResponsePayload, String> {
    // Get configuration
    let config = state.llm_config.read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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

    // TODO: set_chat_client was for HTTP-based Meilisearch.
    // With embedded MeilisearchLib, chat completion goes through embedded_search.inner() directly.
    // This will be replaced with RAG commands in Phase 4.
    //
    // Previously this ensured chat client was configured:
    // {
    //     let manager_guard = manager.write().await;
    //     manager_guard.set_chat_client(state.search_client.host(), Some(&state.sidecar_manager.config().master_key)).await;
    // }

    // Prepare messages - start with system prompt as first message
    let mut messages = vec![
        ChatMessage {
            role: MessageRole::System,
            content: system_prompt,
            images: None,
            name: None,
            tool_calls: None,
            tool_call_id: None,
        },
    ];
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
    let model = config.model_name();

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
