//! Chat session and message database operations
//!
//! This module provides CRUD operations for global chat sessions and messages.

use super::models::{GlobalChatSessionRecord, ChatMessageRecord};
use super::Database;

/// Extension trait for chat-related database operations
pub trait ChatOps {
    // Chat Sessions
    fn create_chat_session(&self, session: &GlobalChatSessionRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_active_chat_session(&self) -> impl std::future::Future<Output = Result<Option<GlobalChatSessionRecord>, sqlx::Error>> + Send;
    fn get_chat_session(&self, id: &str) -> impl std::future::Future<Output = Result<Option<GlobalChatSessionRecord>, sqlx::Error>> + Send;
    fn update_chat_session(&self, session: &GlobalChatSessionRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn archive_chat_session(&self, id: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn link_chat_session_to_game(&self, chat_session_id: &str, game_session_id: &str, campaign_id: Option<&str>) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_chat_sessions_by_game_session(&self, game_session_id: &str) -> impl std::future::Future<Output = Result<Vec<GlobalChatSessionRecord>, sqlx::Error>> + Send;
    fn list_chat_sessions(&self, limit: i32) -> impl std::future::Future<Output = Result<Vec<GlobalChatSessionRecord>, sqlx::Error>> + Send;
    fn get_or_create_active_chat_session(&self) -> impl std::future::Future<Output = Result<GlobalChatSessionRecord, sqlx::Error>> + Send;

    // Chat Messages
    fn add_chat_message(&self, message: &ChatMessageRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_chat_messages(&self, session_id: &str, limit: i32) -> impl std::future::Future<Output = Result<Vec<ChatMessageRecord>, sqlx::Error>> + Send;
    fn get_chat_message(&self, id: &str) -> impl std::future::Future<Output = Result<Option<ChatMessageRecord>, sqlx::Error>> + Send;
    fn update_chat_message(&self, message: &ChatMessageRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn clear_chat_messages(&self, session_id: &str) -> impl std::future::Future<Output = Result<u64, sqlx::Error>> + Send;
}

impl ChatOps for Database {
    // =========================================================================
    // Global Chat Session Operations
    // =========================================================================

    async fn create_chat_session(&self, session: &GlobalChatSessionRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO global_chat_sessions
            (id, status, linked_game_session_id, linked_campaign_id, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&session.id)
        .bind(&session.status)
        .bind(&session.linked_game_session_id)
        .bind(&session.linked_campaign_id)
        .bind(&session.created_at)
        .bind(&session.updated_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_active_chat_session(&self) -> Result<Option<GlobalChatSessionRecord>, sqlx::Error> {
        sqlx::query_as::<_, GlobalChatSessionRecord>(
            "SELECT * FROM global_chat_sessions WHERE status = 'active' ORDER BY created_at DESC LIMIT 1"
        )
        .fetch_optional(self.pool())
        .await
    }

    async fn get_chat_session(&self, id: &str) -> Result<Option<GlobalChatSessionRecord>, sqlx::Error> {
        sqlx::query_as::<_, GlobalChatSessionRecord>(
            "SELECT * FROM global_chat_sessions WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
    }

    async fn update_chat_session(&self, session: &GlobalChatSessionRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE global_chat_sessions
            SET status = ?, linked_game_session_id = ?, linked_campaign_id = ?, updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(&session.status)
        .bind(&session.linked_game_session_id)
        .bind(&session.linked_campaign_id)
        .bind(&session.updated_at)
        .bind(&session.id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn archive_chat_session(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE global_chat_sessions SET status = 'archived', updated_at = ? WHERE id = ?"
        )
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn link_chat_session_to_game(
        &self,
        chat_session_id: &str,
        game_session_id: &str,
        campaign_id: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE global_chat_sessions
            SET linked_game_session_id = ?, linked_campaign_id = ?, updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(game_session_id)
        .bind(campaign_id)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(chat_session_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_chat_sessions_by_game_session(&self, game_session_id: &str) -> Result<Vec<GlobalChatSessionRecord>, sqlx::Error> {
        sqlx::query_as::<_, GlobalChatSessionRecord>(
            "SELECT * FROM global_chat_sessions WHERE linked_game_session_id = ? ORDER BY created_at"
        )
        .bind(game_session_id)
        .fetch_all(self.pool())
        .await
    }

    async fn list_chat_sessions(&self, limit: i32) -> Result<Vec<GlobalChatSessionRecord>, sqlx::Error> {
        sqlx::query_as::<_, GlobalChatSessionRecord>(
            "SELECT * FROM global_chat_sessions ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(self.pool())
        .await
    }

    /// Get or create active chat session
    /// Uses partial unique index to prevent race conditions - if insert fails due to
    /// constraint violation (another active session was created concurrently), retry get
    async fn get_or_create_active_chat_session(&self) -> Result<GlobalChatSessionRecord, sqlx::Error> {
        // First check if one already exists
        if let Some(session) = self.get_active_chat_session().await? {
            return Ok(session);
        }

        // Try to create a new active session
        let session = GlobalChatSessionRecord::new();
        match self.create_chat_session(&session).await {
            Ok(()) => Ok(session),
            Err(e) => {
                // Check if it's a unique constraint violation (race condition)
                let err_str = e.to_string();
                if err_str.contains("UNIQUE constraint failed") {
                    // Another concurrent call created an active session, fetch it
                    self.get_active_chat_session()
                        .await?
                        .ok_or_else(|| sqlx::Error::RowNotFound)
                } else {
                    Err(e)
                }
            }
        }
    }

    // =========================================================================
    // Chat Message Operations
    // =========================================================================

    async fn add_chat_message(&self, message: &ChatMessageRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO chat_messages
            (id, session_id, role, content, tokens_input, tokens_output, is_streaming, metadata, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&message.id)
        .bind(&message.session_id)
        .bind(&message.role)
        .bind(&message.content)
        .bind(message.tokens_input)
        .bind(message.tokens_output)
        .bind(message.is_streaming)
        .bind(&message.metadata)
        .bind(&message.created_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_chat_messages(&self, session_id: &str, limit: i32) -> Result<Vec<ChatMessageRecord>, sqlx::Error> {
        sqlx::query_as::<_, ChatMessageRecord>(
            "SELECT * FROM chat_messages WHERE session_id = ? ORDER BY created_at DESC LIMIT ?"
        )
        .bind(session_id)
        .bind(limit)
        .fetch_all(self.pool())
        .await
        .map(|mut msgs| {
            msgs.reverse(); // Return in chronological order
            msgs
        })
    }

    async fn get_chat_message(&self, id: &str) -> Result<Option<ChatMessageRecord>, sqlx::Error> {
        sqlx::query_as::<_, ChatMessageRecord>(
            "SELECT * FROM chat_messages WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
    }

    async fn update_chat_message(&self, message: &ChatMessageRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE chat_messages
            SET content = ?, tokens_input = ?, tokens_output = ?, is_streaming = ?, metadata = ?
            WHERE id = ?
            "#
        )
        .bind(&message.content)
        .bind(message.tokens_input)
        .bind(message.tokens_output)
        .bind(message.is_streaming)
        .bind(&message.metadata)
        .bind(&message.id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn clear_chat_messages(&self, session_id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM chat_messages WHERE session_id = ?")
            .bind(session_id)
            .execute(self.pool())
            .await?;
        Ok(result.rows_affected())
    }
}
