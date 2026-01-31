//! Wizard Manager Implementation
//!
//! Manages the campaign creation wizard lifecycle with persistent state.
//!
//! # Responsibilities
//!
//! - **Lifecycle Management**: Start/get/list/delete wizards
//! - **Navigation**: Step advancement, going back, skipping optional steps
//! - **Persistence**: All state changes are immediately persisted to SQLite
//! - **Validation**: Ensures step transitions and data are valid
//! - **Completion**: Creates campaign from wizard draft and cleans up
//! - **Recovery**: Supports resuming incomplete wizards after app restart
//!
//! # Usage Example
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use crate::core::campaign::wizard::{WizardManager, StepData, BasicsData};
//!
//! let pool = Arc::new(/* ... */);
//! let manager = WizardManager::new(pool);
//!
//! // Start a new AI-assisted wizard
//! let state = manager.start_wizard(true).await?;
//!
//! // Advance through steps
//! let basics = StepData::Basics(BasicsData { /* ... */ });
//! let state = manager.advance_step(&state.id, basics).await?;
//!
//! // Skip optional steps
//! let state = manager.skip_step(&state.id).await?;
//!
//! // Complete and create campaign
//! let campaign = manager.complete_wizard(&state.id).await?;
//! ```
//!
//! # Thread Safety
//!
//! The [`WizardManager`] is designed to be used from a single Tauri command context.
//! Each operation is atomic at the database level.

use sqlx::sqlite::SqlitePool;
use std::sync::Arc;
use tracing::{debug, info};

use crate::database::{WizardStateRecord, CampaignRecord};

use super::types::{
    PartialCampaign, StepData, WizardError, WizardState, WizardSummary, WizardValidationError,
};

// ============================================================================
// WizardManager
// ============================================================================

/// Manages the campaign creation wizard state machine.
///
/// # Responsibilities
/// - Track wizard step progression
/// - Persist partial campaign data
/// - Validate step transitions
/// - Handle draft recovery
/// - Manage completion and cancellation
pub struct WizardManager {
    pool: Arc<SqlitePool>,
}

impl WizardManager {
    /// Create a new WizardManager with the given database pool
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Lifecycle Operations (Task 2.2)
    // ========================================================================

    /// Start a new campaign creation wizard.
    ///
    /// Creates a new wizard state and persists it to the database.
    ///
    /// # Arguments
    /// * `ai_assisted` - Whether to enable AI assistance for suggestions
    ///
    /// # Returns
    /// The newly created wizard state
    pub async fn start_wizard(&self, ai_assisted: bool) -> Result<WizardState, WizardError> {
        let id = uuid::Uuid::new_v4().to_string();
        let state = WizardState::new(id.clone(), ai_assisted);

        info!(wizard_id = %id, ai_assisted, "Starting new campaign wizard");

        // Persist to database
        self.save_wizard_state(&state).await?;

        Ok(state)
    }

    /// Get a wizard state by ID.
    ///
    /// # Arguments
    /// * `wizard_id` - The wizard's unique identifier
    ///
    /// # Returns
    /// The wizard state if found, None otherwise
    pub async fn get_wizard(&self, wizard_id: &str) -> Result<Option<WizardState>, WizardError> {
        let record = sqlx::query_as::<_, WizardStateRecord>(
            "SELECT * FROM wizard_states WHERE id = ?"
        )
        .bind(wizard_id)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| WizardError::Database(e.to_string()))?;

        match record {
            Some(rec) => {
                let state = WizardState::from_record(rec)?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    /// List all incomplete (non-completed) wizards.
    ///
    /// Useful for showing "resume" options to the user.
    ///
    /// # Returns
    /// List of wizard summaries ordered by last update (most recent first)
    pub async fn list_incomplete_wizards(&self) -> Result<Vec<WizardSummary>, WizardError> {
        let records = sqlx::query_as::<_, WizardStateRecord>(
            r#"
            SELECT * FROM wizard_states
            WHERE current_step != 'review'
               OR (current_step = 'review' AND completed_steps NOT LIKE '%"review"%')
            ORDER BY updated_at DESC
            "#
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| WizardError::Database(e.to_string()))?;

        let mut summaries = Vec::with_capacity(records.len());
        for record in records {
            if let Ok(state) = WizardState::from_record(record) {
                summaries.push(WizardSummary::from(&state));
            }
        }

        Ok(summaries)
    }

    /// Delete a wizard by ID.
    ///
    /// Permanently removes the wizard state from the database.
    ///
    /// # Arguments
    /// * `wizard_id` - The wizard's unique identifier
    pub async fn delete_wizard(&self, wizard_id: &str) -> Result<(), WizardError> {
        info!(wizard_id = %wizard_id, "Deleting wizard");

        let result = sqlx::query("DELETE FROM wizard_states WHERE id = ?")
            .bind(wizard_id)
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| WizardError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(WizardError::NotFound(wizard_id.to_string()));
        }

        Ok(())
    }

    // ========================================================================
    // Step Management (Task 2.3)
    // ========================================================================

    /// Advance to the next step, applying step data.
    ///
    /// Validates the step transition and data, then persists the updated state.
    ///
    /// # Arguments
    /// * `wizard_id` - The wizard's unique identifier
    /// * `step_data` - Data collected at the current step
    ///
    /// # Returns
    /// The updated wizard state
    pub async fn advance_step(
        &self,
        wizard_id: &str,
        step_data: StepData,
    ) -> Result<WizardState, WizardError> {
        let mut state = self.get_wizard_required(wizard_id).await?;

        let provided_step = step_data.step();
        if provided_step != state.current_step {
            return Err(WizardError::InvalidTransition {
                from: state.current_step.to_string(),
                to: format!("StepData for {}", provided_step),
            });
        }

        debug!(
            wizard_id = %wizard_id,
            current_step = %state.current_step,
            "Advancing wizard step"
        );

        // Apply the step data to the draft
        self.apply_step_data(&mut state.campaign_draft, &step_data)?;

        // Validate the step before advancing
        state.validate_current_step()?;

        // Mark current step as completed
        if !state.completed_steps.contains(&state.current_step) {
            state.completed_steps.push(state.current_step);
        }

        // Move to next step
        if let Some(next_step) = state.current_step.next() {
            state.current_step = next_step;
        }

        // Update timestamp
        state.updated_at = chrono::Utc::now().to_rfc3339();

        // Persist
        self.save_wizard_state(&state).await?;

        Ok(state)
    }

    /// Go back to the previous step, preserving data.
    ///
    /// # Arguments
    /// * `wizard_id` - The wizard's unique identifier
    ///
    /// # Returns
    /// The updated wizard state
    pub async fn go_back(&self, wizard_id: &str) -> Result<WizardState, WizardError> {
        let mut state = self.get_wizard_required(wizard_id).await?;

        let prev_step = state.current_step.previous()
            .ok_or_else(|| WizardError::InvalidTransition {
                from: state.current_step.to_string(),
                to: "previous (none exists)".to_string(),
            })?;

        debug!(
            wizard_id = %wizard_id,
            from = %state.current_step,
            to = %prev_step,
            "Going back in wizard"
        );

        state.current_step = prev_step;
        state.updated_at = chrono::Utc::now().to_rfc3339();

        self.save_wizard_state(&state).await?;

        Ok(state)
    }

    /// Skip the current step if it's skippable.
    ///
    /// Marks the step as skipped (not completed) and moves forward.
    ///
    /// # Arguments
    /// * `wizard_id` - The wizard's unique identifier
    ///
    /// # Returns
    /// The updated wizard state
    pub async fn skip_step(&self, wizard_id: &str) -> Result<WizardState, WizardError> {
        let mut state = self.get_wizard_required(wizard_id).await?;

        if !state.current_step.is_skippable() {
            return Err(WizardError::CannotSkip(state.current_step.to_string()));
        }

        let next_step = state.current_step.next()
            .ok_or_else(|| WizardError::InvalidTransition {
                from: state.current_step.to_string(),
                to: "next (none exists)".to_string(),
            })?;

        debug!(
            wizard_id = %wizard_id,
            skipping = %state.current_step,
            to = %next_step,
            "Skipping wizard step"
        );

        // Don't mark as completed - it was skipped
        state.current_step = next_step;
        state.updated_at = chrono::Utc::now().to_rfc3339();

        self.save_wizard_state(&state).await?;

        Ok(state)
    }

    /// Update wizard draft without advancing step.
    ///
    /// Used for partial saves or AI suggestion acceptance.
    ///
    /// # Arguments
    /// * `wizard_id` - The wizard's unique identifier
    /// * `draft` - The updated partial campaign draft
    ///
    /// # Returns
    /// The updated wizard state
    pub async fn update_draft(
        &self,
        wizard_id: &str,
        draft: PartialCampaign,
    ) -> Result<WizardState, WizardError> {
        let mut state = self.get_wizard_required(wizard_id).await?;

        state.campaign_draft = draft;
        state.updated_at = chrono::Utc::now().to_rfc3339();

        self.save_wizard_state(&state).await?;

        Ok(state)
    }

    // ========================================================================
    // Completion and Cancellation (Task 2.4)
    // ========================================================================

    /// Complete the wizard and create a campaign.
    ///
    /// Validates all required data, then creates the campaign and cleans up
    /// the wizard state. These operations are performed atomically within a
    /// transaction to prevent orphaned campaigns or wizard states.
    ///
    /// # Arguments
    /// * `wizard_id` - The wizard's unique identifier
    ///
    /// # Returns
    /// The created campaign record
    pub async fn complete_wizard(&self, wizard_id: &str) -> Result<CampaignRecord, WizardError> {
        let state = self.get_wizard_required(wizard_id).await?;

        // Validate completion
        if !state.is_ready_for_completion() {
            return Err(WizardError::Validation(
                WizardValidationError::IncompleteStep(
                    format!("Wizard at step {} is not at Review", state.current_step)
                )
            ));
        }

        state.campaign_draft.validate_for_completion()?;

        info!(wizard_id = %wizard_id, "Completing wizard and creating campaign");

        // Use a transaction to ensure campaign creation and wizard deletion are atomic.
        // If either operation fails, both are rolled back to prevent orphaned data.
        let mut tx = self.pool.begin().await
            .map_err(|e| WizardError::Database(e.to_string()))?;

        // Create the campaign within the transaction
        let campaign = self.create_campaign_from_draft_tx(&state, &mut tx).await?;

        // Delete wizard state within the same transaction
        let result = sqlx::query("DELETE FROM wizard_states WHERE id = ?")
            .bind(wizard_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| WizardError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            // This shouldn't happen since we just fetched the state, but handle it
            return Err(WizardError::NotFound(wizard_id.to_string()));
        }

        // Commit both operations atomically
        tx.commit().await
            .map_err(|e| WizardError::Database(e.to_string()))?;

        Ok(campaign)
    }

    /// Cancel the wizard with optional draft save.
    ///
    /// # Arguments
    /// * `wizard_id` - The wizard's unique identifier
    /// * `save_draft` - If true, keeps the wizard state for later resumption
    pub async fn cancel_wizard(&self, wizard_id: &str, save_draft: bool) -> Result<(), WizardError> {
        info!(wizard_id = %wizard_id, save_draft, "Cancelling wizard");

        if save_draft {
            // Just update the timestamp to mark it as "paused"
            let mut state = self.get_wizard_required(wizard_id).await?;
            state.updated_at = chrono::Utc::now().to_rfc3339();
            self.save_wizard_state(&state).await?;
        } else {
            // Delete the wizard state
            self.delete_wizard(wizard_id).await?;
        }

        Ok(())
    }

    /// Trigger an auto-save of the current wizard state.
    ///
    /// Updates the auto_saved_at timestamp and persists any pending changes.
    ///
    /// # Arguments
    /// * `wizard_id` - The wizard's unique identifier
    /// * `partial_data` - Optional partial draft updates to apply
    pub async fn auto_save(
        &self,
        wizard_id: &str,
        partial_data: Option<PartialCampaign>,
    ) -> Result<(), WizardError> {
        let mut state = self.get_wizard_required(wizard_id).await?;

        if let Some(draft) = partial_data {
            state.campaign_draft = draft;
        }

        state.auto_saved_at = Some(chrono::Utc::now().to_rfc3339());
        state.updated_at = chrono::Utc::now().to_rfc3339();

        debug!(wizard_id = %wizard_id, "Auto-saving wizard state");

        self.save_wizard_state(&state).await?;

        Ok(())
    }

    // ========================================================================
    // Conversation Thread Management
    // ========================================================================

    /// Link a conversation thread to the wizard for AI assistance.
    ///
    /// # Arguments
    /// * `wizard_id` - The wizard's unique identifier
    /// * `thread_id` - The conversation thread ID to link
    pub async fn link_conversation_thread(
        &self,
        wizard_id: &str,
        thread_id: String,
    ) -> Result<WizardState, WizardError> {
        let mut state = self.get_wizard_required(wizard_id).await?;

        state.conversation_thread_id = Some(thread_id);
        state.updated_at = chrono::Utc::now().to_rfc3339();

        self.save_wizard_state(&state).await?;

        Ok(state)
    }

    // ========================================================================
    // Private Helpers
    // ========================================================================

    /// Get wizard or return NotFound error
    async fn get_wizard_required(&self, wizard_id: &str) -> Result<WizardState, WizardError> {
        self.get_wizard(wizard_id)
            .await?
            .ok_or_else(|| WizardError::NotFound(wizard_id.to_string()))
    }

    /// Save wizard state to database
    async fn save_wizard_state(&self, state: &WizardState) -> Result<(), WizardError> {
        let record = state.to_record();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO wizard_states
            (id, current_step, completed_steps, campaign_draft, conversation_thread_id,
             ai_assisted, created_at, updated_at, auto_saved_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&record.id)
        .bind(&record.current_step)
        .bind(&record.completed_steps)
        .bind(&record.campaign_draft)
        .bind(&record.conversation_thread_id)
        .bind(record.ai_assisted)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .bind(&record.auto_saved_at)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| WizardError::Database(e.to_string()))?;

        Ok(())
    }

    /// Apply step data to the partial campaign draft
    fn apply_step_data(
        &self,
        draft: &mut PartialCampaign,
        step_data: &StepData,
    ) -> Result<(), WizardError> {
        match step_data {
            StepData::Basics(data) => {
                draft.name = Some(data.name.clone());
                draft.system = Some(data.system.clone());
                draft.description = data.description.clone();
            }
            StepData::Intent(data) => {
                draft.intent = Some(data.to_intent());
            }
            StepData::Scope(data) => {
                draft.session_scope = Some(data.to_scope());
            }
            StepData::Players(data) => {
                draft.player_count = Some(data.player_count);
                draft.experience_level = data.experience_level;
            }
            StepData::PartyComposition(data) => {
                draft.party_composition = Some(data.to_composition());
            }
            StepData::ArcStructure(data) => {
                draft.arc_structure = Some(data.to_arc_structure());
            }
            StepData::InitialContent(data) => {
                draft.initial_content = Some(data.to_initial_content());
            }
            StepData::Review => {
                // No data to apply at review step
            }
        }
        Ok(())
    }

    /// Create a campaign from the wizard draft (reserved for future use)
    #[allow(dead_code)]
    async fn create_campaign_from_draft(
        &self,
        state: &WizardState,
    ) -> Result<CampaignRecord, WizardError> {
        let draft = &state.campaign_draft;

        // These are validated as present by validate_for_completion
        let name = draft.name.as_ref().unwrap();
        let system = draft.system.as_ref().unwrap();

        let campaign_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let campaign = CampaignRecord {
            id: campaign_id.clone(),
            name: name.clone(),
            system: system.clone(),
            description: draft.description.clone(),
            setting: None,
            current_in_game_date: None,
            house_rules: None,
            world_state: None,
            created_at: now.clone(),
            updated_at: now,
            archived_at: None,
        };

        // Insert the campaign
        sqlx::query(
            r#"
            INSERT INTO campaigns (id, name, system, description, setting, current_in_game_date,
                house_rules, world_state, created_at, updated_at, archived_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&campaign.id)
        .bind(&campaign.name)
        .bind(&campaign.system)
        .bind(&campaign.description)
        .bind(&campaign.setting)
        .bind(&campaign.current_in_game_date)
        .bind(&campaign.house_rules)
        .bind(&campaign.world_state)
        .bind(&campaign.created_at)
        .bind(&campaign.updated_at)
        .bind(&campaign.archived_at)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| WizardError::Database(e.to_string()))?;

        // If we have intent data, save it
        if let Some(intent) = &draft.intent {
            let intent_id = uuid::Uuid::new_v4().to_string();
            let intent_record = intent.to_record(intent_id).with_campaign(campaign_id.clone());

            sqlx::query(
                r#"
                INSERT INTO campaign_intents
                (id, campaign_id, fantasy, player_experiences, constraints, themes,
                 tone_keywords, avoid, created_at, updated_at, migrated_from)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(&intent_record.id)
            .bind(&intent_record.campaign_id)
            .bind(&intent_record.fantasy)
            .bind(&intent_record.player_experiences)
            .bind(&intent_record.constraints)
            .bind(&intent_record.themes)
            .bind(&intent_record.tone_keywords)
            .bind(&intent_record.avoid)
            .bind(&intent_record.created_at)
            .bind(&intent_record.updated_at)
            .bind(&intent_record.migrated_from)
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| WizardError::Database(e.to_string()))?;
        }

        info!(
            campaign_id = %campaign_id,
            campaign_name = %campaign.name,
            "Created campaign from wizard"
        );

        Ok(campaign)
    }

    /// Create a campaign from the wizard draft within a transaction.
    ///
    /// This is the transactional version used by `complete_wizard` to ensure
    /// atomicity with wizard deletion.
    async fn create_campaign_from_draft_tx(
        &self,
        state: &WizardState,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    ) -> Result<CampaignRecord, WizardError> {
        let draft = &state.campaign_draft;

        // These are validated as present by validate_for_completion
        let name = draft.name.as_ref().unwrap();
        let system = draft.system.as_ref().unwrap();

        let campaign_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let campaign = CampaignRecord {
            id: campaign_id.clone(),
            name: name.clone(),
            system: system.clone(),
            description: draft.description.clone(),
            setting: None,
            current_in_game_date: None,
            house_rules: None,
            world_state: None,
            created_at: now.clone(),
            updated_at: now,
            archived_at: None,
        };

        // Insert the campaign
        sqlx::query(
            r#"
            INSERT INTO campaigns (id, name, system, description, setting, current_in_game_date,
                house_rules, world_state, created_at, updated_at, archived_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&campaign.id)
        .bind(&campaign.name)
        .bind(&campaign.system)
        .bind(&campaign.description)
        .bind(&campaign.setting)
        .bind(&campaign.current_in_game_date)
        .bind(&campaign.house_rules)
        .bind(&campaign.world_state)
        .bind(&campaign.created_at)
        .bind(&campaign.updated_at)
        .bind(&campaign.archived_at)
        .execute(&mut **tx)
        .await
        .map_err(|e| WizardError::Database(e.to_string()))?;

        // If we have intent data, save it
        if let Some(intent) = &draft.intent {
            let intent_id = uuid::Uuid::new_v4().to_string();
            let intent_record = intent.to_record(intent_id).with_campaign(campaign_id.clone());

            sqlx::query(
                r#"
                INSERT INTO campaign_intents
                (id, campaign_id, fantasy, player_experiences, constraints, themes,
                 tone_keywords, avoid, created_at, updated_at, migrated_from)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(&intent_record.id)
            .bind(&intent_record.campaign_id)
            .bind(&intent_record.fantasy)
            .bind(&intent_record.player_experiences)
            .bind(&intent_record.constraints)
            .bind(&intent_record.themes)
            .bind(&intent_record.tone_keywords)
            .bind(&intent_record.avoid)
            .bind(&intent_record.created_at)
            .bind(&intent_record.updated_at)
            .bind(&intent_record.migrated_from)
            .execute(&mut **tx)
            .await
            .map_err(|e| WizardError::Database(e.to_string()))?;
        }

        info!(
            campaign_id = %campaign_id,
            campaign_name = %campaign.name,
            "Created campaign from wizard (transactional)"
        );

        Ok(campaign)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::*;

    // Note: Full integration tests require a database connection.
    // These are unit tests for the logic that doesn't require DB.

    #[tokio::test]
    async fn test_apply_step_data_basics() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let manager = WizardManager::new(Arc::new(pool));

        let mut draft = PartialCampaign::new();
        let step_data = StepData::Basics(BasicsData {
            name: "Test Campaign".to_string(),
            system: "dnd5e".to_string(),
            description: Some("A test campaign".to_string()),
        });

        manager.apply_step_data(&mut draft, &step_data).unwrap();

        assert_eq!(draft.name, Some("Test Campaign".to_string()));
        assert_eq!(draft.system, Some("dnd5e".to_string()));
        assert_eq!(draft.description, Some("A test campaign".to_string()));
    }

    #[tokio::test]
    async fn test_apply_step_data_intent() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let manager = WizardManager::new(Arc::new(pool));

        let mut draft = PartialCampaign::new();
        let step_data = StepData::Intent(IntentData {
            fantasy: "Dark political intrigue".to_string(),
            player_experiences: vec!["mystery".to_string(), "betrayal".to_string()],
            constraints: vec!["low magic".to_string()],
            themes: vec!["power".to_string()],
            tone_keywords: vec!["grim".to_string()],
            avoid: vec!["comedy".to_string()],
        });

        manager.apply_step_data(&mut draft, &step_data).unwrap();

        let intent = draft.intent.unwrap();
        assert_eq!(intent.fantasy, "Dark political intrigue");
        assert_eq!(intent.player_experiences.len(), 2);
        assert_eq!(intent.constraints.len(), 1);
    }

    #[tokio::test]
    async fn test_apply_step_data_players() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let manager = WizardManager::new(Arc::new(pool));

        let mut draft = PartialCampaign::new();
        let step_data = StepData::Players(PlayersData {
            player_count: 5,
            experience_level: Some(ExperienceLevel::Experienced),
        });

        manager.apply_step_data(&mut draft, &step_data).unwrap();

        assert_eq!(draft.player_count, Some(5));
        assert_eq!(draft.experience_level, Some(ExperienceLevel::Experienced));
    }
}
