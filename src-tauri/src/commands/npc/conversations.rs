//! NPC Conversation Commands
//!
//! Commands for managing NPC conversations, messages, and LLM-generated replies.

use tauri::State;
use serde::{Deserialize, Serialize};

use crate::commands::AppState;
use crate::database::{NpcConversation, ConversationMessage, NpcOps};

// ============================================================================
// Types
// ============================================================================

/// Summary information for an NPC with conversation context
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

// ============================================================================
// NPC Conversation Commands
// ============================================================================

/// List all NPC conversations for a campaign
#[tauri::command]
pub async fn list_npc_conversations(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<NpcConversation>, String> {
    state.database.list_npc_conversations(&campaign_id).await.map_err(|e| e.to_string())
}

/// Get a specific NPC conversation
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

/// Add a message to an NPC conversation
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

/// Mark NPC conversation as read
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

/// List NPC summaries with conversation metadata for a campaign
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
             // Truncate safely on char boundary (single-pass for efficiency)
             let chars: Vec<char> = last_text.chars().take(51).collect();
             let truncated = if chars.len() > 50 {
                 format!("{}...", chars.into_iter().take(50).collect::<String>())
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

/// Generate an LLM reply as an NPC
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
        images: None,
        name: None,
        tool_calls: None,
        tool_call_id: None,
    }).collect();

    if llm_messages.is_empty() {
        return Err("No context to reply to.".to_string());
    }

    // 5. Call LLM
    let config = state.llm_config.read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
        .ok_or("LLM not configured")?;
    let client = crate::core::llm::LLMClient::new(config);

    let req = crate::core::llm::ChatRequest {
        messages: llm_messages,
        system_prompt: Some(system_prompt),
        temperature: Some(0.8),
        max_tokens: Some(250),
        provider: None,
        tools: None,
        tool_choice: None,
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
