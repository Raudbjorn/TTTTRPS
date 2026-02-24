//! In-Memory NPC Index Replacements
//!
//! Provides in-memory storage and filtered lookup for NPC generation data,
//! replacing the three Meilisearch indexes:
//!
//! - `ttrpg_vocabulary_banks` → [`InMemoryVocabularyIndex`]
//! - `ttrpg_name_components` → [`InMemoryNameComponentIndex`]
//! - `ttrpg_exclamation_templates` → [`InMemoryExclamationIndex`]
//!
//! All indexes support the same filter patterns as Meilisearch but use
//! linear scan over Vec storage. At the scale of NPC generation data
//! (hundreds to low thousands of documents), this is negligible.

use super::indexes::{
    ExclamationTemplateDocument, NameComponentDocument, NpcIndexStats, VocabularyPhraseDocument,
};

// ============================================================================
// Vocabulary Phrase Index
// ============================================================================

/// In-memory index for vocabulary phrases.
///
/// Replaces Meilisearch `ttrpg_vocabulary_banks` index with linear filtered lookup.
#[derive(Debug, Default, Clone)]
pub struct InMemoryVocabularyIndex {
    docs: Vec<VocabularyPhraseDocument>,
}

impl InMemoryVocabularyIndex {
    /// Create an empty vocabulary index.
    pub fn new() -> Self {
        Self { docs: Vec::new() }
    }

    /// Bulk-load documents into the index.
    pub fn load(&mut self, docs: Vec<VocabularyPhraseDocument>) {
        self.docs = docs;
    }

    /// Add a single document.
    pub fn add(&mut self, doc: VocabularyPhraseDocument) {
        // Replace if ID exists, otherwise append
        if let Some(existing) = self.docs.iter_mut().find(|d| d.id == doc.id) {
            *existing = doc;
        } else {
            self.docs.push(doc);
        }
    }

    /// Get document count.
    pub fn count(&self) -> u64 {
        self.docs.len() as u64
    }

    /// Search phrases with filters.
    ///
    /// All filter parameters are optional — `None` means no filter on that field.
    /// Results are sorted by frequency descending.
    pub fn search(
        &self,
        culture: Option<&str>,
        role: Option<&str>,
        race: Option<&str>,
        category: Option<&str>,
        formality: Option<&str>,
        bank_id: Option<&str>,
        limit: usize,
    ) -> Vec<&VocabularyPhraseDocument> {
        let mut results: Vec<&VocabularyPhraseDocument> = self
            .docs
            .iter()
            .filter(|d| culture.map_or(true, |c| d.culture.as_deref() == Some(c)))
            .filter(|d| role.map_or(true, |r| d.role.as_deref() == Some(r)))
            .filter(|d| race.map_or(true, |r| d.race.as_deref() == Some(r)))
            .filter(|d| category.map_or(true, |c| d.category == c))
            .filter(|d| formality.map_or(true, |f| d.formality == f))
            .filter(|d| bank_id.map_or(true, |b| d.bank_id == b))
            .collect();

        // Sort by frequency descending
        results.sort_by(|a, b| b.frequency.partial_cmp(&a.frequency).unwrap_or(std::cmp::Ordering::Equal));

        if limit > 0 {
            results.truncate(limit);
        }

        results
    }

    /// Full-text search on phrase text.
    pub fn text_search(&self, query: &str, limit: usize) -> Vec<&VocabularyPhraseDocument> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<&VocabularyPhraseDocument> = self
            .docs
            .iter()
            .filter(|d| d.phrase.to_lowercase().contains(&query_lower))
            .collect();

        results.sort_by(|a, b| b.frequency.partial_cmp(&a.frequency).unwrap_or(std::cmp::Ordering::Equal));

        if limit > 0 {
            results.truncate(limit);
        }

        results
    }

    /// Get all unique cultures present in the index.
    pub fn cultures(&self) -> Vec<String> {
        let mut cultures: Vec<String> = self
            .docs
            .iter()
            .filter_map(|d| d.culture.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        cultures.sort();
        cultures
    }
}

// ============================================================================
// Name Component Index
// ============================================================================

/// In-memory index for name components.
///
/// Replaces Meilisearch `ttrpg_name_components` index.
#[derive(Debug, Default, Clone)]
pub struct InMemoryNameComponentIndex {
    docs: Vec<NameComponentDocument>,
}

impl InMemoryNameComponentIndex {
    /// Create an empty name component index.
    pub fn new() -> Self {
        Self { docs: Vec::new() }
    }

    /// Bulk-load documents.
    pub fn load(&mut self, docs: Vec<NameComponentDocument>) {
        self.docs = docs;
    }

    /// Add a single document.
    pub fn add(&mut self, doc: NameComponentDocument) {
        if let Some(existing) = self.docs.iter_mut().find(|d| d.id == doc.id) {
            *existing = doc;
        } else {
            self.docs.push(doc);
        }
    }

    /// Get document count.
    pub fn count(&self) -> u64 {
        self.docs.len() as u64
    }

    /// Search name components with filters.
    ///
    /// All filter parameters are optional.
    /// Results sorted by frequency descending.
    pub fn search(
        &self,
        culture: Option<&str>,
        component_type: Option<&str>,
        gender: Option<&str>,
        limit: usize,
    ) -> Vec<&NameComponentDocument> {
        let mut results: Vec<&NameComponentDocument> = self
            .docs
            .iter()
            .filter(|d| culture.map_or(true, |c| d.culture == c))
            .filter(|d| component_type.map_or(true, |t| d.component_type == t))
            .filter(|d| {
                gender.map_or(true, |g| d.gender == g || d.gender == "any" || d.gender == "neutral")
            })
            .collect();

        results.sort_by(|a, b| b.frequency.partial_cmp(&a.frequency).unwrap_or(std::cmp::Ordering::Equal));

        if limit > 0 {
            results.truncate(limit);
        }

        results
    }

    /// Get all unique cultures present.
    pub fn cultures(&self) -> Vec<String> {
        let mut cultures: Vec<String> = self
            .docs
            .iter()
            .map(|d| d.culture.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        cultures.sort();
        cultures
    }
}

// ============================================================================
// Exclamation Template Index
// ============================================================================

/// In-memory index for exclamation templates.
///
/// Replaces Meilisearch `ttrpg_exclamation_templates` index.
#[derive(Debug, Default, Clone)]
pub struct InMemoryExclamationIndex {
    docs: Vec<ExclamationTemplateDocument>,
}

impl InMemoryExclamationIndex {
    /// Create an empty exclamation index.
    pub fn new() -> Self {
        Self { docs: Vec::new() }
    }

    /// Bulk-load documents.
    pub fn load(&mut self, docs: Vec<ExclamationTemplateDocument>) {
        self.docs = docs;
    }

    /// Add a single document.
    pub fn add(&mut self, doc: ExclamationTemplateDocument) {
        if let Some(existing) = self.docs.iter_mut().find(|d| d.id == doc.id) {
            *existing = doc;
        } else {
            self.docs.push(doc);
        }
    }

    /// Get document count.
    pub fn count(&self) -> u64 {
        self.docs.len() as u64
    }

    /// Search exclamation templates with filters.
    pub fn search(
        &self,
        culture: Option<&str>,
        intensity: Option<&str>,
        emotion: Option<&str>,
        religious: Option<bool>,
        limit: usize,
    ) -> Vec<&ExclamationTemplateDocument> {
        let mut results: Vec<&ExclamationTemplateDocument> = self
            .docs
            .iter()
            .filter(|d| culture.map_or(true, |c| d.culture == c))
            .filter(|d| intensity.map_or(true, |i| d.intensity == i))
            .filter(|d| emotion.map_or(true, |e| d.emotion == e))
            .filter(|d| religious.map_or(true, |r| d.religious == r))
            .collect();

        results.sort_by(|a, b| b.frequency.partial_cmp(&a.frequency).unwrap_or(std::cmp::Ordering::Equal));

        if limit > 0 {
            results.truncate(limit);
        }

        results
    }

    /// Get all unique cultures present.
    pub fn cultures(&self) -> Vec<String> {
        let mut cultures: Vec<String> = self
            .docs
            .iter()
            .map(|d| d.culture.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        cultures.sort();
        cultures
    }
}

// ============================================================================
// Combined NPC Index Manager
// ============================================================================

/// Combined in-memory NPC index manager.
///
/// Replaces the Meilisearch-backed NPC index system with pure in-memory storage.
/// Holds all three NPC indexes and provides aggregate stats.
#[derive(Debug, Default, Clone)]
pub struct InMemoryNpcIndexes {
    pub vocabulary: InMemoryVocabularyIndex,
    pub name_components: InMemoryNameComponentIndex,
    pub exclamations: InMemoryExclamationIndex,
}

impl InMemoryNpcIndexes {
    /// Create empty NPC indexes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get aggregate statistics.
    pub fn stats(&self) -> NpcIndexStats {
        NpcIndexStats {
            vocabulary_phrase_count: self.vocabulary.count(),
            name_component_count: self.name_components.count(),
            exclamation_template_count: self.exclamations.count(),
            indexed_cultures: self.all_cultures(),
        }
    }

    /// Get all unique cultures across all indexes.
    pub fn all_cultures(&self) -> Vec<String> {
        let mut cultures: std::collections::HashSet<String> = std::collections::HashSet::new();
        cultures.extend(self.vocabulary.cultures());
        cultures.extend(self.name_components.cultures());
        cultures.extend(self.exclamations.cultures());
        let mut sorted: Vec<String> = cultures.into_iter().collect();
        sorted.sort();
        sorted
    }

    /// Clear all indexes.
    pub fn clear(&mut self) {
        self.vocabulary = InMemoryVocabularyIndex::new();
        self.name_components = InMemoryNameComponentIndex::new();
        self.exclamations = InMemoryExclamationIndex::new();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_phrase(id: &str, culture: &str, category: &str, freq: f32) -> VocabularyPhraseDocument {
        VocabularyPhraseDocument {
            id: id.to_string(),
            phrase: format!("Test phrase {}", id),
            bank_id: "test_bank".to_string(),
            category: category.to_string(),
            formality: "casual".to_string(),
            culture: Some(culture.to_string()),
            role: None,
            race: None,
            frequency: freq,
            tags: vec![],
        }
    }

    fn sample_name_component(id: &str, culture: &str, ctype: &str) -> NameComponentDocument {
        NameComponentDocument {
            id: id.to_string(),
            component: format!("comp_{}", id),
            culture: culture.to_string(),
            component_type: ctype.to_string(),
            gender: "any".to_string(),
            frequency: 0.5,
            meaning: None,
            phonetic_tags: vec![],
        }
    }

    fn sample_exclamation(id: &str, culture: &str, intensity: &str) -> ExclamationTemplateDocument {
        ExclamationTemplateDocument {
            id: id.to_string(),
            template: format!("Excl {}!", id),
            culture: culture.to_string(),
            intensity: intensity.to_string(),
            emotion: "surprise".to_string(),
            religious: false,
            frequency: 0.5,
        }
    }

    #[test]
    fn test_vocabulary_search_by_culture() {
        let mut idx = InMemoryVocabularyIndex::new();
        idx.load(vec![
            sample_phrase("1", "dwarvish", "greeting", 0.8),
            sample_phrase("2", "elvish", "greeting", 0.9),
            sample_phrase("3", "dwarvish", "farewell", 0.7),
        ]);

        let results = idx.search(Some("dwarvish"), None, None, None, None, None, 10);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_vocabulary_search_by_category() {
        let mut idx = InMemoryVocabularyIndex::new();
        idx.load(vec![
            sample_phrase("1", "dwarvish", "greeting", 0.8),
            sample_phrase("2", "elvish", "greeting", 0.9),
            sample_phrase("3", "dwarvish", "farewell", 0.7),
        ]);

        let results = idx.search(None, None, None, Some("greeting"), None, None, 10);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_vocabulary_sorted_by_frequency() {
        let mut idx = InMemoryVocabularyIndex::new();
        idx.load(vec![
            sample_phrase("1", "dwarvish", "greeting", 0.3),
            sample_phrase("2", "dwarvish", "greeting", 0.9),
            sample_phrase("3", "dwarvish", "greeting", 0.6),
        ]);

        let results = idx.search(None, None, None, None, None, None, 10);
        assert_eq!(results[0].id, "2"); // highest frequency first
        assert_eq!(results[1].id, "3");
        assert_eq!(results[2].id, "1");
    }

    #[test]
    fn test_vocabulary_limit() {
        let mut idx = InMemoryVocabularyIndex::new();
        idx.load(vec![
            sample_phrase("1", "dwarvish", "greeting", 0.8),
            sample_phrase("2", "elvish", "greeting", 0.9),
            sample_phrase("3", "dwarvish", "farewell", 0.7),
        ]);

        let results = idx.search(None, None, None, None, None, None, 2);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_vocabulary_text_search() {
        let mut idx = InMemoryVocabularyIndex::new();
        let mut doc = sample_phrase("1", "dwarvish", "greeting", 0.8);
        doc.phrase = "Well met, stone-brother!".to_string();
        idx.add(doc);

        let results = idx.text_search("stone", 10);
        assert_eq!(results.len(), 1);

        let results = idx.text_search("nonexistent", 10);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_vocabulary_cultures() {
        let mut idx = InMemoryVocabularyIndex::new();
        idx.load(vec![
            sample_phrase("1", "dwarvish", "greeting", 0.8),
            sample_phrase("2", "elvish", "greeting", 0.9),
            sample_phrase("3", "dwarvish", "farewell", 0.7),
        ]);

        let cultures = idx.cultures();
        assert_eq!(cultures, vec!["dwarvish", "elvish"]);
    }

    #[test]
    fn test_name_component_search() {
        let mut idx = InMemoryNameComponentIndex::new();
        idx.load(vec![
            sample_name_component("1", "elvish", "prefix"),
            sample_name_component("2", "elvish", "suffix"),
            sample_name_component("3", "dwarvish", "prefix"),
        ]);

        let results = idx.search(Some("elvish"), Some("prefix"), None, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "1");
    }

    #[test]
    fn test_exclamation_search() {
        let mut idx = InMemoryExclamationIndex::new();
        idx.load(vec![
            sample_exclamation("1", "dwarvish", "strong"),
            sample_exclamation("2", "dwarvish", "mild"),
            sample_exclamation("3", "elvish", "strong"),
        ]);

        let results = idx.search(Some("dwarvish"), Some("strong"), None, None, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "1");
    }

    #[test]
    fn test_combined_stats() {
        let mut indexes = InMemoryNpcIndexes::new();
        indexes.vocabulary.load(vec![
            sample_phrase("1", "dwarvish", "greeting", 0.8),
        ]);
        indexes.name_components.load(vec![
            sample_name_component("1", "elvish", "prefix"),
            sample_name_component("2", "elvish", "suffix"),
        ]);
        indexes.exclamations.load(vec![
            sample_exclamation("1", "dwarvish", "strong"),
        ]);

        let stats = indexes.stats();
        assert_eq!(stats.vocabulary_phrase_count, 1);
        assert_eq!(stats.name_component_count, 2);
        assert_eq!(stats.exclamation_template_count, 1);
        assert_eq!(stats.indexed_cultures, vec!["dwarvish", "elvish"]);
    }

    #[test]
    fn test_clear_indexes() {
        let mut indexes = InMemoryNpcIndexes::new();
        indexes.vocabulary.load(vec![
            sample_phrase("1", "dwarvish", "greeting", 0.8),
        ]);

        assert_eq!(indexes.vocabulary.count(), 1);
        indexes.clear();
        assert_eq!(indexes.vocabulary.count(), 0);
    }

    #[test]
    fn test_add_replaces_existing() {
        let mut idx = InMemoryVocabularyIndex::new();
        idx.add(sample_phrase("1", "dwarvish", "greeting", 0.5));
        idx.add(sample_phrase("1", "elvish", "farewell", 0.9));

        assert_eq!(idx.count(), 1);
        let results = idx.search(None, None, None, None, None, None, 10);
        assert_eq!(results[0].culture, Some("elvish".to_string()));
    }
}
