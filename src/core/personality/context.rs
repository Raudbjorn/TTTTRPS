//! GameplayContext Enum and Context Types
//!
//! Defines the gameplay context types used for automatic personality switching
//! based on the current game situation.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// ============================================================================
// Gameplay Context Enum
// ============================================================================

/// Represents the current gameplay context for personality blending.
///
/// The DM personality system uses these contexts to automatically adjust
/// the personality blend based on what's happening in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GameplayContext {
    /// Active combat encounter with initiative, attacks, and tactical decisions.
    ///
    /// Suggested blend: 60% Tactical Advisor + 40% Active personality
    CombatEncounter,

    /// Social interaction with NPCs, dialogue, and roleplay.
    ///
    /// Suggested blend: 70% Active personality + 30% Storyteller
    SocialInteraction,

    /// Exploration of environments, travel, and discovery.
    ///
    /// Suggested blend: 60% Storyteller + 40% Active personality
    Exploration,

    /// Puzzle solving, investigation, and mystery elements.
    ///
    /// Suggested blend: 50% Rules Lawyer + 50% Active personality
    PuzzleInvestigation,

    /// Lore exposition, history, and world-building.
    ///
    /// Suggested blend: 70% Storyteller + 30% Rules Lawyer
    LoreExposition,

    /// Downtime activities, shopping, and rest periods.
    ///
    /// Suggested blend: 80% Active personality + 20% Rules Lawyer
    Downtime,

    /// Rule clarification, mechanics questions, and disputes.
    ///
    /// Suggested blend: 90% Rules Lawyer + 10% Active personality
    RuleClarification,

    /// Unknown or ambiguous context (fallback).
    ///
    /// Uses default personality without blending.
    #[default]
    Unknown,
}

impl GameplayContext {
    /// Get the string representation used in serialization.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CombatEncounter => "combat_encounter",
            Self::SocialInteraction => "social_interaction",
            Self::Exploration => "exploration",
            Self::PuzzleInvestigation => "puzzle_investigation",
            Self::LoreExposition => "lore_exposition",
            Self::Downtime => "downtime",
            Self::RuleClarification => "rule_clarification",
            Self::Unknown => "unknown",
        }
    }

    /// Get a human-readable display name for the context.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CombatEncounter => "Combat Encounter",
            Self::SocialInteraction => "Social Interaction",
            Self::Exploration => "Exploration",
            Self::PuzzleInvestigation => "Puzzle / Investigation",
            Self::LoreExposition => "Lore Exposition",
            Self::Downtime => "Downtime",
            Self::RuleClarification => "Rule Clarification",
            Self::Unknown => "Unknown",
        }
    }

    /// Get a description of this context type.
    pub fn description(&self) -> &'static str {
        match self {
            Self::CombatEncounter => {
                "Active combat with initiative tracking, attacks, and tactical decisions"
            }
            Self::SocialInteraction => "Dialogue with NPCs, negotiations, and roleplay scenes",
            Self::Exploration => "Traveling, exploring environments, and discovering new areas",
            Self::PuzzleInvestigation => "Solving puzzles, investigating clues, and mystery scenarios",
            Self::LoreExposition => "Sharing world history, legends, and background information",
            Self::Downtime => "Rest periods, shopping, crafting, and other non-adventure activities",
            Self::RuleClarification => "Clarifying game rules, mechanics, and resolving disputes",
            Self::Unknown => "Context could not be determined",
        }
    }

    /// Get the default blend suggestion for this context.
    ///
    /// Returns tuples of (personality_type, weight) for the suggested blend.
    /// The weights sum to 1.0.
    pub fn default_blend_suggestion(&self) -> Vec<(&'static str, f32)> {
        match self {
            Self::CombatEncounter => vec![("tactical_advisor", 0.6), ("active", 0.4)],
            Self::SocialInteraction => vec![("active", 0.7), ("storyteller", 0.3)],
            Self::Exploration => vec![("storyteller", 0.6), ("active", 0.4)],
            Self::PuzzleInvestigation => vec![("rules_lawyer", 0.5), ("active", 0.5)],
            Self::LoreExposition => vec![("storyteller", 0.7), ("rules_lawyer", 0.3)],
            Self::Downtime => vec![("active", 0.8), ("rules_lawyer", 0.2)],
            Self::RuleClarification => vec![("rules_lawyer", 0.9), ("active", 0.1)],
            Self::Unknown => vec![("active", 1.0)],
        }
    }

    /// Get all defined contexts (excluding Unknown).
    pub fn all_defined() -> &'static [GameplayContext] {
        &[
            Self::CombatEncounter,
            Self::SocialInteraction,
            Self::Exploration,
            Self::PuzzleInvestigation,
            Self::LoreExposition,
            Self::Downtime,
            Self::RuleClarification,
        ]
    }

    /// Get all contexts including Unknown.
    pub fn all() -> &'static [GameplayContext] {
        &[
            Self::CombatEncounter,
            Self::SocialInteraction,
            Self::Exploration,
            Self::PuzzleInvestigation,
            Self::LoreExposition,
            Self::Downtime,
            Self::RuleClarification,
            Self::Unknown,
        ]
    }

    /// Check if this context typically involves combat mechanics.
    pub fn is_combat_related(&self) -> bool {
        matches!(self, Self::CombatEncounter)
    }

    /// Check if this context typically involves social/roleplay elements.
    pub fn is_roleplay_related(&self) -> bool {
        matches!(self, Self::SocialInteraction | Self::Downtime)
    }

    /// Check if this context typically involves rules and mechanics.
    pub fn is_rules_related(&self) -> bool {
        matches!(
            self,
            Self::CombatEncounter | Self::RuleClarification | Self::PuzzleInvestigation
        )
    }

    /// Check if this context typically involves narrative elements.
    pub fn is_narrative_related(&self) -> bool {
        matches!(
            self,
            Self::LoreExposition | Self::Exploration | Self::SocialInteraction
        )
    }
}

impl fmt::Display for GameplayContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl FromStr for GameplayContext {
    type Err = ContextParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace(['-', ' '], "_").as_str() {
            "combat_encounter" | "combat" => Ok(Self::CombatEncounter),
            "social_interaction" | "social" | "roleplay" => Ok(Self::SocialInteraction),
            "exploration" | "explore" | "travel" => Ok(Self::Exploration),
            "puzzle_investigation" | "puzzle" | "investigation" | "mystery" => {
                Ok(Self::PuzzleInvestigation)
            }
            "lore_exposition" | "lore" | "exposition" | "history" => Ok(Self::LoreExposition),
            "downtime" | "rest" | "shopping" => Ok(Self::Downtime),
            "rule_clarification" | "rules" | "clarification" | "mechanics" => {
                Ok(Self::RuleClarification)
            }
            "unknown" | "" => Ok(Self::Unknown),
            _ => Err(ContextParseError::UnknownContext(s.to_string())),
        }
    }
}

// ============================================================================
// Context Parse Error
// ============================================================================

/// Error returned when parsing a GameplayContext from a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextParseError {
    /// The provided string does not match any known context.
    UnknownContext(String),
}

impl fmt::Display for ContextParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownContext(s) => write!(f, "unknown gameplay context: '{}'", s),
        }
    }
}

impl std::error::Error for ContextParseError {}

// ============================================================================
// Context Transition
// ============================================================================

/// Represents a transition between gameplay contexts.
///
/// Used for tracking context changes and managing smooth personality blending
/// during transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextTransition {
    /// The previous context.
    pub from: GameplayContext,

    /// The new context.
    pub to: GameplayContext,

    /// Confidence in the transition (0.0 to 1.0).
    pub confidence: f32,

    /// Timestamp of the transition (RFC 3339).
    pub timestamp: String,

    /// Optional trigger that caused the transition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
}

impl ContextTransition {
    /// Create a new context transition.
    pub fn new(from: GameplayContext, to: GameplayContext, confidence: f32) -> Self {
        Self {
            from,
            to,
            confidence: confidence.clamp(0.0, 1.0),
            timestamp: chrono::Utc::now().to_rfc3339(),
            trigger: None,
        }
    }

    /// Create a transition with a trigger description.
    pub fn with_trigger(mut self, trigger: impl Into<String>) -> Self {
        self.trigger = Some(trigger.into());
        self
    }

    /// Check if this is a significant context change (not same -> same).
    pub fn is_significant(&self) -> bool {
        self.from != self.to
    }

    /// Check if the transition involves combat.
    pub fn involves_combat(&self) -> bool {
        self.from.is_combat_related() || self.to.is_combat_related()
    }
}

// ============================================================================
// Context History
// ============================================================================

/// Maintains a history of recent context detections for smoothing.
///
/// This helps prevent rapid context switching by maintaining a sliding window
/// of recent detections.
#[derive(Debug, Clone, Default)]
pub struct ContextHistory {
    /// Recent context detections with timestamps.
    entries: Vec<(GameplayContext, f32, String)>,

    /// Maximum number of entries to keep.
    max_entries: usize,
}

impl ContextHistory {
    /// Create a new context history with the specified capacity.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::with_capacity(max_entries),
            max_entries,
        }
    }

    /// Add a new context detection to the history.
    pub fn add(&mut self, context: GameplayContext, confidence: f32) {
        let timestamp = chrono::Utc::now().to_rfc3339();
        self.entries.push((context, confidence, timestamp));

        // Trim old entries
        while self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }

    /// Get the most recent context.
    pub fn current(&self) -> Option<&GameplayContext> {
        self.entries.last().map(|(ctx, _, _)| ctx)
    }

    /// Get the smoothed context based on recent history.
    ///
    /// Returns the context that appears most frequently in recent history,
    /// weighted by confidence scores.
    pub fn smoothed(&self) -> Option<GameplayContext> {
        if self.entries.is_empty() {
            return None;
        }

        use std::collections::HashMap;

        let mut weights: HashMap<GameplayContext, f32> = HashMap::new();

        // Weight more recent entries higher
        let len = self.entries.len();
        for (i, (ctx, confidence, _)) in self.entries.iter().enumerate() {
            let recency_weight = (i + 1) as f32 / len as f32;
            *weights.entry(*ctx).or_insert(0.0) += confidence * recency_weight;
        }

        weights
            .into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(ctx, _)| ctx)
    }

    /// Get the number of entries in the history.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the history is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear the history.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_as_str() {
        assert_eq!(GameplayContext::CombatEncounter.as_str(), "combat_encounter");
        assert_eq!(
            GameplayContext::SocialInteraction.as_str(),
            "social_interaction"
        );
        assert_eq!(GameplayContext::Exploration.as_str(), "exploration");
        assert_eq!(
            GameplayContext::PuzzleInvestigation.as_str(),
            "puzzle_investigation"
        );
        assert_eq!(GameplayContext::LoreExposition.as_str(), "lore_exposition");
        assert_eq!(GameplayContext::Downtime.as_str(), "downtime");
        assert_eq!(
            GameplayContext::RuleClarification.as_str(),
            "rule_clarification"
        );
        assert_eq!(GameplayContext::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_context_from_str() {
        assert_eq!(
            "combat_encounter".parse::<GameplayContext>().unwrap(),
            GameplayContext::CombatEncounter
        );
        assert_eq!(
            "combat".parse::<GameplayContext>().unwrap(),
            GameplayContext::CombatEncounter
        );
        assert_eq!(
            "social".parse::<GameplayContext>().unwrap(),
            GameplayContext::SocialInteraction
        );
        assert_eq!(
            "roleplay".parse::<GameplayContext>().unwrap(),
            GameplayContext::SocialInteraction
        );
        assert_eq!(
            "puzzle".parse::<GameplayContext>().unwrap(),
            GameplayContext::PuzzleInvestigation
        );
        assert_eq!(
            "lore".parse::<GameplayContext>().unwrap(),
            GameplayContext::LoreExposition
        );
        assert_eq!(
            "rules".parse::<GameplayContext>().unwrap(),
            GameplayContext::RuleClarification
        );
        assert_eq!(
            "unknown".parse::<GameplayContext>().unwrap(),
            GameplayContext::Unknown
        );
        assert_eq!(
            "".parse::<GameplayContext>().unwrap(),
            GameplayContext::Unknown
        );

        // Test error case
        let result = "invalid_context".parse::<GameplayContext>();
        assert!(result.is_err());
    }

    #[test]
    fn test_context_from_str_normalization() {
        // Test with different separators and cases
        assert_eq!(
            "Combat-Encounter".parse::<GameplayContext>().unwrap(),
            GameplayContext::CombatEncounter
        );
        assert_eq!(
            "COMBAT ENCOUNTER".parse::<GameplayContext>().unwrap(),
            GameplayContext::CombatEncounter
        );
        assert_eq!(
            "social-interaction".parse::<GameplayContext>().unwrap(),
            GameplayContext::SocialInteraction
        );
    }

    #[test]
    fn test_context_default() {
        let ctx: GameplayContext = Default::default();
        assert_eq!(ctx, GameplayContext::Unknown);
    }

    #[test]
    fn test_context_display() {
        assert_eq!(GameplayContext::CombatEncounter.to_string(), "Combat Encounter");
        assert_eq!(
            GameplayContext::SocialInteraction.to_string(),
            "Social Interaction"
        );
    }

    #[test]
    fn test_default_blend_suggestions() {
        let blend = GameplayContext::CombatEncounter.default_blend_suggestion();
        assert_eq!(blend.len(), 2);
        let sum: f32 = blend.iter().map(|(_, w)| w).sum();
        assert!((sum - 1.0).abs() < 0.001);

        // Test all contexts have valid blends that sum to 1.0
        for ctx in GameplayContext::all() {
            let blend = ctx.default_blend_suggestion();
            assert!(!blend.is_empty());
            let sum: f32 = blend.iter().map(|(_, w)| w).sum();
            assert!((sum - 1.0).abs() < 0.001, "Context {:?} blend doesn't sum to 1.0", ctx);
        }
    }

    #[test]
    fn test_context_categories() {
        assert!(GameplayContext::CombatEncounter.is_combat_related());
        assert!(!GameplayContext::SocialInteraction.is_combat_related());

        assert!(GameplayContext::SocialInteraction.is_roleplay_related());
        assert!(GameplayContext::Downtime.is_roleplay_related());
        assert!(!GameplayContext::CombatEncounter.is_roleplay_related());

        assert!(GameplayContext::CombatEncounter.is_rules_related());
        assert!(GameplayContext::RuleClarification.is_rules_related());
        assert!(!GameplayContext::LoreExposition.is_rules_related());

        assert!(GameplayContext::LoreExposition.is_narrative_related());
        assert!(GameplayContext::Exploration.is_narrative_related());
        assert!(!GameplayContext::RuleClarification.is_narrative_related());
    }

    #[test]
    fn test_all_contexts() {
        let defined = GameplayContext::all_defined();
        assert_eq!(defined.len(), 7);
        assert!(!defined.contains(&GameplayContext::Unknown));

        let all = GameplayContext::all();
        assert_eq!(all.len(), 8);
        assert!(all.contains(&GameplayContext::Unknown));
    }

    #[test]
    fn test_context_serialization() {
        let ctx = GameplayContext::CombatEncounter;
        let json = serde_json::to_string(&ctx).unwrap();
        assert_eq!(json, "\"combat_encounter\"");

        let parsed: GameplayContext = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ctx);
    }

    #[test]
    fn test_context_transition() {
        let transition = ContextTransition::new(
            GameplayContext::Exploration,
            GameplayContext::CombatEncounter,
            0.85,
        )
        .with_trigger("Initiative rolled");

        assert_eq!(transition.from, GameplayContext::Exploration);
        assert_eq!(transition.to, GameplayContext::CombatEncounter);
        assert_eq!(transition.confidence, 0.85);
        assert!(transition.is_significant());
        assert!(transition.involves_combat());
        assert_eq!(transition.trigger, Some("Initiative rolled".to_string()));
    }

    #[test]
    fn test_context_transition_same_context() {
        let transition = ContextTransition::new(
            GameplayContext::CombatEncounter,
            GameplayContext::CombatEncounter,
            0.9,
        );

        assert!(!transition.is_significant());
    }

    #[test]
    fn test_context_history() {
        let mut history = ContextHistory::new(5);
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);

        history.add(GameplayContext::Exploration, 0.8);
        history.add(GameplayContext::Exploration, 0.7);
        history.add(GameplayContext::CombatEncounter, 0.9);

        assert_eq!(history.len(), 3);
        assert_eq!(history.current(), Some(&GameplayContext::CombatEncounter));

        // Smoothed should favor combat due to high confidence and recency
        let smoothed = history.smoothed();
        assert!(smoothed.is_some());
    }

    #[test]
    fn test_context_history_capacity() {
        let mut history = ContextHistory::new(3);

        history.add(GameplayContext::Exploration, 0.5);
        history.add(GameplayContext::SocialInteraction, 0.5);
        history.add(GameplayContext::CombatEncounter, 0.5);
        history.add(GameplayContext::Downtime, 0.5);

        // Should only keep 3 entries
        assert_eq!(history.len(), 3);
        // Oldest entry (Exploration) should be removed
        assert_ne!(
            history.entries.first().map(|(ctx, _, _)| ctx),
            Some(&GameplayContext::Exploration)
        );
    }

    #[test]
    fn test_context_history_clear() {
        let mut history = ContextHistory::new(5);
        history.add(GameplayContext::Exploration, 0.8);
        history.add(GameplayContext::CombatEncounter, 0.9);

        assert!(!history.is_empty());

        history.clear();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
    }
}
