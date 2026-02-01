//! Combat Management Module
//!
//! Extracted from session_manager.rs to provide better cohesion.
//! Contains combat state, combatant tracking, initiative management,
//! HP tracking, and combat event logging.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::conditions::ConditionTracker;

// ============================================================================
// Combat Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum CombatStatus {
    #[default]
    Active,
    Paused,
    Ended,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatEvent {
    pub round: u32,
    pub turn: usize,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub event_type: CombatEventType,
    pub description: String,
}

// ============================================================================
// Combatant
// ============================================================================

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
    /// Advanced condition tracker with full duration/stacking support (TASK-015)
    #[serde(default)]
    pub condition_tracker: ConditionTracker,
    /// Condition immunities (e.g., "Frightened", "Poisoned")
    #[serde(default)]
    pub condition_immunities: Vec<String>,
    pub is_active: bool,
    pub notes: String,
}

impl Combatant {
    /// Create a new combatant with minimal information
    pub fn new(name: impl Into<String>, initiative: i32, combatant_type: CombatantType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            initiative,
            initiative_modifier: 0,
            combatant_type,
            current_hp: None,
            max_hp: None,
            temp_hp: None,
            armor_class: None,
            condition_tracker: ConditionTracker::new(),
            condition_immunities: vec![],
            is_active: true,
            notes: String::new(),
        }
    }

    /// Apply damage to this combatant
    /// Damages temp HP first, then current HP
    /// Returns the new current HP value
    /// Non-positive damage amounts are ignored
    pub fn apply_damage(&mut self, amount: i32) -> i32 {
        // Ignore non-positive damage (prevents negative damage from healing)
        if amount <= 0 {
            return self.current_hp.unwrap_or(0);
        }

        let mut remaining = amount;

        // Damage temp HP first
        if let Some(temp) = self.temp_hp {
            if temp > 0 {
                if remaining >= temp {
                    remaining -= temp;
                    self.temp_hp = Some(0);
                } else {
                    self.temp_hp = Some(temp - remaining);
                    remaining = 0;
                }
            }
        }

        // Then damage current HP
        if let Some(current) = self.current_hp {
            self.current_hp = Some((current - remaining).max(0));
        }

        self.current_hp.unwrap_or(0)
    }

    /// Heal this combatant
    /// Cannot exceed max HP
    /// Returns the new current HP value
    pub fn heal(&mut self, amount: i32) -> i32 {
        if amount <= 0 {
            return self.current_hp.unwrap_or(0);
        }
        if let (Some(current), Some(max)) = (self.current_hp, self.max_hp) {
            self.current_hp = Some((current + amount).min(max));
        }
        self.current_hp.unwrap_or(0)
    }

    /// Add temporary hit points
    /// Temp HP doesn't stack - uses the higher value
    pub fn add_temp_hp(&mut self, amount: i32) {
        let current_temp = self.temp_hp.unwrap_or(0);
        self.temp_hp = Some(current_temp.max(amount));
    }

    /// Check if this combatant is immune to a condition
    pub fn is_immune_to(&self, condition_name: &str) -> bool {
        self.condition_immunities
            .iter()
            .any(|i| i.to_lowercase() == condition_name.to_lowercase())
    }

    /// Add a condition immunity (normalized to lowercase for consistent matching)
    pub fn add_immunity(&mut self, condition_name: impl Into<String>) {
        let name = condition_name.into().to_lowercase();
        if !self.condition_immunities.iter().any(|i| i.to_lowercase() == name) {
            self.condition_immunities.push(name);
        }
    }

    /// Remove a condition immunity (case-insensitive)
    pub fn remove_immunity(&mut self, condition_name: &str) {
        let target = condition_name.to_lowercase();
        self.condition_immunities.retain(|i| i.to_lowercase() != target);
    }
}

// ============================================================================
// Combat State
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

/// Result of advancing a turn, containing the new current combatant
/// and any conditions that expired during the transition
pub struct TurnResult {
    pub current_combatant: Option<Combatant>,
    pub new_round: bool,
    pub expired_conditions: Vec<(String, String)>, // (combatant_name, condition_name)
}

impl CombatState {
    /// Create a new combat state
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            round: 1,
            current_turn: 0,
            combatants: vec![],
            started_at: Utc::now(),
            status: CombatStatus::Active,
            events: vec![],
        }
    }

    /// Sort combatants by initiative (highest first)
    /// Uses initiative modifier as tiebreaker
    pub fn sort_initiative(&mut self) {
        self.combatants.sort_by(|a, b| {
            b.initiative
                .cmp(&a.initiative)
                .then_with(|| b.initiative_modifier.cmp(&a.initiative_modifier))
        });
    }

    /// Add a combatant and re-sort initiative
    pub fn add_combatant(&mut self, combatant: Combatant) {
        self.combatants.push(combatant);
        self.sort_initiative();
    }

    /// Remove a combatant by ID
    /// Adjusts current_turn if needed
    /// Returns the removed combatant if found
    pub fn remove_combatant(&mut self, combatant_id: &str) -> Option<Combatant> {
        let pos = self.combatants.iter().position(|c| c.id == combatant_id)?;

        let removed = self.combatants.remove(pos);

        // Adjust current turn after removal:
        // - If removed before current, decrement to keep pointing at same combatant
        // - If removed at current position, keep index (now points to next combatant)
        // - Clamp to valid range in case we removed the last combatant
        if pos < self.current_turn && self.current_turn > 0 {
            self.current_turn -= 1;
        }
        self.current_turn = self.current_turn.min(self.combatants.len().saturating_sub(1));

        Some(removed)
    }

    /// Get the current combatant
    pub fn current_combatant(&self) -> Option<&Combatant> {
        self.combatants.get(self.current_turn)
    }

    /// Get mutable reference to the current combatant
    pub fn current_combatant_mut(&mut self) -> Option<&mut Combatant> {
        self.combatants.get_mut(self.current_turn)
    }

    /// Get a combatant by ID
    pub fn get_combatant(&self, combatant_id: &str) -> Option<&Combatant> {
        self.combatants.iter().find(|c| c.id == combatant_id)
    }

    /// Get mutable reference to a combatant by ID
    pub fn get_combatant_mut(&mut self, combatant_id: &str) -> Option<&mut Combatant> {
        self.combatants.iter_mut().find(|c| c.id == combatant_id)
    }

    /// Advance to the next turn
    /// Handles end-of-turn condition ticking, round advancement,
    /// and start-of-turn condition ticking
    /// Returns the new current combatant and any expired conditions
    pub fn next_turn(&mut self) -> TurnResult {
        if self.combatants.is_empty() {
            return TurnResult {
                current_combatant: None,
                new_round: false,
                expired_conditions: vec![],
            };
        }

        let mut expired_conditions = Vec::new();

        // Tick conditions at END of current combatant's turn
        if let Some(current) = self.combatants.get_mut(self.current_turn) {
            let expired = current.condition_tracker.tick_end_of_turn(true);
            for condition in expired {
                self.events.push(CombatEvent {
                    round: self.round,
                    turn: self.current_turn,
                    timestamp: Utc::now(),
                    actor: current.name.clone(),
                    event_type: CombatEventType::ConditionRemoved,
                    description: format!("{} condition expired on {}", condition.name, current.name),
                });
                expired_conditions.push((current.name.clone(), condition.name));
            }
        }

        // Move to next active combatant
        let start = self.current_turn;
        let mut new_round = false;

        loop {
            self.current_turn = (self.current_turn + 1) % self.combatants.len();

            // Check for new round
            if self.current_turn == 0 {
                self.round += 1;
                new_round = true;

                // Tick round-based conditions for all combatants
                for combatant in &mut self.combatants {
                    let expired = combatant.condition_tracker.tick_round();
                    for condition in expired {
                        self.events.push(CombatEvent {
                            round: self.round,
                            turn: 0,
                            timestamp: Utc::now(),
                            actor: combatant.name.clone(),
                            event_type: CombatEventType::ConditionRemoved,
                            description: format!(
                                "{} condition expired on {} (round end)",
                                condition.name, combatant.name
                            ),
                        });
                        expired_conditions.push((combatant.name.clone(), condition.name));
                    }
                }
            }

            // Tick start-of-turn conditions for the new current combatant
            if self.combatants[self.current_turn].is_active {
                let combatant = &mut self.combatants[self.current_turn];
                let expired = combatant.condition_tracker.tick_start_of_turn(true);
                for condition in expired {
                    self.events.push(CombatEvent {
                        round: self.round,
                        turn: self.current_turn,
                        timestamp: Utc::now(),
                        actor: combatant.name.clone(),
                        event_type: CombatEventType::ConditionRemoved,
                        description: format!(
                            "{} condition expired on {} (start of turn)",
                            condition.name, combatant.name
                        ),
                    });
                    expired_conditions.push((combatant.name.clone(), condition.name));
                }
                return TurnResult {
                    current_combatant: Some(self.combatants[self.current_turn].clone()),
                    new_round,
                    expired_conditions,
                };
            }

            // Full loop without finding active combatant
            if self.current_turn == start {
                return TurnResult {
                    current_combatant: None,
                    new_round,
                    expired_conditions,
                };
            }
        }
    }

    /// Go back to the previous turn
    /// Returns the new current combatant
    pub fn previous_turn(&mut self) -> Option<Combatant> {
        if self.combatants.is_empty() {
            return None;
        }

        let start = self.current_turn;
        loop {
            // Move backwards
            if self.current_turn == 0 {
                if self.round > 1 {
                    self.round -= 1;
                    self.current_turn = self.combatants.len() - 1;
                } else {
                    return Some(self.combatants[0].clone());
                }
            } else {
                self.current_turn -= 1;
            }

            if self.combatants[self.current_turn].is_active {
                return Some(self.combatants[self.current_turn].clone());
            }

            if self.current_turn == start {
                return None;
            }
        }
    }

    /// Log a combat event
    pub fn log_event(&mut self, actor: impl Into<String>, event_type: CombatEventType, description: impl Into<String>) {
        self.events.push(CombatEvent {
            round: self.round,
            turn: self.current_turn,
            timestamp: Utc::now(),
            actor: actor.into(),
            event_type,
            description: description.into(),
        });
    }

    /// End the combat
    pub fn end(&mut self) {
        self.status = CombatStatus::Ended;
    }

    /// Pause the combat
    pub fn pause(&mut self) {
        self.status = CombatStatus::Paused;
    }

    /// Resume the combat
    pub fn resume(&mut self) {
        self.status = CombatStatus::Active;
    }
}

impl Default for CombatState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combatant_damage() {
        let mut combatant = Combatant::new("Fighter", 15, CombatantType::Player);
        combatant.current_hp = Some(50);
        combatant.max_hp = Some(50);
        combatant.temp_hp = Some(10);

        // Should hit temp HP first
        let remaining = combatant.apply_damage(15);
        assert_eq!(remaining, 45); // 10 temp absorbed, 5 from HP
        assert_eq!(combatant.temp_hp, Some(0));
        assert_eq!(combatant.current_hp, Some(45));
    }

    #[test]
    fn test_combatant_heal() {
        let mut combatant = Combatant::new("Fighter", 15, CombatantType::Player);
        combatant.current_hp = Some(30);
        combatant.max_hp = Some(50);

        let healed = combatant.heal(100);
        assert_eq!(healed, 50); // Capped at max
    }

    #[test]
    fn test_initiative_sorting() {
        let mut combat = CombatState::new();
        combat.combatants.push(Combatant::new("Wizard", 12, CombatantType::Player));
        combat.combatants.push(Combatant::new("Fighter", 18, CombatantType::Player));
        combat.combatants.push(Combatant::new("Goblin", 15, CombatantType::Monster));

        combat.sort_initiative();

        assert_eq!(combat.combatants[0].name, "Fighter");
        assert_eq!(combat.combatants[1].name, "Goblin");
        assert_eq!(combat.combatants[2].name, "Wizard");
    }

    #[test]
    fn test_turn_advancement() {
        let mut combat = CombatState::new();
        combat.add_combatant(Combatant::new("Fighter", 18, CombatantType::Player));
        combat.add_combatant(Combatant::new("Goblin", 15, CombatantType::Monster));
        combat.add_combatant(Combatant::new("Wizard", 12, CombatantType::Player));

        assert_eq!(combat.current_combatant().unwrap().name, "Fighter");
        assert_eq!(combat.round, 1);

        let result = combat.next_turn();
        assert_eq!(result.current_combatant.unwrap().name, "Goblin");
        assert!(!result.new_round);

        let result = combat.next_turn();
        assert_eq!(result.current_combatant.unwrap().name, "Wizard");
        assert!(!result.new_round);

        let result = combat.next_turn();
        assert_eq!(result.current_combatant.unwrap().name, "Fighter");
        assert!(result.new_round);
        assert_eq!(combat.round, 2);
    }

    #[test]
    fn test_remove_combatant_adjusts_turn() {
        let mut combat = CombatState::new();
        let fighter = Combatant::new("Fighter", 18, CombatantType::Player);
        let fighter_id = fighter.id.clone();
        combat.add_combatant(fighter);
        combat.add_combatant(Combatant::new("Goblin", 15, CombatantType::Monster));
        combat.add_combatant(Combatant::new("Wizard", 12, CombatantType::Player));

        // Advance to Goblin's turn
        combat.next_turn();
        assert_eq!(combat.current_turn, 1);

        // Remove Fighter (before current turn)
        combat.remove_combatant(&fighter_id);

        // current_turn should adjust
        assert_eq!(combat.current_turn, 0);
        assert_eq!(combat.current_combatant().unwrap().name, "Goblin");
    }
}
