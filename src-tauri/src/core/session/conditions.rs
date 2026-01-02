//! Advanced Condition System (TASK-015)
//!
//! Provides duration tracking, auto-removal, custom condition building,
//! effect descriptions, and stacking rules for game conditions.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

// ============================================================================
// Duration Types
// ============================================================================

/// How condition duration is measured
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ConditionDuration {
    /// Lasts for a number of turns (combatant turns)
    Turns(u32),
    /// Lasts for a number of rounds (full initiative cycles)
    Rounds(u32),
    /// Lasts for a number of real-time minutes
    Minutes(u32),
    /// Lasts for a number of real-time hours
    Hours(u32),
    /// Ends at the end of the affected creature's next turn
    EndOfNextTurn,
    /// Ends at the start of the affected creature's next turn
    StartOfNextTurn,
    /// Ends at the end of the source's next turn
    EndOfSourceTurn,
    /// Ends when a saving throw is made (optionally at start/end of turn)
    UntilSave {
        save_type: String,
        dc: u32,
        timing: SaveTiming,
    },
    /// Ends when explicitly removed
    UntilRemoved,
    /// Ends when a specific trigger occurs
    UntilTrigger(String),
    /// Permanent until dispelled
    Permanent,
    /// Custom duration with description
    Custom(String),
}

/// When saves are attempted
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SaveTiming {
    #[default]
    EndOfTurn,
    StartOfTurn,
    OnDamage,
    OnAction,
}

impl ConditionDuration {
    /// Human-readable description of the duration
    pub fn description(&self) -> String {
        match self {
            Self::Turns(n) => format!("{} turn{}", n, if *n == 1 { "" } else { "s" }),
            Self::Rounds(n) => format!("{} round{}", n, if *n == 1 { "" } else { "s" }),
            Self::Minutes(n) => format!("{} minute{}", n, if *n == 1 { "" } else { "s" }),
            Self::Hours(n) => format!("{} hour{}", n, if *n == 1 { "" } else { "s" }),
            Self::EndOfNextTurn => "End of next turn".to_string(),
            Self::StartOfNextTurn => "Start of next turn".to_string(),
            Self::EndOfSourceTurn => "End of source's next turn".to_string(),
            Self::UntilSave { save_type, dc, timing } => {
                format!("DC {} {} save ({:?})", dc, save_type, timing)
            }
            Self::UntilRemoved => "Until removed".to_string(),
            Self::UntilTrigger(trigger) => format!("Until: {}", trigger),
            Self::Permanent => "Permanent".to_string(),
            Self::Custom(desc) => desc.clone(),
        }
    }

    /// Whether this duration expires with time
    pub fn is_timed(&self) -> bool {
        matches!(
            self,
            Self::Turns(_) | Self::Rounds(_) | Self::Minutes(_) | Self::Hours(_) |
            Self::EndOfNextTurn | Self::StartOfNextTurn | Self::EndOfSourceTurn
        )
    }
}

// ============================================================================
// Condition Effects
// ============================================================================

/// Type of mechanical effect a condition has
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConditionEffect {
    /// Advantage on certain rolls
    Advantage {
        applies_to: Vec<String>,
    },
    /// Disadvantage on certain rolls
    Disadvantage {
        applies_to: Vec<String>,
    },
    /// Modifier to rolls or stats
    Modifier {
        stat: String,
        value: i32,
    },
    /// Speed modification
    SpeedChange {
        multiplier: Option<f32>,
        flat_change: Option<i32>,
    },
    /// Cannot take certain action types
    ActionRestriction {
        restricted: Vec<String>,
    },
    /// Auto-fail certain saves
    AutoFail {
        save_types: Vec<String>,
    },
    /// Attacks against this target have advantage
    GrantAdvantage,
    /// Attacks against this target have disadvantage
    GrantDisadvantage,
    /// Damage vulnerability
    Vulnerability {
        damage_types: Vec<String>,
    },
    /// Damage resistance
    Resistance {
        damage_types: Vec<String>,
    },
    /// Damage immunity
    Immunity {
        damage_types: Vec<String>,
    },
    /// Recurring damage (e.g., ongoing fire)
    RecurringDamage {
        dice: String,
        damage_type: String,
        timing: SaveTiming,
    },
    /// Recurring healing
    RecurringHealing {
        dice: String,
        timing: SaveTiming,
    },
    /// Custom effect with description
    Custom {
        description: String,
    },
}

impl ConditionEffect {
    /// Human-readable description
    pub fn description(&self) -> String {
        match self {
            Self::Advantage { applies_to } => {
                format!("Advantage on: {}", applies_to.join(", "))
            }
            Self::Disadvantage { applies_to } => {
                format!("Disadvantage on: {}", applies_to.join(", "))
            }
            Self::Modifier { stat, value } => {
                let sign = if *value >= 0 { "+" } else { "" };
                format!("{}{} to {}", sign, value, stat)
            }
            Self::SpeedChange { multiplier, flat_change } => {
                let mut parts = Vec::new();
                if let Some(m) = multiplier {
                    parts.push(format!("Speed x{}", m));
                }
                if let Some(f) = flat_change {
                    let sign = if *f >= 0 { "+" } else { "" };
                    parts.push(format!("Speed {}{} ft", sign, f));
                }
                parts.join(", ")
            }
            Self::ActionRestriction { restricted } => {
                format!("Cannot: {}", restricted.join(", "))
            }
            Self::AutoFail { save_types } => {
                format!("Auto-fail {} saves", save_types.join(", "))
            }
            Self::GrantAdvantage => "Attacks against have advantage".to_string(),
            Self::GrantDisadvantage => "Attacks against have disadvantage".to_string(),
            Self::Vulnerability { damage_types } => {
                format!("Vulnerable to: {}", damage_types.join(", "))
            }
            Self::Resistance { damage_types } => {
                format!("Resistant to: {}", damage_types.join(", "))
            }
            Self::Immunity { damage_types } => {
                format!("Immune to: {}", damage_types.join(", "))
            }
            Self::RecurringDamage { dice, damage_type, timing } => {
                format!("{} {} damage ({:?})", dice, damage_type, timing)
            }
            Self::RecurringHealing { dice, timing } => {
                format!("{} healing ({:?})", dice, timing)
            }
            Self::Custom { description } => description.clone(),
        }
    }
}

// ============================================================================
// Stacking Rules
// ============================================================================

/// How conditions of the same type interact
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StackingRule {
    /// Only the strongest instance applies
    #[default]
    Highest,
    /// Only the most recent instance applies
    Latest,
    /// Effects stack/add together
    Stack,
    /// Extends duration instead of adding new instance
    ExtendDuration,
    /// Cannot have multiple instances
    NoStack,
    /// Each source can apply one instance
    PerSource,
    /// Custom stacking logic
    Custom(String),
}

// ============================================================================
// Advanced Condition
// ============================================================================

/// An advanced condition with full tracking capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedCondition {
    /// Unique identifier
    pub id: String,
    /// Condition name
    pub name: String,
    /// Detailed description
    pub description: String,
    /// Duration tracking
    pub duration: ConditionDuration,
    /// Remaining duration (for countable durations)
    pub remaining: Option<u32>,
    /// Source of the condition (who applied it)
    pub source_id: Option<String>,
    /// Source name for display
    pub source_name: Option<String>,
    /// Mechanical effects
    pub effects: Vec<ConditionEffect>,
    /// How this condition stacks
    pub stacking: StackingRule,
    /// Condition tags for grouping (e.g., "magical", "curse", "disease")
    pub tags: Vec<String>,
    /// Whether this is a beneficial condition
    pub is_beneficial: bool,
    /// Icon/symbol identifier
    pub icon: Option<String>,
    /// Color for UI display
    pub color: Option<String>,
    /// Custom notes
    pub notes: String,
    /// Track number of times save was attempted
    pub save_attempts: u32,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AdvancedCondition {
    /// Create a new condition
    pub fn new(name: impl Into<String>, description: impl Into<String>, duration: ConditionDuration) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: description.into(),
            duration: duration.clone(),
            remaining: match &duration {
                ConditionDuration::Turns(n) => Some(*n),
                ConditionDuration::Rounds(n) => Some(*n),
                ConditionDuration::Minutes(n) => Some(*n),
                ConditionDuration::Hours(n) => Some(*n),
                _ => None,
            },
            source_id: None,
            source_name: None,
            effects: Vec::new(),
            stacking: StackingRule::default(),
            tags: Vec::new(),
            is_beneficial: false,
            icon: None,
            color: None,
            notes: String::new(),
            save_attempts: 0,
            metadata: HashMap::new(),
        }
    }

    /// Builder: set source
    pub fn from_source(mut self, source_id: impl Into<String>, source_name: impl Into<String>) -> Self {
        self.source_id = Some(source_id.into());
        self.source_name = Some(source_name.into());
        self
    }

    /// Builder: add effect
    pub fn with_effect(mut self, effect: ConditionEffect) -> Self {
        self.effects.push(effect);
        self
    }

    /// Builder: set stacking rule
    pub fn with_stacking(mut self, stacking: StackingRule) -> Self {
        self.stacking = stacking;
        self
    }

    /// Builder: add tags
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(|t| t.into()));
        self
    }

    /// Builder: mark as beneficial
    pub fn beneficial(mut self) -> Self {
        self.is_beneficial = true;
        self
    }

    /// Builder: set icon
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Builder: set color
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Tick the condition at end of turn
    /// Returns true if the condition should be removed
    pub fn tick_end_of_turn(&mut self, is_own_turn: bool) -> bool {
        match &self.duration {
            ConditionDuration::EndOfNextTurn if is_own_turn => true,
            ConditionDuration::Turns(n) if is_own_turn => {
                if let Some(ref mut remaining) = self.remaining {
                    if *remaining <= 1 {
                        return true;
                    }
                    *remaining -= 1;
                }
                false
            }
            _ => false,
        }
    }

    /// Tick the condition at start of turn
    /// Returns true if the condition should be removed
    pub fn tick_start_of_turn(&mut self, is_own_turn: bool) -> bool {
        match &self.duration {
            ConditionDuration::StartOfNextTurn if is_own_turn => true,
            _ => false,
        }
    }

    /// Tick for a full round
    /// Returns true if the condition should be removed
    pub fn tick_round(&mut self) -> bool {
        if let ConditionDuration::Rounds(_) = &self.duration {
            if let Some(ref mut remaining) = self.remaining {
                if *remaining <= 1 {
                    return true;
                }
                *remaining -= 1;
            }
        }
        false
    }

    /// Tick for time passage (in minutes)
    /// Returns true if the condition should be removed
    pub fn tick_time(&mut self, minutes: u32) -> bool {
        match &self.duration {
            ConditionDuration::Minutes(_) => {
                if let Some(ref mut remaining) = self.remaining {
                    if *remaining <= minutes {
                        return true;
                    }
                    *remaining -= minutes;
                }
                false
            }
            ConditionDuration::Hours(_) => {
                // Convert hours to minutes for remaining
                if let Some(ref mut remaining) = self.remaining {
                    let remaining_minutes = *remaining * 60;
                    if remaining_minutes <= minutes {
                        return true;
                    }
                    *remaining = (remaining_minutes - minutes) / 60;
                    if *remaining == 0 && (remaining_minutes - minutes) % 60 > 0 {
                        *remaining = 1; // Less than an hour left
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Attempt a saving throw against this condition
    /// Returns true if the save succeeded (condition should be removed if UntilSave)
    pub fn attempt_save(&mut self, roll: i32) -> bool {
        self.save_attempts += 1;
        if let ConditionDuration::UntilSave { dc, .. } = &self.duration {
            roll >= *dc as i32
        } else {
            false
        }
    }

    /// Check if this condition has expired
    pub fn is_expired(&self) -> bool {
        if let Some(remaining) = self.remaining {
            remaining == 0
        } else {
            false
        }
    }

    /// Get remaining duration as string
    pub fn remaining_text(&self) -> String {
        match &self.duration {
            ConditionDuration::Turns(_) => {
                self.remaining.map(|r| format!("{} turn(s)", r)).unwrap_or_default()
            }
            ConditionDuration::Rounds(_) => {
                self.remaining.map(|r| format!("{} round(s)", r)).unwrap_or_default()
            }
            ConditionDuration::Minutes(_) => {
                self.remaining.map(|r| format!("{} min", r)).unwrap_or_default()
            }
            ConditionDuration::Hours(_) => {
                self.remaining.map(|r| format!("{} hr", r)).unwrap_or_default()
            }
            _ => self.duration.description(),
        }
    }

    /// Get all effect descriptions
    pub fn effect_descriptions(&self) -> Vec<String> {
        self.effects.iter().map(|e| e.description()).collect()
    }
}

// ============================================================================
// Condition Manager
// ============================================================================

/// Manages conditions on a single combatant
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConditionTracker {
    /// Active conditions
    conditions: Vec<AdvancedCondition>,
}

impl ConditionTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a condition, respecting stacking rules
    pub fn add_condition(&mut self, condition: AdvancedCondition) -> Result<(), String> {
        // Check for existing conditions with same name
        let existing: Vec<&AdvancedCondition> = self.conditions
            .iter()
            .filter(|c| c.name == condition.name)
            .collect();

        if !existing.is_empty() {
            match &condition.stacking {
                StackingRule::NoStack => {
                    return Err(format!("Cannot apply multiple instances of {}", condition.name));
                }
                StackingRule::Latest => {
                    self.conditions.retain(|c| c.name != condition.name);
                }
                StackingRule::Highest => {
                    // Compare by remaining duration for now
                    let existing_max = existing.iter()
                        .filter_map(|c| c.remaining)
                        .max()
                        .unwrap_or(0);
                    if condition.remaining.unwrap_or(0) <= existing_max {
                        return Ok(()); // Don't add weaker condition
                    }
                    self.conditions.retain(|c| c.name != condition.name);
                }
                StackingRule::ExtendDuration => {
                    if let Some(existing_cond) = self.conditions.iter_mut().find(|c| c.name == condition.name) {
                        if let Some(ref mut remaining) = existing_cond.remaining {
                            *remaining += condition.remaining.unwrap_or(0);
                        }
                        return Ok(());
                    }
                }
                StackingRule::PerSource => {
                    // Remove only conditions from the same source
                    let source = condition.source_id.clone();
                    self.conditions.retain(|c| {
                        c.name != condition.name || c.source_id != source
                    });
                }
                StackingRule::Stack | StackingRule::Custom(_) => {
                    // Allow stacking
                }
            }
        }

        self.conditions.push(condition);
        Ok(())
    }

    /// Remove a condition by ID
    pub fn remove_condition(&mut self, condition_id: &str) -> Option<AdvancedCondition> {
        let pos = self.conditions.iter().position(|c| c.id == condition_id)?;
        Some(self.conditions.remove(pos))
    }

    /// Remove all conditions with a given name
    pub fn remove_by_name(&mut self, name: &str) -> Vec<AdvancedCondition> {
        let mut removed = Vec::new();
        self.conditions.retain(|c| {
            if c.name == name {
                removed.push(c.clone());
                false
            } else {
                true
            }
        });
        removed
    }

    /// Remove all conditions with a given tag
    pub fn remove_by_tag(&mut self, tag: &str) -> Vec<AdvancedCondition> {
        let mut removed = Vec::new();
        self.conditions.retain(|c| {
            if c.tags.contains(&tag.to_string()) {
                removed.push(c.clone());
                false
            } else {
                true
            }
        });
        removed
    }

    /// Tick all conditions at end of turn
    /// Returns list of expired conditions
    pub fn tick_end_of_turn(&mut self, is_own_turn: bool) -> Vec<AdvancedCondition> {
        let mut expired = Vec::new();
        self.conditions.retain_mut(|c| {
            if c.tick_end_of_turn(is_own_turn) {
                expired.push(c.clone());
                false
            } else {
                true
            }
        });
        expired
    }

    /// Tick all conditions at start of turn
    /// Returns list of expired conditions
    pub fn tick_start_of_turn(&mut self, is_own_turn: bool) -> Vec<AdvancedCondition> {
        let mut expired = Vec::new();
        self.conditions.retain_mut(|c| {
            if c.tick_start_of_turn(is_own_turn) {
                expired.push(c.clone());
                false
            } else {
                true
            }
        });
        expired
    }

    /// Tick for a round
    /// Returns list of expired conditions
    pub fn tick_round(&mut self) -> Vec<AdvancedCondition> {
        let mut expired = Vec::new();
        self.conditions.retain_mut(|c| {
            if c.tick_round() {
                expired.push(c.clone());
                false
            } else {
                true
            }
        });
        expired
    }

    /// Get all active conditions
    pub fn conditions(&self) -> &[AdvancedCondition] {
        &self.conditions
    }

    /// Get condition by ID
    pub fn get(&self, condition_id: &str) -> Option<&AdvancedCondition> {
        self.conditions.iter().find(|c| c.id == condition_id)
    }

    /// Get mutable condition by ID
    pub fn get_mut(&mut self, condition_id: &str) -> Option<&mut AdvancedCondition> {
        self.conditions.iter_mut().find(|c| c.id == condition_id)
    }

    /// Check if any condition with the given name is active
    pub fn has_condition(&self, name: &str) -> bool {
        self.conditions.iter().any(|c| c.name == name)
    }

    /// Check if any condition with the given tag is active
    pub fn has_tag(&self, tag: &str) -> bool {
        self.conditions.iter().any(|c| c.tags.contains(&tag.to_string()))
    }

    /// Get all active beneficial conditions
    pub fn beneficial_conditions(&self) -> Vec<&AdvancedCondition> {
        self.conditions.iter().filter(|c| c.is_beneficial).collect()
    }

    /// Get all active detrimental conditions
    pub fn detrimental_conditions(&self) -> Vec<&AdvancedCondition> {
        self.conditions.iter().filter(|c| !c.is_beneficial).collect()
    }

    /// Check if incapacitated (common check)
    pub fn is_incapacitated(&self) -> bool {
        self.conditions.iter().any(|c| {
            c.effects.iter().any(|e| matches!(
                e,
                ConditionEffect::ActionRestriction { restricted } if restricted.contains(&"actions".to_string())
            ))
        })
    }

    /// Get total modifier for a stat from all conditions
    pub fn total_modifier(&self, stat: &str) -> i32 {
        self.conditions
            .iter()
            .flat_map(|c| &c.effects)
            .filter_map(|e| {
                if let ConditionEffect::Modifier { stat: s, value } = e {
                    if s == stat { Some(*value) } else { None }
                } else {
                    None
                }
            })
            .sum()
    }

    /// Check if the target has advantage for a roll type
    pub fn has_advantage(&self, roll_type: &str) -> bool {
        self.conditions.iter().any(|c| {
            c.effects.iter().any(|e| matches!(
                e,
                ConditionEffect::Advantage { applies_to } if applies_to.contains(&roll_type.to_string())
            ))
        })
    }

    /// Check if the target has disadvantage for a roll type
    pub fn has_disadvantage(&self, roll_type: &str) -> bool {
        self.conditions.iter().any(|c| {
            c.effects.iter().any(|e| matches!(
                e,
                ConditionEffect::Disadvantage { applies_to } if applies_to.contains(&roll_type.to_string())
            ))
        })
    }
}

// ============================================================================
// Common Condition Templates (D&D 5e style)
// ============================================================================

/// Factory for common D&D 5e conditions
pub struct ConditionTemplates;

impl ConditionTemplates {
    pub fn blinded() -> AdvancedCondition {
        AdvancedCondition::new(
            "Blinded",
            "A blinded creature can't see and automatically fails any ability check that requires sight.",
            ConditionDuration::UntilRemoved
        )
        .with_effect(ConditionEffect::AutoFail { save_types: vec!["sight".to_string()] })
        .with_effect(ConditionEffect::Disadvantage { applies_to: vec!["attack".to_string()] })
        .with_effect(ConditionEffect::GrantAdvantage)
        .with_icon("eye-off")
        .with_color("#6b7280")
    }

    pub fn charmed() -> AdvancedCondition {
        AdvancedCondition::new(
            "Charmed",
            "A charmed creature can't attack the charmer and the charmer has advantage on social checks.",
            ConditionDuration::UntilRemoved
        )
        .with_effect(ConditionEffect::ActionRestriction { restricted: vec!["attack charmer".to_string()] })
        .with_icon("heart")
        .with_color("#ec4899")
        .with_tags(["magical"])
    }

    pub fn frightened() -> AdvancedCondition {
        AdvancedCondition::new(
            "Frightened",
            "A frightened creature has disadvantage on ability checks and attack rolls while the source is visible.",
            ConditionDuration::UntilRemoved
        )
        .with_effect(ConditionEffect::Disadvantage { applies_to: vec!["attack".to_string(), "ability".to_string()] })
        .with_effect(ConditionEffect::Custom { description: "Can't willingly move closer to source".to_string() })
        .with_icon("alert-triangle")
        .with_color("#eab308")
    }

    pub fn grappled() -> AdvancedCondition {
        AdvancedCondition::new(
            "Grappled",
            "A grappled creature's speed becomes 0.",
            ConditionDuration::UntilRemoved
        )
        .with_effect(ConditionEffect::SpeedChange { multiplier: Some(0.0), flat_change: None })
        .with_icon("grip-horizontal")
        .with_color("#78716c")
    }

    pub fn incapacitated() -> AdvancedCondition {
        AdvancedCondition::new(
            "Incapacitated",
            "An incapacitated creature can't take actions or reactions.",
            ConditionDuration::UntilRemoved
        )
        .with_effect(ConditionEffect::ActionRestriction { restricted: vec!["actions".to_string(), "reactions".to_string()] })
        .with_icon("pause-circle")
        .with_color("#dc2626")
    }

    pub fn invisible() -> AdvancedCondition {
        AdvancedCondition::new(
            "Invisible",
            "An invisible creature is impossible to see without special senses.",
            ConditionDuration::UntilRemoved
        )
        .with_effect(ConditionEffect::Advantage { applies_to: vec!["attack".to_string()] })
        .with_effect(ConditionEffect::GrantDisadvantage)
        .with_icon("eye-off")
        .with_color("#a855f7")
        .beneficial()
    }

    pub fn paralyzed() -> AdvancedCondition {
        AdvancedCondition::new(
            "Paralyzed",
            "A paralyzed creature is incapacitated, can't move or speak, and automatically fails STR and DEX saves.",
            ConditionDuration::UntilRemoved
        )
        .with_effect(ConditionEffect::ActionRestriction { restricted: vec!["actions".to_string(), "reactions".to_string(), "movement".to_string(), "speech".to_string()] })
        .with_effect(ConditionEffect::AutoFail { save_types: vec!["STR".to_string(), "DEX".to_string()] })
        .with_effect(ConditionEffect::GrantAdvantage)
        .with_effect(ConditionEffect::Custom { description: "Hits within 5ft are critical".to_string() })
        .with_icon("lock")
        .with_color("#7c3aed")
    }

    pub fn poisoned() -> AdvancedCondition {
        AdvancedCondition::new(
            "Poisoned",
            "A poisoned creature has disadvantage on attack rolls and ability checks.",
            ConditionDuration::UntilRemoved
        )
        .with_effect(ConditionEffect::Disadvantage { applies_to: vec!["attack".to_string(), "ability".to_string()] })
        .with_icon("skull")
        .with_color("#22c55e")
        .with_tags(["poison"])
    }

    pub fn prone() -> AdvancedCondition {
        AdvancedCondition::new(
            "Prone",
            "A prone creature's only movement option is to crawl. The creature has disadvantage on attack rolls.",
            ConditionDuration::UntilRemoved
        )
        .with_effect(ConditionEffect::Disadvantage { applies_to: vec!["attack".to_string()] })
        .with_effect(ConditionEffect::Custom { description: "Attacks within 5ft have advantage, beyond have disadvantage".to_string() })
        .with_icon("arrow-down")
        .with_color("#78716c")
    }

    pub fn restrained() -> AdvancedCondition {
        AdvancedCondition::new(
            "Restrained",
            "A restrained creature's speed becomes 0 and attack rolls against it have advantage.",
            ConditionDuration::UntilRemoved
        )
        .with_effect(ConditionEffect::SpeedChange { multiplier: Some(0.0), flat_change: None })
        .with_effect(ConditionEffect::Disadvantage { applies_to: vec!["attack".to_string(), "DEX save".to_string()] })
        .with_effect(ConditionEffect::GrantAdvantage)
        .with_icon("link")
        .with_color("#ea580c")
    }

    pub fn stunned() -> AdvancedCondition {
        AdvancedCondition::new(
            "Stunned",
            "A stunned creature is incapacitated, can't move, and can speak only falteringly.",
            ConditionDuration::UntilRemoved
        )
        .with_effect(ConditionEffect::ActionRestriction { restricted: vec!["actions".to_string(), "reactions".to_string(), "movement".to_string()] })
        .with_effect(ConditionEffect::AutoFail { save_types: vec!["STR".to_string(), "DEX".to_string()] })
        .with_effect(ConditionEffect::GrantAdvantage)
        .with_icon("zap-off")
        .with_color("#fbbf24")
    }

    pub fn unconscious() -> AdvancedCondition {
        AdvancedCondition::new(
            "Unconscious",
            "An unconscious creature is incapacitated, can't move or speak, and is unaware of its surroundings.",
            ConditionDuration::UntilRemoved
        )
        .with_effect(ConditionEffect::ActionRestriction { restricted: vec!["actions".to_string(), "reactions".to_string(), "movement".to_string(), "speech".to_string()] })
        .with_effect(ConditionEffect::AutoFail { save_types: vec!["STR".to_string(), "DEX".to_string()] })
        .with_effect(ConditionEffect::GrantAdvantage)
        .with_effect(ConditionEffect::Custom { description: "Drops held items, falls prone. Hits within 5ft are critical.".to_string() })
        .with_icon("moon")
        .with_color("#1e3a8a")
    }

    pub fn concentrating() -> AdvancedCondition {
        AdvancedCondition::new(
            "Concentrating",
            "Maintaining concentration on a spell. CON save on damage or lose concentration.",
            ConditionDuration::UntilRemoved
        )
        .with_effect(ConditionEffect::Custom { description: "CON save (DC 10 or half damage) on taking damage".to_string() })
        .with_icon("target")
        .with_color("#3b82f6")
        .with_tags(["magical", "concentration"])
    }

    pub fn exhaustion(level: u32) -> AdvancedCondition {
        let effects = match level {
            1 => vec![ConditionEffect::Disadvantage { applies_to: vec!["ability".to_string()] }],
            2 => vec![
                ConditionEffect::Disadvantage { applies_to: vec!["ability".to_string()] },
                ConditionEffect::SpeedChange { multiplier: Some(0.5), flat_change: None },
            ],
            3 => vec![
                ConditionEffect::Disadvantage { applies_to: vec!["ability".to_string(), "attack".to_string(), "save".to_string()] },
                ConditionEffect::SpeedChange { multiplier: Some(0.5), flat_change: None },
            ],
            4 => vec![
                ConditionEffect::Disadvantage { applies_to: vec!["ability".to_string(), "attack".to_string(), "save".to_string()] },
                ConditionEffect::SpeedChange { multiplier: Some(0.5), flat_change: None },
                ConditionEffect::Custom { description: "Hit point maximum halved".to_string() },
            ],
            5 => vec![
                ConditionEffect::Disadvantage { applies_to: vec!["ability".to_string(), "attack".to_string(), "save".to_string()] },
                ConditionEffect::SpeedChange { multiplier: Some(0.0), flat_change: None },
                ConditionEffect::Custom { description: "Hit point maximum halved".to_string() },
            ],
            _ => vec![ConditionEffect::Custom { description: "Death".to_string() }],
        };

        let mut condition = AdvancedCondition::new(
            format!("Exhaustion {}", level),
            format!("Exhaustion level {}. Long rest removes 1 level.", level),
            ConditionDuration::UntilRemoved
        )
        .with_icon("battery-low")
        .with_color("#4b5563")
        .with_tags(["exhaustion"])
        .with_stacking(StackingRule::NoStack);

        for effect in effects {
            condition = condition.with_effect(effect);
        }

        condition
    }

    /// Get a template by name
    pub fn by_name(name: &str) -> Option<AdvancedCondition> {
        match name.to_lowercase().as_str() {
            "blinded" => Some(Self::blinded()),
            "charmed" => Some(Self::charmed()),
            "frightened" => Some(Self::frightened()),
            "grappled" => Some(Self::grappled()),
            "incapacitated" => Some(Self::incapacitated()),
            "invisible" => Some(Self::invisible()),
            "paralyzed" => Some(Self::paralyzed()),
            "poisoned" => Some(Self::poisoned()),
            "prone" => Some(Self::prone()),
            "restrained" => Some(Self::restrained()),
            "stunned" => Some(Self::stunned()),
            "unconscious" => Some(Self::unconscious()),
            "concentrating" => Some(Self::concentrating()),
            "exhaustion 1" | "exhaustion" => Some(Self::exhaustion(1)),
            "exhaustion 2" => Some(Self::exhaustion(2)),
            "exhaustion 3" => Some(Self::exhaustion(3)),
            "exhaustion 4" => Some(Self::exhaustion(4)),
            "exhaustion 5" => Some(Self::exhaustion(5)),
            "exhaustion 6" => Some(Self::exhaustion(6)),
            _ => None,
        }
    }

    /// List all available template names
    pub fn list_names() -> Vec<&'static str> {
        vec![
            "Blinded", "Charmed", "Frightened", "Grappled", "Incapacitated",
            "Invisible", "Paralyzed", "Poisoned", "Prone", "Restrained",
            "Stunned", "Unconscious", "Concentrating",
            "Exhaustion 1", "Exhaustion 2", "Exhaustion 3",
            "Exhaustion 4", "Exhaustion 5", "Exhaustion 6",
        ]
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_creation() {
        let condition = AdvancedCondition::new(
            "Poisoned",
            "Take ongoing poison damage",
            ConditionDuration::Rounds(3)
        )
        .from_source("goblin-1", "Goblin Shaman")
        .with_effect(ConditionEffect::RecurringDamage {
            dice: "1d6".to_string(),
            damage_type: "poison".to_string(),
            timing: SaveTiming::StartOfTurn,
        })
        .with_tags(["poison", "magical"]);

        assert_eq!(condition.remaining, Some(3));
        assert_eq!(condition.effects.len(), 1);
        assert!(condition.tags.contains(&"poison".to_string()));
    }

    #[test]
    fn test_condition_tick() {
        let mut condition = AdvancedCondition::new(
            "Stunned",
            "Cannot act",
            ConditionDuration::Turns(2)
        );

        assert!(!condition.tick_end_of_turn(true)); // 2 -> 1
        assert_eq!(condition.remaining, Some(1));

        assert!(condition.tick_end_of_turn(true)); // 1 -> 0, expired
    }

    #[test]
    fn test_condition_tracker() {
        let mut tracker = ConditionTracker::new();

        let blinded = ConditionTemplates::blinded();
        tracker.add_condition(blinded).unwrap();

        assert!(tracker.has_condition("Blinded"));
        assert!(tracker.has_disadvantage("attack"));
        assert!(!tracker.is_incapacitated());

        let stunned = ConditionTemplates::stunned();
        tracker.add_condition(stunned).unwrap();

        assert!(tracker.is_incapacitated());
    }

    #[test]
    fn test_stacking_rules() {
        let mut tracker = ConditionTracker::new();

        // NoStack should reject second instance
        let exhaustion1 = ConditionTemplates::exhaustion(1);
        let exhaustion2 = ConditionTemplates::exhaustion(1);

        tracker.add_condition(exhaustion1).unwrap();
        let result = tracker.add_condition(exhaustion2);
        assert!(result.is_err());
    }
}
