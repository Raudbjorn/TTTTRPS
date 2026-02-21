//! Conversation Management Module
//!
//! Phase 5 of the Campaign Generation Overhaul.
//!
//! This module provides conversation management for AI-assisted campaign creation,
//! including:
//!
//! - **Types**: Domain types for threads, messages, suggestions, and citations
//! - **Manager**: ConversationManager for CRUD operations and suggestion tracking
//! - **AI**: ConversationAI for generating responses with the LLM router
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌─────────────────────┐
//! │ Tauri Commands  │ --> │ ConversationManager │
//! └─────────────────┘     └─────────────────────┘
//!         │                        │
//!         │                        ▼
//!         │               ┌─────────────────┐
//!         └-------------> │ ConversationAI  │
//!                         └─────────────────┘
//!                                  │
//!                                  ▼
//!                         ┌─────────────────┐
//!                         │   LLMRouter     │
//!                         └─────────────────┘
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::core::campaign::conversation::{
//!     ConversationManager, ConversationThread, ConversationMessage,
//!     ConversationPurpose, MessagePagination, ThreadListOptions,
//! };
//!
//! // Create a manager
//! let manager = ConversationManager::new(pool);
//!
//! // Create a thread for campaign creation
//! let thread = manager
//!     .create_thread_for_campaign(campaign_id, ConversationPurpose::CampaignCreation)
//!     .await?;
//!
//! // Add a user message
//! let user_msg = manager
//!     .add_message(&thread.id, ConversationRole::User, "I want a dark fantasy campaign".to_string())
//!     .await?;
//!
//! // Generate an AI response (via LLM router in Tauri commands)
//! // The response contains content, suggestions, and citations
//!
//! // Save the AI response with suggestions
//! let ai_msg = manager
//!     .add_assistant_message_with_metadata(
//!         &thread.id,
//!         response.content,
//!         response.suggestions,
//!         response.citations,
//!     )
//!     .await?;
//! ```

pub mod ai;
pub mod manager;
pub mod types;

// Re-export main types for convenience
pub use ai::{
    ClarifyingQuestion, GeneratedResponse,
    get_system_prompt, parse_response, parse_clarifying_questions,
    CAMPAIGN_CREATION_SYSTEM_PROMPT, SESSION_PLANNING_SYSTEM_PROMPT,
    NPC_GENERATION_SYSTEM_PROMPT, WORLD_BUILDING_SYSTEM_PROMPT,
};
pub use manager::ConversationManager;
pub use types::{
    Citation, ConversationError, ConversationMessage, ConversationThread, MessagePagination,
    PaginatedMessages, SuggestionAcceptResult, SuggestionRejectResult, ThreadListOptions,
};

// Re-export database types that are commonly used
pub use crate::database::{ConversationPurpose, ConversationRole, Suggestion, SuggestionStatus};

// ============================================================================
// Module Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify that main types are exported correctly
        let _purpose = ConversationPurpose::CampaignCreation;
        let _role = ConversationRole::User;
        let _status = SuggestionStatus::Pending;
    }

    #[test]
    fn test_thread_creation() {
        let thread = ConversationThread::new(
            "test-id".to_string(),
            ConversationPurpose::CampaignCreation,
        );
        assert_eq!(thread.id, "test-id");
        assert_eq!(thread.purpose, ConversationPurpose::CampaignCreation);
    }

    #[test]
    fn test_message_creation() {
        let msg = ConversationMessage::user(
            "msg-id".to_string(),
            "thread-id".to_string(),
            "Hello".to_string(),
        );
        assert!(msg.is_user());
        assert!(!msg.is_assistant());
    }
}
