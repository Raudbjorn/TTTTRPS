//! Combat state database operations
//!
//! This module provides CRUD operations for combat encounters and state tracking.

use super::models::CombatStateRecord;
use super::Database;

/// Extension trait for combat-related database operations
pub trait CombatOps {
    fn save_combat_state(&self, combat: &CombatStateRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_combat_state(&self, id: &str) -> impl std::future::Future<Output = Result<Option<CombatStateRecord>, sqlx::Error>> + Send;
    fn get_active_combat(&self, session_id: &str) -> impl std::future::Future<Output = Result<Option<CombatStateRecord>, sqlx::Error>> + Send;
    fn list_session_combats(&self, session_id: &str) -> impl std::future::Future<Output = Result<Vec<CombatStateRecord>, sqlx::Error>> + Send;
    fn end_combat(&self, id: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn delete_combat_state(&self, id: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
}

impl CombatOps for Database {
    async fn save_combat_state(&self, combat: &CombatStateRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO combat_states
            (id, session_id, name, round, current_turn, is_active, combatants,
             conditions, environment, notes, created_at, updated_at, ended_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&combat.id)
        .bind(&combat.session_id)
        .bind(&combat.name)
        .bind(combat.round)
        .bind(combat.current_turn)
        .bind(combat.is_active)
        .bind(&combat.combatants)
        .bind(&combat.conditions)
        .bind(&combat.environment)
        .bind(&combat.notes)
        .bind(&combat.created_at)
        .bind(&combat.updated_at)
        .bind(&combat.ended_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_combat_state(&self, id: &str) -> Result<Option<CombatStateRecord>, sqlx::Error> {
        sqlx::query_as::<_, CombatStateRecord>(
            "SELECT * FROM combat_states WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
    }

    async fn get_active_combat(&self, session_id: &str) -> Result<Option<CombatStateRecord>, sqlx::Error> {
        sqlx::query_as::<_, CombatStateRecord>(
            "SELECT * FROM combat_states WHERE session_id = ? AND is_active = 1 ORDER BY created_at DESC LIMIT 1"
        )
        .bind(session_id)
        .fetch_optional(self.pool())
        .await
    }

    async fn list_session_combats(&self, session_id: &str) -> Result<Vec<CombatStateRecord>, sqlx::Error> {
        sqlx::query_as::<_, CombatStateRecord>(
            "SELECT * FROM combat_states WHERE session_id = ? ORDER BY created_at DESC"
        )
        .bind(session_id)
        .fetch_all(self.pool())
        .await
    }

    async fn end_combat(&self, id: &str) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE combat_states SET is_active = 0, ended_at = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn delete_combat_state(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM combat_states WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}
