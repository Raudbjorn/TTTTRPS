//! Conversation Domain Types
//!
//! Phase 5 of the Campaign Generation Overhaul.
//!
//! This module provides domain types for conversation management that wrap
//! the database record types with richer functionality.

use serde::{Deserialize, Serialize};

use crate::database::{
    ConversationMessageRecord, ConversationPurpose, ConversationRole, ConversationThreadRecord,
    SourceCitationRecord, Suggestion, SuggestionStatus,
};

// ============================================================================
// Domain Types
// ============================================================================

/// Domain representation of a conversation thread.
///
/// Wraps `ConversationThreadRecord` with additional computed fields and methods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationThread {
    /// Unique identifier
    pub id: String,
    /// Optional campaign ID this thread is linked to
    pub campaign_id: Option<String>,
    /// Optional wizard ID this thread is linked to
    pub wizard_id: Option<String>,
    /// The purpose/category of this conversation
    pub purpose: ConversationPurpose,
    /// Optional title for the thread
    pub title: Option<String>,
    /// Active personality profile (JSON)
    pub active_personality: Option<String>,
    /// Number of messages in the thread
    pub message_count: i32,
    /// ID of thread this was branched from (if any)
    pub branched_from: Option<String>,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
    /// Archive timestamp (None if active)
    pub archived_at: Option<String>,
}

impl ConversationThread {
    /// Create a new conversation thread with the given purpose.
    pub fn new(id: String, purpose: ConversationPurpose) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            campaign_id: None,
            wizard_id: None,
            purpose,
            title: None,
            active_personality: None,
            message_count: 0,
            branched_from: None,
            created_at: now.clone(),
            updated_at: now,
            archived_at: None,
        }
    }

    /// Create from a database record.
    pub fn from_record(record: ConversationThreadRecord) -> Result<Self, String> {
        let purpose = record.purpose_enum()?;
        Ok(Self {
            id: record.id,
            campaign_id: record.campaign_id,
            wizard_id: record.wizard_id,
            purpose,
            title: record.title,
            active_personality: record.active_personality,
            message_count: record.message_count,
            branched_from: record.branched_from,
            created_at: record.created_at,
            updated_at: record.updated_at,
            archived_at: record.archived_at,
        })
    }

    /// Convert to a database record.
    pub fn to_record(&self) -> ConversationThreadRecord {
        ConversationThreadRecord {
            id: self.id.clone(),
            campaign_id: self.campaign_id.clone(),
            wizard_id: self.wizard_id.clone(),
            purpose: self.purpose.to_string(),
            title: self.title.clone(),
            active_personality: self.active_personality.clone(),
            message_count: self.message_count,
            branched_from: self.branched_from.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            archived_at: self.archived_at.clone(),
        }
    }

    /// Check if the thread is archived.
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }

    /// Check if the thread is linked to a campaign.
    pub fn is_campaign_linked(&self) -> bool {
        self.campaign_id.is_some()
    }

    /// Check if the thread is linked to a wizard.
    pub fn is_wizard_linked(&self) -> bool {
        self.wizard_id.is_some()
    }

    /// Check if this thread was branched from another.
    pub fn is_branched(&self) -> bool {
        self.branched_from.is_some()
    }

    /// Link to a campaign.
    pub fn with_campaign(mut self, campaign_id: String) -> Self {
        self.campaign_id = Some(campaign_id);
        self
    }

    /// Link to a wizard.
    pub fn with_wizard(mut self, wizard_id: String) -> Self {
        self.wizard_id = Some(wizard_id);
        self
    }

    /// Set title.
    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    /// Set as branched from another thread.
    pub fn with_branch_from(mut self, parent_thread_id: String) -> Self {
        self.branched_from = Some(parent_thread_id);
        self
    }
}

/// Domain representation of a conversation message.
///
/// Includes parsed suggestions and citations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// Unique identifier
    pub id: String,
    /// Thread this message belongs to
    pub thread_id: String,
    /// Role of the message sender
    pub role: ConversationRole,
    /// Message content
    pub content: String,
    /// Parsed suggestions (for assistant messages)
    pub suggestions: Vec<Suggestion>,
    /// Parsed citations (for assistant messages)
    pub citations: Vec<Citation>,
    /// Creation timestamp
    pub created_at: String,
}

impl ConversationMessage {
    /// Create a new user message.
    pub fn user(id: String, thread_id: String, content: String) -> Self {
        Self {
            id,
            thread_id,
            role: ConversationRole::User,
            content,
            suggestions: Vec::new(),
            citations: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a new assistant message.
    pub fn assistant(id: String, thread_id: String, content: String) -> Self {
        Self {
            id,
            thread_id,
            role: ConversationRole::Assistant,
            content,
            suggestions: Vec::new(),
            citations: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a new system message.
    pub fn system(id: String, thread_id: String, content: String) -> Self {
        Self {
            id,
            thread_id,
            role: ConversationRole::System,
            content,
            suggestions: Vec::new(),
            citations: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create from a database record.
    pub fn from_record(record: ConversationMessageRecord) -> Result<Self, String> {
        let role = record.role_enum()?;
        let suggestions = record.suggestions_vec();
        let citations = parse_citations(&record.citations);

        Ok(Self {
            id: record.id,
            thread_id: record.thread_id,
            role,
            content: record.content,
            suggestions,
            citations,
            created_at: record.created_at,
        })
    }

    /// Convert to a database record.
    pub fn to_record(&self) -> ConversationMessageRecord {
        let mut record = ConversationMessageRecord::new(
            self.id.clone(),
            self.thread_id.clone(),
            self.role,
            self.content.clone(),
        );

        if !self.suggestions.is_empty() {
            record.suggestions = Some(serde_json::to_string(&self.suggestions).unwrap_or_default());
        }

        if !self.citations.is_empty() {
            record.citations = Some(serde_json::to_string(&self.citations).unwrap_or_default());
        }

        record
    }

    /// Add suggestions to the message.
    pub fn with_suggestions(mut self, suggestions: Vec<Suggestion>) -> Self {
        self.suggestions = suggestions;
        self
    }

    /// Add citations to the message.
    pub fn with_citations(mut self, citations: Vec<Citation>) -> Self {
        self.citations = citations;
        self
    }

    /// Check if this is a user message.
    pub fn is_user(&self) -> bool {
        self.role == ConversationRole::User
    }

    /// Check if this is an assistant message.
    pub fn is_assistant(&self) -> bool {
        self.role == ConversationRole::Assistant
    }

    /// Check if this message has suggestions.
    pub fn has_suggestions(&self) -> bool {
        !self.suggestions.is_empty()
    }

    /// Check if this message has citations.
    pub fn has_citations(&self) -> bool {
        !self.citations.is_empty()
    }

    /// Get pending suggestions.
    pub fn pending_suggestions(&self) -> Vec<&Suggestion> {
        self.suggestions
            .iter()
            .filter(|s| s.status == SuggestionStatus::Pending)
            .collect()
    }
}

/// Simplified citation type for conversation messages.
///
/// This is a subset of the full SourceCitationRecord for embedding in messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// Citation ID
    pub id: String,
    /// Source name (e.g., "Player's Handbook")
    pub source_name: String,
    /// Location reference (e.g., "p. 123")
    pub location: Option<String>,
    /// Excerpt from the source
    pub excerpt: Option<String>,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
}

impl Citation {
    /// Create a new citation.
    pub fn new(id: String, source_name: String, confidence: f64) -> Self {
        Self {
            id,
            source_name,
            location: None,
            excerpt: None,
            confidence,
        }
    }

    /// Create from a database record.
    pub fn from_record(record: &SourceCitationRecord) -> Self {
        Self {
            id: record.id.clone(),
            source_name: record.source_name.clone(),
            location: record.location.clone(),
            excerpt: record.excerpt.clone(),
            confidence: record.confidence,
        }
    }

    /// Set location.
    pub fn with_location(mut self, location: String) -> Self {
        self.location = Some(location);
        self
    }

    /// Set excerpt.
    pub fn with_excerpt(mut self, excerpt: String) -> Self {
        self.excerpt = Some(excerpt);
        self
    }
}

/// Parse citations from JSON string.
fn parse_citations(citations_json: &Option<String>) -> Vec<Citation> {
    citations_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default()
}

// ============================================================================
// Pagination Types
// ============================================================================

/// Pagination options for listing messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePagination {
    /// Maximum number of messages to return
    pub limit: i32,
    /// Cursor for pagination (message ID to fetch before)
    pub before: Option<String>,
}

impl Default for MessagePagination {
    fn default() -> Self {
        Self {
            limit: 50,
            before: None,
        }
    }
}

impl MessagePagination {
    /// Create a new pagination with the given limit.
    pub fn with_limit(limit: i32) -> Self {
        Self {
            limit,
            before: None,
        }
    }

    /// Set the cursor for fetching messages before a specific message.
    pub fn before(mut self, message_id: String) -> Self {
        self.before = Some(message_id);
        self
    }
}

/// Paginated list of messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedMessages {
    /// Messages in the page
    pub messages: Vec<ConversationMessage>,
    /// Whether there are more messages before these
    pub has_more: bool,
    /// Cursor for the next page (ID of the first message in this page)
    pub next_cursor: Option<String>,
}

// ============================================================================
// Thread Listing Options
// ============================================================================

/// Options for listing conversation threads.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThreadListOptions {
    /// Filter by campaign ID
    pub campaign_id: Option<String>,
    /// Filter by purpose
    pub purpose: Option<ConversationPurpose>,
    /// Include archived threads
    pub include_archived: bool,
    /// Maximum number of threads to return
    pub limit: i32,
}

impl ThreadListOptions {
    /// Create new list options.
    pub fn new() -> Self {
        Self {
            campaign_id: None,
            purpose: None,
            include_archived: false,
            limit: 50,
        }
    }

    /// Filter by campaign.
    pub fn for_campaign(mut self, campaign_id: String) -> Self {
        self.campaign_id = Some(campaign_id);
        self
    }

    /// Filter by purpose.
    pub fn with_purpose(mut self, purpose: ConversationPurpose) -> Self {
        self.purpose = Some(purpose);
        self
    }

    /// Include archived threads.
    pub fn include_archived(mut self) -> Self {
        self.include_archived = true;
        self
    }

    /// Set limit.
    pub fn limit(mut self, limit: i32) -> Self {
        self.limit = limit;
        self
    }
}

// ============================================================================
// Suggestion Decision Types
// ============================================================================

/// Result of accepting a suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionAcceptResult {
    /// The accepted suggestion
    pub suggestion: Suggestion,
    /// Whether the suggestion was applied to the campaign
    pub applied: bool,
    /// Any error that occurred during application
    pub error: Option<String>,
}

/// Result of rejecting a suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionRejectResult {
    /// The rejected suggestion
    pub suggestion: Suggestion,
    /// Optional reason for rejection
    pub reason: Option<String>,
}

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur in conversation operations.
#[derive(Debug, thiserror::Error)]
pub enum ConversationError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Thread not found: {0}")]
    ThreadNotFound(String),

    #[error("Message not found: {0}")]
    MessageNotFound(String),

    #[error("Suggestion not found: {0}")]
    SuggestionNotFound(String),

    #[error("Thread is archived and cannot be modified")]
    ThreadArchived,

    #[error("Invalid thread purpose: {0}")]
    InvalidPurpose(String),

    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Branch error: {0}")]
    BranchError(String),
}

impl From<sqlx::Error> for ConversationError {
    fn from(err: sqlx::Error) -> Self {
        ConversationError::Database(err.to_string())
    }
}

impl From<serde_json::Error> for ConversationError {
    fn from(err: serde_json::Error) -> Self {
        ConversationError::Serialization(err.to_string())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_thread_creation() {
        let thread = ConversationThread::new(
            "thread-1".to_string(),
            ConversationPurpose::CampaignCreation,
        );

        assert_eq!(thread.id, "thread-1");
        assert_eq!(thread.purpose, ConversationPurpose::CampaignCreation);
        assert_eq!(thread.message_count, 0);
        assert!(!thread.is_archived());
        assert!(!thread.is_campaign_linked());
        assert!(!thread.is_branched());
    }

    #[test]
    fn test_conversation_thread_with_links() {
        let thread = ConversationThread::new(
            "thread-1".to_string(),
            ConversationPurpose::CampaignCreation,
        )
        .with_campaign("campaign-1".to_string())
        .with_wizard("wizard-1".to_string())
        .with_title("Test Thread".to_string());

        assert!(thread.is_campaign_linked());
        assert!(thread.is_wizard_linked());
        assert_eq!(thread.title, Some("Test Thread".to_string()));
    }

    #[test]
    fn test_conversation_message_user() {
        let msg = ConversationMessage::user(
            "msg-1".to_string(),
            "thread-1".to_string(),
            "Hello, world!".to_string(),
        );

        assert!(msg.is_user());
        assert!(!msg.is_assistant());
        assert!(!msg.has_suggestions());
        assert!(!msg.has_citations());
    }

    #[test]
    fn test_conversation_message_assistant_with_suggestions() {
        let suggestion = Suggestion {
            id: "sug-1".to_string(),
            field: "name".to_string(),
            value: serde_json::json!("The Dark Crusade"),
            rationale: "Fits the dark fantasy theme".to_string(),
            status: SuggestionStatus::Pending,
        };

        let msg = ConversationMessage::assistant(
            "msg-1".to_string(),
            "thread-1".to_string(),
            "Here's a suggestion for your campaign name.".to_string(),
        )
        .with_suggestions(vec![suggestion]);

        assert!(msg.is_assistant());
        assert!(msg.has_suggestions());
        assert_eq!(msg.pending_suggestions().len(), 1);
    }

    #[test]
    fn test_citation_creation() {
        let citation = Citation::new(
            "cit-1".to_string(),
            "Player's Handbook".to_string(),
            0.95,
        )
        .with_location("p. 123".to_string())
        .with_excerpt("The fighter class is...".to_string());

        assert_eq!(citation.source_name, "Player's Handbook");
        assert_eq!(citation.location, Some("p. 123".to_string()));
        assert_eq!(citation.confidence, 0.95);
    }

    #[test]
    fn test_message_pagination() {
        let pagination = MessagePagination::with_limit(20).before("msg-50".to_string());

        assert_eq!(pagination.limit, 20);
        assert_eq!(pagination.before, Some("msg-50".to_string()));
    }

    #[test]
    fn test_thread_list_options() {
        let options = ThreadListOptions::new()
            .for_campaign("campaign-1".to_string())
            .with_purpose(ConversationPurpose::SessionPlanning)
            .include_archived()
            .limit(100);

        assert_eq!(options.campaign_id, Some("campaign-1".to_string()));
        assert_eq!(options.purpose, Some(ConversationPurpose::SessionPlanning));
        assert!(options.include_archived);
        assert_eq!(options.limit, 100);
    }

    #[test]
    fn test_thread_to_record_roundtrip() {
        let thread = ConversationThread::new(
            "thread-1".to_string(),
            ConversationPurpose::WorldBuilding,
        )
        .with_campaign("campaign-1".to_string())
        .with_title("World Building Session".to_string());

        let record = thread.to_record();
        let restored = ConversationThread::from_record(record).unwrap();

        assert_eq!(restored.id, thread.id);
        assert_eq!(restored.purpose, thread.purpose);
        assert_eq!(restored.campaign_id, thread.campaign_id);
        assert_eq!(restored.title, thread.title);
    }

    #[test]
    fn test_message_to_record_roundtrip() {
        let suggestion = Suggestion {
            id: "sug-1".to_string(),
            field: "setting".to_string(),
            value: serde_json::json!("Forgotten Realms"),
            rationale: "Classic D&D setting".to_string(),
            status: SuggestionStatus::Pending,
        };

        let msg = ConversationMessage::assistant(
            "msg-1".to_string(),
            "thread-1".to_string(),
            "Consider this setting.".to_string(),
        )
        .with_suggestions(vec![suggestion]);

        let record = msg.to_record();
        let restored = ConversationMessage::from_record(record).unwrap();

        assert_eq!(restored.id, msg.id);
        assert_eq!(restored.role, msg.role);
        assert_eq!(restored.suggestions.len(), 1);
        assert_eq!(restored.suggestions[0].field, "setting");
    }
}
