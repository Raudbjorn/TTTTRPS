//! Personality Blender Core Logic (TASK-PERS-009)
//!
//! Implements dynamic personality blending based on context.
//! Uses weighted interpolation across all personality dimensions.
//!
//! ## Blending Rules
//!
//! - **Numeric fields**: Weighted average (formality, trait intensities)
//! - **Categorical fields**: Select from the highest weight component
//! - **List fields**: Proportional selection based on weights
//! - **Tone scores**: Normalized to sum=1.0
//! - **Formality**: Clamped to [1, 10]
//!
//! ## Caching
//!
//! Uses LRU cache (capacity 100) with tokio::sync::Mutex for async safety.

use super::context::GameplayContext;
use super::errors::{BlendError, PersonalityExtensionError};
use super::types::{BlendComponent, PersonalityId};
use crate::core::personality_base::{
    BehavioralTendencies, PersonalityProfile, PersonalityTrait, SpeechPatterns,
};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use tokio::sync::Mutex;

// ============================================================================
// Constants
// ============================================================================

/// Default LRU cache capacity for blended profiles.
pub const DEFAULT_CACHE_CAPACITY: usize = 100;

/// Weight tolerance for normalization checks.
pub const WEIGHT_TOLERANCE: f32 = 0.001;

/// Minimum formality value.
pub const MIN_FORMALITY: u8 = 1;

/// Maximum formality value.
pub const MAX_FORMALITY: u8 = 10;

/// Multiplier for list blending proportional selection.
/// Uses 2x to ensure adequate representation from lower-weight profiles.
/// Without this, profiles with weight 0.2 contributing only 20% of items
/// may not contribute meaningfully to small lists.
const BLEND_LIST_MULTIPLIER: f32 = 2.0;

// ============================================================================
// BlendSpec - Specification for a blend operation
// ============================================================================

/// Specification for blending multiple personalities.
///
/// Weights are stored as u8 percentages (0-100) for consistency
/// and efficient hashing. Use `weights_as_fractions()` to convert
/// to f32 0.0-1.0 range for calculations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlendSpec {
    /// Components to blend with their weights (as percentages 0-100).
    components: Vec<BlendComponentSpec>,

    /// Optional context hint for the blend.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    context: Option<GameplayContext>,
}

/// A component in a blend specification with percentage weight.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlendComponentSpec {
    /// Personality ID.
    pub personality_id: PersonalityId,

    /// Weight as percentage (0-100).
    pub weight_percent: u8,
}

impl BlendSpec {
    /// Create a new BlendSpec from components with f32 weights.
    ///
    /// Weights are converted to percentages (0-100) and must sum to ~1.0.
    pub fn new(components: Vec<BlendComponent>) -> Result<Self, BlendError> {
        if components.is_empty() {
            return Err(BlendError::EmptyComponents);
        }

        // Validate individual weights
        for c in &components {
            if c.weight < 0.0 || c.weight > 1.0 {
                return Err(BlendError::weight_out_of_range(
                    c.personality_id.to_string(),
                    c.weight,
                ));
            }
        }

        // Check weights sum to 1.0 (within tolerance)
        let sum: f32 = components.iter().map(|c| c.weight).sum();
        if (sum - 1.0).abs() > WEIGHT_TOLERANCE {
            log::warn!(
                "Blend weights sum to {:.4}, normalizing to 1.0",
                sum
            );
        }

        // Normalize and convert to percentages
        let normalized: Vec<BlendComponentSpec> = components
            .iter()
            .map(|c| {
                let normalized_weight = if sum > 0.0 { c.weight / sum } else { 0.0 };
                BlendComponentSpec {
                    personality_id: c.personality_id.clone(),
                    weight_percent: (normalized_weight * 100.0).round() as u8,
                }
            })
            .collect();

        Ok(Self {
            components: normalized,
            context: None,
        })
    }

    /// Create a BlendSpec with context hint.
    pub fn with_context(mut self, context: GameplayContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Create from percentage weights directly.
    ///
    /// Validates that weights sum to exactly 100.
    pub fn from_percentages(
        components: Vec<(PersonalityId, u8)>,
    ) -> Result<Self, BlendError> {
        if components.is_empty() {
            return Err(BlendError::EmptyComponents);
        }

        let sum: u16 = components.iter().map(|(_, w)| *w as u16).sum();
        if sum != 100 {
            return Err(BlendError::invalid_weight_sum(
                sum as f32 / 100.0,
                0.0,
            ));
        }

        let specs: Vec<BlendComponentSpec> = components
            .into_iter()
            .map(|(id, w)| BlendComponentSpec {
                personality_id: id,
                weight_percent: w,
            })
            .collect();

        Ok(Self {
            components: specs,
            context: None,
        })
    }

    /// Get components as f32 fractions (0.0-1.0).
    pub fn weights_as_fractions(&self) -> Vec<(&PersonalityId, f32)> {
        self.components
            .iter()
            .map(|c| (&c.personality_id, c.weight_percent as f32 / 100.0))
            .collect()
    }

    /// Get the components.
    pub fn components(&self) -> &[BlendComponentSpec] {
        &self.components
    }

    /// Get the context hint.
    pub fn context(&self) -> Option<&GameplayContext> {
        self.context.as_ref()
    }

    /// Check if this spec is valid.
    pub fn is_valid(&self) -> bool {
        if self.components.is_empty() {
            return false;
        }
        let sum: u16 = self.components.iter().map(|c| c.weight_percent as u16).sum();
        sum == 100
    }

    /// Create a cache key for this spec.
    fn cache_key(&self) -> BlendCacheKey {
        BlendCacheKey::from_spec(self)
    }
}

// Implement Hash, PartialEq, Eq for BlendSpec (for cache key)
impl Hash for BlendSpec {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Sort components by personality_id for consistent hashing
        let mut sorted: Vec<_> = self.components.iter().collect();
        sorted.sort_by(|a, b| a.personality_id.as_str().cmp(b.personality_id.as_str()));

        for c in sorted {
            c.personality_id.as_str().hash(state);
            c.weight_percent.hash(state);
        }
        self.context.hash(state);
    }
}

impl PartialEq for BlendSpec {
    fn eq(&self, other: &Self) -> bool {
        if self.context != other.context {
            return false;
        }
        if self.components.len() != other.components.len() {
            return false;
        }

        // Sort and compare
        let mut self_sorted: Vec<_> = self.components.iter().collect();
        let mut other_sorted: Vec<_> = other.components.iter().collect();
        self_sorted.sort_by(|a, b| a.personality_id.as_str().cmp(b.personality_id.as_str()));
        other_sorted.sort_by(|a, b| a.personality_id.as_str().cmp(b.personality_id.as_str()));

        for (a, b) in self_sorted.iter().zip(other_sorted.iter()) {
            if a.personality_id != b.personality_id || a.weight_percent != b.weight_percent {
                return false;
            }
        }
        true
    }
}

impl Eq for BlendSpec {}

// ============================================================================
// Cache Key
// ============================================================================

/// Cache key for blended profiles.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BlendCacheKey {
    /// Sorted component IDs with weights for consistent hashing.
    components: Vec<(String, u8)>,
    /// Optional context.
    context: Option<GameplayContext>,
}

impl BlendCacheKey {
    fn from_spec(spec: &BlendSpec) -> Self {
        let mut components: Vec<(String, u8)> = spec
            .components
            .iter()
            .map(|c| (c.personality_id.to_string(), c.weight_percent))
            .collect();
        components.sort_by(|a, b| a.0.cmp(&b.0));

        Self {
            components,
            context: spec.context,
        }
    }
}

// ============================================================================
// Blended Profile Result
// ============================================================================

/// Result of a personality blend operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlendedProfile {
    /// The blended personality profile.
    pub profile: PersonalityProfile,

    /// The spec used to create this blend.
    pub spec: BlendSpec,

    /// Whether this came from cache.
    pub from_cache: bool,
}

// ============================================================================
// PersonalityBlender
// ============================================================================

/// Blends multiple personality profiles based on weighted specifications.
///
/// Uses an LRU cache with tokio::sync::Mutex for async-safe caching.
pub struct PersonalityBlender {
    /// LRU cache for blended profiles.
    cache: Mutex<LruCache<BlendCacheKey, PersonalityProfile>>,
}

impl PersonalityBlender {
    /// Create a new blender with default cache capacity.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CACHE_CAPACITY)
    }

    /// Create a new blender with custom cache capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(1).unwrap());
        Self {
            cache: Mutex::new(LruCache::new(cap)),
        }
    }

    /// Blend personalities according to the given specification.
    ///
    /// Looks up profiles by ID from the provided map and blends them.
    pub async fn blend(
        &self,
        spec: &BlendSpec,
        profiles: &HashMap<PersonalityId, PersonalityProfile>,
    ) -> Result<BlendedProfile, PersonalityExtensionError> {
        // Check cache first
        let cache_key = spec.cache_key();
        {
            let mut cache = self.cache.lock().await;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(BlendedProfile {
                    profile: cached.clone(),
                    spec: spec.clone(),
                    from_cache: true,
                });
            }
        }

        // Gather profiles
        let mut weighted_profiles: Vec<(&PersonalityProfile, f32)> = Vec::new();
        for (id, weight) in spec.weights_as_fractions() {
            let profile = profiles
                .get(id)
                .ok_or_else(|| BlendError::profile_not_found(id.to_string()))?;
            weighted_profiles.push((profile, weight));
        }

        // Perform blend
        let blended = self.blend_profiles(&weighted_profiles, spec.context())?;

        // Cache result
        {
            let mut cache = self.cache.lock().await;
            cache.put(cache_key, blended.clone());
        }

        Ok(BlendedProfile {
            profile: blended,
            spec: spec.clone(),
            from_cache: false,
        })
    }

    /// Clear the blend cache.
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
        log::debug!("Cleared personality blend cache");
    }

    /// Get cache statistics.
    pub async fn cache_stats(&self) -> BlenderCacheStats {
        let cache = self.cache.lock().await;
        BlenderCacheStats {
            len: cache.len(),
            cap: cache.cap().get(),
        }
    }

    /// Perform the actual blending of profiles.
    fn blend_profiles(
        &self,
        profiles: &[(&PersonalityProfile, f32)],
        context: Option<&GameplayContext>,
    ) -> Result<PersonalityProfile, PersonalityExtensionError> {
        if profiles.is_empty() {
            return Err(BlendError::EmptyComponents.into());
        }

        // If only one profile, return a clone
        if profiles.len() == 1 {
            let (profile, _) = profiles[0];
            let mut result = profile.clone();
            result.id = format!("blend_{}", uuid::Uuid::new_v4());
            return Ok(result);
        }

        // Blend each dimension
        let speech_patterns = self.blend_speech_patterns(profiles)?;
        let traits = self.blend_traits(profiles)?;
        let behavioral_tendencies = self.blend_behavioral_tendencies(profiles)?;
        let knowledge_areas = self.blend_list_field(
            profiles,
            |p| &p.knowledge_areas,
        );
        let example_phrases = self.blend_list_field(
            profiles,
            |p| &p.example_phrases,
        );
        let tags = self.blend_tags(profiles);
        let metadata = self.blend_metadata(profiles, context);

        let now = chrono::Utc::now().to_rfc3339();

        Ok(PersonalityProfile {
            id: format!("blend_{}", uuid::Uuid::new_v4()),
            name: self.generate_blend_name(profiles, context),
            source: Some(format!(
                "Blended from {} profiles",
                profiles.len()
            )),
            speech_patterns,
            traits,
            knowledge_areas,
            behavioral_tendencies,
            example_phrases,
            tags,
            metadata,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Blend speech patterns using weighted averages and selection.
    fn blend_speech_patterns(
        &self,
        profiles: &[(&PersonalityProfile, f32)],
    ) -> Result<SpeechPatterns, PersonalityExtensionError> {
        // Formality: weighted average, clamped
        let formality_weighted: f32 = profiles
            .iter()
            .map(|(p, w)| p.speech_patterns.formality as f32 * w)
            .sum();
        let formality = (formality_weighted.round() as u8)
            .max(MIN_FORMALITY)
            .min(MAX_FORMALITY);

        // Common phrases: proportional selection
        let common_phrases = self.blend_list_field(
            profiles,
            |p| &p.speech_patterns.common_phrases,
        );

        // Vocabulary style: select from highest weight
        let vocabulary_style = profiles
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(p, _)| p.speech_patterns.vocabulary_style.clone())
            .unwrap_or_default();

        // Dialect notes: select from highest weight (if any)
        let dialect_notes = profiles
            .iter()
            .filter(|(p, _)| p.speech_patterns.dialect_notes.is_some())
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .and_then(|(p, _)| p.speech_patterns.dialect_notes.clone());

        // Pacing: select from highest weight
        let pacing = profiles
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(p, _)| p.speech_patterns.pacing.clone())
            .unwrap_or_default();

        Ok(SpeechPatterns {
            formality,
            common_phrases,
            vocabulary_style,
            dialect_notes,
            pacing,
        })
    }

    /// Blend personality traits using weighted selection and averaging.
    fn blend_traits(
        &self,
        profiles: &[(&PersonalityProfile, f32)],
    ) -> Result<Vec<PersonalityTrait>, PersonalityExtensionError> {
        // Collect all traits with their weighted intensities
        let mut trait_map: HashMap<String, (f32, String)> = HashMap::new();

        for (profile, weight) in profiles {
            for trait_info in &profile.traits {
                let weighted_intensity = trait_info.intensity as f32 * weight;
                let entry = trait_map
                    .entry(trait_info.trait_name.to_lowercase())
                    .or_insert((0.0, trait_info.manifestation.clone()));
                entry.0 += weighted_intensity;
                // Keep manifestation from higher weight
                if *weight > 0.5 {
                    entry.1 = trait_info.manifestation.clone();
                }
            }
        }

        // Convert back to traits, filtering low-intensity ones
        let mut traits: Vec<PersonalityTrait> = trait_map
            .into_iter()
            .filter(|(_, (intensity, _))| *intensity >= 1.0)
            .map(|(name, (intensity, manifestation))| PersonalityTrait {
                trait_name: name,
                intensity: (intensity.round() as u8).min(10).max(1),
                manifestation,
            })
            .collect();

        // Sort by intensity descending, take top 5-7
        traits.sort_by(|a, b| b.intensity.cmp(&a.intensity));
        traits.truncate(7);

        Ok(traits)
    }

    /// Blend behavioral tendencies by selecting from the highest weight.
    fn blend_behavioral_tendencies(
        &self,
        profiles: &[(&PersonalityProfile, f32)],
    ) -> Result<BehavioralTendencies, PersonalityExtensionError> {
        // For behavioral tendencies, select from highest weight profile
        // as these are categorical/descriptive fields
        let primary = profiles
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(p, _)| p)
            .unwrap();

        Ok(primary.behavioral_tendencies.clone())
    }

    /// Blend a list field using proportional selection.
    fn blend_list_field<F>(
        &self,
        profiles: &[(&PersonalityProfile, f32)],
        extractor: F,
    ) -> Vec<String>
    where
        F: Fn(&PersonalityProfile) -> &Vec<String>,
    {
        let mut result: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        for (profile, weight) in profiles {
            let items = extractor(profile);
            // Take proportional number of items based on weight, with multiplier for representation
            let count = (items.len() as f32 * weight * BLEND_LIST_MULTIPLIER).ceil() as usize;
            for item in items.iter().take(count.max(1)) {
                let item_lower = item.to_lowercase();
                if !seen.contains(&item_lower) {
                    seen.insert(item_lower);
                    result.push(item.clone());
                }
            }
        }

        result
    }

    /// Blend tags from all profiles.
    fn blend_tags(&self, profiles: &[(&PersonalityProfile, f32)]) -> Vec<String> {
        let mut tags: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Add "blended" tag
        tags.push("blended".to_string());
        seen.insert("blended".to_string());

        for (profile, _) in profiles {
            for tag in &profile.tags {
                let tag_lower = tag.to_lowercase();
                if !seen.contains(&tag_lower) {
                    seen.insert(tag_lower);
                    tags.push(tag.clone());
                }
            }
        }

        tags
    }

    /// Blend metadata from all profiles.
    fn blend_metadata(
        &self,
        profiles: &[(&PersonalityProfile, f32)],
        context: Option<&GameplayContext>,
    ) -> HashMap<String, String> {
        let mut metadata: HashMap<String, String> = HashMap::new();

        // Add blend info
        let component_ids: Vec<String> = profiles
            .iter()
            .map(|(p, w)| format!("{}:{:.0}%", p.id, w * 100.0))
            .collect();
        metadata.insert("blend_components".to_string(), component_ids.join(", "));
        metadata.insert(
            "blend_timestamp".to_string(),
            chrono::Utc::now().to_rfc3339(),
        );

        if let Some(ctx) = context {
            metadata.insert("blend_context".to_string(), ctx.as_str().to_string());
        }

        // Merge metadata from highest weight profile
        if let Some((primary, _)) = profiles
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        {
            for (k, v) in &primary.metadata {
                if !metadata.contains_key(k) {
                    metadata.insert(k.clone(), v.clone());
                }
            }
        }

        metadata
    }

    /// Generate a name for the blended profile.
    fn generate_blend_name(
        &self,
        profiles: &[(&PersonalityProfile, f32)],
        context: Option<&GameplayContext>,
    ) -> String {
        // Get names of top 2 contributors
        let mut sorted: Vec<_> = profiles.iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let names: Vec<&str> = sorted
            .iter()
            .take(2)
            .map(|(p, _)| p.name.as_str())
            .collect();

        let context_suffix = context
            .map(|c| format!(" ({})", c.display_name()))
            .unwrap_or_default();

        if names.len() == 1 {
            format!("{}{}", names[0], context_suffix)
        } else {
            format!("{} + {}{}", names[0], names[1], context_suffix)
        }
    }
}

impl Default for PersonalityBlender {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Cache Stats
// ============================================================================

/// Statistics about the blender cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlenderCacheStats {
    /// Number of entries in cache.
    pub len: usize,
    /// Cache capacity.
    pub cap: usize,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_profile(id: &str, name: &str, formality: u8) -> PersonalityProfile {
        let now = chrono::Utc::now().to_rfc3339();
        PersonalityProfile {
            id: id.to_string(),
            name: name.to_string(),
            source: None,
            speech_patterns: SpeechPatterns {
                formality,
                common_phrases: vec![format!("{} phrase 1", name), format!("{} phrase 2", name)],
                vocabulary_style: format!("{} vocabulary", name),
                dialect_notes: Some(format!("{} dialect", name)),
                pacing: format!("{} pacing", name),
            },
            traits: vec![
                PersonalityTrait {
                    trait_name: format!("{}_trait", name),
                    intensity: 7,
                    manifestation: format!("{} trait manifestation", name),
                },
            ],
            knowledge_areas: vec![format!("{} knowledge", name)],
            behavioral_tendencies: BehavioralTendencies {
                conflict_response: format!("{} conflict", name),
                stranger_response: format!("{} stranger", name),
                authority_response: format!("{} authority", name),
                help_response: format!("{} help", name),
                general_attitude: format!("{} attitude", name),
            },
            example_phrases: vec![format!("{} example", name)],
            tags: vec![format!("{}_tag", name)],
            metadata: HashMap::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    #[test]
    fn test_blend_spec_creation() {
        let components = vec![
            BlendComponent::new(PersonalityId::new("a"), 0.6),
            BlendComponent::new(PersonalityId::new("b"), 0.4),
        ];

        let spec = BlendSpec::new(components).unwrap();
        assert!(spec.is_valid());

        let fractions = spec.weights_as_fractions();
        let sum: f32 = fractions.iter().map(|(_, w)| w).sum();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_blend_spec_normalization() {
        // Weights that don't sum to 1.0
        let components = vec![
            BlendComponent::new(PersonalityId::new("a"), 0.3),
            BlendComponent::new(PersonalityId::new("b"), 0.3),
        ];

        let spec = BlendSpec::new(components).unwrap();
        assert!(spec.is_valid());

        // Should be normalized
        let fractions = spec.weights_as_fractions();
        let sum: f32 = fractions.iter().map(|(_, w)| w).sum();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_blend_spec_from_percentages() {
        let spec = BlendSpec::from_percentages(vec![
            (PersonalityId::new("a"), 60),
            (PersonalityId::new("b"), 40),
        ])
        .unwrap();

        assert!(spec.is_valid());

        let fractions = spec.weights_as_fractions();
        assert_eq!(fractions.len(), 2);
    }

    #[test]
    fn test_blend_spec_invalid_percentages() {
        let result = BlendSpec::from_percentages(vec![
            (PersonalityId::new("a"), 60),
            (PersonalityId::new("b"), 30), // Only 90%
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn test_blend_spec_empty() {
        let result = BlendSpec::new(vec![]);
        assert!(matches!(result, Err(BlendError::EmptyComponents)));
    }

    #[test]
    fn test_blend_spec_hash_eq() {
        let spec1 = BlendSpec::from_percentages(vec![
            (PersonalityId::new("a"), 60),
            (PersonalityId::new("b"), 40),
        ])
        .unwrap();

        let spec2 = BlendSpec::from_percentages(vec![
            (PersonalityId::new("b"), 40),
            (PersonalityId::new("a"), 60),
        ])
        .unwrap();

        // Should be equal regardless of order
        assert_eq!(spec1, spec2);

        // Hash should match
        use std::collections::hash_map::DefaultHasher;
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        spec1.hash(&mut h1);
        spec2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[tokio::test]
    async fn test_blender_basic_blend() {
        let blender = PersonalityBlender::new();

        let profile_a = sample_profile("a", "Profile A", 3);
        let profile_b = sample_profile("b", "Profile B", 9);

        let mut profiles = HashMap::new();
        profiles.insert(PersonalityId::new("a"), profile_a);
        profiles.insert(PersonalityId::new("b"), profile_b);

        let spec = BlendSpec::from_percentages(vec![
            (PersonalityId::new("a"), 60),
            (PersonalityId::new("b"), 40),
        ])
        .unwrap();

        let result = blender.blend(&spec, &profiles).await.unwrap();

        // Formality should be weighted average: 0.6*3 + 0.4*9 = 1.8 + 3.6 = 5.4 -> 5
        assert!(result.profile.speech_patterns.formality >= 5);
        assert!(result.profile.speech_patterns.formality <= 6);

        // Should not be from cache on first call
        assert!(!result.from_cache);
    }

    #[tokio::test]
    async fn test_blender_caching() {
        let blender = PersonalityBlender::new();

        let profile_a = sample_profile("a", "Profile A", 5);
        let mut profiles = HashMap::new();
        profiles.insert(PersonalityId::new("a"), profile_a);

        let spec = BlendSpec::from_percentages(vec![
            (PersonalityId::new("a"), 100),
        ])
        .unwrap();

        // First call - not cached
        let result1 = blender.blend(&spec, &profiles).await.unwrap();
        assert!(!result1.from_cache);

        // Second call - should be cached
        let result2 = blender.blend(&spec, &profiles).await.unwrap();
        assert!(result2.from_cache);

        // Check cache stats
        let stats = blender.cache_stats().await;
        assert_eq!(stats.len, 1);
    }

    #[tokio::test]
    async fn test_blender_clear_cache() {
        let blender = PersonalityBlender::new();

        let profile_a = sample_profile("a", "Profile A", 5);
        let mut profiles = HashMap::new();
        profiles.insert(PersonalityId::new("a"), profile_a);

        let spec = BlendSpec::from_percentages(vec![
            (PersonalityId::new("a"), 100),
        ])
        .unwrap();

        // Populate cache
        let _ = blender.blend(&spec, &profiles).await.unwrap();
        assert_eq!(blender.cache_stats().await.len, 1);

        // Clear
        blender.clear_cache().await;
        assert_eq!(blender.cache_stats().await.len, 0);
    }

    #[tokio::test]
    async fn test_blender_profile_not_found() {
        let blender = PersonalityBlender::new();
        let profiles: HashMap<PersonalityId, PersonalityProfile> = HashMap::new();

        let spec = BlendSpec::from_percentages(vec![
            (PersonalityId::new("nonexistent"), 100),
        ])
        .unwrap();

        let result = blender.blend(&spec, &profiles).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_formality_clamping() {
        // Test edge cases for formality weighted average
        let blender = PersonalityBlender::new();

        let profile_a = sample_profile("a", "Profile A", 1);
        let profile_b = sample_profile("b", "Profile B", 1);

        let profiles = vec![(&profile_a, 0.5), (&profile_b, 0.5)];
        let result = blender.blend_speech_patterns(&profiles).unwrap();

        // Should be at least MIN_FORMALITY
        assert!(result.formality >= MIN_FORMALITY);
        assert!(result.formality <= MAX_FORMALITY);
    }

    #[test]
    fn test_trait_blending() {
        let blender = PersonalityBlender::new();

        let mut profile_a = sample_profile("a", "Profile A", 5);
        profile_a.traits = vec![
            PersonalityTrait {
                trait_name: "brave".to_string(),
                intensity: 8,
                manifestation: "Charges into danger".to_string(),
            },
        ];

        let mut profile_b = sample_profile("b", "Profile B", 5);
        profile_b.traits = vec![
            PersonalityTrait {
                trait_name: "brave".to_string(),
                intensity: 4,
                manifestation: "Stands ground".to_string(),
            },
            PersonalityTrait {
                trait_name: "cautious".to_string(),
                intensity: 7,
                manifestation: "Plans carefully".to_string(),
            },
        ];

        let profiles = vec![(&profile_a, 0.6), (&profile_b, 0.4)];
        let result = blender.blend_traits(&profiles).unwrap();

        // Should have both traits combined
        assert!(result.iter().any(|t| t.trait_name == "brave"));

        // Brave intensity should be weighted: 0.6*8 + 0.4*4 = 4.8 + 1.6 = 6.4 -> 6
        let brave = result.iter().find(|t| t.trait_name == "brave").unwrap();
        assert!(brave.intensity >= 5 && brave.intensity <= 7);
    }

    #[test]
    fn test_blend_name_generation() {
        let blender = PersonalityBlender::new();

        let profile_a = sample_profile("a", "Warrior", 5);
        let profile_b = sample_profile("b", "Sage", 5);

        let profiles = vec![(&profile_a, 0.7), (&profile_b, 0.3)];
        let name = blender.generate_blend_name(&profiles, None);

        assert!(name.contains("Warrior"));
        assert!(name.contains("Sage"));
    }

    #[test]
    fn test_blend_name_with_context() {
        let blender = PersonalityBlender::new();

        let profile_a = sample_profile("a", "Warrior", 5);
        let profiles = vec![(&profile_a, 1.0)];

        let name = blender.generate_blend_name(&profiles, Some(&GameplayContext::CombatEncounter));

        assert!(name.contains("Warrior"));
        assert!(name.contains("Combat"));
    }
}
