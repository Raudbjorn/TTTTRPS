//! Conversation Manager Implementation
//!
//! Phase 5 of the Campaign Generation Overhaul.
//!
//! This module provides the ConversationManager for managing conversation threads
//! and messages, including CRUD operations, pagination, and suggestion tracking.

use sqlx::sqlite::SqlitePool;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::database::{
    ConversationMessageRecord, ConversationPurpose, ConversationRole, ConversationThreadRecord,
    Suggestion, SuggestionStatus,
};

use super::types::{
    Citation, ConversationError, ConversationMessage, ConversationThread, MessagePagination,
    PaginatedMessages, SuggestionAcceptResult, SuggestionRejectResult, ThreadListOptions,
};

// ============================================================================
// ConversationManager
// ============================================================================

/// Manages conversation threads and messages for AI-assisted campaign creation.
///
/// # Responsibilities
/// - Create, get, list, and archive conversation threads
/// - Add and retrieve messages with pagination
/// - Track and update suggestion statuses
/// - Support conversation branching
pub struct ConversationManager {
    pool: Arc<SqlitePool>,
}

impl ConversationManager {
    /// Create a new ConversationManager with the given database pool.
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Thread Management (Task 5.1)
    // ========================================================================

    /// Create a new conversation thread.
    ///
    /// # Arguments
    /// * `purpose` - The purpose/category of the conversation
    ///
    /// # Returns
    /// The newly created conversation thread
    pub async fn create_thread(
        &self,
        purpose: ConversationPurpose,
    ) -> Result<ConversationThread, ConversationError> {
        let id = uuid::Uuid::new_v4().to_string();
        let thread = ConversationThread::new(id.clone(), purpose);

        info!(thread_id = %id, purpose = %purpose, "Creating conversation thread");

        self.save_thread(&thread).await?;

        Ok(thread)
    }

    /// Create a thread linked to a campaign.
    pub async fn create_thread_for_campaign(
        &self,
        campaign_id: String,
        purpose: ConversationPurpose,
    ) -> Result<ConversationThread, ConversationError> {
        let id = uuid::Uuid::new_v4().to_string();
        let thread = ConversationThread::new(id.clone(), purpose).with_campaign(campaign_id.clone());

        info!(
            thread_id = %id,
            campaign_id = %campaign_id,
            purpose = %purpose,
            "Creating campaign conversation thread"
        );

        self.save_thread(&thread).await?;

        Ok(thread)
    }

    /// Create a thread linked to a wizard.
    pub async fn create_thread_for_wizard(
        &self,
        wizard_id: String,
        purpose: ConversationPurpose,
    ) -> Result<ConversationThread, ConversationError> {
        let id = uuid::Uuid::new_v4().to_string();
        let thread = ConversationThread::new(id.clone(), purpose).with_wizard(wizard_id.clone());

        info!(
            thread_id = %id,
            wizard_id = %wizard_id,
            purpose = %purpose,
            "Creating wizard conversation thread"
        );

        self.save_thread(&thread).await?;

        Ok(thread)
    }

    /// Get a conversation thread by ID.
    ///
    /// # Arguments
    /// * `thread_id` - The thread's unique identifier
    ///
    /// # Returns
    /// The thread if found, None otherwise
    pub async fn get_thread(
        &self,
        thread_id: &str,
    ) -> Result<Option<ConversationThread>, ConversationError> {
        let record = sqlx::query_as::<_, ConversationThreadRecord>(
            "SELECT * FROM conversation_threads WHERE id = ?",
        )
        .bind(thread_id)
        .fetch_optional(self.pool.as_ref())
        .await?;

        match record {
            Some(rec) => {
                let thread = ConversationThread::from_record(rec)
                    .map_err(|e| ConversationError::InvalidPurpose(e))?;
                Ok(Some(thread))
            }
            None => Ok(None),
        }
    }

    /// Get a thread or return an error if not found.
    pub async fn get_thread_required(
        &self,
        thread_id: &str,
    ) -> Result<ConversationThread, ConversationError> {
        self.get_thread(thread_id)
            .await?
            .ok_or_else(|| ConversationError::ThreadNotFound(thread_id.to_string()))
    }

    /// List conversation threads with filtering options.
    ///
    /// # Arguments
    /// * `options` - Filtering and pagination options
    ///
    /// # Returns
    /// List of matching threads, ordered by most recent first
    pub async fn list_threads(
        &self,
        options: ThreadListOptions,
    ) -> Result<Vec<ConversationThread>, ConversationError> {
        let mut query = String::from("SELECT * FROM conversation_threads WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        // Build dynamic query based on options
        if let Some(campaign_id) = &options.campaign_id {
            query.push_str(" AND campaign_id = ?");
            params.push(campaign_id.clone());
        }

        if let Some(purpose) = &options.purpose {
            query.push_str(" AND purpose = ?");
            params.push(purpose.to_string());
        }

        if !options.include_archived {
            query.push_str(" AND archived_at IS NULL");
        }

        query.push_str(" ORDER BY updated_at DESC LIMIT ?");
        params.push(options.limit.to_string());

        // Execute with dynamic binding
        let records = match params.len() {
            1 => {
                sqlx::query_as::<_, ConversationThreadRecord>(&query)
                    .bind(&params[0])
                    .fetch_all(self.pool.as_ref())
                    .await?
            }
            2 => {
                sqlx::query_as::<_, ConversationThreadRecord>(&query)
                    .bind(&params[0])
                    .bind(&params[1])
                    .fetch_all(self.pool.as_ref())
                    .await?
            }
            3 => {
                sqlx::query_as::<_, ConversationThreadRecord>(&query)
                    .bind(&params[0])
                    .bind(&params[1])
                    .bind(&params[2])
                    .fetch_all(self.pool.as_ref())
                    .await?
            }
            _ => {
                // Just limit param
                sqlx::query_as::<_, ConversationThreadRecord>(
                    "SELECT * FROM conversation_threads WHERE archived_at IS NULL ORDER BY updated_at DESC LIMIT ?",
                )
                .bind(options.limit)
                .fetch_all(self.pool.as_ref())
                .await?
            }
        };

        let mut threads = Vec::with_capacity(records.len());
        for record in records {
            if let Ok(thread) = ConversationThread::from_record(record) {
                threads.push(thread);
            }
        }

        Ok(threads)
    }

    /// Archive a conversation thread.
    ///
    /// Archived threads cannot receive new messages but remain readable.
    ///
    /// # Arguments
    /// * `thread_id` - The thread's unique identifier
    pub async fn archive_thread(&self, thread_id: &str) -> Result<(), ConversationError> {
        let mut thread = self.get_thread_required(thread_id).await?;

        if thread.is_archived() {
            warn!(thread_id = %thread_id, "Thread already archived");
            return Ok(());
        }

        info!(thread_id = %thread_id, "Archiving conversation thread");

        thread.archived_at = Some(chrono::Utc::now().to_rfc3339());
        thread.updated_at = chrono::Utc::now().to_rfc3339();

        self.save_thread(&thread).await?;

        Ok(())
    }

    /// Update thread title.
    pub async fn update_thread_title(
        &self,
        thread_id: &str,
        title: String,
    ) -> Result<ConversationThread, ConversationError> {
        let mut thread = self.get_thread_required(thread_id).await?;

        thread.title = Some(title);
        thread.updated_at = chrono::Utc::now().to_rfc3339();

        self.save_thread(&thread).await?;

        Ok(thread)
    }

    // ========================================================================
    // Message Management (Task 5.2)
    // ========================================================================

    /// Add a message to a conversation thread.
    ///
    /// # Arguments
    /// * `thread_id` - The thread's unique identifier
    /// * `role` - The message role (User, Assistant, System)
    /// * `content` - The message content
    ///
    /// # Returns
    /// The created message
    pub async fn add_message(
        &self,
        thread_id: &str,
        role: ConversationRole,
        content: String,
    ) -> Result<ConversationMessage, ConversationError> {
        let thread = self.get_thread_required(thread_id).await?;

        if thread.is_archived() {
            return Err(ConversationError::ThreadArchived);
        }

        let id = uuid::Uuid::new_v4().to_string();
        let message = match role {
            ConversationRole::User => ConversationMessage::user(id.clone(), thread_id.to_string(), content),
            ConversationRole::Assistant => {
                ConversationMessage::assistant(id.clone(), thread_id.to_string(), content)
            }
            ConversationRole::System => {
                ConversationMessage::system(id.clone(), thread_id.to_string(), content)
            }
        };

        debug!(
            message_id = %id,
            thread_id = %thread_id,
            role = %role,
            "Adding message to thread"
        );

        self.save_message(&message).await?;

        // Update thread message count
        self.increment_message_count(thread_id).await?;

        Ok(message)
    }

    /// Add an assistant message with suggestions and citations.
    pub async fn add_assistant_message_with_metadata(
        &self,
        thread_id: &str,
        content: String,
        suggestions: Vec<Suggestion>,
        citations: Vec<Citation>,
    ) -> Result<ConversationMessage, ConversationError> {
        let thread = self.get_thread_required(thread_id).await?;

        if thread.is_archived() {
            return Err(ConversationError::ThreadArchived);
        }

        let id = uuid::Uuid::new_v4().to_string();
        let message = ConversationMessage::assistant(id.clone(), thread_id.to_string(), content)
            .with_suggestions(suggestions)
            .with_citations(citations);

        debug!(
            message_id = %id,
            thread_id = %thread_id,
            suggestion_count = message.suggestions.len(),
            citation_count = message.citations.len(),
            "Adding assistant message with metadata"
        );

        self.save_message(&message).await?;
        self.increment_message_count(thread_id).await?;

        Ok(message)
    }

    /// Get messages for a thread with pagination.
    ///
    /// # Arguments
    /// * `thread_id` - The thread's unique identifier
    /// * `pagination` - Pagination options
    ///
    /// # Returns
    /// Paginated list of messages
    pub async fn get_messages(
        &self,
        thread_id: &str,
        pagination: MessagePagination,
    ) -> Result<PaginatedMessages, ConversationError> {
        // Verify thread exists
        let _ = self.get_thread_required(thread_id).await?;

        // Fetch one extra to determine if there are more
        let limit = pagination.limit + 1;

        let records = if let Some(before_id) = &pagination.before {
            // Get the timestamp of the cursor message
            let cursor_msg = sqlx::query_as::<_, ConversationMessageRecord>(
                "SELECT * FROM conversation_messages WHERE id = ?",
            )
            .bind(before_id)
            .fetch_optional(self.pool.as_ref())
            .await?;

            match cursor_msg {
                Some(cursor) => {
                    sqlx::query_as::<_, ConversationMessageRecord>(
                        r#"
                        SELECT * FROM conversation_messages
                        WHERE thread_id = ? AND created_at < ?
                        ORDER BY created_at DESC
                        LIMIT ?
                        "#,
                    )
                    .bind(thread_id)
                    .bind(&cursor.created_at)
                    .bind(limit)
                    .fetch_all(self.pool.as_ref())
                    .await?
                }
                None => {
                    // Cursor not found, return from the beginning
                    sqlx::query_as::<_, ConversationMessageRecord>(
                        r#"
                        SELECT * FROM conversation_messages
                        WHERE thread_id = ?
                        ORDER BY created_at DESC
                        LIMIT ?
                        "#,
                    )
                    .bind(thread_id)
                    .bind(limit)
                    .fetch_all(self.pool.as_ref())
                    .await?
                }
            }
        } else {
            sqlx::query_as::<_, ConversationMessageRecord>(
                r#"
                SELECT * FROM conversation_messages
                WHERE thread_id = ?
                ORDER BY created_at DESC
                LIMIT ?
                "#,
            )
            .bind(thread_id)
            .bind(limit)
            .fetch_all(self.pool.as_ref())
            .await?
        };

        let has_more = records.len() > pagination.limit as usize;
        let records: Vec<_> = records
            .into_iter()
            .take(pagination.limit as usize)
            .collect();

        // Use last() for pagination cursor - we want the oldest message in this batch
        // as the cursor for the next "before" query
        let next_cursor = records.last().map(|m| m.id.clone());

        let mut messages = Vec::with_capacity(records.len());
        for record in records {
            if let Ok(msg) = ConversationMessage::from_record(record) {
                messages.push(msg);
            }
        }

        // Reverse to get chronological order
        messages.reverse();

        Ok(PaginatedMessages {
            messages,
            has_more,
            next_cursor,
        })
    }

    /// Get a single message by ID.
    pub async fn get_message(
        &self,
        message_id: &str,
    ) -> Result<Option<ConversationMessage>, ConversationError> {
        let record = sqlx::query_as::<_, ConversationMessageRecord>(
            "SELECT * FROM conversation_messages WHERE id = ?",
        )
        .bind(message_id)
        .fetch_optional(self.pool.as_ref())
        .await?;

        match record {
            Some(rec) => {
                let msg = ConversationMessage::from_record(rec)
                    .map_err(|e| ConversationError::Validation(e))?;
                Ok(Some(msg))
            }
            None => Ok(None),
        }
    }

    // ========================================================================
    // Suggestion Tracking (Task 5.3)
    // ========================================================================

    /// Mark a suggestion as accepted.
    ///
    /// Updates the suggestion status in the message record.
    /// Does not apply the suggestion to the campaign - that's handled separately.
    ///
    /// # Arguments
    /// * `message_id` - The message containing the suggestion
    /// * `suggestion_id` - The suggestion's unique identifier
    ///
    /// # Returns
    /// Result with the updated suggestion
    pub async fn mark_suggestion_accepted(
        &self,
        message_id: &str,
        suggestion_id: &str,
    ) -> Result<SuggestionAcceptResult, ConversationError> {
        let message = self
            .get_message(message_id)
            .await?
            .ok_or_else(|| ConversationError::MessageNotFound(message_id.to_string()))?;

        let mut suggestions = message.suggestions.clone();
        let suggestion_idx = suggestions
            .iter()
            .position(|s| s.id == suggestion_id)
            .ok_or_else(|| ConversationError::SuggestionNotFound(suggestion_id.to_string()))?;

        suggestions[suggestion_idx].status = SuggestionStatus::Accepted;

        info!(
            message_id = %message_id,
            suggestion_id = %suggestion_id,
            field = %suggestions[suggestion_idx].field,
            "Marking suggestion as accepted"
        );

        // Update the message with the new suggestions
        let updated_message = ConversationMessage {
            suggestions: suggestions.clone(),
            ..message
        };
        self.save_message(&updated_message).await?;

        Ok(SuggestionAcceptResult {
            suggestion: suggestions[suggestion_idx].clone(),
            applied: false, // Application to campaign is separate
            error: None,
        })
    }

    /// Mark a suggestion as rejected.
    ///
    /// Updates the suggestion status in the message record.
    ///
    /// # Arguments
    /// * `message_id` - The message containing the suggestion
    /// * `suggestion_id` - The suggestion's unique identifier
    /// * `reason` - Optional reason for rejection
    ///
    /// # Returns
    /// Result with the rejected suggestion info
    pub async fn mark_suggestion_rejected(
        &self,
        message_id: &str,
        suggestion_id: &str,
        reason: Option<String>,
    ) -> Result<SuggestionRejectResult, ConversationError> {
        let message = self
            .get_message(message_id)
            .await?
            .ok_or_else(|| ConversationError::MessageNotFound(message_id.to_string()))?;

        let mut suggestions = message.suggestions.clone();
        let suggestion_idx = suggestions
            .iter()
            .position(|s| s.id == suggestion_id)
            .ok_or_else(|| ConversationError::SuggestionNotFound(suggestion_id.to_string()))?;

        suggestions[suggestion_idx].status = SuggestionStatus::Rejected;

        info!(
            message_id = %message_id,
            suggestion_id = %suggestion_id,
            field = %suggestions[suggestion_idx].field,
            reason = ?reason,
            "Marking suggestion as rejected"
        );

        // Update the message
        let updated_message = ConversationMessage {
            suggestions: suggestions.clone(),
            ..message
        };
        self.save_message(&updated_message).await?;

        Ok(SuggestionRejectResult {
            suggestion: suggestions[suggestion_idx].clone(),
            reason,
        })
    }

    /// Get all pending suggestions from a thread.
    pub async fn get_pending_suggestions(
        &self,
        thread_id: &str,
    ) -> Result<Vec<(String, Suggestion)>, ConversationError> {
        let pagination = MessagePagination {
            limit: 1000,
            before: None,
        };
        let messages = self.get_messages(thread_id, pagination).await?;

        let mut pending = Vec::new();
        for message in messages.messages {
            for suggestion in message.suggestions {
                if suggestion.status == SuggestionStatus::Pending {
                    pending.push((message.id.clone(), suggestion));
                }
            }
        }

        Ok(pending)
    }

    // ========================================================================
    // Conversation Branching (Task 5.4)
    // ========================================================================

    /// Create a new thread branched from a specific message.
    ///
    /// Copies all messages up to and including the branch point message.
    ///
    /// # Arguments
    /// * `source_thread_id` - The thread to branch from
    /// * `branch_message_id` - The message to branch at (inclusive)
    ///
    /// # Returns
    /// The new branched thread
    pub async fn branch_from(
        &self,
        source_thread_id: &str,
        branch_message_id: &str,
    ) -> Result<ConversationThread, ConversationError> {
        let source_thread = self.get_thread_required(source_thread_id).await?;

        // Verify the branch message exists and belongs to the source thread
        let branch_message = self
            .get_message(branch_message_id)
            .await?
            .ok_or_else(|| ConversationError::MessageNotFound(branch_message_id.to_string()))?;

        if branch_message.thread_id != source_thread_id {
            return Err(ConversationError::BranchError(
                "Branch message does not belong to source thread".to_string(),
            ));
        }

        // Create the new thread
        let new_thread_id = uuid::Uuid::new_v4().to_string();
        let mut new_thread = ConversationThread::new(new_thread_id.clone(), source_thread.purpose)
            .with_branch_from(source_thread_id.to_string());

        // Copy campaign/wizard links
        if let Some(campaign_id) = source_thread.campaign_id {
            new_thread = new_thread.with_campaign(campaign_id);
        }
        if let Some(wizard_id) = source_thread.wizard_id {
            new_thread = new_thread.with_wizard(wizard_id);
        }

        // Generate a title for the branched thread
        new_thread.title = Some(format!(
            "Branch of {}",
            source_thread.title.as_deref().unwrap_or("conversation")
        ));

        info!(
            source_thread_id = %source_thread_id,
            new_thread_id = %new_thread_id,
            branch_point = %branch_message_id,
            "Creating branched conversation"
        );

        // Use a transaction to ensure atomic thread creation and message copying
        let mut tx = self.pool.begin().await?;

        // Save the new thread
        let thread_record = new_thread.to_record();
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO conversation_threads
            (id, campaign_id, wizard_id, purpose, title, active_personality,
             message_count, branched_from, created_at, updated_at, archived_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&thread_record.id)
        .bind(&thread_record.campaign_id)
        .bind(&thread_record.wizard_id)
        .bind(&thread_record.purpose)
        .bind(&thread_record.title)
        .bind(&thread_record.active_personality)
        .bind(thread_record.message_count)
        .bind(&thread_record.branched_from)
        .bind(&thread_record.created_at)
        .bind(&thread_record.updated_at)
        .bind(&thread_record.archived_at)
        .execute(&mut *tx)
        .await?;

        // Copy messages up to the branch point
        // Use row value comparison (created_at, id) to handle timestamp collisions safely
        let messages_to_copy = sqlx::query_as::<_, ConversationMessageRecord>(
            r#"
            SELECT * FROM conversation_messages
            WHERE thread_id = ? AND (created_at, id) <= (
                SELECT created_at, id FROM conversation_messages WHERE id = ?
            )
            ORDER BY created_at ASC, id ASC
            "#,
        )
        .bind(source_thread_id)
        .bind(branch_message_id)
        .fetch_all(&mut *tx)
        .await?;

        let copied_count = messages_to_copy.len() as i32;

        for record in messages_to_copy {
            if let Ok(msg) = ConversationMessage::from_record(record) {
                // Create a new message with the same content but new IDs
                let new_msg_id = uuid::Uuid::new_v4().to_string();
                let new_msg_record = ConversationMessage {
                    id: new_msg_id,
                    thread_id: new_thread_id.clone(),
                    ..msg
                }.to_record();
                sqlx::query(
                    r#"
                    INSERT OR REPLACE INTO conversation_messages
                    (id, thread_id, role, content, suggestions, citations, created_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&new_msg_record.id)
                .bind(&new_msg_record.thread_id)
                .bind(&new_msg_record.role)
                .bind(&new_msg_record.content)
                .bind(&new_msg_record.suggestions)
                .bind(&new_msg_record.citations)
                .bind(&new_msg_record.created_at)
                .execute(&mut *tx)
                .await?;
            }
        }

        // Update message count and refresh updated_at timestamp
        new_thread.message_count = copied_count;
        new_thread.updated_at = chrono::Utc::now().to_rfc3339();
        let updated_record = new_thread.to_record();
        sqlx::query(
            r#"
            UPDATE conversation_threads
            SET message_count = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(updated_record.message_count)
        .bind(&updated_record.updated_at)
        .bind(&updated_record.id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(new_thread)
    }

    // ========================================================================
    // Private Helpers
    // ========================================================================

    /// Save a conversation thread to the database.
    async fn save_thread(&self, thread: &ConversationThread) -> Result<(), ConversationError> {
        let record = thread.to_record();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO conversation_threads
            (id, campaign_id, wizard_id, purpose, title, active_personality,
             message_count, branched_from, created_at, updated_at, archived_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.campaign_id)
        .bind(&record.wizard_id)
        .bind(&record.purpose)
        .bind(&record.title)
        .bind(&record.active_personality)
        .bind(record.message_count)
        .bind(&record.branched_from)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .bind(&record.archived_at)
        .execute(self.pool.as_ref())
        .await?;

        Ok(())
    }

    /// Save a conversation message to the database.
    async fn save_message(&self, message: &ConversationMessage) -> Result<(), ConversationError> {
        let record = message.to_record();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO conversation_messages
            (id, thread_id, role, content, suggestions, citations, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.thread_id)
        .bind(&record.role)
        .bind(&record.content)
        .bind(&record.suggestions)
        .bind(&record.citations)
        .bind(&record.created_at)
        .execute(self.pool.as_ref())
        .await?;

        Ok(())
    }

    /// Delete a conversation message by ID.
    ///
    /// Used to clean up orphaned messages (e.g., when AI generation fails).
    /// Also decrements the thread's message count.
    ///
    /// # Arguments
    /// * `message_id` - The message's unique identifier
    ///
    /// # Returns
    /// Ok(()) if the message was deleted or didn't exist
    pub async fn delete_message(&self, message_id: &str) -> Result<(), ConversationError> {
        // First get the message to find its thread_id
        if let Some(message) = self.get_message(message_id).await? {
            debug!(message_id = %message_id, thread_id = %message.thread_id, "Deleting message");

            sqlx::query("DELETE FROM conversation_messages WHERE id = ?")
                .bind(message_id)
                .execute(self.pool.as_ref())
                .await?;

            // Decrement message count
            self.decrement_message_count(&message.thread_id).await?;
        }

        Ok(())
    }

    /// Increment the message count for a thread.
    async fn increment_message_count(&self, thread_id: &str) -> Result<(), ConversationError> {
        sqlx::query(
            r#"
            UPDATE conversation_threads
            SET message_count = message_count + 1,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(thread_id)
        .execute(self.pool.as_ref())
        .await?;

        Ok(())
    }

    /// Decrement the message count for a thread.
    async fn decrement_message_count(&self, thread_id: &str) -> Result<(), ConversationError> {
        sqlx::query(
            r#"
            UPDATE conversation_threads
            SET message_count = MAX(0, message_count - 1),
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(thread_id)
        .execute(self.pool.as_ref())
        .await?;

        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests require a database connection.
    // These are structural tests for types and logic.

    #[test]
    fn test_message_pagination_defaults() {
        let pagination = MessagePagination::default();
        assert_eq!(pagination.limit, 50);
        assert!(pagination.before.is_none());
    }

    #[test]
    fn test_thread_list_options_defaults() {
        let options = ThreadListOptions::default();
        assert!(options.campaign_id.is_none());
        assert!(options.purpose.is_none());
        assert!(!options.include_archived);
        assert_eq!(options.limit, 0); // Default is 0, but new() sets 50
    }

    #[test]
    fn test_thread_list_options_builder() {
        let options = ThreadListOptions::new()
            .for_campaign("campaign-1".to_string())
            .with_purpose(ConversationPurpose::SessionPlanning)
            .include_archived()
            .limit(25);

        assert_eq!(options.campaign_id, Some("campaign-1".to_string()));
        assert_eq!(options.purpose, Some(ConversationPurpose::SessionPlanning));
        assert!(options.include_archived);
        assert_eq!(options.limit, 25);
    }
}
