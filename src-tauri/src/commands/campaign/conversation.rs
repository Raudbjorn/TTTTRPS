//! Conversation Commands Module
//!
//! Phase 5 of the Campaign Generation Overhaul.
//!
//! Tauri commands for AI-assisted conversation management.
//! Provides the frontend interface for:
//! - Creating and managing conversation threads
//! - Sending and receiving messages
//! - Handling AI-generated suggestions
//! - Branching conversations

use std::sync::Arc;
use once_cell::sync::Lazy;
use regex::Regex;
use tauri::State;
use tracing::{debug, error, info};

use crate::commands::AppState;
use crate::core::campaign::conversation::{
    ConversationError, ConversationManager, ConversationMessage,
    ConversationPurpose, ConversationRole, ConversationThread, GeneratedResponse,
    MessagePagination, PaginatedMessages, SuggestionAcceptResult, SuggestionRejectResult,
    ThreadListOptions,
};

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a ConversationManager from AppState.
fn get_conversation_manager(state: &State<'_, AppState>) -> ConversationManager {
    let pool = Arc::new(state.database.pool().clone());
    ConversationManager::new(pool)
}

/// Generate an AI response for a conversation thread.
///
/// Note: Uses the LLMRouter from AppState to generate responses.
async fn get_conversation_ai_response(
    state: &State<'_, AppState>,
    manager: &ConversationManager,
    thread_id: &str,
    content: &str,
) -> Result<GeneratedResponse, ConversationError> {
    use crate::core::llm::router::{ChatMessage, ChatRequest};

    // Get the thread first
    let thread = manager.get_thread_required(thread_id).await?;

    // Get messages for context
    let pagination = MessagePagination::with_limit(50);
    let messages = manager.get_messages(thread_id, pagination).await?;

    // Build context messages
    let system_prompt = get_system_prompt_for_purpose(thread.purpose);

    let mut llm_messages = Vec::new();

    // Add conversation history
    for msg in &messages.messages {
        let chat_msg = match msg.role {
            ConversationRole::User => ChatMessage::user(&msg.content),
            ConversationRole::Assistant => ChatMessage::assistant(&msg.content),
            ConversationRole::System => ChatMessage::system(&msg.content),
        };
        llm_messages.push(chat_msg);
    }

    // Add the new user message
    llm_messages.push(ChatMessage::user(content));

    // Build the request with system prompt
    let request = ChatRequest::new(llm_messages).with_system(system_prompt);

    // Generate response using the router
    let response = {
        let router = state.llm_router.read().await;
        router
            .chat(request)
            .await
            .map_err(|e| ConversationError::LlmError(e.to_string()))?
    };

    // Parse the response for suggestions and citations
    let (parsed_content, suggestions, citations) = parse_response(&response.content);

    // Get token usage from response (use i64 to avoid truncation on large counts)
    let (tokens_in, tokens_out) = response
        .usage
        .map(|u| (u.input_tokens as i64, u.output_tokens as i64))
        .unwrap_or((0, 0));

    Ok(GeneratedResponse {
        content: parsed_content,
        suggestions,
        citations,
        model: response.model,
        tokens_in,
        tokens_out,
    })
}

/// Get the appropriate system prompt for the conversation purpose.
fn get_system_prompt_for_purpose(purpose: ConversationPurpose) -> &'static str {
    match purpose {
        ConversationPurpose::CampaignCreation => CAMPAIGN_CREATION_SYSTEM_PROMPT,
        ConversationPurpose::SessionPlanning => SESSION_PLANNING_SYSTEM_PROMPT,
        ConversationPurpose::NpcGeneration => NPC_GENERATION_SYSTEM_PROMPT,
        ConversationPurpose::WorldBuilding => WORLD_BUILDING_SYSTEM_PROMPT,
        ConversationPurpose::CharacterBackground => CHARACTER_BACKGROUND_SYSTEM_PROMPT,
        ConversationPurpose::General => CAMPAIGN_CREATION_SYSTEM_PROMPT,
    }
}

// Static regex patterns for parsing
static SUGGESTION_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)```suggestion\s*\n(.*?)\n```").unwrap());
static CITATION_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)```citation\s*\n(.*?)\n```").unwrap());
static QUESTIONS_JSON_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?s)\[.*\]").unwrap());

/// Parse the LLM response for suggestions and citations.
fn parse_response(
    response: &str,
) -> (String, Vec<crate::database::Suggestion>, Vec<crate::core::campaign::conversation::Citation>) {
    use crate::database::{Suggestion, SuggestionStatus};
    use crate::core::campaign::conversation::Citation;

    let mut suggestions = Vec::new();
    let mut citations = Vec::new();
    let content = response.to_string();

    // Parse suggestion blocks
    for cap in SUGGESTION_REGEX.captures_iter(response) {
        if let Some(json_str) = cap.get(1) {
            if let Ok(raw) = serde_json::from_str::<serde_json::Value>(json_str.as_str()) {
                if let (Some(field), Some(value), Some(rationale)) = (
                    raw.get("field").and_then(|v| v.as_str()),
                    raw.get("value"),
                    raw.get("rationale").and_then(|v| v.as_str()),
                ) {
                    suggestions.push(Suggestion {
                        id: uuid::Uuid::new_v4().to_string(),
                        field: field.to_string(),
                        value: value.clone(),
                        rationale: rationale.to_string(),
                        status: SuggestionStatus::Pending,
                    });
                }
            }
        }
    }

    // Parse citation blocks
    for cap in CITATION_REGEX.captures_iter(response) {
        if let Some(json_str) = cap.get(1) {
            if let Ok(raw) = serde_json::from_str::<serde_json::Value>(json_str.as_str()) {
                if let Some(source_name) = raw.get("source_name").and_then(|v| v.as_str()) {
                    let mut citation =
                        Citation::new(uuid::Uuid::new_v4().to_string(), source_name.to_string(), 0.8);
                    if let Some(loc) = raw.get("location").and_then(|v| v.as_str()) {
                        citation = citation.with_location(loc.to_string());
                    }
                    if let Some(exc) = raw.get("excerpt").and_then(|v| v.as_str()) {
                        citation = citation.with_excerpt(exc.to_string());
                    }
                    citations.push(citation);
                }
            }
        }
    }

    // Strip markers and clean up content
    let content = SUGGESTION_REGEX.replace_all(&content, "");
    let content = CITATION_REGEX.replace_all(&content, "");
    let content = content.trim().to_string();

    (content, suggestions, citations)
}

// System prompts for different conversation purposes
const CAMPAIGN_CREATION_SYSTEM_PROMPT: &str = r#"You are a helpful TTRPG assistant specializing in campaign creation.
You help Game Masters develop rich, engaging campaigns with coherent themes and interesting plot hooks.

When making suggestions, format them as JSON blocks that can be parsed:
```suggestion
{
  "field": "name",
  "value": "The Shattered Crown",
  "rationale": "This name evokes mystery and conflict, fitting your dark fantasy theme."
}
```

When citing source material, include references:
```citation
{
  "source_name": "Player's Handbook",
  "location": "p. 123",
  "excerpt": "The relevant text..."
}
```

Be concise but thorough. Ask clarifying questions when the user's intent is unclear.
Always maintain the established tone and themes of the campaign."#;

const SESSION_PLANNING_SYSTEM_PROMPT: &str = r#"You are a helpful TTRPG assistant specializing in session planning.
You help Game Masters prepare engaging sessions with balanced pacing and memorable encounters.

Focus on:
- Scene-by-scene breakdowns
- NPC motivations and dialogue hooks
- Encounter balance suggestions
- Backup plans for player divergence

Use the same suggestion and citation formats as campaign creation when applicable."#;

const NPC_GENERATION_SYSTEM_PROMPT: &str = r#"You are a helpful TTRPG assistant specializing in NPC creation.
You help create memorable, three-dimensional characters with clear motivations and flaws.

Consider:
- Personality traits and mannerisms
- Backstory and secrets
- Relationships with other NPCs
- Quest hooks they could provide
- Voice and speech patterns

Format suggestions for NPC attributes using the standard suggestion blocks."#;

const CHARACTER_BACKGROUND_SYSTEM_PROMPT: &str = r#"You are a helpful TTRPG assistant specializing in character backgrounds and history.

Flesh out an NPC's past, motivations, and relationships based on the campaign context. Focus on narrative hooks that the GM can use."#;

const WORLD_BUILDING_SYSTEM_PROMPT: &str = r#"You are a helpful TTRPG assistant specializing in world building.
You help create rich, consistent settings with interesting history and cultures.

Focus on:
- Geographic features and locations
- Political structures and factions
- Cultural traditions and conflicts
- Historical events and their consequences
- Mysteries and secrets of the world

Ground your suggestions in established lore when possible, citing sources."#;

/// Convert ConversationError to String for Tauri IPC.
fn conv_err_to_string(err: ConversationError) -> String {
    error!(error = %err, "Conversation command error");
    err.to_string()
}

// ============================================================================
// Thread Management Commands
// ============================================================================

/// Create a new conversation thread.
///
/// # Arguments
/// * `purpose` - The purpose/category of the conversation
/// * `campaign_id` - Optional campaign to link the thread to
/// * `wizard_id` - Optional wizard to link the thread to
/// * `title` - Optional title for the thread
///
/// # Returns
/// The newly created conversation thread
#[tauri::command]
pub async fn create_conversation_thread(
    purpose: ConversationPurpose,
    campaign_id: Option<String>,
    wizard_id: Option<String>,
    title: Option<String>,
    state: State<'_, AppState>,
) -> Result<ConversationThread, String> {
    info!(
        purpose = %purpose,
        campaign_id = ?campaign_id,
        wizard_id = ?wizard_id,
        "Creating conversation thread"
    );

    let manager = get_conversation_manager(&state);

    // Create the appropriate thread type
    let mut thread = if let Some(campaign_id) = campaign_id {
        manager
            .create_thread_for_campaign(campaign_id, purpose)
            .await
            .map_err(conv_err_to_string)?
    } else if let Some(wizard_id) = wizard_id {
        manager
            .create_thread_for_wizard(wizard_id, purpose)
            .await
            .map_err(conv_err_to_string)?
    } else {
        manager
            .create_thread(purpose)
            .await
            .map_err(conv_err_to_string)?
    };

    // Set title if provided
    if let Some(title) = title {
        thread = manager
            .update_thread_title(&thread.id, title)
            .await
            .map_err(conv_err_to_string)?;
    }

    Ok(thread)
}

/// Get a conversation thread by ID.
///
/// # Arguments
/// * `thread_id` - The thread's unique identifier
///
/// # Returns
/// The thread if found
#[tauri::command]
pub async fn get_conversation_thread(
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<Option<ConversationThread>, String> {
    debug!(thread_id = %thread_id, "Getting conversation thread");

    let manager = get_conversation_manager(&state);
    manager
        .get_thread(&thread_id)
        .await
        .map_err(conv_err_to_string)
}

/// List conversation threads with optional filtering.
///
/// # Arguments
/// * `campaign_id` - Optional campaign filter
/// * `purpose` - Optional purpose filter
/// * `include_archived` - Whether to include archived threads
/// * `limit` - Maximum number of threads to return
///
/// # Returns
/// List of matching threads, ordered by most recent first
#[tauri::command]
pub async fn list_conversation_threads(
    campaign_id: Option<String>,
    purpose: Option<ConversationPurpose>,
    include_archived: Option<bool>,
    limit: Option<i32>,
    state: State<'_, AppState>,
) -> Result<Vec<ConversationThread>, String> {
    debug!(
        campaign_id = ?campaign_id,
        purpose = ?purpose,
        include_archived = ?include_archived,
        "Listing conversation threads"
    );

    let manager = get_conversation_manager(&state);

    let mut options = ThreadListOptions::new();
    if let Some(campaign_id) = campaign_id {
        options = options.for_campaign(campaign_id);
    }
    if let Some(purpose) = purpose {
        options = options.with_purpose(purpose);
    }
    if include_archived.unwrap_or(false) {
        options = options.include_archived();
    }
    if let Some(limit) = limit {
        options = options.limit(limit);
    }

    manager
        .list_threads(options)
        .await
        .map_err(conv_err_to_string)
}

/// Archive a conversation thread.
///
/// Archived threads cannot receive new messages but remain readable.
///
/// # Arguments
/// * `thread_id` - The thread's unique identifier
#[tauri::command]
pub async fn archive_conversation_thread(
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    info!(thread_id = %thread_id, "Archiving conversation thread");

    let manager = get_conversation_manager(&state);
    manager
        .archive_thread(&thread_id)
        .await
        .map_err(conv_err_to_string)
}

/// Update the title of a conversation thread.
///
/// # Arguments
/// * `thread_id` - The thread's unique identifier
/// * `title` - The new title
///
/// # Returns
/// The updated thread
#[tauri::command]
pub async fn update_conversation_thread_title(
    thread_id: String,
    title: String,
    state: State<'_, AppState>,
) -> Result<ConversationThread, String> {
    debug!(thread_id = %thread_id, title = %title, "Updating thread title");

    let manager = get_conversation_manager(&state);
    manager
        .update_thread_title(&thread_id, title)
        .await
        .map_err(conv_err_to_string)
}

// ============================================================================
// Message Commands
// ============================================================================

/// Send a message in a conversation thread and get an AI response.
///
/// This is the main interaction command:
/// 1. Adds the user message to the thread
/// 2. Generates an AI response
/// 3. Adds the AI response with suggestions/citations to the thread
/// 4. Returns both messages
///
/// # Arguments
/// * `thread_id` - The thread's unique identifier
/// * `content` - The user's message content
///
/// # Returns
/// A tuple of (user_message, assistant_message)
#[tauri::command]
pub async fn send_conversation_message(
    thread_id: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<SendMessageResult, String> {
    info!(
        thread_id = %thread_id,
        content_len = content.len(),
        "Sending conversation message"
    );

    let manager = get_conversation_manager(&state);

    // Add the user message
    let user_message = manager
        .add_message(&thread_id, ConversationRole::User, content.clone())
        .await
        .map_err(conv_err_to_string)?;

    // Generate AI response using the router directly from state
    // If generation fails, clean up the orphaned user message
    let response = match get_conversation_ai_response(&state, &manager, &thread_id, &content).await {
        Ok(r) => r,
        Err(e) => {
            // Cleanup: delete the orphaned user message
            if let Err(delete_err) = manager.delete_message(&user_message.id).await {
                error!(
                    message_id = %user_message.id,
                    error = %delete_err,
                    "Failed to cleanup orphaned user message after AI generation failure"
                );
            }
            return Err(conv_err_to_string(e));
        }
    };

    // Add the assistant message with metadata
    let assistant_message = manager
        .add_assistant_message_with_metadata(
            &thread_id,
            response.content,
            response.suggestions,
            response.citations,
        )
        .await
        .map_err(conv_err_to_string)?;

    Ok(SendMessageResult {
        user_message,
        assistant_message,
        model: response.model,
        tokens_in: response.tokens_in,
        tokens_out: response.tokens_out,
    })
}

/// Get messages from a conversation thread with pagination.
///
/// # Arguments
/// * `thread_id` - The thread's unique identifier
/// * `limit` - Maximum number of messages to return
/// * `before` - Optional cursor for pagination (message ID to fetch before)
///
/// # Returns
/// Paginated list of messages
#[tauri::command]
pub async fn get_conversation_messages(
    thread_id: String,
    limit: Option<i32>,
    before: Option<String>,
    state: State<'_, AppState>,
) -> Result<PaginatedMessages, String> {
    debug!(
        thread_id = %thread_id,
        limit = ?limit,
        before = ?before,
        "Getting conversation messages"
    );

    let manager = get_conversation_manager(&state);

    let mut pagination = MessagePagination::with_limit(limit.unwrap_or(50));
    if let Some(before_id) = before {
        pagination = pagination.before(before_id);
    }

    manager
        .get_messages(&thread_id, pagination)
        .await
        .map_err(conv_err_to_string)
}

/// Add a message directly without generating an AI response.
///
/// Useful for adding system messages or manual content.
///
/// # Arguments
/// * `thread_id` - The thread's unique identifier
/// * `role` - The message role (user, assistant, system)
/// * `content` - The message content
///
/// # Returns
/// The created message
#[tauri::command]
pub async fn add_conversation_message(
    thread_id: String,
    role: ConversationRole,
    content: String,
    state: State<'_, AppState>,
) -> Result<ConversationMessage, String> {
    debug!(
        thread_id = %thread_id,
        role = %role,
        "Adding direct message"
    );

    let manager = get_conversation_manager(&state);
    manager
        .add_message(&thread_id, role, content)
        .await
        .map_err(conv_err_to_string)
}

// ============================================================================
// Suggestion Commands
// ============================================================================

/// Accept a suggestion from an AI response.
///
/// Marks the suggestion as accepted. Does not automatically apply it
/// to the campaign - that should be done separately via the appropriate
/// campaign/wizard update commands.
///
/// # Arguments
/// * `message_id` - The message containing the suggestion
/// * `suggestion_id` - The suggestion's unique identifier
///
/// # Returns
/// Result with the accepted suggestion
#[tauri::command]
pub async fn accept_suggestion(
    message_id: String,
    suggestion_id: String,
    state: State<'_, AppState>,
) -> Result<SuggestionAcceptResult, String> {
    info!(
        message_id = %message_id,
        suggestion_id = %suggestion_id,
        "Accepting suggestion"
    );

    let manager = get_conversation_manager(&state);
    manager
        .mark_suggestion_accepted(&message_id, &suggestion_id)
        .await
        .map_err(conv_err_to_string)
}

/// Reject a suggestion from an AI response.
///
/// Marks the suggestion as rejected with an optional reason.
///
/// # Arguments
/// * `message_id` - The message containing the suggestion
/// * `suggestion_id` - The suggestion's unique identifier
/// * `reason` - Optional reason for rejection
///
/// # Returns
/// Result with the rejected suggestion info
#[tauri::command]
pub async fn reject_suggestion(
    message_id: String,
    suggestion_id: String,
    reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<SuggestionRejectResult, String> {
    info!(
        message_id = %message_id,
        suggestion_id = %suggestion_id,
        reason = ?reason,
        "Rejecting suggestion"
    );

    let manager = get_conversation_manager(&state);
    manager
        .mark_suggestion_rejected(&message_id, &suggestion_id, reason)
        .await
        .map_err(conv_err_to_string)
}

/// Get all pending suggestions from a thread.
///
/// Returns suggestions that haven't been accepted or rejected yet.
///
/// # Arguments
/// * `thread_id` - The thread's unique identifier
///
/// # Returns
/// List of (message_id, suggestion) tuples
#[tauri::command]
pub async fn get_pending_suggestions(
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<PendingSuggestion>, String> {
    debug!(thread_id = %thread_id, "Getting pending suggestions");

    let manager = get_conversation_manager(&state);
    let pending = manager
        .get_pending_suggestions(&thread_id)
        .await
        .map_err(conv_err_to_string)?;

    Ok(pending
        .into_iter()
        .map(|(message_id, suggestion)| PendingSuggestion {
            message_id,
            suggestion,
        })
        .collect())
}

// ============================================================================
// Branching Commands
// ============================================================================

/// Create a new thread branched from a specific message.
///
/// Copies all messages up to and including the branch point message
/// to a new thread, allowing the conversation to diverge.
///
/// # Arguments
/// * `source_thread_id` - The thread to branch from
/// * `branch_message_id` - The message to branch at (inclusive)
///
/// # Returns
/// The new branched thread
#[tauri::command]
pub async fn branch_conversation(
    source_thread_id: String,
    branch_message_id: String,
    state: State<'_, AppState>,
) -> Result<ConversationThread, String> {
    info!(
        source_thread_id = %source_thread_id,
        branch_message_id = %branch_message_id,
        "Branching conversation"
    );

    let manager = get_conversation_manager(&state);
    manager
        .branch_from(&source_thread_id, &branch_message_id)
        .await
        .map_err(conv_err_to_string)
}

// ============================================================================
// AI Interaction Commands
// ============================================================================

/// Generate clarifying questions for the current conversation state.
///
/// Useful during wizard steps to guide the user toward complete information.
///
/// # Arguments
/// * `thread_id` - The thread's unique identifier
/// * `context` - Additional context about what information is needed
///
/// # Returns
/// List of clarifying questions
#[tauri::command]
pub async fn generate_clarifying_questions(
    thread_id: String,
    context: String,
    state: State<'_, AppState>,
) -> Result<Vec<ClarifyingQuestion>, String> {
    debug!(
        thread_id = %thread_id,
        context_len = context.len(),
        "Generating clarifying questions"
    );

    let manager = get_conversation_manager(&state);

    let thread = manager
        .get_thread_required(&thread_id)
        .await
        .map_err(conv_err_to_string)?;

    let pagination = MessagePagination::with_limit(50);
    let messages = manager
        .get_messages(&thread_id, pagination)
        .await
        .map_err(conv_err_to_string)?;

    use crate::core::llm::router::{ChatMessage, ChatRequest};

    // Build the prompt for clarifying questions
    let prompt = format!(
        r#"Based on this context about the campaign creation process:
{}

What clarifying questions should I ask to help complete this step?
Return as JSON array:
[
  {{"question": "...", "field": "...", "importance": "required|optional"}}
]

Only include questions for information not already provided in the conversation."#,
        context
    );

    // Build LLM messages
    let system_prompt = get_system_prompt_for_purpose(thread.purpose);
    let mut llm_messages = Vec::new();

    for msg in &messages.messages {
        let chat_msg = match msg.role {
            ConversationRole::User => ChatMessage::user(&msg.content),
            ConversationRole::Assistant => ChatMessage::assistant(&msg.content),
            ConversationRole::System => ChatMessage::system(&msg.content),
        };
        llm_messages.push(chat_msg);
    }

    llm_messages.push(ChatMessage::user(&prompt));

    // Build request with system prompt
    let request = ChatRequest::new(llm_messages).with_system(system_prompt);

    // Generate response
    let response = {
        let router = state.llm_router.read().await;
        router
            .chat(request)
            .await
            .map_err(|e| e.to_string())?
    };

    // Parse the questions from the response
    let questions = parse_clarifying_questions(&response.content);

    Ok(questions)
}

/// Parse clarifying questions from the response.
fn parse_clarifying_questions(response: &str) -> Vec<ClarifyingQuestion> {
    // Try to find a JSON array in the response
    if let Some(json_match) = QUESTIONS_JSON_REGEX.find(response) {
        if let Ok(questions) = serde_json::from_str::<Vec<ClarifyingQuestion>>(json_match.as_str()) {
            return questions;
        }
    }

    // If parsing fails, return empty
    tracing::warn!("Failed to parse clarifying questions from response");
    Vec::new()
}

/// A clarifying question generated by the AI.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClarifyingQuestion {
    /// The question to ask
    pub question: String,
    /// The field this question relates to
    pub field: String,
    /// Whether this information is required or optional
    pub importance: String,
}

// ============================================================================
// Response Types
// ============================================================================

/// Result of sending a message and receiving an AI response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SendMessageResult {
    /// The user message that was sent
    pub user_message: ConversationMessage,
    /// The AI response message
    pub assistant_message: ConversationMessage,
    /// The model used for generation
    pub model: String,
    /// Input tokens used (i64 to avoid truncation on large counts)
    pub tokens_in: i64,
    /// Output tokens generated (i64 to avoid truncation on large counts)
    pub tokens_out: i64,
}

/// A pending suggestion with its message ID.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PendingSuggestion {
    /// The message containing this suggestion
    pub message_id: String,
    /// The suggestion itself
    pub suggestion: crate::database::Suggestion,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_message_result_serialization() {
        let user_msg = ConversationMessage::user(
            "msg-1".to_string(),
            "thread-1".to_string(),
            "Hello".to_string(),
        );
        let assistant_msg = ConversationMessage::assistant(
            "msg-2".to_string(),
            "thread-1".to_string(),
            "Hi there!".to_string(),
        );

        let result = SendMessageResult {
            user_message: user_msg,
            assistant_message: assistant_msg,
            model: "test-model".to_string(),
            tokens_in: 10,
            tokens_out: 20,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Hello"));
        assert!(json.contains("Hi there!"));
        assert!(json.contains("test-model"));
    }

    #[test]
    fn test_pending_suggestion_serialization() {
        use crate::database::{Suggestion, SuggestionStatus};

        let suggestion = Suggestion {
            id: "sug-1".to_string(),
            field: "name".to_string(),
            value: serde_json::json!("Test Campaign"),
            rationale: "Good name".to_string(),
            status: SuggestionStatus::Pending,
        };

        let pending = PendingSuggestion {
            message_id: "msg-1".to_string(),
            suggestion,
        };

        let json = serde_json::to_string(&pending).unwrap();
        assert!(json.contains("msg-1"));
        assert!(json.contains("Test Campaign"));
    }
}
