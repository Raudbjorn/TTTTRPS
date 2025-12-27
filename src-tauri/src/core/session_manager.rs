//! Session Manager Module
//!
//! Handles live game session state including initiative tracking,
//! combat management, and real-time session notes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use thiserror::Error;

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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum SessionStatus {
    #[default]
    Active,
    Paused,
    Ended,
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
    pub conditions: Vec<Condition>,
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
}

// ============================================================================
// Session Manager
// ============================================================================

pub struct SessionManager {
    sessions: RwLock<HashMap<String, GameSession>>,
    campaign_sessions: RwLock<HashMap<String, Vec<String>>>,
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

        session
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
                ids.iter()
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
                    })
                    .collect()
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
        };

        Ok(summary)
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
        Ok(combat)
    }

    pub fn end_combat(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;

        let combat = session.combat.as_mut()
            .ok_or(SessionError::NoCombatActive)?;

        combat.status = CombatStatus::Ended;
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

        // Decrement round-based conditions for current combatant
        if let Some(current) = combat.combatants.get_mut(combat.current_turn) {
            self.tick_conditions(&mut current.conditions);
        }

        // Move to next active combatant
        let start = combat.current_turn;
        loop {
            combat.current_turn = (combat.current_turn + 1) % combat.combatants.len();

            // Check for new round
            if combat.current_turn == 0 {
                combat.round += 1;
            }

            // Found active combatant
            if combat.combatants[combat.current_turn].is_active {
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
