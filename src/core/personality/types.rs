//! Core Data Models for Personality Extensions
//!
//! Defines the core types used throughout the personality blending and
//! context detection system, including newtype wrappers for type safety.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// Newtype ID Wrappers
// ============================================================================

/// Strongly-typed wrapper for template IDs.
///
/// Provides type safety to prevent accidentally mixing template IDs with other
/// string identifiers like personality IDs or campaign IDs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TemplateId(String);

impl TemplateId {
    /// Create a new TemplateId from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new random TemplateId using UUID v4.
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Get the inner string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume self and return the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Deref for TemplateId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for TemplateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for TemplateId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for TemplateId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for TemplateId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Strongly-typed wrapper for personality IDs.
///
/// Provides type safety to prevent accidentally mixing personality IDs with
/// other string identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PersonalityId(String);

impl PersonalityId {
    /// Create a new PersonalityId from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new random PersonalityId using UUID v4.
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Get the inner string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume self and return the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Deref for PersonalityId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for PersonalityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for PersonalityId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for PersonalityId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for PersonalityId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Strongly-typed wrapper for blend rule IDs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BlendRuleId(String);

impl BlendRuleId {
    /// Create a new BlendRuleId from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new random BlendRuleId using UUID v4.
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Get the inner string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume self and return the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Deref for BlendRuleId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for BlendRuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for BlendRuleId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for BlendRuleId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for BlendRuleId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// Setting-Specific Personality Template
// ============================================================================

/// A setting-specific personality template that extends a base profile.
///
/// Templates allow customizing personalities for specific game settings
/// (e.g., Forgotten Realms, Eberron, Ravenloft) with appropriate vocabulary,
/// cultural markers, and tone adjustments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingPersonalityTemplate {
    /// Unique identifier for the template.
    pub id: TemplateId,

    /// Human-readable name for the template.
    pub name: String,

    /// Optional description of the template.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Base profile ID to extend (e.g., "storyteller", "rules_lawyer").
    pub base_profile: PersonalityId,

    /// Game system this template is designed for (e.g., "dnd5e", "pf2e").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_system: Option<String>,

    /// Setting name (e.g., "Forgotten Realms", "Eberron").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting_name: Option<String>,

    /// Whether this is a built-in template (not user-editable).
    #[serde(default)]
    pub is_builtin: bool,

    /// Tags for categorization and search.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Tone dimension overrides (dimension name -> intensity 0.0-1.0).
    #[serde(default)]
    pub tone_overrides: HashMap<String, f32>,

    /// Setting-specific vocabulary with usage frequency weights.
    #[serde(default)]
    pub vocabulary: HashMap<String, f32>,

    /// Common phrases characteristic of this setting.
    #[serde(default)]
    pub common_phrases: Vec<String>,

    /// Cultural markers and setting-specific elements.
    #[serde(default)]
    pub cultural_markers: Vec<String>,

    /// Campaign ID if this template is campaign-specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_id: Option<String>,

    /// Creation timestamp (RFC 3339 format).
    pub created_at: String,

    /// Last update timestamp (RFC 3339 format).
    pub updated_at: String,
}

impl SettingPersonalityTemplate {
    /// Create a new template with the given name and base profile.
    pub fn new(name: impl Into<String>, base_profile: PersonalityId) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: TemplateId::generate(),
            name: name.into(),
            description: None,
            base_profile,
            game_system: None,
            setting_name: None,
            is_builtin: false,
            tags: Vec::new(),
            tone_overrides: HashMap::new(),
            vocabulary: HashMap::new(),
            common_phrases: Vec::new(),
            cultural_markers: Vec::new(),
            campaign_id: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Set the game system.
    pub fn with_game_system(mut self, game_system: impl Into<String>) -> Self {
        self.game_system = Some(game_system.into());
        self
    }

    /// Set the setting name.
    pub fn with_setting_name(mut self, setting_name: impl Into<String>) -> Self {
        self.setting_name = Some(setting_name.into());
        self
    }

    /// Add a tone override.
    pub fn with_tone_override(mut self, dimension: impl Into<String>, intensity: f32) -> Self {
        self.tone_overrides
            .insert(dimension.into(), intensity.clamp(0.0, 1.0));
        self
    }

    /// Add vocabulary with frequency weight.
    pub fn with_vocabulary(mut self, term: impl Into<String>, frequency: f32) -> Self {
        self.vocabulary
            .insert(term.into(), frequency.clamp(0.0, 1.0));
        self
    }

    /// Add a common phrase.
    pub fn with_common_phrase(mut self, phrase: impl Into<String>) -> Self {
        self.common_phrases.push(phrase.into());
        self
    }

    /// Add a cultural marker.
    pub fn with_cultural_marker(mut self, marker: impl Into<String>) -> Self {
        self.cultural_markers.push(marker.into());
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Mark as a built-in template.
    pub fn as_builtin(mut self) -> Self {
        self.is_builtin = true;
        self
    }

    /// Touch the updated_at timestamp.
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

impl Default for SettingPersonalityTemplate {
    fn default() -> Self {
        Self::new("Unnamed Template", PersonalityId::new("default"))
    }
}

// ============================================================================
// Blend Rule Definition
// ============================================================================

/// A rule that defines how personalities should be blended for a specific context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlendRule {
    /// Unique identifier for the rule.
    pub id: BlendRuleId,

    /// Human-readable name for the rule.
    pub name: String,

    /// Optional description of what this rule does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The gameplay context this rule applies to.
    pub context: String,

    /// Priority for rule ordering (higher = evaluated first).
    #[serde(default)]
    pub priority: i32,

    /// Whether this rule is active.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Whether this is a built-in rule (not user-editable).
    #[serde(default)]
    pub is_builtin: bool,

    /// Campaign ID if this rule is campaign-specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_id: Option<String>,

    /// Blend components: map of personality ID to weight.
    pub blend_weights: HashMap<PersonalityId, f32>,

    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Creation timestamp (RFC 3339 format).
    pub created_at: String,

    /// Last update timestamp (RFC 3339 format).
    pub updated_at: String,
}

fn default_true() -> bool {
    true
}

impl BlendRule {
    /// Create a new blend rule for the given context.
    pub fn new(name: impl Into<String>, context: impl Into<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: BlendRuleId::generate(),
            name: name.into(),
            description: None,
            context: context.into(),
            priority: 0,
            enabled: true,
            is_builtin: false,
            campaign_id: None,
            blend_weights: HashMap::new(),
            tags: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Add a blend component with weight.
    pub fn with_component(mut self, personality_id: PersonalityId, weight: f32) -> Self {
        self.blend_weights
            .insert(personality_id, weight.clamp(0.0, 1.0));
        self
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Mark as built-in.
    pub fn as_builtin(mut self) -> Self {
        self.is_builtin = true;
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Normalize blend weights to sum to 1.0.
    pub fn normalize_weights(&mut self) {
        let sum: f32 = self.blend_weights.values().sum();
        if sum > 0.0 && (sum - 1.0).abs() > 0.001 {
            for weight in self.blend_weights.values_mut() {
                *weight /= sum;
            }
        }
    }

    /// Touch the updated_at timestamp.
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

impl Default for BlendRule {
    fn default() -> Self {
        Self::new("Default Rule", "unknown")
    }
}

// ============================================================================
// Blend Weight
// ============================================================================

/// A single component in a personality blend with its weight.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlendComponent {
    /// The personality ID.
    pub personality_id: PersonalityId,

    /// The weight for this component (0.0 to 1.0).
    pub weight: f32,
}

impl BlendComponent {
    /// Create a new blend component.
    pub fn new(personality_id: PersonalityId, weight: f32) -> Self {
        Self {
            personality_id,
            weight: weight.clamp(0.0, 1.0),
        }
    }
}

// ============================================================================
// Detection Result
// ============================================================================

/// Result of context detection with confidence scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextDetectionResult {
    /// The detected context.
    pub context: String,

    /// Confidence score for the detection (0.0 to 1.0).
    pub confidence: f32,

    /// Alternative contexts with their confidence scores.
    #[serde(default)]
    pub alternatives: Vec<(String, f32)>,

    /// Keywords that contributed to the detection.
    #[serde(default)]
    pub matched_keywords: Vec<String>,

    /// Whether the detection was ambiguous (multiple high-confidence results).
    #[serde(default)]
    pub is_ambiguous: bool,
}

impl ContextDetectionResult {
    /// Create a new detection result.
    pub fn new(context: impl Into<String>, confidence: f32) -> Self {
        Self {
            context: context.into(),
            confidence: confidence.clamp(0.0, 1.0),
            alternatives: Vec::new(),
            matched_keywords: Vec::new(),
            is_ambiguous: false,
        }
    }

    /// Add an alternative context.
    pub fn with_alternative(mut self, context: impl Into<String>, confidence: f32) -> Self {
        self.alternatives
            .push((context.into(), confidence.clamp(0.0, 1.0)));
        self
    }

    /// Add matched keywords.
    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.matched_keywords = keywords;
        self
    }

    /// Mark as ambiguous.
    pub fn mark_ambiguous(mut self) -> Self {
        self.is_ambiguous = true;
        self
    }
}

impl Default for ContextDetectionResult {
    fn default() -> Self {
        Self::new("unknown", 0.0)
    }
}

// ============================================================================
// Async-Safe Cache Type
// ============================================================================

/// Type alias for an async-safe cache using tokio's RwLock.
///
/// Use this instead of std::sync::RwLock for caches that need to be accessed
/// from async contexts.
pub type AsyncCache<K, V> = RwLock<HashMap<K, V>>;

/// Create a new empty async cache.
pub fn new_async_cache<K, V>() -> AsyncCache<K, V> {
    RwLock::new(HashMap::new())
}

// ============================================================================
// Meilisearch Document Types (for IPC)
// ============================================================================

/// Meilisearch document representation of a personality template.
///
/// Uses camelCase for frontend IPC compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDocument {
    /// Primary key for Meilisearch.
    pub id: String,

    /// Template name (searchable).
    pub name: String,

    /// Template description (searchable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Base profile ID.
    pub base_profile: String,

    /// Game system (filterable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_system: Option<String>,

    /// Setting name (filterable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting_name: Option<String>,

    /// Built-in flag (filterable).
    #[serde(default)]
    pub is_builtin: bool,

    /// Tags (filterable).
    #[serde(default)]
    pub tags: Vec<String>,

    /// Campaign ID (filterable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_id: Option<String>,

    /// Vocabulary keys for search.
    #[serde(default)]
    pub vocabulary_keys: Vec<String>,

    /// Common phrases for search.
    #[serde(default)]
    pub common_phrases: Vec<String>,

    /// Creation timestamp (sortable).
    pub created_at: String,

    /// Update timestamp (sortable).
    pub updated_at: String,
}

impl From<SettingPersonalityTemplate> for TemplateDocument {
    fn from(template: SettingPersonalityTemplate) -> Self {
        Self {
            id: template.id.into_inner(),
            name: template.name,
            description: template.description,
            base_profile: template.base_profile.into_inner(),
            game_system: template.game_system,
            setting_name: template.setting_name,
            is_builtin: template.is_builtin,
            tags: template.tags,
            campaign_id: template.campaign_id,
            vocabulary_keys: template.vocabulary.keys().cloned().collect(),
            common_phrases: template.common_phrases,
            created_at: template.created_at,
            updated_at: template.updated_at,
        }
    }
}

/// A single blend weight entry for serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlendWeightEntry {
    /// Personality ID.
    pub personality_id: String,

    /// Weight (0.0-1.0).
    pub weight: f32,
}

/// Meilisearch document representation of a blend rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlendRuleDocument {
    /// Primary key for Meilisearch.
    pub id: String,

    /// Rule name (searchable).
    pub name: String,

    /// Rule description (searchable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Context this rule applies to (filterable).
    pub context: String,

    /// Priority (sortable).
    pub priority: i32,

    /// Whether the rule is enabled (filterable).
    pub enabled: bool,

    /// Built-in flag (filterable).
    pub is_builtin: bool,

    /// Campaign ID (filterable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_id: Option<String>,

    /// Tags (filterable).
    #[serde(default)]
    pub tags: Vec<String>,

    /// Blend weights as a vec of (personality_id, weight) pairs.
    /// Stored as a nested structure for Meilisearch.
    #[serde(default)]
    pub blend_weights: Vec<BlendWeightEntry>,

    /// Creation timestamp (sortable).
    pub created_at: String,

    /// Update timestamp (sortable).
    pub updated_at: String,
}

impl From<BlendRule> for BlendRuleDocument {
    fn from(rule: BlendRule) -> Self {
        let blend_weights = rule
            .blend_weights
            .iter()
            .map(|(id, w)| BlendWeightEntry {
                personality_id: id.to_string(),
                weight: *w,
            })
            .collect();

        Self {
            id: rule.id.into_inner(),
            name: rule.name,
            description: rule.description,
            context: rule.context,
            priority: rule.priority,
            enabled: rule.enabled,
            is_builtin: rule.is_builtin,
            campaign_id: rule.campaign_id,
            tags: rule.tags,
            blend_weights,
            created_at: rule.created_at,
            updated_at: rule.updated_at,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_id_creation() {
        let id1 = TemplateId::new("test-id");
        assert_eq!(id1.as_str(), "test-id");
        assert_eq!(id1.to_string(), "test-id");

        let id2 = TemplateId::generate();
        assert!(!id2.as_str().is_empty());
        assert!(id2.as_str().contains('-')); // UUID format

        let id3: TemplateId = "from-str".into();
        assert_eq!(id3.as_str(), "from-str");

        let id4: TemplateId = String::from("from-string").into();
        assert_eq!(id4.as_str(), "from-string");
    }

    #[test]
    fn test_personality_id_creation() {
        let id1 = PersonalityId::new("storyteller");
        assert_eq!(id1.as_str(), "storyteller");

        let id2 = PersonalityId::generate();
        assert!(!id2.as_str().is_empty());
    }

    #[test]
    fn test_template_id_equality() {
        let id1 = TemplateId::new("same-id");
        let id2 = TemplateId::new("same-id");
        let id3 = TemplateId::new("different-id");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_template_id_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(TemplateId::new("id1"));
        set.insert(TemplateId::new("id2"));
        set.insert(TemplateId::new("id1")); // Duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_setting_personality_template_builder() {
        let template = SettingPersonalityTemplate::new(
            "Forgotten Realms Sage",
            PersonalityId::new("storyteller"),
        )
        .with_game_system("dnd5e")
        .with_setting_name("Forgotten Realms")
        .with_tone_override("scholarly", 0.9)
        .with_tone_override("mysterious", 0.4)
        .with_vocabulary("ancient texts", 0.05)
        .with_vocabulary("Mystra's blessing", 0.03)
        .with_common_phrase("As the annals of Candlekeep record")
        .with_common_phrase("The Weave reveals")
        .with_cultural_marker("References Mystra, Oghma")
        .with_cultural_marker("Uses 'tenday' instead of 'week'")
        .with_tag("sage")
        .with_tag("lore");

        assert_eq!(template.name, "Forgotten Realms Sage");
        assert_eq!(template.base_profile.as_str(), "storyteller");
        assert_eq!(template.game_system, Some("dnd5e".to_string()));
        assert_eq!(template.setting_name, Some("Forgotten Realms".to_string()));
        assert_eq!(template.tone_overrides.get("scholarly"), Some(&0.9));
        assert_eq!(template.vocabulary.len(), 2);
        assert_eq!(template.common_phrases.len(), 2);
        assert_eq!(template.cultural_markers.len(), 2);
        assert_eq!(template.tags.len(), 2);
        assert!(!template.is_builtin);
    }

    #[test]
    fn test_blend_rule_builder() {
        let rule = BlendRule::new("Combat Blend", "combat_encounter")
            .with_component(PersonalityId::new("tactical_advisor"), 0.6)
            .with_component(PersonalityId::new("active"), 0.4)
            .with_priority(10)
            .with_tag("combat")
            .as_builtin();

        assert_eq!(rule.name, "Combat Blend");
        assert_eq!(rule.context, "combat_encounter");
        assert_eq!(rule.priority, 10);
        assert!(rule.is_builtin);
        assert_eq!(rule.blend_weights.len(), 2);
        assert_eq!(
            rule.blend_weights.get(&PersonalityId::new("tactical_advisor")),
            Some(&0.6)
        );
    }

    #[test]
    fn test_blend_rule_normalize_weights() {
        let rule = BlendRule::new("Test", "test")
            .with_component(PersonalityId::new("a"), 0.3)
            .with_component(PersonalityId::new("b"), 0.3)
            .with_component(PersonalityId::new("c"), 0.4);

        // Already normalized
        let sum_before: f32 = rule.blend_weights.values().sum();
        assert!((sum_before - 1.0).abs() < 0.001);

        // Test with unnormalized weights
        let mut rule2 = BlendRule::new("Test2", "test")
            .with_component(PersonalityId::new("a"), 0.5)
            .with_component(PersonalityId::new("b"), 0.5)
            .with_component(PersonalityId::new("c"), 0.5);

        rule2.normalize_weights();
        let sum_after: f32 = rule2.blend_weights.values().sum();
        assert!((sum_after - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_context_detection_result() {
        let result = ContextDetectionResult::new("combat_encounter", 0.85)
            .with_alternative("exploration", 0.15)
            .with_keywords(vec!["initiative".to_string(), "attack".to_string()]);

        assert_eq!(result.context, "combat_encounter");
        assert_eq!(result.confidence, 0.85);
        assert_eq!(result.alternatives.len(), 1);
        assert_eq!(result.matched_keywords.len(), 2);
        assert!(!result.is_ambiguous);
    }

    #[test]
    fn test_template_document_from_template() {
        let template =
            SettingPersonalityTemplate::new("Test Template", PersonalityId::new("base"))
                .with_vocabulary("term1", 0.5)
                .with_vocabulary("term2", 0.3)
                .with_common_phrase("phrase1")
                .with_tag("tag1");

        let doc: TemplateDocument = template.into();

        assert_eq!(doc.name, "Test Template");
        assert_eq!(doc.base_profile, "base");
        assert_eq!(doc.vocabulary_keys.len(), 2);
        assert!(doc.vocabulary_keys.contains(&"term1".to_string()));
        assert_eq!(doc.common_phrases.len(), 1);
        assert_eq!(doc.tags.len(), 1);
    }

    #[test]
    fn test_blend_rule_document_from_rule() {
        let rule = BlendRule::new("Test Rule", "combat")
            .with_priority(5)
            .with_tag("test")
            .as_builtin();

        let doc: BlendRuleDocument = rule.into();

        assert_eq!(doc.name, "Test Rule");
        assert_eq!(doc.context, "combat");
        assert_eq!(doc.priority, 5);
        assert!(doc.is_builtin);
        assert!(doc.enabled);
    }

    #[test]
    fn test_weight_clamping() {
        let component = BlendComponent::new(PersonalityId::new("test"), 1.5);
        assert_eq!(component.weight, 1.0);

        let component2 = BlendComponent::new(PersonalityId::new("test"), -0.5);
        assert_eq!(component2.weight, 0.0);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let template =
            SettingPersonalityTemplate::new("Test", PersonalityId::new("base")).with_tag("tag");

        let json = serde_json::to_string(&template).unwrap();
        let parsed: SettingPersonalityTemplate = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, template.name);
        assert_eq!(parsed.tags, template.tags);
    }

    #[test]
    fn test_camel_case_serialization() {
        let template = SettingPersonalityTemplate::new("Test", PersonalityId::new("base"))
            .with_game_system("dnd5e")
            .with_setting_name("Eberron");

        let json = serde_json::to_string(&template).unwrap();

        // Verify camelCase field names
        assert!(json.contains("\"gameSystem\""));
        assert!(json.contains("\"settingName\""));
        assert!(json.contains("\"baseProfile\""));
        assert!(json.contains("\"isBuiltin\""));
        assert!(json.contains("\"createdAt\""));
        assert!(json.contains("\"updatedAt\""));
    }
}
