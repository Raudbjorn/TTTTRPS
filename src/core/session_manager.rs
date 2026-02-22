//! Session Manager Module
//!
//! Handles live game session state including initiative tracking,
//! combat management, and real-time session notes.
//!
//! Enhanced for TASK-015: Advanced Condition System
//! - Duration tracking (turns, rounds, minutes, until save, etc.)
//! - Auto-removal on expiry
//! - Stacking rules
//! - Condition immunity tracking
//! - Custom condition builder support

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;
use uuid::Uuid;

use super::session::conditions::{
    AdvancedCondition, ConditionDuration as AdvancedConditionDuration, ConditionTemplates,
};

// TASK-014: Timeline imports
use super::session::timeline::{
    EventSeverity, SessionTimeline, TimelineEvent, TimelineEventType, TimelineSummary,
};

// TASK-017: Notes imports
use super::session::notes::{
    EntityType as NoteEntityType, NoteCategory, NotesManager, SessionNote,
};

// ============================================================================
// Re-exports for backward compatibility
// ============================================================================

pub use super::session::combat::{
    CombatEvent, CombatEventType, CombatState, CombatStatus, Combatant, CombatantType,
};

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Combatant not found: {0}")]
    CombatantNotFound(String),

    #[error("No active combat")]
    NoCombatActive,

    #[error("Combat already in progress")]
    CombatAlreadyActive,

    #[error("Invalid initiative order")]
    InvalidInitiativeOrder,
}

pub type Result<T> = std::result::Result<T, SessionError>;

// ============================================================================
// Session Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSession {
    pub id: String,
    pub campaign_id: String,
    pub session_number: u32,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub status: SessionStatus,
    pub combat: Option<CombatState>,
    pub notes: Vec<SessionLogEntry>,
    pub active_scene: Option<String>,
    pub title: Option<String>,
    pub order_index: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum SessionStatus {
    #[default]
    Active,
    Paused,
    Ended,
    Planned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub entry_type: LogEntryType,
    pub content: String,
    pub actor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogEntryType {
    Narrative,
    Combat,
    RollResult,
    NPCAction,
    PlayerAction,
    SystemMessage,
    Note,
}

// ============================================================================
// Session Summary
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub campaign_id: String,
    pub session_number: u32,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_minutes: Option<i64>,
    pub status: SessionStatus,
    pub note_count: usize,
    pub had_combat: bool,
    pub order_index: i32,
}

// ============================================================================
// Session Manager
// ============================================================================

pub struct SessionManager {
    sessions: RwLock<HashMap<String, GameSession>>,
    campaign_sessions: RwLock<HashMap<String, Vec<String>>>,
    // TASK-014: Timeline storage per session
    timelines: RwLock<HashMap<String, SessionTimeline>>,
    // TASK-017: Notes manager
    notes_manager: RwLock<NotesManager>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            campaign_sessions: RwLock::new(HashMap::new()),
            timelines: RwLock::new(HashMap::new()),
            notes_manager: RwLock::new(NotesManager::new()),
        }
    }

    // ========================================================================
    // Private Helpers - Reduce Lock Boilerplate
    // ========================================================================

    /// Execute a closure with mutable access to a session
    fn with_session_mut<F, R>(&self, session_id: &str, f: F) -> Result<R>
    where
        F: FnOnce(&mut GameSession) -> R,
    {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        Ok(f(session))
    }

    /// Execute a closure with mutable access to a session's combat state
    fn with_combat_mut<F, R>(&self, session_id: &str, f: F) -> Result<R>
    where
        F: FnOnce(&mut CombatState) -> R,
    {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        let combat = session.combat.as_mut().ok_or(SessionError::NoCombatActive)?;
        Ok(f(combat))
    }

    /// Find a combatant index in the combat state
    fn find_combatant_index(combat: &CombatState, combatant_id: &str) -> Option<usize> {
        combat.combatants.iter().position(|c| c.id == combatant_id)
    }

    // ========================================================================
    // Session CRUD
    // ========================================================================

    pub fn start_session(&self, campaign_id: &str, session_number: u32) -> GameSession {
        let session = GameSession {
            id: Uuid::new_v4().to_string(),
            campaign_id: campaign_id.to_string(),
            session_number,
            started_at: Utc::now(),
            ended_at: None,
            status: SessionStatus::Active,
            combat: None,
            notes: vec![],
            active_scene: None,
            title: None,
            order_index: 0,
        };

        // Store session
        self.sessions
            .write()
            .unwrap()
            .insert(session.id.clone(), session.clone());

        // Link to campaign
        self.campaign_sessions
            .write()
            .unwrap()
            .entry(campaign_id.to_string())
            .or_default()
            .push(session.id.clone());

        // Log session start
        self.add_log_entry(
            &session.id,
            LogEntryType::SystemMessage,
            format!("Session {} started", session_number),
            None,
        );

        // TASK-014: Initialize timeline and add session start event
        self.ensure_timeline_exists(&session.id);
        let _ = self.log_session_event(
            &session.id,
            TimelineEventType::SessionStart,
            &format!("Session {} Started", session_number),
            &format!("Campaign session {} has begun", session_number),
        );

        session
    }

    pub fn create_planned_session(&self, campaign_id: &str, title: Option<String>) -> GameSession {
        // Calculate next session number
        let campaigns = self.campaign_sessions.read().unwrap();
        let session_ids = campaigns.get(campaign_id).cloned().unwrap_or_default();
        drop(campaigns);

        let sessions = self.sessions.read().unwrap();
        let max_num = session_ids
            .iter()
            .filter_map(|id| sessions.get(id))
            .map(|s| s.session_number)
            .max()
            .unwrap_or(0);
        let session_number = max_num + 1;
        drop(sessions);

        let session = GameSession {
            id: Uuid::new_v4().to_string(),
            campaign_id: campaign_id.to_string(),
            session_number,
            started_at: Utc::now(),
            ended_at: None,
            status: SessionStatus::Planned,
            combat: None,
            notes: vec![],
            active_scene: None,
            title,
            order_index: session_number as i32,
        };

        self.sessions
            .write()
            .unwrap()
            .insert(session.id.clone(), session.clone());
        self.campaign_sessions
            .write()
            .unwrap()
            .entry(campaign_id.to_string())
            .or_default()
            .push(session.id.clone());

        session
    }

    pub fn start_planned_session(&self, session_id: &str) -> Result<GameSession> {
        self.with_session_mut(session_id, |session| {
            session.status = SessionStatus::Active;
            session.started_at = Utc::now();
            session.notes.push(SessionLogEntry {
                id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                entry_type: LogEntryType::SystemMessage,
                content: format!("Session {} started (Planned)", session.session_number),
                actor: None,
            });
            session.clone()
        })
    }

    pub fn get_session(&self, session_id: &str) -> Option<GameSession> {
        self.sessions.read().unwrap().get(session_id).cloned()
    }

    pub fn get_active_session(&self, campaign_id: &str) -> Option<GameSession> {
        let sessions = self.sessions.read().unwrap();
        let campaign_sessions = self.campaign_sessions.read().unwrap();

        campaign_sessions.get(campaign_id).and_then(|ids| {
            ids.iter()
                .filter_map(|id| sessions.get(id))
                .find(|s| s.status == SessionStatus::Active)
                .cloned()
        })
    }

    pub fn list_sessions(&self, campaign_id: &str) -> Vec<SessionSummary> {
        let sessions = self.sessions.read().unwrap();
        let campaign_sessions = self.campaign_sessions.read().unwrap();

        campaign_sessions
            .get(campaign_id)
            .map(|ids| {
                let mut summaries: Vec<SessionSummary> = ids
                    .iter()
                    .filter_map(|id| sessions.get(id))
                    .map(|s| SessionSummary {
                        id: s.id.clone(),
                        campaign_id: s.campaign_id.clone(),
                        session_number: s.session_number,
                        started_at: s.started_at,
                        ended_at: s.ended_at,
                        duration_minutes: s.ended_at.map(|end| (end - s.started_at).num_minutes()),
                        status: s.status.clone(),
                        note_count: s.notes.len(),
                        had_combat: s.combat.is_some(),
                        order_index: s.order_index,
                    })
                    .collect();

                // Sort: Active -> Planned (asc order_index) -> Ended (desc date) -> Others
                summaries.sort_by(|a, b| match (&a.status, &b.status) {
                    (SessionStatus::Active, SessionStatus::Active) => {
                        b.started_at.cmp(&a.started_at)
                    }
                    (SessionStatus::Active, _) => std::cmp::Ordering::Less,
                    (_, SessionStatus::Active) => std::cmp::Ordering::Greater,
                    (SessionStatus::Planned, SessionStatus::Planned) => {
                        a.order_index.cmp(&b.order_index)
                    }
                    (SessionStatus::Planned, _) => std::cmp::Ordering::Less,
                    (_, SessionStatus::Planned) => std::cmp::Ordering::Greater,
                    _ => b.started_at.cmp(&a.started_at),
                });

                summaries
            })
            .unwrap_or_default()
    }

    pub fn pause_session(&self, session_id: &str) -> Result<()> {
        self.with_session_mut(session_id, |session| {
            session.status = SessionStatus::Paused;
        })
    }

    pub fn resume_session(&self, session_id: &str) -> Result<()> {
        self.with_session_mut(session_id, |session| {
            session.status = SessionStatus::Active;
        })
    }

    pub fn end_session(&self, session_id: &str) -> Result<SessionSummary> {
        let (summary, session_number, session_id_owned) = {
            let mut sessions = self.sessions.write().unwrap();
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

            session.status = SessionStatus::Ended;
            session.ended_at = Some(Utc::now());

            // End combat if active
            if let Some(ref mut combat) = session.combat {
                combat.end();
            }

            let summary = SessionSummary {
                id: session.id.clone(),
                campaign_id: session.campaign_id.clone(),
                session_number: session.session_number,
                started_at: session.started_at,
                ended_at: session.ended_at,
                duration_minutes: session.ended_at.map(|end| (end - session.started_at).num_minutes()),
                status: session.status.clone(),
                note_count: session.notes.len(),
                had_combat: session.combat.is_some(),
                order_index: session.order_index,
            };

            (summary, session.session_number, session.id.clone())
        };

        // TASK-014: Log session end event to timeline
        let _ = self.log_session_event(
            &session_id_owned,
            TimelineEventType::SessionEnd,
            &format!("Session {} Ended", session_number),
            &format!(
                "Session concluded after {} minutes",
                summary.duration_minutes.unwrap_or(0)
            ),
        );

        Ok(summary)
    }

    pub fn reorder_session(&self, session_id: &str, new_order: i32) -> Result<()> {
        self.with_session_mut(session_id, |session| {
            session.order_index = new_order;
        })
    }

    // ========================================================================
    // Session Logging
    // ========================================================================

    pub fn add_log_entry(
        &self,
        session_id: &str,
        entry_type: LogEntryType,
        content: String,
        actor: Option<String>,
    ) -> Option<SessionLogEntry> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)?;

        let entry = SessionLogEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            entry_type,
            content,
            actor,
        };

        session.notes.push(entry.clone());
        Some(entry)
    }

    pub fn set_active_scene(&self, session_id: &str, scene: Option<String>) -> Result<()> {
        self.with_session_mut(session_id, |session| {
            session.active_scene = scene;
        })
    }

    // ========================================================================
    // Combat Management
    // ========================================================================

    pub fn start_combat(&self, session_id: &str) -> Result<CombatState> {
        let combat = {
            let mut sessions = self.sessions.write().unwrap();
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

            if session
                .combat
                .as_ref()
                .map(|c| c.status == CombatStatus::Active)
                .unwrap_or(false)
            {
                return Err(SessionError::CombatAlreadyActive);
            }

            let combat = CombatState::new();
            session.combat = Some(combat.clone());
            combat
        };

        // TASK-014: Log combat start event to timeline
        let _ = self.log_combat_timeline_event(
            session_id,
            TimelineEventType::CombatStart,
            "Combat Initiated",
            "Roll for initiative!",
            EventSeverity::Notable,
        );

        Ok(combat)
    }

    pub fn end_combat(&self, session_id: &str) -> Result<()> {
        let rounds = self.with_combat_mut(session_id, |combat| {
            let rounds = combat.round;
            combat.end();
            rounds
        })?;

        // TASK-014: Log combat end event to timeline
        let _ = self.log_combat_timeline_event(
            session_id,
            TimelineEventType::CombatEnd,
            "Combat Concluded",
            &format!("Combat ended after {} rounds", rounds),
            EventSeverity::Notable,
        );

        Ok(())
    }

    pub fn get_combat(&self, session_id: &str) -> Option<CombatState> {
        self.sessions
            .read()
            .unwrap()
            .get(session_id)
            .and_then(|s| s.combat.clone())
    }

    // ========================================================================
    // Initiative Tracking
    // ========================================================================

    pub fn add_combatant(&self, session_id: &str, combatant: Combatant) -> Result<()> {
        self.with_combat_mut(session_id, |combat| {
            combat.add_combatant(combatant);
        })
    }

    pub fn add_combatant_quick(
        &self,
        session_id: &str,
        name: &str,
        initiative: i32,
        combatant_type: CombatantType,
    ) -> Result<Combatant> {
        let combatant = Combatant::new(name, initiative, combatant_type);
        self.add_combatant(session_id, combatant.clone())?;
        Ok(combatant)
    }

    pub fn remove_combatant(&self, session_id: &str, combatant_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        let combat = session.combat.as_mut().ok_or(SessionError::NoCombatActive)?;

        combat.remove_combatant(combatant_id)
            .map(|_| ())
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))
    }

    pub fn update_combatant(&self, session_id: &str, combatant: Combatant) -> Result<()> {
        self.with_combat_mut(session_id, |combat| {
            if let Some(existing) = combat.get_combatant_mut(&combatant.id) {
                let old_initiative = existing.initiative;
                *existing = combatant;
                if existing.initiative != old_initiative {
                    combat.sort_initiative();
                }
            }
        })
    }

    pub fn set_initiative(&self, session_id: &str, combatant_id: &str, initiative: i32) -> Result<()> {
        self.with_combat_mut(session_id, |combat| {
            if let Some(combatant) = combat.get_combatant_mut(combatant_id) {
                combatant.initiative = initiative;
                combat.sort_initiative();
            }
        })
    }

    pub fn next_turn(&self, session_id: &str) -> Result<Option<Combatant>> {
        self.with_combat_mut(session_id, |combat| {
            let result = combat.next_turn();
            result.current_combatant
        })
    }

    pub fn previous_turn(&self, session_id: &str) -> Result<Option<Combatant>> {
        self.with_combat_mut(session_id, |combat| combat.previous_turn())
    }

    pub fn get_current_combatant(&self, session_id: &str) -> Option<Combatant> {
        self.sessions
            .read()
            .unwrap()
            .get(session_id)
            .and_then(|s| s.combat.as_ref())
            .and_then(|c| c.current_combatant().cloned())
    }

    // ========================================================================
    // HP Tracking (Delegates to Combatant methods)
    // ========================================================================

    pub fn damage_combatant(&self, session_id: &str, combatant_id: &str, amount: i32) -> Result<i32> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        let combat = session.combat.as_mut().ok_or(SessionError::NoCombatActive)?;
        let idx = Self::find_combatant_index(combat, combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        let combatant = &mut combat.combatants[idx];
        let new_hp = combatant.apply_damage(amount);
        let name = combatant.name.clone();

        combat.log_event(&name, CombatEventType::Damage, format!("{} takes {} damage", name, amount));
        Ok(new_hp)
    }

    pub fn heal_combatant(&self, session_id: &str, combatant_id: &str, amount: i32) -> Result<i32> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        let combat = session.combat.as_mut().ok_or(SessionError::NoCombatActive)?;
        let idx = Self::find_combatant_index(combat, combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        let combatant = &mut combat.combatants[idx];
        let new_hp = combatant.heal(amount);
        let name = combatant.name.clone();

        combat.log_event(&name, CombatEventType::Healing, format!("{} heals {} HP", name, amount));
        Ok(new_hp)
    }

    pub fn add_temp_hp(&self, session_id: &str, combatant_id: &str, amount: i32) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        let combat = session.combat.as_mut().ok_or(SessionError::NoCombatActive)?;
        let idx = Self::find_combatant_index(combat, combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        combat.combatants[idx].add_temp_hp(amount);
        Ok(())
    }

    // ========================================================================
    // Advanced Conditions (TASK-015)
    // ========================================================================

    /// Add an advanced condition with full duration/stacking support
    pub fn add_advanced_condition(
        &self,
        session_id: &str,
        combatant_id: &str,
        condition: AdvancedCondition,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        let combat = session.combat.as_mut().ok_or(SessionError::NoCombatActive)?;
        let idx = Self::find_combatant_index(combat, combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        let combatant = &mut combat.combatants[idx];

        // Check for immunity
        if combatant.is_immune_to(&condition.name) {
            let name = combatant.name.clone();
            let cond_name = condition.name.clone();
            combat.log_event(
                &name,
                CombatEventType::ConditionApplied,
                format!("{} is immune to {}", name, cond_name),
            );
            return Ok(());
        }

        let condition_name = condition.name.clone();
        let combatant_name = combatant.name.clone();

        match combatant.condition_tracker.add_condition(condition) {
            Ok(()) => {
                combat.log_event(
                    combatant_name,
                    CombatEventType::ConditionApplied,
                    format!("Gained condition: {}", condition_name),
                );
            }
            Err(msg) => {
                combat.log_event(
                    combatant_name,
                    CombatEventType::Other,
                    format!("Condition not applied: {}", msg),
                );
            }
        }
        Ok(())
    }

    /// Add a standard condition by name with optional duration
    pub fn add_condition_by_name(
        &self,
        session_id: &str,
        combatant_id: &str,
        condition_name: &str,
        duration: Option<AdvancedConditionDuration>,
        source_id: Option<String>,
        source_name: Option<String>,
    ) -> Result<()> {
        // Get or create condition from template
        let mut condition = ConditionTemplates::by_name(condition_name).unwrap_or_else(|| {
            AdvancedCondition::new(
                condition_name,
                format!("Custom condition: {}", condition_name),
                duration
                    .clone()
                    .unwrap_or(AdvancedConditionDuration::UntilRemoved),
            )
        });

        // Override duration if specified
        if let Some(dur) = duration {
            condition.duration = dur.clone();
            condition.remaining = match &dur {
                AdvancedConditionDuration::Turns(n) => Some(*n),
                AdvancedConditionDuration::Rounds(n) => Some(*n),
                AdvancedConditionDuration::Minutes(n) => Some(*n),
                AdvancedConditionDuration::Hours(n) => Some(*n),
                _ => None,
            };
        }

        // Set source if provided
        if let (Some(src_id), Some(src_name)) = (source_id, source_name) {
            condition.source_id = Some(src_id);
            condition.source_name = Some(src_name);
        }

        self.add_advanced_condition(session_id, combatant_id, condition)
    }

    /// Remove an advanced condition by ID
    pub fn remove_advanced_condition(
        &self,
        session_id: &str,
        combatant_id: &str,
        condition_id: &str,
    ) -> Result<Option<AdvancedCondition>> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        let combat = session.combat.as_mut().ok_or(SessionError::NoCombatActive)?;
        let idx = Self::find_combatant_index(combat, combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        let combatant = &mut combat.combatants[idx];
        if let Some(removed) = combatant.condition_tracker.remove_condition(condition_id) {
            let name = combatant.name.clone();
            let cond_name = removed.name.clone();
            combat.log_event(
                &name,
                CombatEventType::ConditionRemoved,
                format!("{} loses condition: {}", name, cond_name),
            );
            Ok(Some(removed))
        } else {
            Ok(None)
        }
    }

    /// Remove all advanced conditions with a given name
    pub fn remove_advanced_condition_by_name(
        &self,
        session_id: &str,
        combatant_id: &str,
        condition_name: &str,
    ) -> Result<Vec<AdvancedCondition>> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        let combat = session.combat.as_mut().ok_or(SessionError::NoCombatActive)?;
        let idx = Self::find_combatant_index(combat, combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        let combatant = &mut combat.combatants[idx];
        let removed = combatant.condition_tracker.remove_by_name(condition_name);
        let name = combatant.name.clone();

        for condition in &removed {
            combat.log_event(
                &name,
                CombatEventType::ConditionRemoved,
                format!("{} loses condition: {}", name, condition.name),
            );
        }
        Ok(removed)
    }

    /// Get all advanced conditions for a combatant
    pub fn get_combatant_conditions(
        &self,
        session_id: &str,
        combatant_id: &str,
    ) -> Result<Vec<AdvancedCondition>> {
        let sessions = self.sessions.read().unwrap();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        let combat = session.combat.as_ref().ok_or(SessionError::NoCombatActive)?;
        let combatant = combat
            .get_combatant(combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        Ok(combatant.condition_tracker.conditions().to_vec())
    }

    /// Add a condition immunity to a combatant
    pub fn add_condition_immunity(
        &self,
        session_id: &str,
        combatant_id: &str,
        condition_name: &str,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        let combat = session.combat.as_mut().ok_or(SessionError::NoCombatActive)?;
        let idx = Self::find_combatant_index(combat, combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        combat.combatants[idx].add_immunity(condition_name);
        Ok(())
    }

    /// Remove a condition immunity from a combatant
    pub fn remove_condition_immunity(
        &self,
        session_id: &str,
        combatant_id: &str,
        condition_name: &str,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        let combat = session.combat.as_mut().ok_or(SessionError::NoCombatActive)?;
        let idx = Self::find_combatant_index(combat, combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        combat.combatants[idx].remove_immunity(condition_name);
        Ok(())
    }

    /// Attempt a saving throw against a condition
    pub fn attempt_condition_save(
        &self,
        session_id: &str,
        combatant_id: &str,
        condition_id: &str,
        roll: i32,
    ) -> Result<bool> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        let combat = session.combat.as_mut().ok_or(SessionError::NoCombatActive)?;
        let idx = Self::find_combatant_index(combat, combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        let combatant = &mut combat.combatants[idx];
        if let Some(cond) = combatant.condition_tracker.get_mut(condition_id) {
            let success = cond.attempt_save(roll);
            let name = combatant.name.clone();
            let cond_name = cond.name.clone();

            if success {
                combatant.condition_tracker.remove_condition(condition_id);
                combat.log_event(
                    &name,
                    CombatEventType::ConditionRemoved,
                    format!("{} saved against {} (roll: {})", name, cond_name, roll),
                );
            } else {
                combat.log_event(
                    &name,
                    CombatEventType::Other,
                    format!("{} failed save against {} (roll: {})", name, cond_name, roll),
                );
            }
            Ok(success)
        } else {
            Ok(false)
        }
    }

    /// Tick conditions at end of turn for a specific combatant
    /// Returns names of expired conditions
    pub fn tick_conditions_end_of_turn(
        &self,
        session_id: &str,
        combatant_id: &str,
    ) -> Result<Vec<String>> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        let combat = session.combat.as_mut().ok_or(SessionError::NoCombatActive)?;
        let idx = Self::find_combatant_index(combat, combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        let combatant = &mut combat.combatants[idx];
        let expired = combatant.condition_tracker.tick_end_of_turn(true);
        let expired_names: Vec<String> = expired.iter().map(|c| c.name.clone()).collect();

        let name = combatant.name.clone();
        for cond in &expired {
            combat.log_event(
                &name,
                CombatEventType::ConditionRemoved,
                format!("{} expired on {}", cond.name, name),
            );
        }

        Ok(expired_names)
    }

    /// Tick conditions at start of turn for a specific combatant
    /// Returns names of expired conditions
    pub fn tick_conditions_start_of_turn(
        &self,
        session_id: &str,
        combatant_id: &str,
    ) -> Result<Vec<String>> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        let combat = session.combat.as_mut().ok_or(SessionError::NoCombatActive)?;
        let idx = Self::find_combatant_index(combat, combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        let combatant = &mut combat.combatants[idx];
        let expired = combatant.condition_tracker.tick_start_of_turn(true);
        let expired_names: Vec<String> = expired.iter().map(|c| c.name.clone()).collect();

        let name = combatant.name.clone();
        for cond in &expired {
            combat.log_event(
                &name,
                CombatEventType::ConditionRemoved,
                format!("{} expired on {} (start of turn)", cond.name, name),
            );
        }

        Ok(expired_names)
    }

    /// Get list of available condition templates
    pub fn list_condition_templates() -> Vec<&'static str> {
        ConditionTemplates::list_names()
    }

    // ========================================================================
    // Combat Events
    // ========================================================================

    pub fn log_combat_event(
        &self,
        session_id: &str,
        actor: &str,
        event_type: CombatEventType,
        description: &str,
    ) -> Result<()> {
        self.with_combat_mut(session_id, |combat| {
            combat.log_event(actor, event_type, description);
        })
    }

    pub fn get_combat_log(&self, session_id: &str) -> Vec<CombatEvent> {
        self.sessions
            .read()
            .unwrap()
            .get(session_id)
            .and_then(|s| s.combat.as_ref())
            .map(|c| c.events.clone())
            .unwrap_or_default()
    }

    // ========================================================================
    // TASK-017: Session Notes Management
    // ========================================================================

    /// Create a new session note
    pub fn create_note(&self, note: SessionNote) -> Result<()> {
        let mut manager = self.notes_manager.write().unwrap();
        manager.create_note(note);
        Ok(())
    }

    /// Get a note by ID
    pub fn get_note(&self, note_id: &str) -> Option<SessionNote> {
        let manager = self.notes_manager.read().unwrap();
        manager.get_note(note_id).cloned()
    }

    /// Update an existing note
    pub fn update_note(&self, note: SessionNote) -> Result<SessionNote> {
        let mut manager = self.notes_manager.write().unwrap();
        manager
            .update_note(note.clone())
            .cloned()
            .map_err(SessionError::SessionNotFound)
    }

    /// Delete a note
    pub fn delete_note(&self, note_id: &str) -> Result<()> {
        let mut manager = self.notes_manager.write().unwrap();
        manager
            .delete_note(note_id)
            .ok_or_else(|| SessionError::SessionNotFound(note_id.to_string()))?;
        Ok(())
    }

    /// List all notes for a session
    pub fn list_notes_for_session(&self, session_id: &str) -> Vec<SessionNote> {
        let manager = self.notes_manager.read().unwrap();
        manager
            .notes_for_session(session_id)
            .into_iter()
            .cloned()
            .collect()
    }

    /// Search notes by query
    pub fn search_notes(&self, query: &str, session_id: Option<&str>) -> Vec<SessionNote> {
        let manager = self.notes_manager.read().unwrap();
        if let Some(sid) = session_id {
            manager
                .search_in_session(sid, query)
                .into_iter()
                .cloned()
                .collect()
        } else {
            manager.search(query).into_iter().cloned().collect()
        }
    }

    /// Get notes by category
    pub fn get_notes_by_category(
        &self,
        category: &NoteCategory,
        session_id: Option<&str>,
    ) -> Vec<SessionNote> {
        let manager = self.notes_manager.read().unwrap();
        let notes = manager.notes_in_category(category);
        if let Some(sid) = session_id {
            notes
                .into_iter()
                .filter(|n| n.session_id == sid)
                .cloned()
                .collect()
        } else {
            notes.into_iter().cloned().collect()
        }
    }

    /// Get notes by tag
    pub fn get_notes_by_tag(&self, tag: &str) -> Vec<SessionNote> {
        let manager = self.notes_manager.read().unwrap();
        manager.notes_with_tag(tag).into_iter().cloned().collect()
    }

    /// Link an entity to a note
    pub fn link_entity_to_note(
        &self,
        note_id: &str,
        entity_type: NoteEntityType,
        entity_id: &str,
        entity_name: &str,
    ) -> Result<()> {
        let mut manager = self.notes_manager.write().unwrap();
        if let Some(note) = manager.get_note_mut(note_id) {
            note.link_entity(entity_type, entity_id, entity_name);
            Ok(())
        } else {
            Err(SessionError::SessionNotFound(format!(
                "Note not found: {}",
                note_id
            )))
        }
    }

    /// Unlink an entity from a note
    pub fn unlink_entity_from_note(&self, note_id: &str, entity_id: &str) -> Result<()> {
        let mut manager = self.notes_manager.write().unwrap();
        if let Some(note) = manager.get_note_mut(note_id) {
            note.unlink_entity(entity_id);
            Ok(())
        } else {
            Err(SessionError::SessionNotFound(format!(
                "Note not found: {}",
                note_id
            )))
        }
    }
}

// ============================================================================
// TASK-014: Timeline Methods on SessionManager
// ============================================================================

impl SessionManager {
    /// Add a timeline event to a session
    pub fn add_timeline_event(&self, session_id: &str, event: TimelineEvent) -> Result<()> {
        // Verify session exists
        let sessions = self.sessions.read().unwrap();
        if !sessions.contains_key(session_id) {
            return Err(SessionError::SessionNotFound(session_id.to_string()));
        }
        drop(sessions);

        // Get or create timeline for session
        let mut timelines = self.timelines.write().unwrap();
        let timeline = timelines
            .entry(session_id.to_string())
            .or_insert_with(|| SessionTimeline::new(session_id));

        timeline.add_event(event);
        Ok(())
    }

    /// Get all timeline events for a session
    pub fn get_timeline_events(&self, session_id: &str) -> Vec<TimelineEvent> {
        let timelines = self.timelines.read().unwrap();
        timelines
            .get(session_id)
            .map(|t| t.events().to_vec())
            .unwrap_or_default()
    }

    /// Get timeline events filtered by type
    pub fn get_timeline_events_by_type(
        &self,
        session_id: &str,
        event_type: &TimelineEventType,
    ) -> Vec<TimelineEvent> {
        let timelines = self.timelines.read().unwrap();
        timelines
            .get(session_id)
            .map(|t| t.events_by_type(event_type).into_iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get timeline events filtered by severity
    pub fn get_timeline_events_by_severity(
        &self,
        session_id: &str,
        min_severity: EventSeverity,
    ) -> Vec<TimelineEvent> {
        let timelines = self.timelines.read().unwrap();
        timelines
            .get(session_id)
            .map(|t| {
                t.events_by_severity(min_severity)
                    .into_iter()
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get timeline events involving a specific entity
    pub fn get_timeline_events_for_entity(
        &self,
        session_id: &str,
        entity_id: &str,
    ) -> Vec<TimelineEvent> {
        let timelines = self.timelines.read().unwrap();
        timelines
            .get(session_id)
            .map(|t| t.events_for_entity(entity_id).into_iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get timeline summary for a session
    pub fn get_timeline_summary(&self, session_id: &str) -> Result<TimelineSummary> {
        let timelines = self.timelines.read().unwrap();
        timelines
            .get(session_id)
            .map(|t| t.generate_summary())
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))
    }

    /// Get timeline narrative (text summary for AI consumption)
    pub fn get_timeline_narrative(&self, session_id: &str) -> Option<String> {
        let timelines = self.timelines.read().unwrap();
        timelines.get(session_id).map(|t| t.generate_narrative())
    }

    /// Get recent timeline events
    pub fn get_recent_timeline_events(&self, session_id: &str, count: usize) -> Vec<TimelineEvent> {
        let timelines = self.timelines.read().unwrap();
        timelines
            .get(session_id)
            .map(|t| t.recent_events(count).into_iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Log a session lifecycle event to the timeline
    pub fn log_session_event(
        &self,
        session_id: &str,
        event_type: TimelineEventType,
        title: &str,
        description: &str,
    ) -> Result<()> {
        let event = TimelineEvent::new(session_id, event_type, title, description);
        self.add_timeline_event(session_id, event)
    }

    /// Log a combat event to the timeline with appropriate severity
    pub fn log_combat_timeline_event(
        &self,
        session_id: &str,
        event_type: TimelineEventType,
        title: &str,
        description: &str,
        severity: EventSeverity,
    ) -> Result<()> {
        let event =
            TimelineEvent::new(session_id, event_type, title, description).with_severity(severity);
        self.add_timeline_event(session_id, event)
    }

    /// Create a timeline for a new session (called automatically on session start)
    fn ensure_timeline_exists(&self, session_id: &str) {
        let mut timelines = self.timelines.write().unwrap();
        timelines
            .entry(session_id.to_string())
            .or_insert_with(|| SessionTimeline::new(session_id));
    }

    /// Get the raw timeline for direct manipulation (internal use)
    pub fn get_timeline(&self, session_id: &str) -> Option<SessionTimeline> {
        let timelines = self.timelines.read().unwrap();
        timelines.get(session_id).cloned()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_lifecycle() {
        let manager = SessionManager::new();

        // Start session
        let session = manager.start_session("campaign-1", 1);
        assert_eq!(session.session_number, 1);
        assert_eq!(session.status, SessionStatus::Active);

        // Pause and resume
        manager.pause_session(&session.id).unwrap();
        let paused = manager.get_session(&session.id).unwrap();
        assert_eq!(paused.status, SessionStatus::Paused);

        manager.resume_session(&session.id).unwrap();
        let resumed = manager.get_session(&session.id).unwrap();
        assert_eq!(resumed.status, SessionStatus::Active);

        // End session
        let summary = manager.end_session(&session.id).unwrap();
        assert_eq!(summary.status, SessionStatus::Ended);
        assert!(summary.ended_at.is_some());
    }

    #[test]
    fn test_combat_initiative() {
        let manager = SessionManager::new();
        let session = manager.start_session("campaign-1", 1);

        // Start combat
        manager.start_combat(&session.id).unwrap();

        // Add combatants
        manager
            .add_combatant_quick(&session.id, "Fighter", 18, CombatantType::Player)
            .unwrap();
        manager
            .add_combatant_quick(&session.id, "Wizard", 12, CombatantType::Player)
            .unwrap();
        manager
            .add_combatant_quick(&session.id, "Goblin", 15, CombatantType::Monster)
            .unwrap();

        // Check initiative order
        let combat = manager.get_combat(&session.id).unwrap();
        assert_eq!(combat.combatants[0].name, "Fighter");
        assert_eq!(combat.combatants[1].name, "Goblin");
        assert_eq!(combat.combatants[2].name, "Wizard");

        // Test turn advancement
        let current = manager.get_current_combatant(&session.id).unwrap();
        assert_eq!(current.name, "Fighter");

        manager.next_turn(&session.id).unwrap();
        let current = manager.get_current_combatant(&session.id).unwrap();
        assert_eq!(current.name, "Goblin");
    }

    #[test]
    fn test_hp_tracking() {
        let manager = SessionManager::new();
        let session = manager.start_session("campaign-1", 1);
        manager.start_combat(&session.id).unwrap();

        let mut combatant = Combatant::new("Fighter", 15, CombatantType::Player);
        combatant.current_hp = Some(50);
        combatant.max_hp = Some(50);
        combatant.temp_hp = Some(10);

        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant).unwrap();

        // Damage should hit temp HP first
        let remaining = manager
            .damage_combatant(&session.id, &combatant_id, 15)
            .unwrap();
        assert_eq!(remaining, 45); // 10 temp absorbed, 5 from HP

        // Heal
        let healed = manager
            .heal_combatant(&session.id, &combatant_id, 10)
            .unwrap();
        assert_eq!(healed, 50); // Back to max
    }

    #[test]
    fn test_advanced_conditions() {
        let manager = SessionManager::new();
        let session = manager.start_session("campaign-1", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = manager
            .add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player)
            .unwrap();

        // Add condition by name
        manager
            .add_condition_by_name(&session.id, &combatant.id, "Stunned", None, None, None)
            .unwrap();

        let conditions = manager
            .get_combatant_conditions(&session.id, &combatant.id)
            .unwrap();
        assert_eq!(conditions.len(), 1);
        assert_eq!(conditions[0].name, "Stunned");

        // Remove by name
        let removed = manager
            .remove_advanced_condition_by_name(&session.id, &combatant.id, "Stunned")
            .unwrap();
        assert_eq!(removed.len(), 1);

        let conditions = manager
            .get_combatant_conditions(&session.id, &combatant.id)
            .unwrap();
        assert!(conditions.is_empty());
    }
}
