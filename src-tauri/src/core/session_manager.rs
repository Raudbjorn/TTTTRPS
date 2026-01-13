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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use thiserror::Error;

use super::session::conditions::{
    AdvancedCondition, ConditionDuration as AdvancedConditionDuration, ConditionTracker,
    ConditionTemplates,
};

// TASK-014: Timeline imports
use super::session::timeline::{
    TimelineEvent, TimelineEventType, EventSeverity, SessionTimeline, TimelineSummary,
};

// TASK-017: Notes imports
use super::session::notes::{
    SessionNote, NoteCategory, EntityType as NoteEntityType, NotesManager,
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
// Combat Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatState {
    pub id: String,
    pub round: u32,
    pub current_turn: usize,
    pub combatants: Vec<Combatant>,
    pub started_at: DateTime<Utc>,
    pub status: CombatStatus,
    pub events: Vec<CombatEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum CombatStatus {
    #[default]
    Active,
    Paused,
    Ended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Combatant {
    pub id: String,
    pub name: String,
    pub initiative: i32,
    pub initiative_modifier: i32,
    pub combatant_type: CombatantType,
    pub current_hp: Option<i32>,
    pub max_hp: Option<i32>,
    pub temp_hp: Option<i32>,
    pub armor_class: Option<i32>,
    /// Legacy simple conditions (for backward compatibility)
    pub conditions: Vec<Condition>,
    /// Advanced condition tracker with full duration/stacking support (TASK-015)
    #[serde(default)]
    pub condition_tracker: ConditionTracker,
    /// Condition immunities (e.g., "Frightened", "Poisoned")
    #[serde(default)]
    pub condition_immunities: Vec<String>,
    pub is_active: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CombatantType {
    Player,
    NPC,
    Monster,
    Ally,
    Environment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub name: String,
    pub duration: Option<ConditionDuration>,
    pub source: Option<String>,
    pub effects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionDuration {
    Rounds(u32),
    Minutes(u32),
    UntilSave,
    UntilRemoved,
    EndOfTurn,
    StartOfTurn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatEvent {
    pub round: u32,
    pub turn: usize,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub event_type: CombatEventType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CombatEventType {
    Attack,
    Damage,
    Healing,
    ConditionApplied,
    ConditionRemoved,
    Movement,
    Action,
    BonusAction,
    Reaction,
    Death,
    Stabilized,
    Other,
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
    // TASK-015: Advanced condition trackers per combatant
    condition_trackers: RwLock<HashMap<String, ConditionTracker>>,
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
            condition_trackers: RwLock::new(HashMap::new()),
        }
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
        self.sessions.write().unwrap()
            .insert(session.id.clone(), session.clone());

        // Link to campaign
        self.campaign_sessions.write().unwrap()
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
         // Determine next session number
         // (Simplification: Getting max session + 1 from existing sessions for this campaign)
         // Note: Logic for session number is not strictly enforced here but generally sequential.
         // For now, we'll assign 0 or calculate?
         // Task doesn't specify session number logic. start_session takes it as arg.
         // Let's assume the frontend provides it or we calculate it.
         // Ideally `create_planned_session` shouldn't require session number if it's "next".
         // But let's look at `start_session` signature: `start_session(..., session_number: u32)`.
         // I'll add `session_number` to `create_planned_session` to be safe/explicit.
         // Or better, make it auto-increment?
         // Let's stick to explicit for now to match `start_session` pattern, or check if I can find max.
         // Since I'm lazy on finding max efficiently in HashMap, I'll ask for it or mock it.
         // actually `start_session` takes `session_number`.
         // I will change `create_planned_session` to take `session_number` as well.

         // Wait, the command signature I planned: `create_planned_session(campaign_id, title)`.
         // So I need to calculate it.
         let campaigns = self.campaign_sessions.read().unwrap();
         let session_ids = campaigns.get(campaign_id).cloned().unwrap_or_default();
         drop(campaigns); // Release lock

         let sessions = self.sessions.read().unwrap();
         let max_num = session_ids.iter()
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
            started_at: Utc::now(), // Scheduled time = created time for now
            ended_at: None,
            status: SessionStatus::Planned,
            combat: None,
            notes: vec![],
            active_scene: None,
            title,
            order_index: session_number as i32, // Default order = session number
        };

        // Use title somewhere? GameSession struct doesn't have title field in the file I viewed!
        // Step 109 `GameSession` definition:
        // pub struct GameSession { ... active_scene: Option<String> }
        // It does NOT have title.
        // I need to add `title` to `GameSession` in `session_manager.rs` as well!
        // The `SessionRecord` in `models.rs` has it (I added it).
        // `GameSession` in `session_manager` is different from `SessionRecord` in `models.rs`.

        self.sessions.write().unwrap()
            .insert(session.id.clone(), session.clone());
        self.campaign_sessions.write().unwrap()
            .entry(campaign_id.to_string())
            .or_default()
            .push(session.id.clone());

        session
    }

    pub fn start_planned_session(&self, session_id: &str) -> Result<GameSession> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        if session.status != SessionStatus::Planned {
             // Maybe allow restarting Ended? No, mostly for Planned -> Active.
        }

        session.status = SessionStatus::Active;
        session.started_at = Utc::now(); // Actual start time

        // Log
        session.notes.push(SessionLogEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            entry_type: LogEntryType::SystemMessage,
            content: format!("Session {} started (Planned)", session.session_number),
            actor: None,
        });

        Ok(session.clone())
    }

    pub fn get_session(&self, session_id: &str) -> Option<GameSession> {
        self.sessions.read().unwrap().get(session_id).cloned()
    }

    pub fn get_active_session(&self, campaign_id: &str) -> Option<GameSession> {
        let sessions = self.sessions.read().unwrap();
        let campaign_sessions = self.campaign_sessions.read().unwrap();

        campaign_sessions.get(campaign_id)
            .and_then(|ids| {
                ids.iter()
                    .filter_map(|id| sessions.get(id))
                    .find(|s| s.status == SessionStatus::Active)
                    .cloned()
            })
    }

    pub fn list_sessions(&self, campaign_id: &str) -> Vec<SessionSummary> {
        let sessions = self.sessions.read().unwrap();
        let campaign_sessions = self.campaign_sessions.read().unwrap();

        campaign_sessions.get(campaign_id)
            .map(|ids| {
                let mut summaries: Vec<SessionSummary> = ids.iter()
                    .filter_map(|id| sessions.get(id))
                    .map(|s| SessionSummary {
                        id: s.id.clone(),
                        campaign_id: s.campaign_id.clone(),
                        session_number: s.session_number,
                        started_at: s.started_at,
                        ended_at: s.ended_at,
                        duration_minutes: s.ended_at.map(|end| {
                            (end - s.started_at).num_minutes()
                        }),
                        status: s.status.clone(),
                        note_count: s.notes.len(),
                        had_combat: s.combat.is_some(),
                        order_index: s.order_index,
                    })
                    .collect();

                // Sort: Active -> Planned (asc order_index) -> Ended (desc date) -> Others
                summaries.sort_by(|a, b| {
                    match (&a.status, &b.status) {
                        (SessionStatus::Active, SessionStatus::Active) => b.started_at.cmp(&a.started_at), // Newest active first?
                        (SessionStatus::Active, _) => std::cmp::Ordering::Less,
                        (_, SessionStatus::Active) => std::cmp::Ordering::Greater,

                        (SessionStatus::Planned, SessionStatus::Planned) => a.order_index.cmp(&b.order_index),
                        (SessionStatus::Planned, _) => std::cmp::Ordering::Less,
                        (_, SessionStatus::Planned) => std::cmp::Ordering::Greater,

                        _ => b.started_at.cmp(&a.started_at), // Newest first for others (Ended/Paused)
                    }
                });

                summaries
            })
            .unwrap_or_default()
    }

    pub fn pause_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        session.status = SessionStatus::Paused;
        Ok(())
    }

    pub fn resume_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        session.status = SessionStatus::Active;
        Ok(())
    }

    pub fn end_session(&self, session_id: &str) -> Result<SessionSummary> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        session.status = SessionStatus::Ended;
        session.ended_at = Some(Utc::now());

        // End combat if active
        if let Some(ref mut combat) = session.combat {
            combat.status = CombatStatus::Ended;
        }

        let session_number = session.session_number;
        let session_id_owned = session.id.clone();

        let summary = SessionSummary {
            id: session.id.clone(),
            campaign_id: session.campaign_id.clone(),
            session_number: session.session_number,
            started_at: session.started_at,
            ended_at: session.ended_at,
            duration_minutes: session.ended_at.map(|end| {
                (end - session.started_at).num_minutes()
            }),
            status: session.status.clone(),
            note_count: session.notes.len(),
            had_combat: session.combat.is_some(),
            order_index: session.order_index,
        };

        // Release lock before calling timeline methods
        drop(sessions);

        // TASK-014: Log session end event to timeline
        let _ = self.log_session_event(
            &session_id_owned,
            TimelineEventType::SessionEnd,
            &format!("Session {} Ended", session_number),
            &format!("Session concluded after {} minutes", summary.duration_minutes.unwrap_or(0)),
        );

        Ok(summary)
    }

    pub fn reorder_session(&self, session_id: &str, new_order: i32) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        session.order_index = new_order;
        Ok(())
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
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        session.active_scene = scene;
        Ok(())
    }

    // ========================================================================
    // Combat Management
    // ========================================================================

    pub fn start_combat(&self, session_id: &str) -> Result<CombatState> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        if session.combat.as_ref().map(|c| c.status == CombatStatus::Active).unwrap_or(false) {
            return Err(SessionError::CombatAlreadyActive);
        }

        let combat = CombatState {
            id: Uuid::new_v4().to_string(),
            round: 1,
            current_turn: 0,
            combatants: vec![],
            started_at: Utc::now(),
            status: CombatStatus::Active,
            events: vec![],
        };

        session.combat = Some(combat.clone());
        let session_id_owned = session_id.to_string();
        drop(sessions);

        // TASK-014: Log combat start event to timeline
        let _ = self.log_combat_timeline_event(
            &session_id_owned,
            TimelineEventType::CombatStart,
            "Combat Initiated",
            "Roll for initiative!",
            EventSeverity::Notable,
        );

        Ok(combat)
    }

    pub fn end_combat(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        let rounds = combat.round;
        combat.status = CombatStatus::Ended;
        let session_id_owned = session_id.to_string();
        drop(sessions);

        // TASK-014: Log combat end event to timeline
        let _ = self.log_combat_timeline_event(
            &session_id_owned,
            TimelineEventType::CombatEnd,
            "Combat Concluded",
            &format!("Combat ended after {} rounds", rounds),
            EventSeverity::Notable,
        );

        Ok(())
    }

    pub fn get_combat(&self, session_id: &str) -> Option<CombatState> {
        self.sessions.read().unwrap()
            .get(session_id)
            .and_then(|s| s.combat.clone())
    }

    // ========================================================================
    // Initiative Tracking
    // ========================================================================

    pub fn add_combatant(&self, session_id: &str, combatant: Combatant) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        combat.combatants.push(combatant);
        self.sort_initiative_internal(combat);

        Ok(())
    }

    pub fn add_combatant_quick(
        &self,
        session_id: &str,
        name: &str,
        initiative: i32,
        combatant_type: CombatantType,
    ) -> Result<Combatant> {
        let combatant = Combatant {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            initiative,
            initiative_modifier: 0,
            combatant_type,
            current_hp: None,
            max_hp: None,
            temp_hp: None,
            armor_class: None,
            conditions: vec![],
            condition_tracker: ConditionTracker::new(),
            condition_immunities: vec![],
            is_active: true,
            notes: String::new(),
        };

        self.add_combatant(session_id, combatant.clone())?;
        Ok(combatant)
    }

    pub fn remove_combatant(&self, session_id: &str, combatant_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        let pos = combat.combatants.iter()
            .position(|c| c.id == combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        // Adjust current turn if needed
        if combat.current_turn > pos && combat.current_turn > 0 {
            combat.current_turn -= 1;
        }

        combat.combatants.remove(pos);
        Ok(())
    }

    pub fn update_combatant(&self, session_id: &str, combatant: Combatant) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        let pos = combat.combatants.iter()
            .position(|c| c.id == combatant.id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant.id.clone()))?;

        let old_initiative = combat.combatants[pos].initiative;
        combat.combatants[pos] = combatant;

        // Re-sort if initiative changed
        if combat.combatants[pos].initiative != old_initiative {
            self.sort_initiative_internal(combat);
        }

        Ok(())
    }

    pub fn set_initiative(&self, session_id: &str, combatant_id: &str, initiative: i32) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        let combatant = combat.combatants.iter_mut()
            .find(|c| c.id == combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        combatant.initiative = initiative;
        self.sort_initiative_internal(combat);

        Ok(())
    }

    fn sort_initiative_internal(&self, combat: &mut CombatState) {
        // Sort by initiative (highest first), then by modifier as tiebreaker
        combat.combatants.sort_by(|a, b| {
            b.initiative.cmp(&a.initiative)
                .then_with(|| b.initiative_modifier.cmp(&a.initiative_modifier))
        });
    }

    pub fn next_turn(&self, session_id: &str) -> Result<Option<Combatant>> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        if combat.combatants.is_empty() {
            return Ok(None);
        }

        // Tick conditions at END of current combatant's turn (TASK-015)
        if let Some(current) = combat.combatants.get_mut(combat.current_turn) {
            // Legacy condition tick
            self.tick_conditions(&mut current.conditions);

            // Advanced condition tick - end of turn for the combatant whose turn is ending
            let expired = current.condition_tracker.tick_end_of_turn(true);

            // Log expired conditions
            for condition in expired {
                combat.events.push(CombatEvent {
                    round: combat.round,
                    turn: combat.current_turn,
                    timestamp: Utc::now(),
                    actor: current.name.clone(),
                    event_type: CombatEventType::ConditionRemoved,
                    description: format!("{} condition expired on {}", condition.name, current.name),
                });
            }
        }

        // Move to next active combatant
        let start = combat.current_turn;
        loop {
            combat.current_turn = (combat.current_turn + 1) % combat.combatants.len();

            // Check for new round
            if combat.current_turn == 0 {
                combat.round += 1;

                // Tick round-based conditions for all combatants at round end
                for combatant in &mut combat.combatants {
                    let expired = combatant.condition_tracker.tick_round();
                    for condition in expired {
                        combat.events.push(CombatEvent {
                            round: combat.round,
                            turn: 0,
                            timestamp: Utc::now(),
                            actor: combatant.name.clone(),
                            event_type: CombatEventType::ConditionRemoved,
                            description: format!("{} condition expired on {} (round end)", condition.name, combatant.name),
                        });
                    }
                }
            }

            // Tick start-of-turn conditions for the new current combatant
            if combat.combatants[combat.current_turn].is_active {
                let combatant = &mut combat.combatants[combat.current_turn];
                let expired = combatant.condition_tracker.tick_start_of_turn(true);
                for condition in expired {
                    combat.events.push(CombatEvent {
                        round: combat.round,
                        turn: combat.current_turn,
                        timestamp: Utc::now(),
                        actor: combatant.name.clone(),
                        event_type: CombatEventType::ConditionRemoved,
                        description: format!("{} condition expired on {} (start of turn)", condition.name, combatant.name),
                    });
                }
                return Ok(Some(combat.combatants[combat.current_turn].clone()));
            }

            // Full loop without finding active combatant
            if combat.current_turn == start {
                return Ok(None);
            }
        }
    }

    pub fn previous_turn(&self, session_id: &str) -> Result<Option<Combatant>> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        if combat.combatants.is_empty() {
            return Ok(None);
        }

        let start = combat.current_turn;
        loop {
            // Move backwards
            if combat.current_turn == 0 {
                if combat.round > 1 {
                    combat.round -= 1;
                    combat.current_turn = combat.combatants.len() - 1;
                } else {
                    return Ok(Some(combat.combatants[0].clone()));
                }
            } else {
                combat.current_turn -= 1;
            }

            if combat.combatants[combat.current_turn].is_active {
                return Ok(Some(combat.combatants[combat.current_turn].clone()));
            }

            if combat.current_turn == start {
                return Ok(None);
            }
        }
    }

    pub fn get_current_combatant(&self, session_id: &str) -> Option<Combatant> {
        self.sessions.read().unwrap()
            .get(session_id)
            .and_then(|s| s.combat.as_ref())
            .and_then(|c| c.combatants.get(c.current_turn).cloned())
    }

    // ========================================================================
    // HP Tracking
    // ========================================================================

    pub fn damage_combatant(
        &self,
        session_id: &str,
        combatant_id: &str,
        amount: i32,
    ) -> Result<i32> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        let combatant = combat.combatants.iter_mut()
            .find(|c| c.id == combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        let mut remaining = amount;

        // Damage temp HP first
        if let Some(temp) = combatant.temp_hp {
            if temp > 0 {
                if remaining >= temp {
                    remaining -= temp;
                    combatant.temp_hp = Some(0);
                } else {
                    combatant.temp_hp = Some(temp - remaining);
                    remaining = 0;
                }
            }
        }

        // Then damage current HP
        if let Some(current) = combatant.current_hp {
            combatant.current_hp = Some((current - remaining).max(0));
        }

        // Log the damage
        combat.events.push(CombatEvent {
            round: combat.round,
            turn: combat.current_turn,
            timestamp: Utc::now(),
            actor: combatant.name.clone(),
            event_type: CombatEventType::Damage,
            description: format!("{} takes {} damage", combatant.name, amount),
        });

        Ok(combatant.current_hp.unwrap_or(0))
    }

    pub fn heal_combatant(
        &self,
        session_id: &str,
        combatant_id: &str,
        amount: i32,
    ) -> Result<i32> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        let combatant = combat.combatants.iter_mut()
            .find(|c| c.id == combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        if let (Some(current), Some(max)) = (combatant.current_hp, combatant.max_hp) {
            combatant.current_hp = Some((current + amount).min(max));
        }

        combat.events.push(CombatEvent {
            round: combat.round,
            turn: combat.current_turn,
            timestamp: Utc::now(),
            actor: combatant.name.clone(),
            event_type: CombatEventType::Healing,
            description: format!("{} heals {} HP", combatant.name, amount),
        });

        Ok(combatant.current_hp.unwrap_or(0))
    }

    pub fn add_temp_hp(&self, session_id: &str, combatant_id: &str, amount: i32) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        let combatant = combat.combatants.iter_mut()
            .find(|c| c.id == combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        // Temp HP doesn't stack - use higher value
        let current_temp = combatant.temp_hp.unwrap_or(0);
        combatant.temp_hp = Some(current_temp.max(amount));

        Ok(())
    }

    // ========================================================================
    // Conditions
    // ========================================================================

    pub fn add_condition(
        &self,
        session_id: &str,
        combatant_id: &str,
        condition: Condition,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        let combatant = combat.combatants.iter_mut()
            .find(|c| c.id == combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        // Log the condition
        combat.events.push(CombatEvent {
            round: combat.round,
            turn: combat.current_turn,
            timestamp: Utc::now(),
            actor: combatant.name.clone(),
            event_type: CombatEventType::ConditionApplied,
            description: format!("{} gains condition: {}", combatant.name, condition.name),
        });

        combatant.conditions.push(condition);
        Ok(())
    }

    pub fn remove_condition(
        &self,
        session_id: &str,
        combatant_id: &str,
        condition_name: &str,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        let combatant = combat.combatants.iter_mut()
            .find(|c| c.id == combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        combatant.conditions.retain(|c| c.name != condition_name);

        combat.events.push(CombatEvent {
            round: combat.round,
            turn: combat.current_turn,
            timestamp: Utc::now(),
            actor: combatant.name.clone(),
            event_type: CombatEventType::ConditionRemoved,
            description: format!("{} loses condition: {}", combatant.name, condition_name),
        });

        Ok(())
    }

    fn tick_conditions(&self, conditions: &mut Vec<Condition>) {
        conditions.retain_mut(|condition| {
            match &mut condition.duration {
                Some(ConditionDuration::Rounds(rounds)) => {
                    if *rounds <= 1 {
                        return false;
                    }
                    *rounds -= 1;
                    true
                }
                Some(ConditionDuration::EndOfTurn) => false,
                _ => true,
            }
        });
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
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        let combatant = combat.combatants.iter_mut()
            .find(|c| c.id == combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        // Check for immunity
        if combatant.condition_immunities.iter()
            .any(|i| i.to_lowercase() == condition.name.to_lowercase())
        {
            combat.events.push(CombatEvent {
                round: combat.round,
                turn: combat.current_turn,
                timestamp: Utc::now(),
                actor: combatant.name.clone(),
                event_type: CombatEventType::ConditionApplied,
                description: format!("{} is immune to {}", combatant.name, condition.name),
            });
            return Ok(());
        }

        let condition_name = condition.name.clone();
        let combatant_name = combatant.name.clone();

        // Try to add the condition (respecting stacking rules)
        match combatant.condition_tracker.add_condition(condition) {
            Ok(()) => {
                combat.events.push(CombatEvent {
                    round: combat.round,
                    turn: combat.current_turn,
                    timestamp: Utc::now(),
                    actor: combatant_name,
                    event_type: CombatEventType::ConditionApplied,
                    description: format!("Gained condition: {}", condition_name),
                });
            }
            Err(msg) => {
                combat.events.push(CombatEvent {
                    round: combat.round,
                    turn: combat.current_turn,
                    timestamp: Utc::now(),
                    actor: combatant_name,
                    event_type: CombatEventType::Other,
                    description: format!("Condition not applied: {}", msg),
                });
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
        // Try to get a template condition
        let mut condition = ConditionTemplates::by_name(condition_name)
            .unwrap_or_else(|| {
                // Create a custom condition if not a standard one
                AdvancedCondition::new(
                    condition_name,
                    format!("Custom condition: {}", condition_name),
                    duration.clone().unwrap_or(AdvancedConditionDuration::UntilRemoved),
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
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        let combatant = combat.combatants.iter_mut()
            .find(|c| c.id == combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        if let Some(removed) = combatant.condition_tracker.remove_condition(condition_id) {
            combat.events.push(CombatEvent {
                round: combat.round,
                turn: combat.current_turn,
                timestamp: Utc::now(),
                actor: combatant.name.clone(),
                event_type: CombatEventType::ConditionRemoved,
                description: format!("{} loses condition: {}", combatant.name, removed.name),
            });
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
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        let combatant = combat.combatants.iter_mut()
            .find(|c| c.id == combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        let removed = combatant.condition_tracker.remove_by_name(condition_name);

        for condition in &removed {
            combat.events.push(CombatEvent {
                round: combat.round,
                turn: combat.current_turn,
                timestamp: Utc::now(),
                actor: combatant.name.clone(),
                event_type: CombatEventType::ConditionRemoved,
                description: format!("{} loses condition: {}", combatant.name, condition.name),
            });
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
        let session = sessions.get(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_ref()
            .ok_or(SessionError::NoCombatActive)?;

        let combatant = combat.combatants.iter()
            .find(|c| c.id == combatant_id)
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
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        let combatant = combat.combatants.iter_mut()
            .find(|c| c.id == combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        if !combatant.condition_immunities.contains(&condition_name.to_string()) {
            combatant.condition_immunities.push(condition_name.to_string());
        }

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
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        let combatant = combat.combatants.iter_mut()
            .find(|c| c.id == combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        combatant.condition_immunities.retain(|i| i != condition_name);

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
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        let combatant = combat.combatants.iter_mut()
            .find(|c| c.id == combatant_id)
            .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;

        let condition = combatant.condition_tracker.get_mut(condition_id);

        if let Some(cond) = condition {
            let success = cond.attempt_save(roll);

            if success {
                let cond_name = cond.name.clone();
                // Remove the condition on successful save
                combatant.condition_tracker.remove_condition(condition_id);

                combat.events.push(CombatEvent {
                    round: combat.round,
                    turn: combat.current_turn,
                    timestamp: Utc::now(),
                    actor: combatant.name.clone(),
                    event_type: CombatEventType::ConditionRemoved,
                    description: format!("{} saved against {} (roll: {})", combatant.name, cond_name, roll),
                });
            } else {
                combat.events.push(CombatEvent {
                    round: combat.round,
                    turn: combat.current_turn,
                    timestamp: Utc::now(),
                    actor: combatant.name.clone(),
                    event_type: CombatEventType::Other,
                    description: format!("{} failed save against {} (roll: {})", combatant.name, cond.name, roll),
                });
            }

            Ok(success)
        } else {
            Ok(false)
        }
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
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        combat.events.push(CombatEvent {
            round: combat.round,
            turn: combat.current_turn,
            timestamp: Utc::now(),
            actor: actor.to_string(),
            event_type,
            description: description.to_string(),
        });

        Ok(())
    }

    pub fn get_combat_log(&self, session_id: &str) -> Vec<CombatEvent> {
        self.sessions.read().unwrap()
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
        manager.update_note(note.clone())
            .map(|n| n.clone())
            .map_err(|e| SessionError::SessionNotFound(e))
    }

    /// Delete a note
    pub fn delete_note(&self, note_id: &str) -> Result<()> {
        let mut manager = self.notes_manager.write().unwrap();
        manager.delete_note(note_id)
            .ok_or_else(|| SessionError::SessionNotFound(note_id.to_string()))?;
        Ok(())
    }

    /// List all notes for a session
    pub fn list_notes_for_session(&self, session_id: &str) -> Vec<SessionNote> {
        let manager = self.notes_manager.read().unwrap();
        manager.notes_for_session(session_id)
            .into_iter()
            .cloned()
            .collect()
    }

    /// Search notes by query
    pub fn search_notes(&self, query: &str, session_id: Option<&str>) -> Vec<SessionNote> {
        let manager = self.notes_manager.read().unwrap();
        if let Some(sid) = session_id {
            manager.search_in_session(sid, query)
                .into_iter()
                .cloned()
                .collect()
        } else {
            manager.search(query)
                .into_iter()
                .cloned()
                .collect()
        }
    }

    /// Get notes by category
    pub fn get_notes_by_category(&self, category: &NoteCategory, session_id: Option<&str>) -> Vec<SessionNote> {
        let manager = self.notes_manager.read().unwrap();
        let notes = manager.notes_in_category(category);
        if let Some(sid) = session_id {
            notes.into_iter()
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
        manager.notes_with_tag(tag)
            .into_iter()
            .cloned()
            .collect()
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
            Err(SessionError::SessionNotFound(format!("Note not found: {}", note_id)))
        }
    }

    /// Unlink an entity from a note
    pub fn unlink_entity_from_note(&self, note_id: &str, entity_id: &str) -> Result<()> {
        let mut manager = self.notes_manager.write().unwrap();
        if let Some(note) = manager.get_note_mut(note_id) {
            note.unlink_entity(entity_id);
            Ok(())
        } else {
            Err(SessionError::SessionNotFound(format!("Note not found: {}", note_id)))
        }
    }

    // ========================================================================
    // TASK-015: Advanced Condition Management
    // ========================================================================

    /// Apply an advanced condition to a combatant
    pub fn apply_advanced_condition(
        &self,
        session_id: &str,
        combatant_id: &str,
        condition: AdvancedCondition,
    ) -> Result<()> {
        // Verify session and combatant exist
        {
            let sessions = self.sessions.read().unwrap();
            let session = sessions.get(session_id)
                .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
            let combat = session.combat.as_ref()
                .ok_or(SessionError::NoCombatActive)?;
            combat.combatants.iter()
                .find(|c| c.id == combatant_id)
                .ok_or_else(|| SessionError::CombatantNotFound(combatant_id.to_string()))?;
        }

        let mut trackers = self.condition_trackers.write().unwrap();
        let tracker = trackers.entry(combatant_id.to_string())
            .or_insert_with(ConditionTracker::new);
        let _ = tracker.add_condition(condition);
        Ok(())
    }



    /// Tick conditions at end of combatant's turn
    pub fn tick_conditions_end_of_turn(&self, session_id: &str, combatant_id: &str) -> Result<Vec<String>> {
        // Verify session exists
        {
            let sessions = self.sessions.read().unwrap();
            sessions.get(session_id)
                .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        }

        let mut trackers = self.condition_trackers.write().unwrap();
        if let Some(tracker) = trackers.get_mut(combatant_id) {
            let expired = tracker.tick_end_of_turn(true);
            Ok(expired.into_iter().map(|c| c.name).collect())
        } else {
            Ok(vec![])
        }
    }

    /// Tick conditions at start of combatant's turn
    pub fn tick_conditions_start_of_turn(&self, session_id: &str, combatant_id: &str) -> Result<Vec<String>> {
        // Verify session exists
        {
            let sessions = self.sessions.read().unwrap();
            sessions.get(session_id)
                .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        }

        let mut trackers = self.condition_trackers.write().unwrap();
        if let Some(tracker) = trackers.get_mut(combatant_id) {
            let expired = tracker.tick_start_of_turn(true);
            Ok(expired.into_iter().map(|c| c.name).collect())
        } else {
            Ok(vec![])
        }
    }
}

// ============================================================================
// Common Conditions (D&D 5e style)
// ============================================================================

pub fn create_common_condition(name: &str) -> Option<Condition> {
    match name.to_lowercase().as_str() {
        "blinded" => Some(Condition {
            name: "Blinded".to_string(),
            duration: None,
            source: None,
            effects: vec![
                "Can't see, auto-fails sight checks".to_string(),
                "Attack rolls have disadvantage".to_string(),
                "Attacks against have advantage".to_string(),
            ],
        }),
        "charmed" => Some(Condition {
            name: "Charmed".to_string(),
            duration: None,
            source: None,
            effects: vec![
                "Can't attack charmer".to_string(),
                "Charmer has advantage on social checks".to_string(),
            ],
        }),
        "frightened" => Some(Condition {
            name: "Frightened".to_string(),
            duration: None,
            source: None,
            effects: vec![
                "Disadvantage on ability checks and attacks while source visible".to_string(),
                "Can't willingly move closer to source".to_string(),
            ],
        }),
        "grappled" => Some(Condition {
            name: "Grappled".to_string(),
            duration: Some(ConditionDuration::UntilRemoved),
            source: None,
            effects: vec![
                "Speed becomes 0".to_string(),
            ],
        }),
        "incapacitated" => Some(Condition {
            name: "Incapacitated".to_string(),
            duration: None,
            source: None,
            effects: vec![
                "Can't take actions or reactions".to_string(),
            ],
        }),
        "invisible" => Some(Condition {
            name: "Invisible".to_string(),
            duration: None,
            source: None,
            effects: vec![
                "Impossible to see without special sense".to_string(),
                "Attack rolls have advantage".to_string(),
                "Attacks against have disadvantage".to_string(),
            ],
        }),
        "paralyzed" => Some(Condition {
            name: "Paralyzed".to_string(),
            duration: None,
            source: None,
            effects: vec![
                "Incapacitated, can't move or speak".to_string(),
                "Auto-fails STR/DEX saves".to_string(),
                "Attacks against have advantage".to_string(),
                "Hits within 5ft are critical".to_string(),
            ],
        }),
        "poisoned" => Some(Condition {
            name: "Poisoned".to_string(),
            duration: None,
            source: None,
            effects: vec![
                "Disadvantage on attack rolls and ability checks".to_string(),
            ],
        }),
        "prone" => Some(Condition {
            name: "Prone".to_string(),
            duration: Some(ConditionDuration::UntilRemoved),
            source: None,
            effects: vec![
                "Can only crawl".to_string(),
                "Disadvantage on attack rolls".to_string(),
                "Attacks within 5ft have advantage, else disadvantage".to_string(),
            ],
        }),
        "restrained" => Some(Condition {
            name: "Restrained".to_string(),
            duration: None,
            source: None,
            effects: vec![
                "Speed becomes 0".to_string(),
                "Attacks have disadvantage".to_string(),
                "Attacks against have advantage".to_string(),
                "Disadvantage on DEX saves".to_string(),
            ],
        }),
        "stunned" => Some(Condition {
            name: "Stunned".to_string(),
            duration: None,
            source: None,
            effects: vec![
                "Incapacitated, can't move".to_string(),
                "Can only speak falteringly".to_string(),
                "Auto-fails STR/DEX saves".to_string(),
                "Attacks against have advantage".to_string(),
            ],
        }),
        "unconscious" => Some(Condition {
            name: "Unconscious".to_string(),
            duration: None,
            source: None,
            effects: vec![
                "Incapacitated, can't move or speak".to_string(),
                "Unaware of surroundings".to_string(),
                "Drops held items, falls prone".to_string(),
                "Auto-fails STR/DEX saves".to_string(),
                "Attacks against have advantage".to_string(),
                "Hits within 5ft are critical".to_string(),
            ],
        }),
        "concentrating" => Some(Condition {
            name: "Concentrating".to_string(),
            duration: None,
            source: None,
            effects: vec![
                "Maintaining concentration on a spell".to_string(),
                "CON save on damage or lose concentration".to_string(),
            ],
        }),
        _ => None,
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
            .map(|t| t.events_by_severity(min_severity).into_iter().cloned().collect())
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
        timelines
            .get(session_id)
            .map(|t| t.generate_narrative())
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
    pub fn log_session_event(&self, session_id: &str, event_type: TimelineEventType, title: &str, description: &str) -> Result<()> {
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
        let event = TimelineEvent::new(session_id, event_type, title, description)
            .with_severity(severity);
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
        manager.add_combatant_quick(&session.id, "Fighter", 18, CombatantType::Player).unwrap();
        manager.add_combatant_quick(&session.id, "Wizard", 12, CombatantType::Player).unwrap();
        manager.add_combatant_quick(&session.id, "Goblin", 15, CombatantType::Monster).unwrap();

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

        let combatant = Combatant {
            id: Uuid::new_v4().to_string(),
            name: "Fighter".to_string(),
            initiative: 15,
            initiative_modifier: 2,
            combatant_type: CombatantType::Player,
            current_hp: Some(50),
            max_hp: Some(50),
            temp_hp: Some(10),
            armor_class: Some(18),
            conditions: vec![],
            condition_tracker: ConditionTracker::default(),
            condition_immunities: vec![],
            is_active: true,
            notes: String::new(),
        };

        manager.add_combatant(&session.id, combatant.clone()).unwrap();

        // Damage should hit temp HP first
        let remaining = manager.damage_combatant(&session.id, &combatant.id, 15).unwrap();
        assert_eq!(remaining, 45); // 10 temp absorbed, 5 from HP

        // Heal
        let healed = manager.heal_combatant(&session.id, &combatant.id, 10).unwrap();
        assert_eq!(healed, 50); // Back to max
    }

    #[test]
    fn test_conditions() {
        let manager = SessionManager::new();
        let session = manager.start_session("campaign-1", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = manager.add_combatant_quick(
            &session.id,
            "Fighter",
            15,
            CombatantType::Player,
        ).unwrap();

        // Add condition
        let stunned = create_common_condition("stunned").unwrap();
        manager.add_condition(&session.id, &combatant.id, stunned).unwrap();

        let combat = manager.get_combat(&session.id).unwrap();
        assert_eq!(combat.combatants[0].conditions.len(), 1);

        // Remove condition
        manager.remove_condition(&session.id, &combatant.id, "Stunned").unwrap();

        let combat = manager.get_combat(&session.id).unwrap();
        assert_eq!(combat.combatants[0].conditions.len(), 0);
    }
}
