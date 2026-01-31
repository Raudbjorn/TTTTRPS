//! Voice profile database operations
//!
//! This module provides CRUD operations for TTS voice profiles.

use super::models::VoiceProfileRecord;
use super::Database;

/// Extension trait for voice profile database operations
pub trait VoiceProfileOps {
    fn save_voice_profile(&self, profile: &VoiceProfileRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_voice_profile(&self, id: &str) -> impl std::future::Future<Output = Result<Option<VoiceProfileRecord>, sqlx::Error>> + Send;
    fn list_voice_profiles(&self) -> impl std::future::Future<Output = Result<Vec<VoiceProfileRecord>, sqlx::Error>> + Send;
    fn list_voice_profile_presets(&self) -> impl std::future::Future<Output = Result<Vec<VoiceProfileRecord>, sqlx::Error>> + Send;
    fn delete_voice_profile(&self, id: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
}

impl VoiceProfileOps for Database {
    async fn save_voice_profile(&self, profile: &VoiceProfileRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO voice_profiles
            (id, name, provider, voice_id, settings, age_range, gender,
             personality_traits, is_preset, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&profile.id)
        .bind(&profile.name)
        .bind(&profile.provider)
        .bind(&profile.voice_id)
        .bind(&profile.settings)
        .bind(&profile.age_range)
        .bind(&profile.gender)
        .bind(&profile.personality_traits)
        .bind(profile.is_preset)
        .bind(&profile.created_at)
        .bind(&profile.updated_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_voice_profile(&self, id: &str) -> Result<Option<VoiceProfileRecord>, sqlx::Error> {
        sqlx::query_as::<_, VoiceProfileRecord>(
            "SELECT * FROM voice_profiles WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
    }

    async fn list_voice_profiles(&self) -> Result<Vec<VoiceProfileRecord>, sqlx::Error> {
        sqlx::query_as::<_, VoiceProfileRecord>(
            "SELECT * FROM voice_profiles ORDER BY is_preset DESC, name ASC"
        )
        .fetch_all(self.pool())
        .await
    }

    async fn list_voice_profile_presets(&self) -> Result<Vec<VoiceProfileRecord>, sqlx::Error> {
        sqlx::query_as::<_, VoiceProfileRecord>(
            "SELECT * FROM voice_profiles WHERE is_preset = 1 ORDER BY name"
        )
        .fetch_all(self.pool())
        .await
    }

    async fn delete_voice_profile(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM voice_profiles WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}
