//! Contextual Personality Integration (TASK-PERS-017)
//!
//! Provides the `ContextualPersonalityManager` for integrating:
//! - Context detection
//! - Blend rule lookup
//! - Personality blending
//!
//! This module ties together Phases 1-3 to provide automatic,
//! context-aware personality switching for sessions.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use personality::contextual::ContextualPersonalityManager;
//!
//! let manager = ContextualPersonalityManager::new(
//!     blender,
//!     context_detector,
//!     blend_rule_store,
//!     personality_store,
//! );
//!
//! // Get contextual personality for a session
//! let profile = manager.get_contextual_personality(
//!     "campaign_123",
//!     Some(&session_state),
//!     "I attack the goblin!",
//! ).await?;
//! ```

use super::blender::{BlendSpec, PersonalityBlender};
use super::blend_rules::BlendRuleStore;
use super::context::GameplayContext;
use super::context_detector::{GameplayContextDetector, SessionStateSnapshot};
use super::errors::PersonalityExtensionError;
use super::types::{BlendComponent, BlendRule, ContextDetectionResult, PersonalityId};
use crate::core::personality_base::{PersonalityProfile, PersonalityStore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for contextual personality behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextualConfig {
    /// Minimum confidence required to apply a context-specific blend.
    /// Below this threshold, fall back to default personality.
    pub min_confidence_threshold: f32,

    /// Whether to use blend rules (true) or just the default personality (false).
    pub use_blend_rules: bool,

    /// Whether to cache blended profiles for repeated context detections.
    /// Note: Caching is currently delegated to the PersonalityBlender component.
    /// This field is reserved for future manager-level cache control.
    #[allow(dead_code)]
    pub enable_caching: bool,

    /// Campaign-specific default personality (if no blend rule matches).
    pub default_personality_id: Option<String>,
}

impl Default for ContextualConfig {
    fn default() -> Self {
        Self {
            min_confidence_threshold: 0.3,
            use_blend_rules: true,
            enable_caching: true,
            default_personality_id: None,
        }
    }
}

// ============================================================================
// Contextual Personality Result
// ============================================================================

/// Result of a contextual personality lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextualPersonalityResult {
    /// The resulting personality profile.
    pub profile: PersonalityProfile,

    /// The detected context (if any).
    pub detected_context: Option<String>,

    /// Confidence in the context detection.
    pub confidence: f32,

    /// Whether a blend was applied (vs. single personality).
    pub blended: bool,

    /// The blend rule ID used (if any).
    pub blend_rule_id: Option<String>,

    /// Whether the result came from cache.
    pub from_cache: bool,
}

// ============================================================================
// Contextual Personality Manager
// ============================================================================

/// Manager for context-aware personality selection and blending.
///
/// Combines context detection, blend rule lookup, and personality blending
/// into a single high-level interface.
pub struct ContextualPersonalityManager {
    /// The personality blender.
    blender: Arc<PersonalityBlender>,

    /// Context detector with session state integration.
    context_detector: RwLock<GameplayContextDetector>,

    /// Blend rule store.
    rule_store: Arc<BlendRuleStore>,

    /// Personality store for looking up base profiles.
    personality_store: Arc<PersonalityStore>,

    /// Configuration.
    config: RwLock<ContextualConfig>,
}

impl ContextualPersonalityManager {
    /// Create a new contextual personality manager.
    pub fn new(
        blender: Arc<PersonalityBlender>,
        rule_store: Arc<BlendRuleStore>,
        personality_store: Arc<PersonalityStore>,
    ) -> Self {
        Self {
            blender,
            context_detector: RwLock::new(GameplayContextDetector::new()),
            rule_store,
            personality_store,
            config: RwLock::new(ContextualConfig::default()),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(
        blender: Arc<PersonalityBlender>,
        rule_store: Arc<BlendRuleStore>,
        personality_store: Arc<PersonalityStore>,
        config: ContextualConfig,
    ) -> Self {
        Self {
            blender,
            context_detector: RwLock::new(GameplayContextDetector::new()),
            rule_store,
            personality_store,
            config: RwLock::new(config),
        }
    }

    /// Get the current configuration.
    pub async fn config(&self) -> ContextualConfig {
        self.config.read().await.clone()
    }

    /// Update the configuration.
    pub async fn set_config(&self, config: ContextualConfig) {
        *self.config.write().await = config;
    }

    /// Get a context-aware personality for a campaign/session.
    ///
    /// This is the main entry point for contextual personality selection.
    /// It:
    /// 1. Detects the gameplay context from user input + session state
    /// 2. Looks up a blend rule for that context
    /// 3. Blends personalities according to the rule
    /// 4. Falls back to default personality if no rule matches
    ///
    /// # Arguments
    /// * `campaign_id` - The campaign ID for rule lookup
    /// * `session_state` - Optional session state for context detection
    /// * `user_input` - The user's input text for context detection
    ///
    /// # Returns
    /// A `ContextualPersonalityResult` with the selected/blended profile.
    pub async fn get_contextual_personality(
        &self,
        campaign_id: &str,
        session_state: Option<&SessionStateSnapshot>,
        user_input: &str,
    ) -> Result<ContextualPersonalityResult, PersonalityExtensionError> {
        let config = self.config.read().await.clone();

        // Step 1: Detect context
        let detection = {
            let mut detector = self.context_detector.write().await;
            detector.detect(user_input, session_state)?
        };

        log::debug!(
            "Detected context: {} (confidence: {:.2})",
            detection.context,
            detection.confidence
        );

        // Step 2: Check confidence threshold
        if detection.confidence < config.min_confidence_threshold {
            log::debug!(
                "Confidence {:.2} below threshold {:.2}, using default",
                detection.confidence,
                config.min_confidence_threshold
            );
            return self.get_default_personality(campaign_id, &detection).await;
        }

        // Step 3: Look up blend rule (if enabled)
        if !config.use_blend_rules {
            return self.get_default_personality(campaign_id, &detection).await;
        }

        let context: GameplayContext = detection.context.parse().unwrap_or(GameplayContext::Unknown);
        let rule = self
            .rule_store
            .get_rule_for_context(Some(campaign_id), &context)
            .await?;

        // Step 4: Apply blend rule or fall back to default
        match rule {
            Some(rule) if rule.enabled => {
                self.apply_blend_rule(&rule, &detection).await
            }
            _ => {
                // No rule or disabled - try global rule
                let global_rule = self
                    .rule_store
                    .get_rule_for_context(None, &context)
                    .await?;

                match global_rule {
                    Some(rule) if rule.enabled => {
                        self.apply_blend_rule(&rule, &detection).await
                    }
                    _ => {
                        log::debug!("No blend rule found for context {}, using default", detection.context);
                        self.get_default_personality(campaign_id, &detection).await
                    }
                }
            }
        }
    }

    /// Detect context without applying blend rules.
    ///
    /// Useful for UI feedback showing detected context.
    pub async fn detect_context(
        &self,
        user_input: &str,
        session_state: Option<&SessionStateSnapshot>,
    ) -> Result<ContextDetectionResult, PersonalityExtensionError> {
        let mut detector = self.context_detector.write().await;
        Ok(detector.detect(user_input, session_state)?)
    }

    /// Clear the context detection history.
    pub async fn clear_context_history(&self) {
        let mut detector = self.context_detector.write().await;
        detector.clear_history();
    }

    /// Get the current smoothed context.
    pub async fn current_context(&self) -> Option<GameplayContext> {
        let detector = self.context_detector.read().await;
        detector.current_context()
    }

    // ========================================================================
    // Internal Methods
    // ========================================================================

    /// Apply a blend rule to create a blended personality.
    async fn apply_blend_rule(
        &self,
        rule: &BlendRule,
        detection: &ContextDetectionResult,
    ) -> Result<ContextualPersonalityResult, PersonalityExtensionError> {
        // Convert rule weights to BlendSpec
        let components: Vec<BlendComponent> = rule
            .blend_weights
            .iter()
            .map(|(id, weight)| BlendComponent::new(id.clone(), *weight))
            .collect();

        if components.is_empty() {
            log::warn!("Blend rule {} has no components", rule.id);
            return self.get_fallback_personality(detection).await;
        }

        let spec = BlendSpec::new(components)?;

        // Gather personality profiles
        let mut profiles: HashMap<PersonalityId, PersonalityProfile> = HashMap::new();
        for id in rule.blend_weights.keys() {
            match self.personality_store.get(id.as_str()) {
                Ok(profile) => {
                    profiles.insert(id.clone(), profile);
                }
                Err(e) => {
                    log::warn!("Personality {} not found for blend rule {}: {}", id, rule.id, e);
                }
            }
        }

        if profiles.is_empty() {
            log::warn!("No valid profiles for blend rule {}", rule.id);
            return self.get_fallback_personality(detection).await;
        }

        // Perform blend
        let blended = self.blender.blend(&spec, &profiles).await?;

        Ok(ContextualPersonalityResult {
            profile: blended.profile,
            detected_context: Some(detection.context.clone()),
            confidence: detection.confidence,
            blended: true,
            blend_rule_id: Some(rule.id.to_string()),
            from_cache: blended.from_cache,
        })
    }

    /// Get the default personality for a campaign.
    /// Note: campaign_id reserved for future campaign-specific defaults.
    async fn get_default_personality(
        &self,
        _campaign_id: &str,
        detection: &ContextDetectionResult,
    ) -> Result<ContextualPersonalityResult, PersonalityExtensionError> {
        let config = self.config.read().await;

        // First try campaign-specific default
        if let Some(ref default_id) = config.default_personality_id {
            if let Ok(profile) = self.personality_store.get(default_id) {
                return Ok(ContextualPersonalityResult {
                    profile,
                    detected_context: Some(detection.context.clone()),
                    confidence: detection.confidence,
                    blended: false,
                    blend_rule_id: None,
                    from_cache: false,
                });
            }
        }

        // Fall back to first available personality
        self.get_fallback_personality(detection).await
    }

    /// Get a fallback personality when no rules match.
    async fn get_fallback_personality(
        &self,
        detection: &ContextDetectionResult,
    ) -> Result<ContextualPersonalityResult, PersonalityExtensionError> {
        // Get first available personality from store
        let profiles = self.personality_store.list();

        if let Some(profile) = profiles.into_iter().next() {
            Ok(ContextualPersonalityResult {
                profile,
                detected_context: Some(detection.context.clone()),
                confidence: detection.confidence,
                blended: false,
                blend_rule_id: None,
                from_cache: false,
            })
        } else {
            Err(PersonalityExtensionError::internal(
                "No personalities available in store",
            ))
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::personality_base::{BehavioralTendencies, SpeechPatterns};

    #[test]
    fn test_contextual_config_default() {
        let config = ContextualConfig::default();
        assert_eq!(config.min_confidence_threshold, 0.3);
        assert!(config.use_blend_rules);
        assert!(config.enable_caching);
        assert!(config.default_personality_id.is_none());
    }

    #[test]
    fn test_contextual_result_serialization() {
        // Create a test personality profile
        let profile = PersonalityProfile {
            id: "test_profile".to_string(),
            name: "Test Profile".to_string(),
            source: None,
            speech_patterns: SpeechPatterns::default(),
            traits: vec![],
            knowledge_areas: vec![],
            behavioral_tendencies: BehavioralTendencies::default(),
            example_phrases: vec![],
            tags: vec![],
            metadata: std::collections::HashMap::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        let result = ContextualPersonalityResult {
            profile,
            detected_context: Some("combat_encounter".to_string()),
            confidence: 0.85,
            blended: true,
            blend_rule_id: Some("rule_123".to_string()),
            from_cache: false,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"detectedContext\":\"combat_encounter\""));
        assert!(json.contains("\"confidence\":0.85"));
        assert!(json.contains("\"blended\":true"));
    }
}
