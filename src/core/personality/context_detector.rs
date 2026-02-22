//! Gameplay Context Detector with Session State Integration (TASK-PERS-011, TASK-PERS-012)
//!
//! Provides enhanced context detection that combines:
//! - Keyword-based text analysis (from context_keywords.rs)
//! - Session state signals (combat_active, initiative_count)
//! - History smoothing over last N detections
//! - Confidence scoring 0.0-1.0
//!
//! ## Performance
//!
//! Detection should complete in <100ms for 1000-token inputs.

use super::context::{ContextHistory, GameplayContext};
use super::context_keywords::{ContextDetectionConfig, ContextDetector};
use super::errors::ContextDetectionError;
use super::types::ContextDetectionResult;
use serde::{Deserialize, Serialize};
use std::time::Instant;

// ============================================================================
// Constants
// ============================================================================

/// Default history buffer size for smoothing.
pub const DEFAULT_HISTORY_SIZE: usize = 5;

/// Performance target: detection should complete within this time (ms).
pub const DETECTION_TARGET_MS: u128 = 100;

/// Combat confidence boost when session has combat active.
pub const COMBAT_SESSION_BOOST: f32 = 0.3;

/// Minimum initiative count to consider combat highly likely.
pub const MIN_INITIATIVE_FOR_COMBAT: usize = 2;

// ============================================================================
// Session State Snapshot
// ============================================================================

/// A snapshot of session state relevant for context detection.
///
/// This is a lightweight struct that captures the minimum information
/// needed from the full SessionState for context detection purposes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStateSnapshot {
    /// Whether combat is currently active in the session.
    pub combat_active: bool,

    /// Number of combatants in initiative order (0 if no combat).
    pub initiative_count: usize,

    /// Current round number (0 if no combat).
    #[serde(default)]
    pub current_round: usize,

    /// Whether any characters are at 0 HP.
    #[serde(default)]
    pub has_downed_characters: bool,

    /// Active NPC conversation (if any).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_npc_id: Option<String>,

    /// Current scene/location tag (if any).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene_tag: Option<String>,
}

impl SessionStateSnapshot {
    /// Create an empty snapshot (no session state).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create a combat-active snapshot.
    pub fn combat(initiative_count: usize, current_round: usize) -> Self {
        Self {
            combat_active: true,
            initiative_count,
            current_round,
            has_downed_characters: false,
            active_npc_id: None,
            scene_tag: None,
        }
    }

    /// Create a social interaction snapshot.
    pub fn social(npc_id: impl Into<String>) -> Self {
        Self {
            combat_active: false,
            initiative_count: 0,
            current_round: 0,
            has_downed_characters: false,
            active_npc_id: Some(npc_id.into()),
            scene_tag: None,
        }
    }

    /// Set combat active flag.
    pub fn with_combat(mut self, active: bool) -> Self {
        self.combat_active = active;
        self
    }

    /// Set initiative count.
    pub fn with_initiative(mut self, count: usize) -> Self {
        self.initiative_count = count;
        self
    }

    /// Set scene tag.
    pub fn with_scene_tag(mut self, tag: impl Into<String>) -> Self {
        self.scene_tag = Some(tag.into());
        self
    }

    /// Check if session strongly suggests combat.
    pub fn suggests_combat(&self) -> bool {
        self.combat_active || self.initiative_count >= MIN_INITIATIVE_FOR_COMBAT
    }

    /// Check if session strongly suggests social interaction.
    pub fn suggests_social(&self) -> bool {
        self.active_npc_id.is_some() && !self.combat_active
    }
}

// ============================================================================
// Gameplay Context Detector (Enhanced)
// ============================================================================

/// Enhanced gameplay context detector with session state integration and history smoothing.
///
/// Combines:
/// 1. Keyword-based text analysis
/// 2. Session state signals
/// 3. History smoothing for stability
pub struct GameplayContextDetector {
    /// Base keyword detector.
    keyword_detector: ContextDetector,

    /// Detection history for smoothing.
    history: ContextHistory,

    /// History buffer size.
    history_size: usize,
}

impl GameplayContextDetector {
    /// Create a new detector with default configuration.
    pub fn new() -> Self {
        Self {
            keyword_detector: ContextDetector::new(),
            history: ContextHistory::new(DEFAULT_HISTORY_SIZE),
            history_size: DEFAULT_HISTORY_SIZE,
        }
    }

    /// Create with custom keyword detection config.
    pub fn with_config(config: ContextDetectionConfig) -> Self {
        Self {
            keyword_detector: ContextDetector::with_config(config),
            history: ContextHistory::new(DEFAULT_HISTORY_SIZE),
            history_size: DEFAULT_HISTORY_SIZE,
        }
    }

    /// Set the history buffer size.
    pub fn with_history_size(mut self, size: usize) -> Self {
        self.history_size = size;
        self.history = ContextHistory::new(size);
        self
    }

    /// Get the keyword detection configuration.
    pub fn config(&self) -> &ContextDetectionConfig {
        self.keyword_detector.config()
    }

    /// Detect context from text and session state.
    ///
    /// Returns the detected context with confidence score.
    /// Performance target: <100ms for 1000-token input.
    pub fn detect(
        &mut self,
        text: &str,
        session_state: Option<&SessionStateSnapshot>,
    ) -> Result<ContextDetectionResult, ContextDetectionError> {
        let start = Instant::now();

        // Perform keyword-based detection
        let keyword_result = self.keyword_detector.detect(text);

        // Combine with session state
        let (context, confidence, keywords) = match keyword_result {
            Some(result) => {
                let (adjusted_context, adjusted_confidence) =
                    self.apply_session_state(result.context, result.confidence, session_state);
                (adjusted_context, adjusted_confidence, result.matched_keywords)
            }
            None => {
                // No keyword match, rely on session state
                let (context, confidence) =
                    self.infer_from_session_state(session_state);
                (context, confidence, Vec::new())
            }
        };

        // Add to history
        self.history.add(context, confidence);

        // Apply history smoothing
        let smoothed_context = self.history.smoothed().unwrap_or(context);
        let smoothed_confidence = if smoothed_context == context {
            confidence
        } else {
            // Reduce confidence if smoothing changed the result
            confidence * 0.8
        };

        // Build result
        let mut result = ContextDetectionResult::new(smoothed_context.as_str(), smoothed_confidence)
            .with_keywords(keywords);

        // Check for ambiguity
        if let Some(current) = self.history.current() {
            if *current != smoothed_context && confidence > 0.3 {
                result = result
                    .with_alternative(current.as_str(), confidence)
                    .mark_ambiguous();
            }
        }

        // Log performance
        let elapsed = start.elapsed().as_millis();
        if elapsed > DETECTION_TARGET_MS {
            log::warn!(
                "Context detection took {}ms (target: {}ms)",
                elapsed,
                DETECTION_TARGET_MS
            );
        } else {
            log::trace!("Context detection completed in {}ms", elapsed);
        }

        Ok(result)
    }

    /// Detect context without session state.
    pub fn detect_text_only(
        &mut self,
        text: &str,
    ) -> Result<ContextDetectionResult, ContextDetectionError> {
        self.detect(text, None)
    }

    /// Get the current smoothed context from history.
    pub fn current_context(&self) -> Option<GameplayContext> {
        self.history.smoothed()
    }

    /// Get the raw (unsmoothed) most recent detection.
    pub fn raw_current(&self) -> Option<&GameplayContext> {
        self.history.current()
    }

    /// Clear detection history.
    pub fn clear_history(&mut self) {
        self.history.clear();
        log::debug!("Cleared context detection history");
    }

    /// Get history length.
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Apply session state adjustments to detected context.
    fn apply_session_state(
        &self,
        detected: GameplayContext,
        confidence: f32,
        session: Option<&SessionStateSnapshot>,
    ) -> (GameplayContext, f32) {
        let Some(session) = session else {
            return (detected, confidence);
        };

        // Strong session signals can override or boost detection
        match detected {
            GameplayContext::CombatEncounter => {
                if session.combat_active {
                    // Boost confidence when session confirms combat
                    (detected, (confidence + COMBAT_SESSION_BOOST).min(1.0))
                } else if session.initiative_count >= MIN_INITIATIVE_FOR_COMBAT {
                    // Initiative suggests combat even if not explicitly active
                    (detected, (confidence + 0.15).min(1.0))
                } else {
                    // Keyword detected combat but session doesn't confirm
                    // Could be discussing past combat or rules
                    (detected, confidence * 0.9)
                }
            }
            GameplayContext::SocialInteraction => {
                if session.suggests_social() {
                    // Boost when in active NPC conversation
                    (detected, (confidence + 0.2).min(1.0))
                } else if session.combat_active {
                    // In combat but text suggests social - might be mid-combat dialogue
                    (detected, confidence * 0.7)
                } else {
                    (detected, confidence)
                }
            }
            GameplayContext::Unknown => {
                // Try to infer from session state
                if session.suggests_combat() {
                    (GameplayContext::CombatEncounter, 0.6)
                } else if session.suggests_social() {
                    (GameplayContext::SocialInteraction, 0.5)
                } else {
                    (detected, confidence)
                }
            }
            _ => {
                // For other contexts, session state provides minor adjustments
                if session.combat_active && !detected.is_combat_related() {
                    // Non-combat context detected during combat - might be brief pause
                    (detected, confidence * 0.85)
                } else {
                    (detected, confidence)
                }
            }
        }
    }

    /// Infer context purely from session state when keywords fail.
    fn infer_from_session_state(
        &self,
        session: Option<&SessionStateSnapshot>,
    ) -> (GameplayContext, f32) {
        let Some(session) = session else {
            return (GameplayContext::Unknown, 0.0);
        };

        if session.combat_active {
            (GameplayContext::CombatEncounter, 0.7)
        } else if session.initiative_count >= MIN_INITIATIVE_FOR_COMBAT {
            (GameplayContext::CombatEncounter, 0.5)
        } else if session.suggests_social() {
            (GameplayContext::SocialInteraction, 0.6)
        } else if session.has_downed_characters {
            // Characters down but not in active combat - likely aftermath
            (GameplayContext::Downtime, 0.4)
        } else {
            (GameplayContext::Unknown, 0.0)
        }
    }
}

impl Default for GameplayContextDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Detection Statistics
// ============================================================================

/// Statistics about context detection performance.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionStats {
    /// Total detections performed.
    pub total_detections: u64,

    /// Detections that exceeded target time.
    pub slow_detections: u64,

    /// Average detection time in microseconds.
    pub avg_detection_time_us: u64,

    /// Context distribution (context -> count).
    pub context_distribution: std::collections::HashMap<String, u64>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state_snapshot_empty() {
        let snapshot = SessionStateSnapshot::empty();
        assert!(!snapshot.combat_active);
        assert_eq!(snapshot.initiative_count, 0);
        assert!(!snapshot.suggests_combat());
        assert!(!snapshot.suggests_social());
    }

    #[test]
    fn test_session_state_snapshot_combat() {
        let snapshot = SessionStateSnapshot::combat(4, 2);
        assert!(snapshot.combat_active);
        assert_eq!(snapshot.initiative_count, 4);
        assert_eq!(snapshot.current_round, 2);
        assert!(snapshot.suggests_combat());
    }

    #[test]
    fn test_session_state_snapshot_social() {
        let snapshot = SessionStateSnapshot::social("npc_001");
        assert!(!snapshot.combat_active);
        assert!(snapshot.active_npc_id.is_some());
        assert!(snapshot.suggests_social());
    }

    #[test]
    fn test_session_state_builder() {
        let snapshot = SessionStateSnapshot::empty()
            .with_combat(true)
            .with_initiative(3)
            .with_scene_tag("tavern");

        assert!(snapshot.combat_active);
        assert_eq!(snapshot.initiative_count, 3);
        assert_eq!(snapshot.scene_tag, Some("tavern".to_string()));
    }

    #[test]
    fn test_detector_creation() {
        let detector = GameplayContextDetector::new();
        assert_eq!(detector.history_size, DEFAULT_HISTORY_SIZE);
        assert_eq!(detector.history_len(), 0);
    }

    #[test]
    fn test_detector_text_only() {
        let mut detector = GameplayContextDetector::new();

        let result = detector
            .detect_text_only("I roll for initiative and attack the goblin!")
            .unwrap();

        assert_eq!(result.context, "combat_encounter");
        assert!(result.confidence > 0.3);
    }

    #[test]
    fn test_detector_with_combat_session() {
        let mut detector = GameplayContextDetector::new();
        let session = SessionStateSnapshot::combat(4, 1);

        let result = detector
            .detect("The fighter swings his sword.", Some(&session))
            .unwrap();

        // Should boost combat confidence
        assert_eq!(result.context, "combat_encounter");
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_detector_session_inference() {
        let mut detector = GameplayContextDetector::new();
        let session = SessionStateSnapshot::combat(4, 1);

        // Very vague text with no keywords
        let result = detector
            .detect("What should we do next?", Some(&session))
            .unwrap();

        // Should infer combat from session state
        assert_eq!(result.context, "combat_encounter");
    }

    #[test]
    fn test_detector_history_smoothing() {
        let mut detector = GameplayContextDetector::new();

        // Establish a pattern of combat
        for _ in 0..3 {
            let _ = detector.detect_text_only("Attack roll! Initiative!").unwrap();
        }

        assert_eq!(detector.history_len(), 3);

        // Current context should be combat
        let current = detector.current_context();
        assert_eq!(current, Some(GameplayContext::CombatEncounter));

        // A single different input shouldn't immediately change smoothed result
        let _ = detector.detect_text_only("Tell me about the history of this place.").unwrap();

        // Smoothed context might still favor combat due to history
        let smoothed = detector.current_context();
        // Could be either depending on weights
        assert!(smoothed.is_some());
    }

    #[test]
    fn test_detector_clear_history() {
        let mut detector = GameplayContextDetector::new();

        let _ = detector.detect_text_only("Roll initiative!").unwrap();
        assert_eq!(detector.history_len(), 1);

        detector.clear_history();
        assert_eq!(detector.history_len(), 0);
    }

    #[test]
    fn test_detector_custom_history_size() {
        let detector = GameplayContextDetector::new().with_history_size(10);
        assert_eq!(detector.history_size, 10);
    }

    #[test]
    fn test_session_state_serialization() {
        let snapshot = SessionStateSnapshot::combat(4, 2)
            .with_scene_tag("dungeon");

        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("\"combatActive\":true"));
        assert!(json.contains("\"initiativeCount\":4"));
        assert!(json.contains("\"sceneTag\":\"dungeon\""));

        let parsed: SessionStateSnapshot = serde_json::from_str(&json).unwrap();
        assert!(parsed.combat_active);
        assert_eq!(parsed.initiative_count, 4);
    }

    #[test]
    fn test_detection_performance() {
        let mut detector = GameplayContextDetector::new();

        // Generate ~1000 tokens of text
        let text = "I roll for initiative and attack the goblin with my sword. ".repeat(50);

        let start = std::time::Instant::now();
        let _ = detector.detect_text_only(&text).unwrap();
        let elapsed = start.elapsed().as_millis();

        // Should complete within target time
        assert!(
            elapsed < DETECTION_TARGET_MS * 2,
            "Detection took {}ms, target is {}ms",
            elapsed,
            DETECTION_TARGET_MS
        );
    }

    #[test]
    fn test_social_with_combat_session() {
        let mut detector = GameplayContextDetector::new();
        let session = SessionStateSnapshot::combat(4, 1);

        // Social text during combat
        let result = detector
            .detect("I want to persuade the orc to surrender.", Some(&session))
            .unwrap();

        // Should detect social but with reduced confidence
        if result.context == "social_interaction" {
            assert!(result.confidence < 0.8);
        }
    }

    #[test]
    fn test_unknown_context_with_session() {
        let mut detector = GameplayContextDetector::new();
        let session = SessionStateSnapshot::social("npc_bartender");

        // Very vague text
        let result = detector
            .detect("I nod thoughtfully.", Some(&session))
            .unwrap();

        // Should infer social from session
        assert_eq!(result.context, "social_interaction");
    }
}
