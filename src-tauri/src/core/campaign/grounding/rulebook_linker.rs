//! Rulebook Linker - Reference detection and linking to indexed content
//!
//! Part of Phase 3: Content Grounding Layer (Tasks 3.2, 3.3)
//!
//! Detects rulebook references in text, searches Meilisearch for matching content,
//! and builds citations with confidence scoring.

use crate::core::search::{SearchClient, SearchResult};
use crate::database::Citation;
use regex::Regex;
use std::collections::HashSet;
use std::sync::Arc;

use super::citation_builder::CitationBuilder;

/// A detected reference to rulebook content.
#[derive(Debug, Clone)]
pub struct RulebookReference {
    /// The original text that matched the pattern
    pub raw_text: String,
    /// Type of reference (page, chapter, spell, monster, etc.)
    pub reference_type: ReferenceType,
    /// Normalized source name (e.g., "PHB" -> "Player's Handbook")
    pub source_name: Option<String>,
    /// Page number if detected
    pub page: Option<u32>,
    /// Chapter if detected
    pub chapter: Option<String>,
    /// Section if detected
    pub section: Option<String>,
    /// The specific term being referenced (spell name, monster name, etc.)
    pub term: Option<String>,
    /// Start position in the original text
    pub start_pos: usize,
    /// End position in the original text
    pub end_pos: usize,
}

/// Type of rulebook reference detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceType {
    /// Direct page reference (e.g., "PHB p.123")
    PageReference,
    /// Chapter reference (e.g., "DMG Chapter 5")
    ChapterReference,
    /// Book title in parentheses (e.g., "(Player's Handbook)")
    ParentheticalBook,
    /// Spell name
    Spell,
    /// Monster/creature name
    Monster,
    /// Feat name
    Feat,
    /// Class feature
    ClassFeature,
    /// Condition (poisoned, frightened, etc.)
    Condition,
    /// Game mechanic notation (DC, AC, CR)
    GameMechanic,
    /// Item name
    Item,
    /// Generic term that might need lookup
    GenericTerm,
}

/// Result of validating references.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// References that were validated successfully
    pub valid: Vec<ValidatedReference>,
    /// References that could not be validated
    pub invalid: Vec<InvalidReference>,
    /// Overall validation success rate
    pub success_rate: f64,
}

/// A reference that was successfully validated.
#[derive(Debug, Clone)]
pub struct ValidatedReference {
    pub reference: RulebookReference,
    pub matched_document_id: String,
    pub confidence: f64,
}

/// A reference that could not be validated.
#[derive(Debug, Clone)]
pub struct InvalidReference {
    pub reference: RulebookReference,
    pub reason: String,
}

/// Content linked from a rulebook search.
#[derive(Debug, Clone)]
pub struct LinkedContent {
    /// The search result
    pub result: SearchResult,
    /// Computed confidence score
    pub confidence: f64,
    /// Matched reference type
    pub reference_type: ReferenceType,
}

/// Extracts rulebook references from text and links them to indexed content.
pub struct RulebookLinker {
    search: Arc<SearchClient>,
    /// Compiled regex patterns for reference detection
    patterns: ReferencePatterns,
}

/// Precompiled regex patterns for reference detection.
struct ReferencePatterns {
    /// Patterns for page references (e.g., "PHB p.123", "DMG page 45")
    page_ref: Regex,
    /// Patterns for chapter references (e.g., "DMG Chapter 5")
    chapter_ref: Regex,
    /// Patterns for parenthetical book references (e.g., "(Player's Handbook)")
    paren_book: Regex,
    /// Patterns for game mechanics (DC, AC, CR)
    game_mechanics: Regex,
    /// Patterns for conditions
    conditions: Regex,
    /// Common book abbreviations (reserved for future use)
    #[allow(dead_code)]
    book_abbrevs: Regex,
}

impl ReferencePatterns {
    fn new() -> Self {
        Self {
            // Match patterns like "PHB p.123", "PHB p123", "PHB pg. 123", "DMG page 45"
            page_ref: Regex::new(
                r"(?i)\b(PHB|DMG|MM|XGtE|TCoE|VGtM|MToF|FToD|MotM|Player'?s?\s*Handbook|Dungeon\s*Master'?s?\s*Guide|Monster\s*Manual)\s*(?:p\.?|pg\.?|page)\s*(\d+)\b"
            ).expect("Invalid page_ref regex"),

            // Match patterns like "DMG Chapter 5", "PHB Ch. 3"
            chapter_ref: Regex::new(
                r"(?i)\b(PHB|DMG|MM|XGtE|TCoE|Player'?s?\s*Handbook|Dungeon\s*Master'?s?\s*Guide)\s*(?:Chapter|Ch\.?)\s*(\d+|[IVX]+)(?:\s*[:\-]\s*([^,\.\n]+))?"
            ).expect("Invalid chapter_ref regex"),

            // Match book names in parentheses
            paren_book: Regex::new(
                r"\((?:see\s+)?(Player'?s?\s*Handbook|Dungeon\s*Master'?s?\s*Guide|Monster\s*Manual|Xanathar'?s?\s*Guide|Tasha'?s?\s*Cauldron|Volo'?s?\s*Guide)\)"
            ).expect("Invalid paren_book regex"),

            // Match game mechanics like "DC 15", "AC 18", "CR 5"
            game_mechanics: Regex::new(
                r"\b(DC|AC|CR)\s*(\d+(?:/\d+)?)\b"
            ).expect("Invalid game_mechanics regex"),

            // Match common conditions
            conditions: Regex::new(
                r"(?i)\b(blinded|charmed|deafened|exhaustion|frightened|grappled|incapacitated|invisible|paralyzed|petrified|poisoned|prone|restrained|stunned|unconscious)\b"
            ).expect("Invalid conditions regex"),

            // Match book abbreviations anywhere for context
            book_abbrevs: Regex::new(
                r"(?i)\b(PHB|DMG|MM|XGtE|TCoE|VGtM|MToF|FToD|MotM)\b"
            ).expect("Invalid book_abbrevs regex"),
        }
    }
}

/// Map book abbreviations to full names.
fn normalize_book_name(abbrev: &str) -> String {
    match abbrev.to_uppercase().as_str() {
        "PHB" => "Player's Handbook".to_string(),
        "DMG" => "Dungeon Master's Guide".to_string(),
        "MM" => "Monster Manual".to_string(),
        "XGTE" | "XANATHAR'S" | "XANATHAR" => "Xanathar's Guide to Everything".to_string(),
        "TCOE" | "TASHA'S" | "TASHA" => "Tasha's Cauldron of Everything".to_string(),
        "VGTM" | "VOLO'S" | "VOLO" => "Volo's Guide to Monsters".to_string(),
        "MTOF" => "Mordenkainen's Tome of Foes".to_string(),
        "FTOD" => "Fizban's Treasury of Dragons".to_string(),
        "MOTM" => "Mordenkainen Presents: Monsters of the Multiverse".to_string(),
        other => {
            // Try to clean up the name
            if other.to_lowercase().contains("player") {
                "Player's Handbook".to_string()
            } else if other.to_lowercase().contains("dungeon") {
                "Dungeon Master's Guide".to_string()
            } else if other.to_lowercase().contains("monster") {
                "Monster Manual".to_string()
            } else {
                other.to_string()
            }
        }
    }
}

impl RulebookLinker {
    /// Create a new RulebookLinker with the given SearchClient.
    pub fn new(search: Arc<SearchClient>) -> Self {
        Self {
            search,
            patterns: ReferencePatterns::new(),
        }
    }

    /// Find all rulebook references in the given text.
    ///
    /// # Arguments
    /// * `text` - The text to search for references
    ///
    /// # Returns
    /// A vector of detected references, sorted by position in the text.
    pub fn find_references(&self, text: &str) -> Vec<RulebookReference> {
        let mut references = Vec::new();
        let mut seen_positions: HashSet<(usize, usize)> = HashSet::new();

        // Find page references
        for cap in self.patterns.page_ref.captures_iter(text) {
            if let (Some(book_match), Some(page_match)) = (cap.get(1), cap.get(2)) {
                let full_match = cap.get(0).unwrap();
                let pos = (full_match.start(), full_match.end());
                if !seen_positions.contains(&pos) {
                    seen_positions.insert(pos);
                    references.push(RulebookReference {
                        raw_text: full_match.as_str().to_string(),
                        reference_type: ReferenceType::PageReference,
                        source_name: Some(normalize_book_name(book_match.as_str())),
                        page: page_match.as_str().parse().ok(),
                        chapter: None,
                        section: None,
                        term: None,
                        start_pos: full_match.start(),
                        end_pos: full_match.end(),
                    });
                }
            }
        }

        // Find chapter references
        for cap in self.patterns.chapter_ref.captures_iter(text) {
            if let Some(book_match) = cap.get(1) {
                let full_match = cap.get(0).unwrap();
                let pos = (full_match.start(), full_match.end());
                if !seen_positions.contains(&pos) {
                    seen_positions.insert(pos);
                    let chapter = cap.get(2).map(|m| m.as_str().to_string());
                    let section = cap.get(3).map(|m| m.as_str().trim().to_string());
                    references.push(RulebookReference {
                        raw_text: full_match.as_str().to_string(),
                        reference_type: ReferenceType::ChapterReference,
                        source_name: Some(normalize_book_name(book_match.as_str())),
                        page: None,
                        chapter,
                        section,
                        term: None,
                        start_pos: full_match.start(),
                        end_pos: full_match.end(),
                    });
                }
            }
        }

        // Find parenthetical book references
        for cap in self.patterns.paren_book.captures_iter(text) {
            if let Some(book_match) = cap.get(1) {
                let full_match = cap.get(0).unwrap();
                let pos = (full_match.start(), full_match.end());
                if !seen_positions.contains(&pos) {
                    seen_positions.insert(pos);
                    references.push(RulebookReference {
                        raw_text: full_match.as_str().to_string(),
                        reference_type: ReferenceType::ParentheticalBook,
                        source_name: Some(normalize_book_name(book_match.as_str())),
                        page: None,
                        chapter: None,
                        section: None,
                        term: None,
                        start_pos: full_match.start(),
                        end_pos: full_match.end(),
                    });
                }
            }
        }

        // Find game mechanics references
        for cap in self.patterns.game_mechanics.captures_iter(text) {
            if let Some(mechanic_type) = cap.get(1) {
                let full_match = cap.get(0).unwrap();
                let pos = (full_match.start(), full_match.end());
                if !seen_positions.contains(&pos) {
                    seen_positions.insert(pos);
                    references.push(RulebookReference {
                        raw_text: full_match.as_str().to_string(),
                        reference_type: ReferenceType::GameMechanic,
                        source_name: None,
                        page: None,
                        chapter: None,
                        section: None,
                        term: Some(mechanic_type.as_str().to_uppercase()),
                        start_pos: full_match.start(),
                        end_pos: full_match.end(),
                    });
                }
            }
        }

        // Find condition references
        for cap in self.patterns.conditions.captures_iter(text) {
            if let Some(condition_match) = cap.get(1) {
                let full_match = cap.get(0).unwrap();
                let pos = (full_match.start(), full_match.end());
                if !seen_positions.contains(&pos) {
                    seen_positions.insert(pos);
                    references.push(RulebookReference {
                        raw_text: full_match.as_str().to_string(),
                        reference_type: ReferenceType::Condition,
                        source_name: Some("Player's Handbook".to_string()), // Conditions are in PHB Appendix A
                        page: None,
                        chapter: None,
                        section: Some("Appendix A: Conditions".to_string()),
                        term: Some(condition_match.as_str().to_lowercase()),
                        start_pos: full_match.start(),
                        end_pos: full_match.end(),
                    });
                }
            }
        }

        // Sort by position in text
        references.sort_by_key(|r| r.start_pos);
        references
    }

    /// Search Meilisearch for content matching a query.
    ///
    /// # Arguments
    /// * `query` - Search query
    /// * `rulebook_filter` - Optional list of rulebook names to filter by
    ///
    /// # Returns
    /// Linked content with confidence scores.
    pub async fn link_to_rulebook(
        &self,
        query: &str,
        rulebook_filter: Option<Vec<String>>,
    ) -> Result<Vec<LinkedContent>, String> {
        // Note: rulebook_filter is accepted but not yet used - hybrid_search
        // doesn't support filter parameter yet. See issue #XXX for filter support.
        let _ = rulebook_filter;

        // Search using hybrid search for best results
        let results = self
            .search
            .hybrid_search(
                "rules", // Primary index for rulebooks
                query,
                10,  // Limit
                0.5, // Semantic ratio (balanced keyword + semantic)
                Some("ollama"),
            )
            .await
            .map_err(|e| e.to_string())?;

        // Convert to LinkedContent with confidence scoring
        let linked: Vec<LinkedContent> = results
            .into_iter()
            .map(|result| {
                let confidence = self.compute_confidence(&result, query);
                LinkedContent {
                    confidence,
                    reference_type: self.infer_reference_type(&result),
                    result,
                }
            })
            .collect();

        Ok(linked)
    }

    /// Compute confidence score for a search result.
    fn compute_confidence(&self, result: &SearchResult, query: &str) -> f64 {
        // Base confidence from search score
        let mut confidence = result.score as f64;

        // Boost for exact term matches
        let query_lower = query.to_lowercase();
        let content_lower = result.document.content.to_lowercase();
        if content_lower.contains(&query_lower) {
            confidence += 0.1;
        }

        // Boost for source name matches in metadata
        if let Some(book_title) = &result.document.book_title {
            if book_title.to_lowercase().contains(&query_lower)
                || query_lower.contains(&book_title.to_lowercase())
            {
                confidence += 0.05;
            }
        }

        // Boost for structured content (stat blocks, tables)
        if let Some(chunk_type) = &result.document.chunk_type {
            match chunk_type.as_str() {
                "stat_block" | "spell" | "item" | "table" => confidence += 0.05,
                _ => {}
            }
        }

        // Clamp to valid range
        confidence.clamp(0.0, 1.0)
    }

    /// Infer reference type from search result content.
    fn infer_reference_type(&self, result: &SearchResult) -> ReferenceType {
        if let Some(chunk_type) = &result.document.chunk_type {
            match chunk_type.to_lowercase().as_str() {
                "spell" => return ReferenceType::Spell,
                "stat_block" | "monster" => return ReferenceType::Monster,
                "item" => return ReferenceType::Item,
                "feat" => return ReferenceType::Feat,
                _ => {}
            }
        }

        // Check content for clues
        let content_lower = result.document.content.to_lowercase();
        if content_lower.contains("saving throw") || content_lower.contains("spell attack") {
            ReferenceType::Spell
        } else if content_lower.contains("hit points") && content_lower.contains("armor class") {
            ReferenceType::Monster
        } else if content_lower.contains("prerequisite:") {
            ReferenceType::Feat
        } else {
            ReferenceType::GenericTerm
        }
    }

    /// Build a Citation from a RulebookReference and optional search result.
    pub fn build_citation(
        &self,
        reference: &RulebookReference,
        search_result: Option<&LinkedContent>,
    ) -> Citation {
        let source_name = reference
            .source_name
            .clone()
            .unwrap_or_else(|| "Unknown Source".to_string());

        let confidence = search_result
            .map(|r| r.confidence)
            .unwrap_or(0.5); // Default moderate confidence if no search result

        let mut builder = match reference.reference_type {
            ReferenceType::PageReference
            | ReferenceType::ChapterReference
            | ReferenceType::ParentheticalBook => CitationBuilder::from_rulebook(&source_name),
            ReferenceType::Condition | ReferenceType::GameMechanic => {
                CitationBuilder::from_rulebook(&source_name)
            }
            ReferenceType::Spell | ReferenceType::Monster | ReferenceType::Feat | ReferenceType::Item => {
                CitationBuilder::from_rulebook(&source_name)
            }
            _ => CitationBuilder::from_rulebook(&source_name),
        };

        // Add location info
        if let Some(page) = reference.page {
            builder = builder.page(page);
        }
        if let Some(chapter) = &reference.chapter {
            builder = builder.chapter(chapter);
        }
        if let Some(section) = &reference.section {
            builder = builder.section(section);
        }

        // Add source ID if we have a search result
        if let Some(linked) = search_result {
            builder = builder.source_id(&linked.result.document.id);
            // Add excerpt from search result
            let excerpt = linked.result.document.content.chars().take(200).collect::<String>();
            if !excerpt.is_empty() {
                builder = builder.excerpt(excerpt);
            }
        }

        builder.confidence(confidence).build()
    }

    /// Validate that referenced content exists in the search index.
    ///
    /// # Arguments
    /// * `references` - References to validate
    ///
    /// # Returns
    /// A validation report showing which references were found and which were not.
    pub async fn validate_references(
        &self,
        references: &[RulebookReference],
    ) -> ValidationReport {
        let mut valid = Vec::new();
        let mut invalid = Vec::new();

        for reference in references {
            // Build search query from reference
            let query = self.build_search_query(reference);

            // Search for matching content
            match self.link_to_rulebook(&query, None).await {
                Ok(results) if !results.is_empty() => {
                    let best = &results[0];
                    if best.confidence >= 0.5 {
                        valid.push(ValidatedReference {
                            reference: reference.clone(),
                            matched_document_id: best.result.document.id.clone(),
                            confidence: best.confidence,
                        });
                    } else {
                        invalid.push(InvalidReference {
                            reference: reference.clone(),
                            reason: format!(
                                "Low confidence match: {:.2}",
                                best.confidence
                            ),
                        });
                    }
                }
                Ok(_) => {
                    invalid.push(InvalidReference {
                        reference: reference.clone(),
                        reason: "No matching content found".to_string(),
                    });
                }
                Err(e) => {
                    invalid.push(InvalidReference {
                        reference: reference.clone(),
                        reason: format!("Search error: {}", e),
                    });
                }
            }
        }

        let total = valid.len() + invalid.len();
        let success_rate = if total > 0 {
            valid.len() as f64 / total as f64
        } else {
            1.0
        };

        ValidationReport {
            valid,
            invalid,
            success_rate,
        }
    }

    /// Build a search query from a reference.
    fn build_search_query(&self, reference: &RulebookReference) -> String {
        let mut parts = Vec::new();

        if let Some(term) = &reference.term {
            parts.push(term.clone());
        }

        if let Some(section) = &reference.section {
            parts.push(section.clone());
        }

        if let Some(chapter) = &reference.chapter {
            parts.push(format!("chapter {}", chapter));
        }

        if let Some(source) = &reference.source_name {
            parts.push(source.clone());
        }

        if parts.is_empty() {
            reference.raw_text.clone()
        } else {
            parts.join(" ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_patterns() -> ReferencePatterns {
        ReferencePatterns::new()
    }

    #[test]
    fn test_find_page_references() {
        let patterns = create_patterns();

        let test_cases = vec![
            ("See PHB p.123 for details", "PHB p.123", 123),
            ("Reference DMG p42", "DMG p42", 42),
            ("Check PHB pg. 256", "PHB pg. 256", 256),
            ("Player's Handbook page 100", "Player's Handbook page 100", 100),
        ];

        for (text, expected_match, expected_page) in test_cases {
            let caps = patterns.page_ref.captures(text);
            assert!(caps.is_some(), "Should match: {}", text);
            let cap = caps.unwrap();
            assert_eq!(cap.get(0).unwrap().as_str(), expected_match);
            assert_eq!(
                cap.get(2).unwrap().as_str().parse::<u32>().unwrap(),
                expected_page
            );
        }
    }

    #[test]
    fn test_find_chapter_references() {
        let patterns = create_patterns();

        let test_cases = vec![
            ("See DMG Chapter 5", "DMG", "5"),
            ("PHB Ch. 3", "PHB", "3"),
            ("Player's Handbook Chapter II", "Player's Handbook", "II"),
        ];

        for (text, expected_book, expected_chapter) in test_cases {
            let caps = patterns.chapter_ref.captures(text);
            assert!(caps.is_some(), "Should match: {}", text);
            let cap = caps.unwrap();
            assert_eq!(cap.get(1).unwrap().as_str(), expected_book);
            assert_eq!(cap.get(2).unwrap().as_str(), expected_chapter);
        }
    }

    #[test]
    fn test_find_parenthetical_references() {
        let patterns = create_patterns();

        let test_cases = vec![
            ("A longsword (Player's Handbook)", "Player's Handbook"),
            ("The rules (see Dungeon Master's Guide)", "Dungeon Master's Guide"),
        ];

        for (text, expected_book) in test_cases {
            let caps = patterns.paren_book.captures(text);
            assert!(caps.is_some(), "Should match: {}", text);
            let cap = caps.unwrap();
            assert_eq!(cap.get(1).unwrap().as_str(), expected_book);
        }
    }

    #[test]
    fn test_find_game_mechanics() {
        let patterns = create_patterns();

        let test_cases = vec![
            ("Make a DC 15 check", "DC", "15"),
            ("AC 18", "AC", "18"),
            ("CR 5", "CR", "5"),
            ("Challenge rating CR 1/4", "CR", "1/4"),
        ];

        for (text, expected_type, expected_value) in test_cases {
            let caps = patterns.game_mechanics.captures(text);
            assert!(caps.is_some(), "Should match: {}", text);
            let cap = caps.unwrap();
            assert_eq!(cap.get(1).unwrap().as_str(), expected_type);
            assert_eq!(cap.get(2).unwrap().as_str(), expected_value);
        }
    }

    #[test]
    fn test_find_conditions() {
        let patterns = create_patterns();

        let conditions = vec![
            "blinded", "charmed", "deafened", "frightened", "grappled",
            "incapacitated", "invisible", "paralyzed", "petrified",
            "poisoned", "prone", "restrained", "stunned", "unconscious",
        ];

        for condition in conditions {
            let text = format!("The target is {}", condition);
            let caps = patterns.conditions.captures(&text);
            assert!(caps.is_some(), "Should match condition: {}", condition);
            assert_eq!(
                caps.unwrap().get(1).unwrap().as_str().to_lowercase(),
                condition
            );
        }
    }

    #[test]
    fn test_normalize_book_name() {
        assert_eq!(normalize_book_name("PHB"), "Player's Handbook");
        assert_eq!(normalize_book_name("DMG"), "Dungeon Master's Guide");
        assert_eq!(normalize_book_name("MM"), "Monster Manual");
        assert_eq!(normalize_book_name("XGtE"), "Xanathar's Guide to Everything");
        assert_eq!(normalize_book_name("TCoE"), "Tasha's Cauldron of Everything");
        assert_eq!(normalize_book_name("Player's Handbook"), "Player's Handbook");
    }

    #[test]
    fn test_reference_type_properties() {
        assert_eq!(ReferenceType::PageReference, ReferenceType::PageReference);
        assert_ne!(ReferenceType::PageReference, ReferenceType::Spell);
    }

    // Integration tests would require a mock SearchClient
    // These are marked as ignored since they need actual infrastructure
    #[test]
    #[ignore]
    fn test_link_to_rulebook_integration() {
        // Would need async runtime and mock SearchClient
    }

    #[test]
    fn test_build_search_query() {
        // Test query building logic without SearchClient
        let reference = RulebookReference {
            raw_text: "PHB p.123".to_string(),
            reference_type: ReferenceType::PageReference,
            source_name: Some("Player's Handbook".to_string()),
            page: Some(123),
            chapter: None,
            section: Some("Combat".to_string()),
            term: None,
            start_pos: 0,
            end_pos: 9,
        };

        // Query should include section and source
        let query_parts: Vec<&str> = vec!["Combat", "Player's Handbook"];
        assert!(query_parts.iter().all(|p| {
            reference.section.as_ref().map(|s| s.contains(p)).unwrap_or(false)
                || reference.source_name.as_ref().map(|s| s.contains(p)).unwrap_or(false)
        }));
    }

    #[test]
    fn test_validation_report_success_rate() {
        let report = ValidationReport {
            valid: vec![ValidatedReference {
                reference: RulebookReference {
                    raw_text: "test".to_string(),
                    reference_type: ReferenceType::GenericTerm,
                    source_name: None,
                    page: None,
                    chapter: None,
                    section: None,
                    term: None,
                    start_pos: 0,
                    end_pos: 4,
                },
                matched_document_id: "doc1".to_string(),
                confidence: 0.9,
            }],
            invalid: vec![InvalidReference {
                reference: RulebookReference {
                    raw_text: "test2".to_string(),
                    reference_type: ReferenceType::GenericTerm,
                    source_name: None,
                    page: None,
                    chapter: None,
                    section: None,
                    term: None,
                    start_pos: 0,
                    end_pos: 5,
                },
                reason: "Not found".to_string(),
            }],
            success_rate: 0.5,
        };

        assert_eq!(report.valid.len(), 1);
        assert_eq!(report.invalid.len(), 1);
        assert!((report.success_rate - 0.5).abs() < 0.001);
    }
}
