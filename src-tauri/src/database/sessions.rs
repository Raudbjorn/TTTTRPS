//! Session database operations
//!
//! This module provides CRUD operations for sessions, session notes, and session events.

use super::models::{SessionRecord, SessionNoteRecord, SessionEventRecord};
use super::Database;

/// Extension trait for session-related database operations
pub trait SessionOps {
    // Session CRUD
    fn create_session(&self, session: &SessionRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_session(&self, id: &str) -> impl std::future::Future<Output = Result<Option<SessionRecord>, sqlx::Error>> + Send;
    fn list_sessions(&self, campaign_id: &str) -> impl std::future::Future<Output = Result<Vec<SessionRecord>, sqlx::Error>> + Send;
    fn get_active_session(&self, campaign_id: &str) -> impl std::future::Future<Output = Result<Option<SessionRecord>, sqlx::Error>> + Send;
    fn update_session(&self, session: &SessionRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;

    // Session Notes
    fn save_session_note(&self, note: &SessionNoteRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_session_note(&self, id: &str) -> impl std::future::Future<Output = Result<Option<SessionNoteRecord>, sqlx::Error>> + Send;
    fn list_session_notes(&self, session_id: &str) -> impl std::future::Future<Output = Result<Vec<SessionNoteRecord>, sqlx::Error>> + Send;
    fn list_campaign_notes(&self, campaign_id: &str) -> impl std::future::Future<Output = Result<Vec<SessionNoteRecord>, sqlx::Error>> + Send;
    fn delete_session_note(&self, id: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;

    // Session Events (Timeline)
    fn save_session_event(&self, event: &SessionEventRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_session_event(&self, id: &str) -> impl std::future::Future<Output = Result<Option<SessionEventRecord>, sqlx::Error>> + Send;
    fn list_session_events(&self, session_id: &str) -> impl std::future::Future<Output = Result<Vec<SessionEventRecord>, sqlx::Error>> + Send;
    fn list_session_events_by_type(&self, session_id: &str, event_type: &str) -> impl std::future::Future<Output = Result<Vec<SessionEventRecord>, sqlx::Error>> + Send;
    fn delete_session_event(&self, id: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
}

impl SessionOps for Database {
    // =========================================================================
    // Session Operations
    // =========================================================================

    async fn create_session(&self, session: &SessionRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO sessions (id, campaign_id, session_number, status, started_at, ended_at, notes)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&session.id)
        .bind(&session.campaign_id)
        .bind(session.session_number)
        .bind(&session.status)
        .bind(&session.started_at)
        .bind(&session.ended_at)
        .bind(&session.notes)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_session(&self, id: &str) -> Result<Option<SessionRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionRecord>(
            "SELECT * FROM sessions WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
    }

    async fn list_sessions(&self, campaign_id: &str) -> Result<Vec<SessionRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionRecord>(
            "SELECT * FROM sessions WHERE campaign_id = ? ORDER BY session_number DESC"
        )
        .bind(campaign_id)
        .fetch_all(self.pool())
        .await
    }

    async fn get_active_session(&self, campaign_id: &str) -> Result<Option<SessionRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionRecord>(
            "SELECT * FROM sessions WHERE campaign_id = ? AND status = 'active' LIMIT 1"
        )
        .bind(campaign_id)
        .fetch_optional(self.pool())
        .await
    }

    async fn update_session(&self, session: &SessionRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE sessions
            SET status = ?, ended_at = ?, notes = ?
            WHERE id = ?
            "#
        )
        .bind(&session.status)
        .bind(&session.ended_at)
        .bind(&session.notes)
        .bind(&session.id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    // =========================================================================
    // Session Notes Operations
    // =========================================================================

    async fn save_session_note(&self, note: &SessionNoteRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO session_notes
            (id, session_id, campaign_id, content, tags, entity_links, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&note.id)
        .bind(&note.session_id)
        .bind(&note.campaign_id)
        .bind(&note.content)
        .bind(&note.tags)
        .bind(&note.entity_links)
        .bind(&note.created_at)
        .bind(&note.updated_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_session_note(&self, id: &str) -> Result<Option<SessionNoteRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionNoteRecord>(
            "SELECT * FROM session_notes WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
    }

    async fn list_session_notes(&self, session_id: &str) -> Result<Vec<SessionNoteRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionNoteRecord>(
            "SELECT * FROM session_notes WHERE session_id = ? ORDER BY created_at DESC"
        )
        .bind(session_id)
        .fetch_all(self.pool())
        .await
    }

    async fn list_campaign_notes(&self, campaign_id: &str) -> Result<Vec<SessionNoteRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionNoteRecord>(
            "SELECT * FROM session_notes WHERE campaign_id = ? ORDER BY created_at DESC"
        )
        .bind(campaign_id)
        .fetch_all(self.pool())
        .await
    }

    async fn delete_session_note(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM session_notes WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    // =========================================================================
    // Session Events (Timeline) Operations
    // =========================================================================

    async fn save_session_event(&self, event: &SessionEventRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO session_events
            (id, session_id, timestamp, event_type, description, entities, metadata, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&event.id)
        .bind(&event.session_id)
        .bind(&event.timestamp)
        .bind(&event.event_type)
        .bind(&event.description)
        .bind(&event.entities)
        .bind(&event.metadata)
        .bind(&event.created_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_session_event(&self, id: &str) -> Result<Option<SessionEventRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionEventRecord>(
            "SELECT * FROM session_events WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
    }

    async fn list_session_events(&self, session_id: &str) -> Result<Vec<SessionEventRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionEventRecord>(
            "SELECT * FROM session_events WHERE session_id = ? ORDER BY timestamp ASC"
        )
        .bind(session_id)
        .fetch_all(self.pool())
        .await
    }

    async fn list_session_events_by_type(
        &self,
        session_id: &str,
        event_type: &str,
    ) -> Result<Vec<SessionEventRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionEventRecord>(
            "SELECT * FROM session_events WHERE session_id = ? AND event_type = ? ORDER BY timestamp ASC"
        )
        .bind(session_id)
        .bind(event_type)
        .fetch_all(self.pool())
        .await
    }

    async fn delete_session_event(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM session_events WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}
