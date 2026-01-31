//! Quick reference card database operations
//!
//! This module provides operations for pinned cards, cheat sheet preferences,
//! and card caching.

use super::models::{PinnedCardRecord, CheatSheetPreferenceRecord, CardCacheRecord};
use super::Database;
use sqlx::Row;

// ============================================================================
// Error Types
// ============================================================================

/// Errors specific to quick reference operations
#[derive(Debug, thiserror::Error)]
pub enum QuickReferenceError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Card ID mismatch: {0}")]
    CardIdMismatch(String),

    #[error("Maximum pinned cards ({0}) reached")]
    MaxPinsReached(i32),
}

impl From<QuickReferenceError> for sqlx::Error {
    fn from(e: QuickReferenceError) -> Self {
        match e {
            QuickReferenceError::Database(e) => e,
            QuickReferenceError::CardIdMismatch(msg) => sqlx::Error::Protocol(msg),
            QuickReferenceError::MaxPinsReached(limit) => sqlx::Error::Protocol(format!("Maximum pinned cards ({}) reached", limit)),
        }
    }
}

/// Extension trait for quick reference database operations
pub trait QuickReferenceOps {
    fn pin_card_with_limit(&self, card: &PinnedCardRecord, max_pins: i32) -> impl std::future::Future<Output = Result<(), QuickReferenceError>> + Send;
    fn unpin_card(&self, card_id: &str) -> impl std::future::Future<Output = Result<(), QuickReferenceError>> + Send;
    fn unpin_and_reorder(&self, session_id: &str, entity_type: &str, entity_id: &str) -> impl std::future::Future<Output = Result<(), QuickReferenceError>> + Send;
    fn get_pinned_cards(&self, session_id: &str) -> impl std::future::Future<Output = Result<Vec<PinnedCardRecord>, QuickReferenceError>> + Send;
    fn count_pinned_cards(&self, session_id: &str) -> impl std::future::Future<Output = Result<i32, QuickReferenceError>> + Send;
    fn reorder_pinned_cards(&self, session_id: &str, card_ids_in_order: &[String]) -> impl std::future::Future<Output = Result<(), QuickReferenceError>> + Send;
    fn update_pinned_card_disclosure(&self, card_id: &str, disclosure_level: &str) -> impl std::future::Future<Output = Result<(), QuickReferenceError>> + Send;
    fn is_entity_pinned(&self, session_id: &str, entity_type: &str, entity_id: &str) -> impl std::future::Future<Output = Result<bool, QuickReferenceError>> + Send;

    // Cheat Sheet Preferences
    fn save_cheat_sheet_preference(&self, pref: &CheatSheetPreferenceRecord) -> impl std::future::Future<Output = Result<(), QuickReferenceError>> + Send;
    fn get_cheat_sheet_preferences(&self, campaign_id: &str) -> impl std::future::Future<Output = Result<Vec<CheatSheetPreferenceRecord>, QuickReferenceError>> + Send;
    fn get_session_cheat_sheet_preferences(&self, campaign_id: &str, session_id: &str) -> impl std::future::Future<Output = Result<Vec<CheatSheetPreferenceRecord>, QuickReferenceError>> + Send;
    fn delete_cheat_sheet_preference(&self, id: &str) -> impl std::future::Future<Output = Result<(), QuickReferenceError>> + Send;

    // Card Cache
    fn get_card_cache(&self, entity_type: &str, entity_id: &str, disclosure_level: &str) -> impl std::future::Future<Output = Result<Option<CardCacheRecord>, QuickReferenceError>> + Send;
    fn save_card_cache(&self, cache: &CardCacheRecord) -> impl std::future::Future<Output = Result<(), QuickReferenceError>> + Send;
    fn invalidate_card_cache(&self, entity_type: &str, entity_id: &str) -> impl std::future::Future<Output = Result<u64, QuickReferenceError>> + Send;
    fn cleanup_expired_card_cache(&self) -> impl std::future::Future<Output = Result<u64, QuickReferenceError>> + Send;
}

impl QuickReferenceOps for Database {
    // =========================================================================
    // Pinned Card Operations
    // =========================================================================

    async fn pin_card_with_limit(&self, card: &PinnedCardRecord, max_pins: i32) -> Result<(), QuickReferenceError> {
        let mut tx = self.pool().begin().await?;

        // Check count inside transaction
        let count: i32 = sqlx::query("SELECT COUNT(*) as count FROM pinned_cards WHERE session_id = ?")
            .bind(&card.session_id)
            .fetch_one(&mut *tx)
            .await?
            .get::<i64, _>("count") as i32;

        if count >= max_pins {
            return Err(QuickReferenceError::MaxPinsReached(max_pins));
        }

        sqlx::query(
            r#"
            INSERT INTO pinned_cards
            (id, session_id, entity_type, entity_id, display_order, disclosure_level, pinned_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&card.id)
        .bind(&card.session_id)
        .bind(&card.entity_type)
        .bind(&card.entity_id)
        .bind(card.display_order)
        .bind(&card.disclosure_level)
        .bind(&card.pinned_at)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn unpin_card(&self, card_id: &str) -> Result<(), QuickReferenceError> {
        sqlx::query("DELETE FROM pinned_cards WHERE id = ?")
            .bind(card_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    async fn unpin_and_reorder(
        &self,
        session_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<(), QuickReferenceError> {
        let mut tx = self.pool().begin().await?;

        // Delete the card
        let result = sqlx::query(
            "DELETE FROM pinned_cards WHERE session_id = ? AND entity_type = ? AND entity_id = ?"
        )
        .bind(session_id)
        .bind(entity_type)
        .bind(entity_id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(()); // Nothing to do
        }

        // Fetch remaining cards to reorder
        let remaining = sqlx::query_as::<_, PinnedCardRecord>(
            "SELECT * FROM pinned_cards WHERE session_id = ? ORDER BY display_order"
        )
        .bind(session_id)
        .fetch_all(&mut *tx)
        .await?;

        // Update display orders
        for (index, card) in remaining.iter().enumerate() {
            sqlx::query("UPDATE pinned_cards SET display_order = ? WHERE id = ?")
                .bind(index as i32)
                .bind(&card.id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_pinned_cards(&self, session_id: &str) -> Result<Vec<PinnedCardRecord>, QuickReferenceError> {
        sqlx::query_as::<_, PinnedCardRecord>(
            "SELECT * FROM pinned_cards WHERE session_id = ? ORDER BY display_order"
        )
        .bind(session_id)
        .fetch_all(self.pool())
        .await
        .map_err(QuickReferenceError::from)
    }

    async fn count_pinned_cards(&self, session_id: &str) -> Result<i32, QuickReferenceError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM pinned_cards WHERE session_id = ?")
            .bind(session_id)
            .fetch_one(self.pool())
            .await?;
        Ok(row.get::<i64, _>("count") as i32)
    }

    /// Reorder pinned cards for a session
    ///
    /// Uses a transaction to ensure all updates succeed atomically. If any update
    /// fails, all changes are rolled back to prevent inconsistent display_order values.
    ///
    /// Validates that the provided card IDs match the session's current pinned cards
    /// to prevent orphaned or missing card ordering.
    async fn reorder_pinned_cards(
        &self,
        session_id: &str,
        card_ids_in_order: &[String],
    ) -> Result<(), QuickReferenceError> {
        // First, fetch the current pinned cards for this session
        let current_cards = self.get_pinned_cards(session_id).await?;
        let current_ids: std::collections::HashSet<&str> =
            current_cards.iter().map(|c| c.id.as_str()).collect();
        let provided_ids: std::collections::HashSet<&str> =
            card_ids_in_order.iter().map(|s| s.as_str()).collect();

        // Validate that provided IDs exactly match current session cards
        if current_ids != provided_ids {
            return Err(QuickReferenceError::CardIdMismatch(
                "Provided card IDs do not match session's current pinned cards".to_string()
            ).into());
        }

        let mut tx = self.pool().begin().await?;

        for (index, card_id) in card_ids_in_order.iter().enumerate() {
            sqlx::query("UPDATE pinned_cards SET display_order = ? WHERE id = ? AND session_id = ?")
                .bind(index as i32)
                .bind(card_id)
                .bind(session_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn update_pinned_card_disclosure(
        &self,
        card_id: &str,
        disclosure_level: &str,
    ) -> Result<(), QuickReferenceError> {
        sqlx::query("UPDATE pinned_cards SET disclosure_level = ? WHERE id = ?")
            .bind(disclosure_level)
            .bind(card_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    async fn is_entity_pinned(
        &self,
        session_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<bool, QuickReferenceError> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM pinned_cards WHERE session_id = ? AND entity_type = ? AND entity_id = ?"
        )
        .bind(session_id)
        .bind(entity_type)
        .bind(entity_id)
        .fetch_one(self.pool())
        .await?;
        Ok(row.get::<i64, _>("count") > 0)
    }

    // =========================================================================
    // Cheat Sheet Preference Operations
    // =========================================================================

    async fn save_cheat_sheet_preference(
        &self,
        pref: &CheatSheetPreferenceRecord,
    ) -> Result<(), QuickReferenceError> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO cheat_sheet_preferences
            (id, campaign_id, session_id, preference_type, entity_type, entity_id,
             include_status, default_disclosure_level, priority, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&pref.id)
        .bind(&pref.campaign_id)
        .bind(&pref.session_id)
        .bind(&pref.preference_type)
        .bind(&pref.entity_type)
        .bind(&pref.entity_id)
        .bind(&pref.include_status)
        .bind(&pref.default_disclosure_level)
        .bind(pref.priority)
        .bind(&pref.created_at)
        .bind(&pref.updated_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_cheat_sheet_preferences(
        &self,
        campaign_id: &str,
    ) -> Result<Vec<CheatSheetPreferenceRecord>, QuickReferenceError> {
        sqlx::query_as::<_, CheatSheetPreferenceRecord>(
            "SELECT * FROM cheat_sheet_preferences WHERE campaign_id = ? ORDER BY priority DESC"
        )
        .bind(campaign_id)
        .fetch_all(self.pool())
        .await
        .map_err(QuickReferenceError::from)
    }

    async fn get_session_cheat_sheet_preferences(
        &self,
        campaign_id: &str,
        session_id: &str,
    ) -> Result<Vec<CheatSheetPreferenceRecord>, QuickReferenceError> {
        sqlx::query_as::<_, CheatSheetPreferenceRecord>(
            r#"
            SELECT * FROM cheat_sheet_preferences
            WHERE campaign_id = ? AND (session_id IS NULL OR session_id = ?)
            ORDER BY priority DESC, session_id DESC
            "#
        )
        .bind(campaign_id)
        .bind(session_id)
        .fetch_all(self.pool())
        .await
        .map_err(QuickReferenceError::from)
    }

    async fn delete_cheat_sheet_preference(&self, id: &str) -> Result<(), QuickReferenceError> {
        sqlx::query("DELETE FROM cheat_sheet_preferences WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    // =========================================================================
    // Card Cache Operations
    // =========================================================================

    async fn get_card_cache(
        &self,
        entity_type: &str,
        entity_id: &str,
        disclosure_level: &str,
    ) -> Result<Option<CardCacheRecord>, QuickReferenceError> {
        sqlx::query_as::<_, CardCacheRecord>(
            r#"
            SELECT * FROM card_cache
            WHERE entity_type = ? AND entity_id = ? AND disclosure_level = ?
            "#
        )
        .bind(entity_type)
        .bind(entity_id)
        .bind(disclosure_level)
        .fetch_optional(self.pool())
        .await
        .map_err(QuickReferenceError::from)
    }

    async fn save_card_cache(&self, cache: &CardCacheRecord) -> Result<(), QuickReferenceError> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO card_cache
            (id, entity_type, entity_id, disclosure_level, html_content, generated_at, expires_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&cache.id)
        .bind(&cache.entity_type)
        .bind(&cache.entity_id)
        .bind(&cache.disclosure_level)
        .bind(&cache.html_content)
        .bind(&cache.generated_at)
        .bind(&cache.expires_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn invalidate_card_cache(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<u64, QuickReferenceError> {
        let result = sqlx::query(
            "DELETE FROM card_cache WHERE entity_type = ? AND entity_id = ?"
        )
        .bind(entity_type)
        .bind(entity_id)
        .execute(self.pool())
        .await?;
        Ok(result.rows_affected())
    }

    async fn cleanup_expired_card_cache(&self) -> Result<u64, QuickReferenceError> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query("DELETE FROM card_cache WHERE expires_at < ?")
            .bind(&now)
            .execute(self.pool())
            .await?;
        Ok(result.rows_affected())
    }
}
