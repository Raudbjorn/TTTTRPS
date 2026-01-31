//! Chat Session Commands
//!
//! Commands for managing global chat sessions and messages.
//! Chat sessions provide persistent LLM chat history.

use tauri::State;

use crate::commands::AppState;
use crate::database::{ChatOps, GlobalChatSessionRecord, ChatMessageRecord};

// ============================================================================
// Global Chat Session Commands (Persistent LLM Chat History)
// ============================================================================

/// Get or create the active global chat session.
///
/// If no active chat session exists, creates a new one.
///
/// # Returns
/// The active chat session record.
#[tauri::command]
pub async fn get_or_create_chat_session(
    state: State<'_, AppState>,
) -> Result<GlobalChatSessionRecord, String> {
    state.database.get_or_create_active_chat_session()
        .await
        .map_err(|e| e.to_string())
}

/// Get the current active chat session.
///
/// # Returns
/// The active chat session if one exists, None otherwise.
#[tauri::command]
pub async fn get_active_chat_session(
    state: State<'_, AppState>,
) -> Result<Option<GlobalChatSessionRecord>, String> {
    state.database.get_active_chat_session()
        .await
        .map_err(|e| e.to_string())
}

/// Get messages for a chat session.
///
/// # Arguments
/// * `session_id` - The chat session ID
/// * `limit` - Maximum number of messages to return (default: 100)
///
/// # Returns
/// List of chat messages for the session.
#[tauri::command]
pub async fn get_chat_messages(
    session_id: String,
    limit: Option<i32>,
    state: State<'_, AppState>,
) -> Result<Vec<ChatMessageRecord>, String> {
    state.database.get_chat_messages(&session_id, limit.unwrap_or(100))
        .await
        .map_err(|e| e.to_string())
}

/// Add a message to the chat session.
///
/// # Arguments
/// * `session_id` - The chat session ID
/// * `role` - Message role (e.g., "user", "assistant")
/// * `content` - Message content
/// * `tokens` - Optional tuple of (input_tokens, output_tokens)
///
/// # Returns
/// The created chat message record.
#[tauri::command]
pub async fn add_chat_message(
    session_id: String,
    role: String,
    content: String,
    tokens: Option<(i32, i32)>,
    state: State<'_, AppState>,
) -> Result<ChatMessageRecord, String> {
    let mut message = ChatMessageRecord::new(session_id, role, content);
    if let Some((input, output)) = tokens {
        message = message.with_tokens(input, output);
    }
    state.database.add_chat_message(&message)
        .await
        .map_err(|e| e.to_string())?;
    Ok(message)
}

/// Update a chat message (e.g., after streaming completes).
///
/// Fetches existing record and merges fields to preserve existing tokens/metadata.
///
/// # Arguments
/// * `message_id` - The message ID to update
/// * `content` - New message content
/// * `tokens` - Optional tuple of (input_tokens, output_tokens)
/// * `is_streaming` - Whether the message is still streaming
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

/// Link the current chat session to a game session.
///
/// # Arguments
/// * `chat_session_id` - The chat session ID to link
/// * `game_session_id` - The game session ID to link to
/// * `campaign_id` - Optional campaign ID
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

/// Archive the current chat session and create a new one.
///
/// Used when ending a game session.
///
/// Note: Archives first due to unique index constraint (only one active session allowed).
/// If new session creation fails after archiving, call get_or_create_chat_session
/// which handles the race-condition-safe creation.
///
/// # Arguments
/// * `chat_session_id` - The chat session ID to archive
///
/// # Returns
/// The newly created chat session.
#[tauri::command]
pub async fn end_chat_session_and_spawn_new(
    chat_session_id: String,
    state: State<'_, AppState>,
) -> Result<GlobalChatSessionRecord, String> {
    // Archive current session first (removes the 'active' constraint)
    state.database.archive_chat_session(&chat_session_id)
        .await
        .map_err(|e| e.to_string())?;

    // Now create new session (only one active session allowed by unique index)
    let new_session = GlobalChatSessionRecord::new();
    state.database.create_chat_session(&new_session)
        .await
        .map_err(|e| e.to_string())?;

    Ok(new_session)
}

/// Clear all messages in a chat session.
///
/// # Arguments
/// * `session_id` - The chat session ID
///
/// # Returns
/// Number of messages deleted.
#[tauri::command]
pub async fn clear_chat_messages(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    state.database.clear_chat_messages(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// List recent chat sessions (all statuses, ordered by most recent).
///
/// # Arguments
/// * `limit` - Maximum number of sessions to return (default: 50)
///
/// # Returns
/// List of chat session records.
#[tauri::command]
pub async fn list_chat_sessions(
    limit: Option<i32>,
    state: State<'_, AppState>,
) -> Result<Vec<GlobalChatSessionRecord>, String> {
    state.database.list_chat_sessions(limit.unwrap_or(50))
        .await
        .map_err(|e| e.to_string())
}

/// Get chat sessions linked to a specific game session.
///
/// # Arguments
/// * `game_session_id` - The game session ID
///
/// # Returns
/// List of chat sessions linked to the game session.
#[tauri::command]
pub async fn get_chat_sessions_for_game(
    game_session_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<GlobalChatSessionRecord>, String> {
    state.database.get_chat_sessions_by_game_session(&game_session_id)
        .await
        .map_err(|e| e.to_string())
}
