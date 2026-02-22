//! Archetype Resolution Engine with Hierarchical Merging.
//!
//! The [`ArchetypeResolver`] implements the multi-layer resolution strategy
//! that combines archetypes from different sources into a final resolved form.
//!
//! # Resolution Order
//!
//! Resolution follows strict priority ordering (lowest to highest):
//!
//! ```text
//! Priority (lowest to highest):
//! 1. Base Role (e.g., "merchant" archetype)
//! 2. Race (e.g., "dwarf" archetype)
//! 3. Class (e.g., "fighter" archetype)
//! 4. Setting Pack Override (e.g., "forgotten_realms" overrides)
//! 5. Direct Archetype ID (explicit archetype takes final precedence)
//! ```
//!
//! # Merge Semantics
//!
//! - **Scalar fields**: Later layers override earlier ones
//! - **Arrays**: Replace entirely (not merged)
//! - **Optional fields**: Only override if the later layer provides `Some` value
//! - **Vocabulary banks**: Merge with overlay precedence
//!
//! # Lock-Free Pattern (CRITICAL-ARCH-001)
//!
//! Inheritance chain resolution uses a lock-free pattern to prevent deadlocks:
//!
//! 1. Collect all inheritance chain IDs in a single lock acquisition
//! 2. Release the lock
//! 3. Resolve each archetype individually without holding locks
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::core::archetype::{ArchetypeResolver, ResolutionQuery};
//!
//! let resolver = ArchetypeResolver::new(
//!     registry.archetypes(),
//!     registry.setting_packs(),
//!     registry.active_packs(),
//! );
//!
//! let query = ResolutionQuery::for_npc("merchant")
//!     .with_race("dwarf")
//!     .with_setting("forgotten_realms");
//!
//! let resolved = resolver.resolve(&query).await?;
//! ```

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::RwLock;

use super::error::{ArchetypeError, Result};
use super::resolution::{ResolutionMetadata, ResolutionQuery, ResolvedArchetype};
use super::setting_pack::{ArchetypeOverride, SettingPack};
use super::types::{Archetype, ArchetypeCategory, PersonalityAffinity};

// ============================================================================
// Constants
// ============================================================================

/// Maximum number of merge operations allowed per resolution.
///
/// Prevents runaway resolution for overly complex archetype hierarchies.
const MAX_MERGE_OPERATIONS: usize = 50;

/// Maximum inheritance depth allowed.
///
/// Prevents infinite loops in deeply nested inheritance chains.
const MAX_INHERITANCE_DEPTH: usize = 10;

// ============================================================================
// ArchetypeResolver
// ============================================================================

/// Hierarchical archetype resolution engine.
///
/// The resolver takes a [`ResolutionQuery`] and produces a [`ResolvedArchetype`]
/// by merging data from multiple sources according to the resolution priority.
///
/// # Thread Safety
///
/// The resolver holds `Arc<RwLock<_>>` references to the registry's data.
/// It uses a lock-free pattern for inheritance resolution to prevent deadlocks.
pub struct ArchetypeResolver {
    /// Reference to all registered archetypes.
    archetypes: Arc<RwLock<HashMap<String, Archetype>>>,

    /// Reference to loaded setting packs.
    setting_packs: Arc<RwLock<HashMap<String, SettingPack>>>,

    /// Reference to active setting pack per campaign.
    active_packs: Arc<RwLock<HashMap<String, String>>>,
}

impl ArchetypeResolver {
    /// Create a new resolver with references to registry data.
    ///
    /// # Arguments
    ///
    /// * `archetypes` - Arc reference to the archetypes map
    /// * `setting_packs` - Arc reference to the setting packs map
    /// * `active_packs` - Arc reference to the active packs map
    pub fn new(
        archetypes: Arc<RwLock<HashMap<String, Archetype>>>,
        setting_packs: Arc<RwLock<HashMap<String, SettingPack>>>,
        active_packs: Arc<RwLock<HashMap<String, String>>>,
    ) -> Self {
        Self {
            archetypes,
            setting_packs,
            active_packs,
        }
    }

    /// Resolve a query through the full hierarchy.
    ///
    /// This is the main entry point for resolution. It applies all layers
    /// in priority order and returns the final resolved archetype.
    ///
    /// # Arguments
    ///
    /// * `query` - The resolution query specifying which layers to apply
    ///
    /// # Returns
    ///
    /// The resolved archetype with merged data from all applicable layers.
    ///
    /// # Errors
    ///
    /// - `ArchetypeError::NotFound` if no archetype data is found
    /// - `ArchetypeError::ResolutionTooComplex` if merge limit exceeded
    /// - `ArchetypeError::CircularResolution` if inheritance cycle detected
    /// - `ArchetypeError::InheritanceTooDeep` if depth limit exceeded
    pub async fn resolve(&self, query: &ResolutionQuery) -> Result<ResolvedArchetype> {
        let start_time = std::time::Instant::now();
        let mut layers_checked = Vec::new();
        let mut merge_count = 0;

        // Start with empty resolved archetype
        let mut resolved = ResolvedArchetype::new();

        // Layer 1: Base Role (lowest priority)
        if let Some(role) = &query.npc_role {
            layers_checked.push(format!("role:{}", role));
            if let Some(role_archetype) = self.find_by_role(role).await {
                let role_resolved = self.archetype_to_resolved(&role_archetype);
                resolved = self.merge_archetype(resolved, role_resolved, &mut merge_count)?;
                self.check_merge_limit(merge_count)?;
            }
        }

        // Layer 2: Race-Specific
        if let Some(race) = &query.race {
            layers_checked.push(format!("race:{}", race));
            if let Some(race_archetype) = self.find_by_race(race).await {
                let race_resolved = self.archetype_to_resolved(&race_archetype);
                resolved = self.merge_archetype(resolved, race_resolved, &mut merge_count)?;
                self.check_merge_limit(merge_count)?;
            }
        }

        // Layer 3: Class-Specific
        if let Some(class) = &query.class {
            layers_checked.push(format!("class:{}", class));
            if let Some(class_archetype) = self.find_by_class(class).await {
                let class_resolved = self.archetype_to_resolved(&class_archetype);
                resolved = self.merge_archetype(resolved, class_resolved, &mut merge_count)?;
                self.check_merge_limit(merge_count)?;
            }
        }

        // Layer 4: Setting Pack Override
        let setting_id = self.determine_setting_id(query).await;
        if let Some(ref setting) = setting_id {
            layers_checked.push(format!("setting:{}", setting));
            if let Some(overrides) = self.get_setting_overrides(setting, query).await {
                resolved =
                    self.apply_setting_overrides(resolved, overrides, &mut merge_count)?;
                self.check_merge_limit(merge_count)?;
            }
        }

        // Layer 5: Direct archetype ID (highest priority)
        if let Some(archetype_id) = &query.archetype_id {
            layers_checked.push(format!("direct:{}", archetype_id));
            let direct = self
                .resolve_inheritance_chain(archetype_id, &mut merge_count)
                .await?;
            resolved = self.merge_archetype(resolved, direct, &mut merge_count)?;
            self.check_merge_limit(merge_count)?;
        }

        // Validate we found something
        if resolved.is_empty() {
            return Err(ArchetypeError::NotFound {
                id: query.archetype_id.clone().unwrap_or_else(|| "query".to_string()),
                layers_checked,
            });
        }

        // Add resolution metadata
        let elapsed = start_time.elapsed();
        resolved.resolution_metadata = Some(
            ResolutionMetadata::new(layers_checked, merge_count)
                .with_time(elapsed.as_millis() as u64)
                .with_query(query.clone()),
        );

        Ok(resolved)
    }

    // ========================================================================
    // Layer Finders
    // ========================================================================

    /// Find archetype by NPC role.
    ///
    /// Searches for archetypes with:
    /// 1. Matching ID (e.g., "merchant")
    /// 2. Category == Role and matching tags
    pub async fn find_by_role(&self, role: &str) -> Option<Archetype> {
        let archetypes = self.archetypes.read().await;

        // First try direct ID match
        if let Some(archetype) = archetypes.get(role) {
            if archetype.category == ArchetypeCategory::Role {
                return Some(archetype.clone());
            }
        }

        // Then search by role in NPC role mapping
        archetypes
            .values()
            .find(|a| {
                a.category == ArchetypeCategory::Role
                    && a.npc_role_mapping.iter().any(|m| m.role == role)
            })
            .cloned()
    }

    /// Find archetype by race.
    ///
    /// Searches for archetypes with:
    /// 1. Matching ID (e.g., "dwarf")
    /// 2. Category == Race
    pub async fn find_by_race(&self, race: &str) -> Option<Archetype> {
        let archetypes = self.archetypes.read().await;

        // First try direct ID match
        if let Some(archetype) = archetypes.get(race) {
            if archetype.category == ArchetypeCategory::Race {
                return Some(archetype.clone());
            }
        }

        // Then search by race tag
        archetypes
            .values()
            .find(|a| a.category == ArchetypeCategory::Race && a.tags.contains(&race.to_string()))
            .cloned()
    }

    /// Find archetype by class.
    ///
    /// Searches for archetypes with:
    /// 1. Matching ID (e.g., "fighter")
    /// 2. Category == Class
    pub async fn find_by_class(&self, class: &str) -> Option<Archetype> {
        let archetypes = self.archetypes.read().await;

        // First try direct ID match
        if let Some(archetype) = archetypes.get(class) {
            if archetype.category == ArchetypeCategory::Class {
                return Some(archetype.clone());
            }
        }

        // Then search by class tag
        archetypes
            .values()
            .find(|a| {
                a.category == ArchetypeCategory::Class && a.tags.contains(&class.to_string())
            })
            .cloned()
    }

    /// Determine the setting ID to use for resolution.
    ///
    /// Priority:
    /// 1. Explicit setting in query
    /// 2. Active pack for campaign (if campaign_id specified)
    async fn determine_setting_id(&self, query: &ResolutionQuery) -> Option<String> {
        // Explicit setting takes precedence
        if let Some(ref setting) = query.setting {
            return Some(setting.clone());
        }

        // Check for active campaign pack
        if let Some(ref campaign_id) = query.campaign_id {
            let active = self.active_packs.read().await;
            return active.get(campaign_id).cloned();
        }

        None
    }

    /// Get setting pack overrides applicable to the query.
    pub async fn get_setting_overrides(
        &self,
        setting_id: &str,
        query: &ResolutionQuery,
    ) -> Option<Vec<ArchetypeOverride>> {
        let packs = self.setting_packs.read().await;
        let pack = packs.get(setting_id)?;

        let mut overrides = Vec::new();

        // Collect overrides for each layer in the query
        if let Some(ref role) = query.npc_role {
            if let Some(override_def) = pack.archetype_overrides.get(role) {
                overrides.push(override_def.clone());
            }
        }

        if let Some(ref race) = query.race {
            if let Some(override_def) = pack.archetype_overrides.get(race) {
                overrides.push(override_def.clone());
            }
        }

        if let Some(ref class) = query.class {
            if let Some(override_def) = pack.archetype_overrides.get(class) {
                overrides.push(override_def.clone());
            }
        }

        if let Some(ref archetype_id) = query.archetype_id {
            if let Some(override_def) = pack.archetype_overrides.get(archetype_id) {
                overrides.push(override_def.clone());
            }
        }

        if overrides.is_empty() {
            None
        } else {
            Some(overrides)
        }
    }

    // ========================================================================
    // Inheritance Chain Resolution (CRITICAL-ARCH-001)
    // ========================================================================

    /// Resolve an archetype with its full inheritance chain.
    ///
    /// # Lock-Free Pattern
    ///
    /// This method uses a lock-free pattern to prevent deadlocks:
    ///
    /// 1. Acquire lock and collect all inheritance chain IDs
    /// 2. Release lock
    /// 3. Resolve each archetype individually
    ///
    /// # Arguments
    ///
    /// * `id` - The archetype ID to resolve
    /// * `merge_count` - Running count of merge operations
    ///
    /// # Errors
    ///
    /// - `ArchetypeError::NotFound` if archetype doesn't exist
    /// - `ArchetypeError::CircularResolution` if inheritance cycle detected
    /// - `ArchetypeError::InheritanceTooDeep` if depth limit exceeded
    pub async fn resolve_inheritance_chain(
        &self,
        id: &str,
        merge_count: &mut usize,
    ) -> Result<ResolvedArchetype> {
        // Step 1: Collect inheritance chain IDs in single lock acquisition
        let chain_ids = {
            let archetypes = self.archetypes.read().await;
            self.collect_inheritance_chain(&archetypes, id)?
        };

        // Step 2: Resolve each archetype in chain order (parent first)
        let mut resolved = ResolvedArchetype::new();

        for chain_id in chain_ids {
            // Get archetype (separate lock acquisition per archetype)
            let archetype = {
                let archetypes = self.archetypes.read().await;
                archetypes.get(&chain_id).cloned().ok_or_else(|| {
                    ArchetypeError::NotFound {
                        id: chain_id.clone(),
                        layers_checked: vec![format!("inheritance:{}", chain_id)],
                    }
                })?
            };

            // Merge onto resolved
            let archetype_resolved = self.archetype_to_resolved(&archetype);
            resolved = self.merge_archetype(resolved, archetype_resolved, merge_count)?;
            self.check_merge_limit(*merge_count)?;
        }

        Ok(resolved)
    }

    /// Collect the inheritance chain IDs in parent-first order.
    ///
    /// This method performs the following:
    /// 1. Starts from the target archetype
    /// 2. Follows parent_id links to build the chain
    /// 3. Detects circular references
    /// 4. Enforces depth limit
    /// 5. Returns chain in parent-first order (for correct merge precedence)
    ///
    /// # Arguments
    ///
    /// * `archetypes` - Reference to the archetypes map (already locked)
    /// * `id` - Starting archetype ID
    ///
    /// # Returns
    ///
    /// Vector of archetype IDs in parent-first order.
    fn collect_inheritance_chain(
        &self,
        archetypes: &HashMap<String, Archetype>,
        id: &str,
    ) -> Result<Vec<String>> {
        let mut chain = Vec::new();
        let mut visited = HashSet::new();
        let mut current_id = id.to_string();
        let mut depth = 0;

        loop {
            // Check for circular reference
            if visited.contains(&current_id) {
                // Build cycle path
                let cycle_start = chain.iter().position(|x| x == &current_id).unwrap();
                let cycle_path: Vec<String> = chain[cycle_start..]
                    .iter()
                    .cloned()
                    .chain(std::iter::once(current_id))
                    .collect();
                return Err(ArchetypeError::CircularResolution {
                    cycle_path,
                });
            }

            // Check depth limit
            if depth >= MAX_INHERITANCE_DEPTH {
                return Err(ArchetypeError::InheritanceTooDeep {
                    archetype_id: current_id,
                    depth,
                });
            }

            // Get archetype
            let archetype = archetypes.get(&current_id).ok_or_else(|| {
                ArchetypeError::NotFound {
                    id: current_id.clone(),
                    layers_checked: vec![format!("inheritance:{}", current_id)],
                }
            })?;

            // Add to chain and visited
            chain.push(current_id.clone());
            visited.insert(current_id.clone());

            // Check for parent
            match &archetype.parent_id {
                Some(parent_id) => {
                    current_id = parent_id.to_string();
                    depth += 1;
                }
                None => break,
            }
        }

        // Reverse to get parent-first order
        chain.reverse();

        Ok(chain)
    }

    // ========================================================================
    // Merge Logic
    // ========================================================================

    /// Merge two resolved archetypes.
    ///
    /// # Merge Semantics (per AR-201.3)
    ///
    /// - **Scalar fields** (id, display_name, category): Overlay wins if present
    /// - **Arrays** (personality_affinity, npc_role_mapping, naming_cultures):
    ///   Replace entirely if overlay is non-empty
    /// - **Optional fields** (vocabulary_bank_id, stat_tendencies):
    ///   Overlay wins if Some
    /// - **Tags**: Concatenate and deduplicate
    ///
    /// # Arguments
    ///
    /// * `base` - The base resolved archetype
    /// * `overlay` - The overlay to merge on top
    /// * `merge_count` - Running count of merge operations
    pub fn merge_archetype(
        &self,
        base: ResolvedArchetype,
        overlay: ResolvedArchetype,
        merge_count: &mut usize,
    ) -> Result<ResolvedArchetype> {
        *merge_count += 1;

        Ok(ResolvedArchetype {
            // Scalar fields: overlay wins if present
            id: overlay.id.or(base.id),
            display_name: overlay.display_name.or(base.display_name),
            category: overlay.category.or(base.category),

            // Array fields: replace entirely if overlay is non-empty
            personality_affinity: if overlay.personality_affinity.is_empty() {
                base.personality_affinity
            } else {
                overlay.personality_affinity
            },

            npc_role_mapping: if overlay.npc_role_mapping.is_empty() {
                base.npc_role_mapping
            } else {
                overlay.npc_role_mapping
            },

            naming_cultures: if overlay.naming_cultures.is_empty() {
                base.naming_cultures
            } else {
                overlay.naming_cultures
            },

            // Optional fields: overlay wins if Some
            vocabulary_bank: overlay.vocabulary_bank.or(base.vocabulary_bank),
            vocabulary_bank_id: overlay.vocabulary_bank_id.or(base.vocabulary_bank_id),
            stat_tendencies: overlay.stat_tendencies.or(base.stat_tendencies),

            // Tags: concatenate and deduplicate
            tags: {
                let mut combined = base.tags;
                for tag in overlay.tags {
                    if !combined.contains(&tag) {
                        combined.push(tag);
                    }
                }
                combined
            },

            // Metadata: preserve from overlay (will be overwritten at end)
            resolution_metadata: overlay.resolution_metadata,
        })
    }

    /// Apply setting pack overrides to a resolved archetype.
    ///
    /// # Arguments
    ///
    /// * `base` - The base resolved archetype
    /// * `overrides` - List of overrides to apply in order
    /// * `merge_count` - Running count of merge operations
    pub fn apply_setting_overrides(
        &self,
        mut resolved: ResolvedArchetype,
        overrides: Vec<ArchetypeOverride>,
        merge_count: &mut usize,
    ) -> Result<ResolvedArchetype> {
        for override_def in overrides {
            *merge_count += 1;
            self.check_merge_limit(*merge_count)?;

            // Apply display name override
            if let Some(ref name) = override_def.display_name {
                resolved.display_name = Some(name.clone().into());
            }

            // Apply personality affinity changes
            if let Some(ref replacement) = override_def.personality_affinity_replacement {
                // Complete replacement
                resolved.personality_affinity = replacement.clone();
            } else if !override_def.personality_affinity_additions.is_empty() {
                // Merge additions
                resolved.personality_affinity =
                    self.merge_personality_affinities(
                        &resolved.personality_affinity,
                        &override_def.personality_affinity_additions,
                    );
            }

            // Apply vocabulary bank override
            if let Some(ref bank_id) = override_def.vocabulary_bank_id {
                resolved.vocabulary_bank_id = Some(bank_id.clone());
            }

            // Apply naming cultures override
            if let Some(ref cultures) = override_def.naming_cultures {
                resolved.naming_cultures = cultures.clone();
            }

            // Apply stat tendencies override
            if let Some(ref tendencies) = override_def.stat_tendencies {
                resolved.stat_tendencies = Some(tendencies.clone());
            }

            // Add additional tags
            for tag in &override_def.additional_tags {
                if !resolved.tags.contains(tag) {
                    resolved.tags.push(tag.clone());
                }
            }

            // Apply nullified fields
            for field in &override_def.nullified_fields {
                match field.as_str() {
                    "vocabulary_bank" | "vocabularyBank" => {
                        resolved.vocabulary_bank = None;
                        resolved.vocabulary_bank_id = None;
                    }
                    "stat_tendencies" | "statTendencies" => {
                        resolved.stat_tendencies = None;
                    }
                    "personality_affinity" | "personalityAffinity" => {
                        resolved.personality_affinity.clear();
                    }
                    "npc_role_mapping" | "npcRoleMapping" => {
                        resolved.npc_role_mapping.clear();
                    }
                    "naming_cultures" | "namingCultures" => {
                        resolved.naming_cultures.clear();
                    }
                    "description" => {
                        // Description is not in ResolvedArchetype, skip
                    }
                    _ => {} // Unknown field, ignore
                }
            }
        }

        Ok(resolved)
    }

    /// Merge personality affinities with overlay precedence.
    ///
    /// Overlay values override base values for the same trait_id.
    fn merge_personality_affinities(
        &self,
        base: &[PersonalityAffinity],
        overlay: &[PersonalityAffinity],
    ) -> Vec<PersonalityAffinity> {
        let mut result: Vec<PersonalityAffinity> = Vec::new();

        // Add all base affinities
        for affinity in base {
            // Check if overlay has this trait
            if overlay.iter().any(|o| o.trait_id == affinity.trait_id) {
                // Skip - overlay will provide this trait
                continue;
            }
            result.push(affinity.clone());
        }

        // Add all overlay affinities
        for affinity in overlay {
            result.push(affinity.clone());
        }

        result
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    /// Convert an Archetype to a ResolvedArchetype.
    fn archetype_to_resolved(&self, archetype: &Archetype) -> ResolvedArchetype {
        ResolvedArchetype {
            id: Some(archetype.id.clone()),
            display_name: Some(archetype.display_name.clone()),
            category: Some(archetype.category.clone()),
            personality_affinity: archetype.personality_affinity.clone(),
            npc_role_mapping: archetype.npc_role_mapping.clone(),
            vocabulary_bank: None, // Vocabulary banks are resolved separately
            vocabulary_bank_id: archetype.vocabulary_bank_id.clone(),
            naming_cultures: archetype.naming_cultures.clone(),
            stat_tendencies: archetype.stat_tendencies.clone(),
            tags: archetype.tags.clone(),
            resolution_metadata: None,
        }
    }

    /// Check if merge count exceeds limit.
    fn check_merge_limit(&self, count: usize) -> Result<()> {
        if count > MAX_MERGE_OPERATIONS {
            Err(ArchetypeError::ResolutionTooComplex { merge_count: count })
        } else {
            Ok(())
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{ArchetypeId, NamingCultureWeight, NpcRoleMapping, StatTendencies};
    use std::sync::Arc;

    fn create_test_archetype(
        id: &str,
        category: ArchetypeCategory,
        parent_id: Option<&str>,
    ) -> Archetype {
        let mut archetype = Archetype::new(id, id.to_uppercase(), category);
        if let Some(parent) = parent_id {
            archetype = archetype.with_parent(parent);
        }
        archetype
    }

    fn create_resolver_with_archetypes(
        archetypes: Vec<Archetype>,
    ) -> ArchetypeResolver {
        let mut map = HashMap::new();
        for archetype in archetypes {
            map.insert(archetype.id.to_string(), archetype);
        }

        ArchetypeResolver::new(
            Arc::new(RwLock::new(map)),
            Arc::new(RwLock::new(HashMap::new())),
            Arc::new(RwLock::new(HashMap::new())),
        )
    }

    // -------------------------------------------------------------------------
    // Inheritance Chain Tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_collect_inheritance_chain_no_parent() {
        let archetypes = vec![create_test_archetype("knight", ArchetypeCategory::Class, None)];

        let resolver = create_resolver_with_archetypes(archetypes);
        let map = resolver.archetypes.read().await;
        let chain = resolver.collect_inheritance_chain(&map, "knight").unwrap();

        assert_eq!(chain, vec!["knight"]);
    }

    #[tokio::test]
    async fn test_collect_inheritance_chain_single_parent() {
        let archetypes = vec![
            create_test_archetype("warrior", ArchetypeCategory::Class, None),
            create_test_archetype("knight", ArchetypeCategory::Class, Some("warrior")),
        ];

        let resolver = create_resolver_with_archetypes(archetypes);
        let map = resolver.archetypes.read().await;
        let chain = resolver.collect_inheritance_chain(&map, "knight").unwrap();

        // Should be parent-first order
        assert_eq!(chain, vec!["warrior", "knight"]);
    }

    #[tokio::test]
    async fn test_collect_inheritance_chain_multi_level() {
        let archetypes = vec![
            create_test_archetype("humanoid", ArchetypeCategory::Race, None),
            create_test_archetype("dwarf", ArchetypeCategory::Race, Some("humanoid")),
            create_test_archetype(
                "mountain_dwarf",
                ArchetypeCategory::Race,
                Some("dwarf"),
            ),
        ];

        let resolver = create_resolver_with_archetypes(archetypes);
        let map = resolver.archetypes.read().await;
        let chain = resolver
            .collect_inheritance_chain(&map, "mountain_dwarf")
            .unwrap();

        assert_eq!(chain, vec!["humanoid", "dwarf", "mountain_dwarf"]);
    }

    #[tokio::test]
    async fn test_collect_inheritance_chain_circular_detection() {
        // Create a circular reference: a -> b -> c -> a
        let a = create_test_archetype("a", ArchetypeCategory::Role, Some("c"));
        let b = create_test_archetype("b", ArchetypeCategory::Role, Some("a"));
        let c = create_test_archetype("c", ArchetypeCategory::Role, Some("b"));

        // Note: In reality, we'd prevent this at registration time,
        // but we test detection here
        let archetypes = vec![a, b, c];

        let resolver = create_resolver_with_archetypes(archetypes);
        let map = resolver.archetypes.read().await;
        let result = resolver.collect_inheritance_chain(&map, "a");

        assert!(result.is_err());
        match result.unwrap_err() {
            ArchetypeError::CircularResolution { cycle_path } => {
                assert!(cycle_path.len() >= 2);
            }
            _ => panic!("Expected CircularResolution error"),
        }
    }

    #[tokio::test]
    async fn test_collect_inheritance_chain_depth_limit() {
        // Create a chain deeper than MAX_INHERITANCE_DEPTH
        let mut archetypes = Vec::new();
        let depth = MAX_INHERITANCE_DEPTH + 2;

        for i in 0..depth {
            let id = format!("level_{}", i);
            let parent = if i > 0 {
                Some(format!("level_{}", i - 1))
            } else {
                None
            };

            let archetype = create_test_archetype(
                &id,
                ArchetypeCategory::Role,
                parent.as_deref(),
            );
            archetypes.push(archetype);
        }

        let resolver = create_resolver_with_archetypes(archetypes);
        let map = resolver.archetypes.read().await;
        let deepest_id = format!("level_{}", depth - 1);
        let result = resolver.collect_inheritance_chain(&map, &deepest_id);

        assert!(result.is_err());
        match result.unwrap_err() {
            ArchetypeError::InheritanceTooDeep { depth, .. } => {
                assert_eq!(depth, MAX_INHERITANCE_DEPTH);
            }
            _ => panic!("Expected InheritanceTooDeep error"),
        }
    }

    #[tokio::test]
    async fn test_collect_inheritance_chain_not_found() {
        let archetypes = vec![create_test_archetype("knight", ArchetypeCategory::Class, None)];

        let resolver = create_resolver_with_archetypes(archetypes);
        let map = resolver.archetypes.read().await;
        let result = resolver.collect_inheritance_chain(&map, "nonexistent");

        assert!(result.is_err());
        match result.unwrap_err() {
            ArchetypeError::NotFound { id, .. } => {
                assert_eq!(id, "nonexistent");
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    // -------------------------------------------------------------------------
    // Merge Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_merge_archetype_scalars() {
        let resolver = create_resolver_with_archetypes(vec![]);

        let base = ResolvedArchetype {
            id: Some(ArchetypeId::new("base")),
            display_name: Some("Base".into()),
            category: Some(ArchetypeCategory::Role),
            ..Default::default()
        };

        let overlay = ResolvedArchetype {
            id: Some(ArchetypeId::new("overlay")),
            display_name: Some("Overlay".into()),
            ..Default::default()
        };

        let mut merge_count = 0;
        let result = resolver.merge_archetype(base, overlay, &mut merge_count).unwrap();

        // Overlay wins for scalars
        assert_eq!(result.id.unwrap().as_str(), "overlay");
        assert_eq!(result.display_name.unwrap().as_ref(), "Overlay");
        // Base wins if overlay is None
        assert_eq!(result.category.unwrap(), ArchetypeCategory::Role);
    }

    #[test]
    fn test_merge_archetype_arrays_replace() {
        let resolver = create_resolver_with_archetypes(vec![]);

        let base = ResolvedArchetype {
            personality_affinity: vec![
                PersonalityAffinity::new("brave", 0.8),
                PersonalityAffinity::new("cautious", 0.5),
            ],
            ..Default::default()
        };

        let overlay = ResolvedArchetype {
            personality_affinity: vec![PersonalityAffinity::new("curious", 0.9)],
            ..Default::default()
        };

        let mut merge_count = 0;
        let result = resolver.merge_archetype(base, overlay, &mut merge_count).unwrap();

        // Overlay replaces entirely when non-empty
        assert_eq!(result.personality_affinity.len(), 1);
        assert_eq!(result.personality_affinity[0].trait_id, "curious");
    }

    #[test]
    fn test_merge_archetype_arrays_preserve_base() {
        let resolver = create_resolver_with_archetypes(vec![]);

        let base = ResolvedArchetype {
            personality_affinity: vec![
                PersonalityAffinity::new("brave", 0.8),
                PersonalityAffinity::new("cautious", 0.5),
            ],
            ..Default::default()
        };

        let overlay = ResolvedArchetype {
            // Empty arrays don't override
            personality_affinity: vec![],
            ..Default::default()
        };

        let mut merge_count = 0;
        let result = resolver.merge_archetype(base, overlay, &mut merge_count).unwrap();

        // Base preserved when overlay is empty
        assert_eq!(result.personality_affinity.len(), 2);
    }

    #[test]
    fn test_merge_archetype_tags_deduplicate() {
        let resolver = create_resolver_with_archetypes(vec![]);

        let base = ResolvedArchetype {
            tags: vec!["tag1".to_string(), "tag2".to_string()],
            ..Default::default()
        };

        let overlay = ResolvedArchetype {
            tags: vec!["tag2".to_string(), "tag3".to_string()],
            ..Default::default()
        };

        let mut merge_count = 0;
        let result = resolver.merge_archetype(base, overlay, &mut merge_count).unwrap();

        // Tags concatenated and deduplicated
        assert_eq!(result.tags.len(), 3);
        assert!(result.tags.contains(&"tag1".to_string()));
        assert!(result.tags.contains(&"tag2".to_string()));
        assert!(result.tags.contains(&"tag3".to_string()));
    }

    #[test]
    fn test_merge_count_increment() {
        let resolver = create_resolver_with_archetypes(vec![]);

        let base = ResolvedArchetype::new();
        let overlay = ResolvedArchetype::new();

        let mut merge_count = 0;
        let _ = resolver.merge_archetype(base, overlay, &mut merge_count).unwrap();

        assert_eq!(merge_count, 1);
    }

    #[test]
    fn test_merge_limit_exceeded() {
        let resolver = create_resolver_with_archetypes(vec![]);

        let result = resolver.check_merge_limit(MAX_MERGE_OPERATIONS + 1);

        assert!(result.is_err());
        match result.unwrap_err() {
            ArchetypeError::ResolutionTooComplex { merge_count } => {
                assert_eq!(merge_count, MAX_MERGE_OPERATIONS + 1);
            }
            _ => panic!("Expected ResolutionTooComplex error"),
        }
    }

    // -------------------------------------------------------------------------
    // Setting Override Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_apply_setting_override_display_name() {
        let resolver = create_resolver_with_archetypes(vec![]);

        let resolved = ResolvedArchetype {
            display_name: Some("Original".into()),
            ..Default::default()
        };

        let override_def = ArchetypeOverride::new().with_display_name("Overridden");

        let mut merge_count = 0;
        let result = resolver
            .apply_setting_overrides(resolved, vec![override_def], &mut merge_count)
            .unwrap();

        assert_eq!(result.display_name.unwrap().as_ref(), "Overridden");
    }

    #[test]
    fn test_apply_setting_override_affinity_replacement() {
        let resolver = create_resolver_with_archetypes(vec![]);

        let resolved = ResolvedArchetype {
            personality_affinity: vec![
                PersonalityAffinity::new("original1", 0.5),
                PersonalityAffinity::new("original2", 0.5),
            ],
            ..Default::default()
        };

        let override_def = ArchetypeOverride::new()
            .with_affinity_replacement(vec![PersonalityAffinity::new("replaced", 0.9)]);

        let mut merge_count = 0;
        let result = resolver
            .apply_setting_overrides(resolved, vec![override_def], &mut merge_count)
            .unwrap();

        assert_eq!(result.personality_affinity.len(), 1);
        assert_eq!(result.personality_affinity[0].trait_id, "replaced");
    }

    #[test]
    fn test_apply_setting_override_affinity_additions() {
        let resolver = create_resolver_with_archetypes(vec![]);

        let resolved = ResolvedArchetype {
            personality_affinity: vec![
                PersonalityAffinity::new("base_trait", 0.5),
            ],
            ..Default::default()
        };

        let override_def = ArchetypeOverride::new()
            .with_affinity_additions(vec![PersonalityAffinity::new("added_trait", 0.8)]);

        let mut merge_count = 0;
        let result = resolver
            .apply_setting_overrides(resolved, vec![override_def], &mut merge_count)
            .unwrap();

        assert_eq!(result.personality_affinity.len(), 2);
    }

    #[test]
    fn test_apply_setting_override_nullified_fields() {
        let resolver = create_resolver_with_archetypes(vec![]);

        let resolved = ResolvedArchetype {
            vocabulary_bank_id: Some("some_bank".to_string()),
            stat_tendencies: Some(StatTendencies::default()),
            personality_affinity: vec![PersonalityAffinity::new("trait", 0.5)],
            ..Default::default()
        };

        let override_def = ArchetypeOverride::new()
            .with_nullified_fields(vec![
                "vocabulary_bank".to_string(),
                "stat_tendencies".to_string(),
                "personality_affinity".to_string(),
            ]);

        let mut merge_count = 0;
        let result = resolver
            .apply_setting_overrides(resolved, vec![override_def], &mut merge_count)
            .unwrap();

        assert!(result.vocabulary_bank_id.is_none());
        assert!(result.stat_tendencies.is_none());
        assert!(result.personality_affinity.is_empty());
    }

    // -------------------------------------------------------------------------
    // Personality Affinity Merge Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_merge_personality_affinities_overlay_precedence() {
        let resolver = create_resolver_with_archetypes(vec![]);

        let base = vec![
            PersonalityAffinity::new("brave", 0.5),
            PersonalityAffinity::new("cautious", 0.5),
        ];

        let overlay = vec![
            PersonalityAffinity::new("brave", 0.9), // Override existing
            PersonalityAffinity::new("curious", 0.7), // Add new
        ];

        let result = resolver.merge_personality_affinities(&base, &overlay);

        // Should have: cautious from base, brave and curious from overlay
        assert_eq!(result.len(), 3);

        let brave = result.iter().find(|a| a.trait_id == "brave").unwrap();
        assert_eq!(brave.weight, 0.9); // Overlay value

        let cautious = result.iter().find(|a| a.trait_id == "cautious").unwrap();
        assert_eq!(cautious.weight, 0.5); // Base value preserved

        assert!(result.iter().any(|a| a.trait_id == "curious"));
    }

    // -------------------------------------------------------------------------
    // Full Resolution Tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_resolve_single_archetype() {
        let archetypes = vec![create_test_archetype("knight", ArchetypeCategory::Class, None)];

        let resolver = create_resolver_with_archetypes(archetypes);
        let _query = ResolutionQuery::single("knight");
        let mut merge_count = 0;

        let result = resolver
            .resolve_inheritance_chain("knight", &mut merge_count)
            .await
            .unwrap();

        assert_eq!(result.id.unwrap().as_str(), "knight");
    }

    #[tokio::test]
    async fn test_resolve_with_inheritance() {
        let archetypes = vec![
            Archetype::new("warrior", "Warrior", ArchetypeCategory::Class)
                .with_personality_affinity(vec![PersonalityAffinity::new("brave", 0.8)]),
            Archetype::new("knight", "Knight", ArchetypeCategory::Class)
                .with_parent("warrior")
                .with_personality_affinity(vec![PersonalityAffinity::new("honorable", 0.9)]),
        ];

        let resolver = create_resolver_with_archetypes(archetypes);
        let mut merge_count = 0;

        let result = resolver
            .resolve_inheritance_chain("knight", &mut merge_count)
            .await
            .unwrap();

        // Knight's values should override warrior's
        assert_eq!(result.id.unwrap().as_str(), "knight");
        assert_eq!(result.personality_affinity.len(), 1);
        assert_eq!(result.personality_affinity[0].trait_id, "honorable");
    }

    #[tokio::test]
    async fn test_resolve_full_query() {
        let archetypes = vec![
            Archetype::new("merchant", "Merchant", ArchetypeCategory::Role)
                .with_npc_role_mapping(vec![NpcRoleMapping::new("merchant", 1.0)]),
            Archetype::new("dwarf", "Dwarf", ArchetypeCategory::Race)
                .with_naming_cultures(vec![NamingCultureWeight::new("dwarvish", 1.0)]),
        ];

        let resolver = create_resolver_with_archetypes(archetypes);

        let query = ResolutionQuery::for_npc("merchant").with_race("dwarf");

        let result = resolver.resolve(&query).await.unwrap();

        // Should have both role and race contributions
        assert!(!result.npc_role_mapping.is_empty());
        assert!(!result.naming_cultures.is_empty());

        // Check metadata
        let metadata = result.resolution_metadata.unwrap();
        assert!(metadata.layers_checked.contains(&"role:merchant".to_string()));
        assert!(metadata.layers_checked.contains(&"race:dwarf".to_string()));
    }

    #[tokio::test]
    async fn test_resolve_not_found() {
        let resolver = create_resolver_with_archetypes(vec![]);

        let query = ResolutionQuery::single("nonexistent");
        let result = resolver.resolve(&query).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ArchetypeError::NotFound { id, layers_checked } => {
                assert_eq!(id, "nonexistent");
                assert!(!layers_checked.is_empty());
            }
            _ => panic!("Expected NotFound error"),
        }
    }
}
