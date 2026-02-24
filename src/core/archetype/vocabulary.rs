//! Vocabulary Bank management for the Archetype Registry.
//!
//! This module provides storage and retrieval of vocabulary banks used for
//! NPC speech pattern generation. It supports:
//!
//! - Bank registration with validation
//! - Phrase retrieval with category, formality, and tone filtering
//! - Bank merging with overlay precedence
//! - Session-based usage tracking to avoid repetition
//!
//! # Architecture
//!
//! ```text
//!               VocabularyBankManager
//!                        |
//!     +------------------+------------------+
//!     |                  |                  |
//!     v                  v                  v
//!   Banks             Usage             Meilisearch
//!  (storage)         Tracking          (persistence)
//! ```
//!
//! # Thread Safety (CRITICAL-ARCH-002)
//!
//! All mutable state is protected by `tokio::sync::RwLock` for async-safe
//! access in the Tauri async command context.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::core::archetype::vocabulary::VocabularyBankManager;
//!
//! let manager = VocabularyBankManager::new();
//!
//! // Register a bank
//! let bank = VocabularyBankDefinition::new("dwarvish_merchant", "Dwarvish Merchant")
//!     .add_phrases("greetings", vec![PhraseDefinition::new("Stone and steel, friend!")]);
//! manager.register(bank).await?;
//!
//! // Get phrases with filters
//! let options = PhraseFilterOptions {
//!     category: "greetings".to_string(),
//!     formality_range: Some((3, 7)),
//!     tone: Some("friendly".to_string()),
//! };
//! let phrases = manager.get_phrases("dwarvish_merchant", options, "session_1").await?;
//! ```

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use crate::core::wilysearch::core::{Meilisearch, SearchQuery};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::error::{ArchetypeError, Result};
use crate::core::npc_gen::INDEX_VOCABULARY_BANKS;

use super::setting_pack::{PhraseDefinition, VocabularyBankDefinition};

// ============================================================================
// Constants
// ============================================================================

/// Maximum phrases per category after merging (per AR-303.11).
const MAX_PHRASES_PER_CATEGORY: usize = 100;

/// Timeout for Meilisearch task completion.
const TASK_TIMEOUT_SECS: u64 = 30;

/// Polling interval for task completion checks.
const TASK_POLL_INTERVAL_MS: u64 = 100;

// ============================================================================
// VocabularyBank - Extended bank with runtime metadata
// ============================================================================

/// A vocabulary bank with runtime metadata.
///
/// Extends `VocabularyBankDefinition` with additional fields for
/// runtime tracking and management.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VocabularyBank {
    /// Bank definition containing all phrase data.
    #[serde(flatten)]
    pub definition: VocabularyBankDefinition,

    /// Whether this is a built-in bank (cannot be deleted).
    #[serde(default)]
    pub is_builtin: bool,

    /// IDs of archetypes that reference this bank.
    ///
    /// Used to prevent deletion of banks that are in use.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub referencing_archetypes: Vec<String>,

    /// Creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Last update timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl VocabularyBank {
    /// Create a new vocabulary bank from a definition.
    pub fn from_definition(definition: VocabularyBankDefinition) -> Self {
        let now = chrono::Utc::now();
        Self {
            definition,
            is_builtin: false,
            referencing_archetypes: Vec::new(),
            created_at: Some(now),
            updated_at: Some(now),
        }
    }

    /// Mark this bank as a built-in bank.
    pub fn as_builtin(mut self) -> Self {
        self.is_builtin = true;
        self
    }

    /// Add an archetype reference.
    pub fn add_reference(&mut self, archetype_id: &str) {
        if !self.referencing_archetypes.contains(&archetype_id.to_string()) {
            self.referencing_archetypes.push(archetype_id.to_string());
        }
    }

    /// Remove an archetype reference.
    pub fn remove_reference(&mut self, archetype_id: &str) {
        self.referencing_archetypes.retain(|id| id != archetype_id);
    }

    /// Check if the bank is referenced by any archetypes.
    pub fn is_referenced(&self) -> bool {
        !self.referencing_archetypes.is_empty()
    }

    /// Update the `updated_at` timestamp.
    pub fn touch(&mut self) {
        self.updated_at = Some(chrono::Utc::now());
    }
}

impl From<VocabularyBankDefinition> for VocabularyBank {
    fn from(definition: VocabularyBankDefinition) -> Self {
        Self::from_definition(definition)
    }
}

// ============================================================================
// PhraseFilterOptions - Options for phrase retrieval
// ============================================================================

/// Options for filtering phrases during retrieval.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhraseFilterOptions {
    /// Category to filter by (e.g., "greetings", "farewells").
    pub category: String,

    /// Optional formality range (min, max) where 1 = casual, 10 = formal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formality_range: Option<(u8, u8)>,

    /// Optional tone marker to filter by (e.g., "friendly", "hostile").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<String>,

    /// Maximum number of phrases to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

impl PhraseFilterOptions {
    /// Create options for a specific category.
    pub fn for_category(category: impl Into<String>) -> Self {
        Self {
            category: category.into(),
            ..Default::default()
        }
    }

    /// Add formality range filter.
    pub fn with_formality(mut self, min: u8, max: u8) -> Self {
        self.formality_range = Some((min.clamp(1, 10), max.clamp(1, 10)));
        self
    }

    /// Add tone filter.
    pub fn with_tone(mut self, tone: impl Into<String>) -> Self {
        self.tone = Some(tone.into());
        self
    }

    /// Add limit.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

// ============================================================================
// BankListFilter - Options for listing banks
// ============================================================================

/// Filter options for listing vocabulary banks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankListFilter {
    /// Filter by culture (e.g., "dwarvish", "elvish").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub culture: Option<String>,

    /// Filter by role (e.g., "merchant", "guard").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Filter by race (e.g., "dwarf", "elf").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub race: Option<String>,

    /// Include only built-in banks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builtin_only: Option<bool>,
}

impl BankListFilter {
    /// Filter by culture.
    pub fn by_culture(culture: impl Into<String>) -> Self {
        Self {
            culture: Some(culture.into()),
            ..Default::default()
        }
    }

    /// Filter by role.
    pub fn by_role(role: impl Into<String>) -> Self {
        Self {
            role: Some(role.into()),
            ..Default::default()
        }
    }
}

// ============================================================================
// VocabularyBankManager
// ============================================================================

/// Manages vocabulary banks for NPC speech pattern generation.
///
/// The manager provides:
/// - Bank registration with validation
/// - Phrase retrieval with filtering
/// - Bank merging for archetype composition
/// - Session usage tracking to avoid repetition
///
/// # Thread Safety
///
/// All mutable state uses `tokio::sync::RwLock` for async-safe access.
///
/// # Persistence
///
/// Banks are persisted to Meilisearch for durability and searchability.
pub struct VocabularyBankManager {
    /// In-memory vocabulary banks indexed by ID.
    banks: RwLock<HashMap<String, VocabularyBank>>,

    /// Usage tracking: session_id -> bank_id -> category -> Set<phrase_text>
    session_usage: RwLock<HashMap<String, HashMap<String, HashMap<String, HashSet<String>>>>>,

    /// Optional Meilisearch client for persistence.
    meili: Option<Arc<Meilisearch>>,
}

impl VocabularyBankManager {
    /// Create a new vocabulary bank manager without persistence.
    pub fn new() -> Self {
        Self {
            banks: RwLock::new(HashMap::new()),
            session_usage: RwLock::new(HashMap::new()),
            meili: None,
        }
    }

    /// Create a vocabulary bank manager with Meilisearch persistence.
    pub fn with_meilisearch(meili: Arc<Meilisearch>) -> Self {
        Self {
            banks: RwLock::new(HashMap::new()),
            session_usage: RwLock::new(HashMap::new()),
            meili: Some(meili),
        }
    }

    // ========================================================================
    // CRUD Operations
    // ========================================================================

    /// Register a new vocabulary bank.
    ///
    /// # Arguments
    ///
    /// * `bank` - The vocabulary bank to register
    ///
    /// # Validation
    ///
    /// - Bank ID must be non-empty
    /// - Bank must have at least one phrase category
    ///
    /// # Errors
    ///
    /// Returns `ArchetypeError::InvalidVocabularyBank` if validation fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let bank = VocabularyBankDefinition::new("dwarvish_merchant", "Dwarvish Merchant")
    ///     .add_phrases("greetings", vec![PhraseDefinition::new("Stone and steel!")]);
    /// manager.register(bank).await?;
    /// ```
    pub async fn register(&self, bank: impl Into<VocabularyBank>) -> Result<String> {
        let bank = bank.into();

        // Validate
        self.validate_bank(&bank)?;

        let id = bank.definition.id.clone();

        // Persist to Meilisearch if available
        if let Some(ref client) = self.meili {
            self.persist_bank(client, &bank).await?;
        }

        // Store in memory
        {
            let mut banks = self.banks.write().await;
            banks.insert(id.clone(), bank);
        }

        log::info!("Registered vocabulary bank: {}", id);

        Ok(id)
    }

    /// Get a vocabulary bank by ID.
    ///
    /// # Errors
    ///
    /// Returns `ArchetypeError::VocabularyBankNotFound` if the bank doesn't exist.
    pub async fn get_bank(&self, id: &str) -> Result<VocabularyBank> {
        let banks = self.banks.read().await;
        banks
            .get(id)
            .cloned()
            .ok_or_else(|| ArchetypeError::VocabularyBankNotFound(id.to_string()))
    }

    /// List all vocabulary banks with optional filtering.
    ///
    /// # Arguments
    ///
    /// * `filter` - Optional filter criteria
    ///
    /// # Returns
    ///
    /// Vector of bank summaries matching the filter.
    pub async fn list_banks(&self, filter: Option<BankListFilter>) -> Vec<VocabularyBankSummary> {
        let banks = self.banks.read().await;

        banks
            .values()
            .filter(|bank| {
                if let Some(ref f) = filter {
                    // Apply filters
                    if let Some(ref culture) = f.culture {
                        if bank.definition.culture.as_deref() != Some(culture.as_str()) {
                            return false;
                        }
                    }
                    if let Some(ref role) = f.role {
                        if bank.definition.role.as_deref() != Some(role.as_str()) {
                            return false;
                        }
                    }
                    if let Some(ref race) = f.race {
                        if bank.definition.race.as_deref() != Some(race.as_str()) {
                            return false;
                        }
                    }
                    if let Some(builtin_only) = f.builtin_only {
                        if builtin_only && !bank.is_builtin {
                            return false;
                        }
                    }
                }
                true
            })
            .map(VocabularyBankSummary::from)
            .collect()
    }

    /// Update an existing vocabulary bank.
    ///
    /// # Arguments
    ///
    /// * `bank` - The updated bank (must have existing ID)
    ///
    /// # Errors
    ///
    /// Returns `ArchetypeError::VocabularyBankNotFound` if the bank doesn't exist.
    pub async fn update(&self, mut bank: VocabularyBank) -> Result<()> {
        let id = bank.definition.id.clone();

        // Check exists
        {
            let banks = self.banks.read().await;
            if !banks.contains_key(&id) {
                return Err(ArchetypeError::VocabularyBankNotFound(id));
            }
        }

        // Validate
        self.validate_bank(&bank)?;

        // Update timestamp
        bank.touch();

        // Persist to Meilisearch if available
        if let Some(ref meili) = self.meili {
            self.persist_bank(meili, &bank).await?;
        }

        // Store in memory
        {
            let mut banks = self.banks.write().await;
            banks.insert(id.clone(), bank);
        }

        log::info!("Updated vocabulary bank: {}", id);

        Ok(())
    }

    /// Delete a vocabulary bank.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the bank to delete
    ///
    /// # Errors
    ///
    /// - `ArchetypeError::VocabularyBankNotFound` if the bank doesn't exist
    /// - `ArchetypeError::VocabularyBankInUse` if archetypes reference the bank
    ///
    /// # Note
    ///
    /// Built-in banks cannot be deleted.
    pub async fn delete_bank(&self, id: &str) -> Result<()> {
        // Check exists and get reference info
        let (is_builtin, references) = {
            let banks = self.banks.read().await;
            let bank = banks
                .get(id)
                .ok_or_else(|| ArchetypeError::VocabularyBankNotFound(id.to_string()))?;

            (bank.is_builtin, bank.referencing_archetypes.clone())
        };

        // Cannot delete built-in banks
        if is_builtin {
            return Err(ArchetypeError::InvalidVocabularyBank {
                reason: format!("Cannot delete built-in bank: {}", id),
            });
        }

        // Check for references
        if !references.is_empty() {
            return Err(ArchetypeError::VocabularyBankInUse {
                bank_id: id.to_string(),
                archetype_ids: references,
            });
        }

        // Delete from Meilisearch if available
        if let Some(ref meili) = self.meili {
            self.delete_from_meilisearch(meili, id).await?;
        }

        // Remove from memory
        {
            let mut banks = self.banks.write().await;
            banks.remove(id);
        }

        log::info!("Deleted vocabulary bank: {}", id);

        Ok(())
    }

    /// Check if a bank exists.
    pub async fn exists(&self, id: &str) -> bool {
        let banks = self.banks.read().await;
        banks.contains_key(id)
    }

    /// Get the count of registered banks.
    pub async fn count(&self) -> usize {
        let banks = self.banks.read().await;
        banks.len()
    }

    // ========================================================================
    // Phrase Retrieval
    // ========================================================================

    /// Get phrases from a bank with filtering and usage tracking.
    ///
    /// This method:
    /// 1. Retrieves phrases matching the filter criteria
    /// 2. Excludes phrases already used in this session
    /// 3. Resets usage when all phrases exhausted (per AR 1.2.10)
    ///
    /// # Arguments
    ///
    /// * `bank_id` - The vocabulary bank ID
    /// * `options` - Filter options (category, formality, tone)
    /// * `session_id` - Session ID for usage tracking
    ///
    /// # Returns
    ///
    /// Vector of phrase texts matching the criteria.
    ///
    /// # Errors
    ///
    /// Returns `ArchetypeError::VocabularyBankNotFound` if bank doesn't exist.
    pub async fn get_phrases(
        &self,
        bank_id: &str,
        options: PhraseFilterOptions,
        session_id: &str,
    ) -> Result<Vec<String>> {
        let bank = self.get_bank(bank_id).await?;

        // Get phrases for category
        let category_phrases = bank
            .definition
            .phrases
            .get(&options.category)
            .cloned()
            .unwrap_or_default();

        // Apply filters
        let filtered: Vec<PhraseDefinition> = category_phrases
            .into_iter()
            .filter(|p| {
                // Formality filter
                if let Some((min, max)) = options.formality_range {
                    if p.formality < min || p.formality > max {
                        return false;
                    }
                }

                // Tone filter
                if let Some(ref tone) = options.tone {
                    if !p.tone_markers.iter().any(|t| t == tone) {
                        return false;
                    }
                }

                true
            })
            .collect();

        // Get used phrases for this session/bank/category
        let used_phrases = {
            let usage = self.session_usage.read().await;
            usage
                .get(session_id)
                .and_then(|banks| banks.get(bank_id))
                .and_then(|categories| categories.get(&options.category))
                .cloned()
                .unwrap_or_default()
        };

        // Filter out used phrases
        let available: Vec<String> = filtered
            .iter()
            .filter(|p| !used_phrases.contains(&p.text))
            .map(|p| p.text.clone())
            .collect();

        // If all phrases exhausted, reset usage and return all
        if available.is_empty() && !filtered.is_empty() {
            // Clear usage for this category
            {
                let mut usage = self.session_usage.write().await;
                if let Some(banks) = usage.get_mut(session_id) {
                    if let Some(categories) = banks.get_mut(bank_id) {
                        categories.remove(&options.category);
                    }
                }
            }

            // Return all phrases
            let all_phrases: Vec<String> = filtered.iter().map(|p| p.text.clone()).collect();

            // Apply limit
            if let Some(limit) = options.limit {
                return Ok(all_phrases.into_iter().take(limit).collect());
            }

            return Ok(all_phrases);
        }

        // Apply limit
        if let Some(limit) = options.limit {
            return Ok(available.into_iter().take(limit).collect());
        }

        Ok(available)
    }

    /// Mark a phrase as used in a session.
    ///
    /// # Arguments
    ///
    /// * `bank_id` - The vocabulary bank ID
    /// * `category` - The phrase category
    /// * `phrase` - The phrase text that was used
    /// * `session_id` - Session ID for tracking
    pub async fn mark_used(&self, bank_id: &str, category: &str, phrase: &str, session_id: &str) {
        let mut usage = self.session_usage.write().await;

        usage
            .entry(session_id.to_string())
            .or_default()
            .entry(bank_id.to_string())
            .or_default()
            .entry(category.to_string())
            .or_default()
            .insert(phrase.to_string());
    }

    /// Clear usage tracking for a session.
    pub async fn clear_session_usage(&self, session_id: &str) {
        let mut usage = self.session_usage.write().await;
        usage.remove(session_id);
    }

    // ========================================================================
    // Bank Merging
    // ========================================================================

    /// Merge two vocabulary banks with overlay precedence.
    ///
    /// The overlay bank's phrases take precedence over the base bank.
    /// Duplicate phrases (by text) are deduplicated, keeping the overlay version.
    /// Categories are truncated to 100 phrases max, sorted by priority.
    ///
    /// # Arguments
    ///
    /// * `base_id` - The base bank ID
    /// * `overlay_id` - The overlay bank ID (takes precedence)
    ///
    /// # Returns
    ///
    /// A new merged bank (not persisted).
    ///
    /// # Errors
    ///
    /// Returns `ArchetypeError::VocabularyBankNotFound` if either bank doesn't exist.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Merge dwarvish base with merchant overlay
    /// let merged = manager.merge_banks("dwarvish", "merchant").await?;
    /// // merged.id = "merged_dwarvish_merchant"
    /// ```
    pub async fn merge_banks(&self, base_id: &str, overlay_id: &str) -> Result<VocabularyBank> {
        let base = self.get_bank(base_id).await?;
        let overlay = self.get_bank(overlay_id).await?;

        let merged = self.merge_bank_definitions(&base.definition, &overlay.definition);

        Ok(VocabularyBank::from_definition(merged))
    }

    /// Merge two bank definitions (internal helper).
    pub fn merge_bank_definitions(
        &self,
        base: &VocabularyBankDefinition,
        overlay: &VocabularyBankDefinition,
    ) -> VocabularyBankDefinition {
        let mut merged_phrases: HashMap<String, Vec<PhraseDefinition>> = base.phrases.clone();

        for (category, overlay_phrases) in &overlay.phrases {
            let entry = merged_phrases.entry(category.clone()).or_default();

            for phrase in overlay_phrases {
                // Remove duplicates (by text), keeping overlay version
                entry.retain(|p| p.text != phrase.text);
                entry.push(phrase.clone());
            }

            // Keep only the top MAX_PHRASES_PER_CATEGORY by priority (higher first)
            if entry.len() > MAX_PHRASES_PER_CATEGORY {
                let n = MAX_PHRASES_PER_CATEGORY;
                // Partially partition so the top `n` elements (by descending priority)
                // are in the first `n` positions, then sort only that prefix.
                entry.select_nth_unstable_by(n - 1, |a, b| b.priority.cmp(&a.priority));
                entry[..n].sort_by(|a, b| b.priority.cmp(&a.priority));
                entry.truncate(n);
            }
        }

        VocabularyBankDefinition {
            id: format!("merged_{}_{}", base.id, overlay.id),
            display_name: format!("{} + {}", base.display_name, overlay.display_name),
            description: overlay.description.clone().or_else(|| base.description.clone()),
            culture: overlay.culture.clone().or_else(|| base.culture.clone()),
            role: overlay.role.clone().or_else(|| base.role.clone()),
            race: overlay.race.clone().or_else(|| base.race.clone()),
            phrases: merged_phrases,
            version: "1.0.0".to_string(),
        }
    }

    // ========================================================================
    // Reference Tracking
    // ========================================================================

    /// Add an archetype reference to a bank.
    ///
    /// Called when an archetype starts using this vocabulary bank.
    pub async fn add_archetype_reference(&self, bank_id: &str, archetype_id: &str) -> Result<()> {
        let mut banks = self.banks.write().await;

        let bank = banks
            .get_mut(bank_id)
            .ok_or_else(|| ArchetypeError::VocabularyBankNotFound(bank_id.to_string()))?;

        bank.add_reference(archetype_id);

        Ok(())
    }

    /// Remove an archetype reference from a bank.
    ///
    /// Called when an archetype stops using this vocabulary bank.
    pub async fn remove_archetype_reference(&self, bank_id: &str, archetype_id: &str) -> Result<()> {
        let mut banks = self.banks.write().await;

        if let Some(bank) = banks.get_mut(bank_id) {
            bank.remove_reference(archetype_id);
        }

        Ok(())
    }

    // ========================================================================
    // Loading
    // ========================================================================

    /// Load banks from Meilisearch into memory.
    pub async fn load_from_meilisearch(&self) -> Result<()> {
        let meili = self
            .meili
            .as_ref()
            .ok_or_else(|| ArchetypeError::Meilisearch("No Meilisearch client configured".to_string()))?;

        let index = meili.get_index(INDEX_VOCABULARY_BANKS)
            .map_err(|e| ArchetypeError::Meilisearch(e.to_string()))?;

        let mut offset = 0;
        let limit = 100;

        loop {
            let mut query = SearchQuery::new("");
            query.limit = limit;
            query.offset = offset;

            let results = index.search(&query)
                .map_err(|e| ArchetypeError::Meilisearch(e.to_string()))?;

            let count = results.hits.len();
            if count == 0 {
                break;
            }

            {
                let mut banks = self.banks.write().await;
                for hit in results.hits {
                    if let Ok(bank) = serde_json::from_value::<VocabularyBank>(hit.document) {
                        banks.insert(bank.definition.id.clone(), bank);
                    }
                }
            }

            offset += count;

            if count < limit {
                break;
            }
        }

        log::info!("Loaded {} vocabulary banks from Meilisearch", offset);

        Ok(())
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    /// Validate a vocabulary bank.
    fn validate_bank(&self, bank: &VocabularyBank) -> Result<()> {
        // ID validation
        if bank.definition.id.is_empty() {
            return Err(ArchetypeError::InvalidVocabularyBank {
                reason: "Bank ID cannot be empty".to_string(),
            });
        }

        // Must have at least one phrase category
        if bank.definition.phrases.is_empty() {
            return Err(ArchetypeError::InvalidVocabularyBank {
                reason: "Bank must contain at least one phrase category".to_string(),
            });
        }

        Ok(())
    }

    /// Persist a bank to Meilisearch.
    async fn persist_bank(&self, meili: &Arc<Meilisearch>, bank: &VocabularyBank) -> Result<()> {
        let index = meili.get_index(INDEX_VOCABULARY_BANKS)
            .map_err(|e| ArchetypeError::Meilisearch(e.to_string()))?;

        let doc = serde_json::to_value(bank)?;
        index.add_documents(vec![doc], Some("definition.id"))
            .map_err(|e| ArchetypeError::Meilisearch(e.to_string()))?;

        Ok(())
    }

    /// Delete a bank from Meilisearch.
    async fn delete_from_meilisearch(&self, meili: &Arc<Meilisearch>, id: &str) -> Result<()> {
        let index = meili.get_index(INDEX_VOCABULARY_BANKS)
            .map_err(|e| ArchetypeError::Meilisearch(e.to_string()))?;

        index.delete_document(id)
            .map_err(|e| ArchetypeError::Meilisearch(e.to_string()))?;

        Ok(())
    }
}

impl Default for VocabularyBankManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// VocabularyBankSummary - Lightweight listing type
// ============================================================================

/// Lightweight summary of a vocabulary bank for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VocabularyBankSummary {
    /// Bank ID.
    pub id: String,

    /// Display name.
    pub display_name: String,

    /// Culture context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub culture: Option<String>,

    /// Role context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Whether this is a built-in bank.
    pub is_builtin: bool,

    /// Number of phrase categories.
    pub category_count: usize,

    /// Total phrase count across all categories.
    pub phrase_count: usize,
}

impl From<&VocabularyBank> for VocabularyBankSummary {
    fn from(bank: &VocabularyBank) -> Self {
        Self {
            id: bank.definition.id.clone(),
            display_name: bank.definition.display_name.clone(),
            culture: bank.definition.culture.clone(),
            role: bank.definition.role.clone(),
            is_builtin: bank.is_builtin,
            category_count: bank.definition.phrases.len(),
            phrase_count: bank.definition.phrase_count(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Helper functions
    // -------------------------------------------------------------------------

    fn create_test_bank(id: &str) -> VocabularyBank {
        let definition = VocabularyBankDefinition::new(id, format!("Test Bank {}", id))
            .add_phrases(
                "greetings",
                vec![
                    PhraseDefinition::new("Hello there!").with_formality(5),
                    PhraseDefinition::new("Greetings, traveler!").with_formality(7),
                    PhraseDefinition::new("Hey!").with_formality(2),
                ],
            )
            .add_phrases(
                "farewells",
                vec![
                    PhraseDefinition::new("Farewell!").with_formality(6),
                    PhraseDefinition::new("See ya!").with_formality(2),
                ],
            );

        VocabularyBank::from_definition(definition)
    }

    fn create_bank_with_tones(id: &str) -> VocabularyBank {
        let definition = VocabularyBankDefinition::new(id, format!("Tone Bank {}", id)).add_phrases(
            "greetings",
            vec![
                PhraseDefinition::new("Welcome, friend!")
                    .with_formality(5)
                    .with_tone(vec!["friendly".to_string()]),
                PhraseDefinition::new("State your business.")
                    .with_formality(6)
                    .with_tone(vec!["hostile".to_string(), "formal".to_string()]),
                PhraseDefinition::new("Greetings!")
                    .with_formality(5)
                    .with_tone(vec!["neutral".to_string()]),
            ],
        );

        VocabularyBank::from_definition(definition)
    }

    // -------------------------------------------------------------------------
    // VocabularyBank tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_vocabulary_bank_creation() {
        let bank = create_test_bank("test");

        assert_eq!(bank.definition.id, "test");
        assert!(!bank.is_builtin);
        assert!(bank.referencing_archetypes.is_empty());
        assert!(bank.created_at.is_some());
    }

    #[test]
    fn test_vocabulary_bank_as_builtin() {
        let bank = create_test_bank("test").as_builtin();
        assert!(bank.is_builtin);
    }

    #[test]
    fn test_vocabulary_bank_references() {
        let mut bank = create_test_bank("test");

        bank.add_reference("dwarf");
        assert!(bank.is_referenced());
        assert!(bank.referencing_archetypes.contains(&"dwarf".to_string()));

        // Adding same reference again shouldn't duplicate
        bank.add_reference("dwarf");
        assert_eq!(bank.referencing_archetypes.len(), 1);

        bank.remove_reference("dwarf");
        assert!(!bank.is_referenced());
    }

    // -------------------------------------------------------------------------
    // PhraseFilterOptions tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_phrase_filter_options() {
        let options = PhraseFilterOptions::for_category("greetings")
            .with_formality(3, 7)
            .with_tone("friendly")
            .with_limit(5);

        assert_eq!(options.category, "greetings");
        assert_eq!(options.formality_range, Some((3, 7)));
        assert_eq!(options.tone, Some("friendly".to_string()));
        assert_eq!(options.limit, Some(5));
    }

    #[test]
    fn test_phrase_filter_formality_clamping() {
        let options = PhraseFilterOptions::for_category("test").with_formality(0, 15);

        // Should be clamped to 1-10 range
        assert_eq!(options.formality_range, Some((1, 10)));
    }

    // -------------------------------------------------------------------------
    // VocabularyBankManager CRUD tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_manager_register_and_get() {
        let manager = VocabularyBankManager::new();
        let bank = create_test_bank("test_bank");

        let id = manager.register(bank.clone()).await.unwrap();
        assert_eq!(id, "test_bank");

        let retrieved = manager.get_bank("test_bank").await.unwrap();
        assert_eq!(retrieved.definition.id, "test_bank");
    }

    #[tokio::test]
    async fn test_manager_register_validation_empty_id() {
        let manager = VocabularyBankManager::new();

        let definition = VocabularyBankDefinition::new("", "Empty ID").add_phrases(
            "greetings",
            vec![PhraseDefinition::new("Hello!")],
        );
        let bank = VocabularyBank::from_definition(definition);

        let result = manager.register(bank).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("ID cannot be empty"));
    }

    #[tokio::test]
    async fn test_manager_register_validation_no_phrases() {
        let manager = VocabularyBankManager::new();

        let definition = VocabularyBankDefinition::new("empty_bank", "Empty Phrases");
        let bank = VocabularyBank::from_definition(definition);

        let result = manager.register(bank).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("at least one phrase category"));
    }

    #[tokio::test]
    async fn test_manager_get_not_found() {
        let manager = VocabularyBankManager::new();

        let result = manager.get_bank("nonexistent").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ArchetypeError::VocabularyBankNotFound(id) => {
                assert_eq!(id, "nonexistent");
            }
            _ => panic!("Expected VocabularyBankNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_manager_list_banks() {
        let manager = VocabularyBankManager::new();

        manager.register(create_test_bank("bank1")).await.unwrap();
        manager.register(create_test_bank("bank2")).await.unwrap();

        let list = manager.list_banks(None).await;
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_manager_list_banks_with_filter() {
        let manager = VocabularyBankManager::new();

        let mut bank1_def = VocabularyBankDefinition::new("bank1", "Bank 1")
            .add_phrases("greetings", vec![PhraseDefinition::new("Hello!")]);
        bank1_def.culture = Some("dwarvish".to_string());
        manager
            .register(VocabularyBank::from_definition(bank1_def))
            .await
            .unwrap();

        let mut bank2_def = VocabularyBankDefinition::new("bank2", "Bank 2")
            .add_phrases("greetings", vec![PhraseDefinition::new("Greetings!")]);
        bank2_def.culture = Some("elvish".to_string());
        manager
            .register(VocabularyBank::from_definition(bank2_def))
            .await
            .unwrap();

        // Filter by culture
        let filter = BankListFilter::by_culture("dwarvish");
        let list = manager.list_banks(Some(filter)).await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "bank1");
    }

    #[tokio::test]
    async fn test_manager_update() {
        let manager = VocabularyBankManager::new();

        let bank = create_test_bank("update_test");
        manager.register(bank).await.unwrap();

        // Get and modify
        let mut bank = manager.get_bank("update_test").await.unwrap();
        bank.definition.display_name = "Updated Name".to_string();

        manager.update(bank).await.unwrap();

        let retrieved = manager.get_bank("update_test").await.unwrap();
        assert_eq!(retrieved.definition.display_name, "Updated Name");
    }

    #[tokio::test]
    async fn test_manager_delete() {
        let manager = VocabularyBankManager::new();

        manager.register(create_test_bank("delete_test")).await.unwrap();
        assert!(manager.exists("delete_test").await);

        manager.delete_bank("delete_test").await.unwrap();
        assert!(!manager.exists("delete_test").await);
    }

    #[tokio::test]
    async fn test_manager_delete_in_use() {
        let manager = VocabularyBankManager::new();

        manager.register(create_test_bank("in_use")).await.unwrap();
        manager
            .add_archetype_reference("in_use", "dwarf_merchant")
            .await
            .unwrap();

        let result = manager.delete_bank("in_use").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ArchetypeError::VocabularyBankInUse {
                bank_id,
                archetype_ids,
            } => {
                assert_eq!(bank_id, "in_use");
                assert!(archetype_ids.contains(&"dwarf_merchant".to_string()));
            }
            _ => panic!("Expected VocabularyBankInUse error"),
        }
    }

    #[tokio::test]
    async fn test_manager_delete_builtin() {
        let manager = VocabularyBankManager::new();

        let bank = create_test_bank("builtin_test").as_builtin();
        manager.register(bank).await.unwrap();

        let result = manager.delete_bank("builtin_test").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot delete built-in"));
    }

    // -------------------------------------------------------------------------
    // Phrase retrieval tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_get_phrases_basic() {
        let manager = VocabularyBankManager::new();
        manager.register(create_test_bank("test")).await.unwrap();

        let options = PhraseFilterOptions::for_category("greetings");
        let phrases = manager.get_phrases("test", options, "session_1").await.unwrap();

        assert_eq!(phrases.len(), 3);
        assert!(phrases.contains(&"Hello there!".to_string()));
    }

    #[tokio::test]
    async fn test_get_phrases_with_formality_filter() {
        let manager = VocabularyBankManager::new();
        manager.register(create_test_bank("test")).await.unwrap();

        // Filter for medium-high formality (5-10)
        let options = PhraseFilterOptions::for_category("greetings").with_formality(5, 10);
        let phrases = manager.get_phrases("test", options, "session_1").await.unwrap();

        // Should exclude "Hey!" (formality 2)
        assert_eq!(phrases.len(), 2);
        assert!(!phrases.contains(&"Hey!".to_string()));
    }

    #[tokio::test]
    async fn test_get_phrases_with_tone_filter() {
        let manager = VocabularyBankManager::new();
        manager.register(create_bank_with_tones("test")).await.unwrap();

        let options = PhraseFilterOptions::for_category("greetings").with_tone("friendly");
        let phrases = manager.get_phrases("test", options, "session_1").await.unwrap();

        assert_eq!(phrases.len(), 1);
        assert_eq!(phrases[0], "Welcome, friend!");
    }

    #[tokio::test]
    async fn test_get_phrases_with_limit() {
        let manager = VocabularyBankManager::new();
        manager.register(create_test_bank("test")).await.unwrap();

        let options = PhraseFilterOptions::for_category("greetings").with_limit(1);
        let phrases = manager.get_phrases("test", options, "session_1").await.unwrap();

        assert_eq!(phrases.len(), 1);
    }

    #[tokio::test]
    async fn test_get_phrases_usage_tracking() {
        let manager = VocabularyBankManager::new();
        manager.register(create_test_bank("test")).await.unwrap();

        let options = PhraseFilterOptions::for_category("greetings");

        // First retrieval
        let phrases1 = manager
            .get_phrases("test", options.clone(), "session_1")
            .await
            .unwrap();
        assert_eq!(phrases1.len(), 3);

        // Mark all as used
        for phrase in &phrases1 {
            manager
                .mark_used("test", "greetings", phrase, "session_1")
                .await;
        }

        // Second retrieval should reset and return all (per AR 1.2.10)
        let phrases2 = manager
            .get_phrases("test", options.clone(), "session_1")
            .await
            .unwrap();
        assert_eq!(phrases2.len(), 3);
    }

    #[tokio::test]
    async fn test_mark_used() {
        let manager = VocabularyBankManager::new();
        manager.register(create_test_bank("test")).await.unwrap();

        manager
            .mark_used("test", "greetings", "Hello there!", "session_1")
            .await;

        let options = PhraseFilterOptions::for_category("greetings");
        let phrases = manager.get_phrases("test", options, "session_1").await.unwrap();

        // Should exclude the used phrase
        assert_eq!(phrases.len(), 2);
        assert!(!phrases.contains(&"Hello there!".to_string()));
    }

    #[tokio::test]
    async fn test_clear_session_usage() {
        let manager = VocabularyBankManager::new();
        manager.register(create_test_bank("test")).await.unwrap();

        // Mark some phrases used
        manager
            .mark_used("test", "greetings", "Hello there!", "session_1")
            .await;
        manager
            .mark_used("test", "greetings", "Greetings, traveler!", "session_1")
            .await;

        // Clear session
        manager.clear_session_usage("session_1").await;

        // All phrases should be available again
        let options = PhraseFilterOptions::for_category("greetings");
        let phrases = manager.get_phrases("test", options, "session_1").await.unwrap();
        assert_eq!(phrases.len(), 3);
    }

    // -------------------------------------------------------------------------
    // Bank merging tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_merge_banks() {
        let manager = VocabularyBankManager::new();

        // Base bank with greetings
        let base_def = VocabularyBankDefinition::new("base", "Base Bank").add_phrases(
            "greetings",
            vec![
                PhraseDefinition::new("Hello!"),
                PhraseDefinition::new("Hi there!"),
            ],
        );
        manager
            .register(VocabularyBank::from_definition(base_def))
            .await
            .unwrap();

        // Overlay bank with different greeting and farewells
        let overlay_def = VocabularyBankDefinition::new("overlay", "Overlay Bank")
            .add_phrases(
                "greetings",
                vec![
                    PhraseDefinition::new("Hello!"), // Duplicate - should override
                    PhraseDefinition::new("Greetings!"), // New
                ],
            )
            .add_phrases("farewells", vec![PhraseDefinition::new("Goodbye!")]);
        manager
            .register(VocabularyBank::from_definition(overlay_def))
            .await
            .unwrap();

        let merged = manager.merge_banks("base", "overlay").await.unwrap();

        assert_eq!(merged.definition.id, "merged_base_overlay");

        // Should have 3 unique greetings (Hello deduplicated)
        let greetings = merged.definition.phrases.get("greetings").unwrap();
        assert_eq!(greetings.len(), 3);

        // Should have farewells from overlay
        assert!(merged.definition.phrases.contains_key("farewells"));
    }

    #[tokio::test]
    async fn test_merge_banks_truncation() {
        let manager = VocabularyBankManager::new();

        // Create banks with lots of phrases
        let mut base_phrases = vec![];
        for i in 0..60 {
            base_phrases.push(PhraseDefinition::new(format!("Base phrase {}", i)));
        }

        let base_def = VocabularyBankDefinition::new("base", "Base").add_phrases("greetings", base_phrases);
        manager
            .register(VocabularyBank::from_definition(base_def))
            .await
            .unwrap();

        let mut overlay_phrases = vec![];
        for i in 0..60 {
            let mut phrase = PhraseDefinition::new(format!("Overlay phrase {}", i));
            phrase.priority = (60 - i) as u8; // Higher priority for earlier phrases
            overlay_phrases.push(phrase);
        }

        let overlay_def =
            VocabularyBankDefinition::new("overlay", "Overlay").add_phrases("greetings", overlay_phrases);
        manager
            .register(VocabularyBank::from_definition(overlay_def))
            .await
            .unwrap();

        let merged = manager.merge_banks("base", "overlay").await.unwrap();

        // Should be truncated to 100 phrases
        let greetings = merged.definition.phrases.get("greetings").unwrap();
        assert_eq!(greetings.len(), MAX_PHRASES_PER_CATEGORY);
    }

    // -------------------------------------------------------------------------
    // Reference tracking tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_archetype_reference_tracking() {
        let manager = VocabularyBankManager::new();
        manager.register(create_test_bank("test")).await.unwrap();

        // Add reference
        manager
            .add_archetype_reference("test", "dwarf")
            .await
            .unwrap();

        let bank = manager.get_bank("test").await.unwrap();
        assert!(bank.referencing_archetypes.contains(&"dwarf".to_string()));

        // Remove reference
        manager
            .remove_archetype_reference("test", "dwarf")
            .await
            .unwrap();

        let bank = manager.get_bank("test").await.unwrap();
        assert!(!bank.referencing_archetypes.contains(&"dwarf".to_string()));
    }

    // -------------------------------------------------------------------------
    // VocabularyBankSummary tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_vocabulary_bank_summary() {
        let bank = create_test_bank("test");
        let summary = VocabularyBankSummary::from(&bank);

        assert_eq!(summary.id, "test");
        assert_eq!(summary.category_count, 2); // greetings and farewells
        assert_eq!(summary.phrase_count, 5); // 3 greetings + 2 farewells
        assert!(!summary.is_builtin);
    }
}
