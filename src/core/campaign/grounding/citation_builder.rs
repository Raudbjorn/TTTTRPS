//! Citation Builder - Fluent API for constructing citations
//!
//! Part of Phase 3: Content Grounding Layer (Task 3.1)
//!
//! Provides a builder pattern for creating Citation objects with various
//! source types, locations, and confidence levels.

use crate::database::{Citation, SourceCitationRecord, SourceLocation, SourceType};

/// Builder for constructing Citation objects with a fluent API.
///
/// # Example
///
/// ```rust
/// use crate::core::campaign::grounding::CitationBuilder;
///
/// let citation = CitationBuilder::from_rulebook("Player's Handbook")
///     .page(42)
///     .section("Combat")
///     .excerpt("When you take the Attack action...")
///     .confidence(0.95)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct CitationBuilder {
    source_type: SourceType,
    source_name: String,
    source_id: Option<String>,
    page: Option<u32>,
    section: Option<String>,
    chapter: Option<String>,
    paragraph: Option<u32>,
    excerpt: Option<String>,
    confidence: f64,
}

impl Default for CitationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CitationBuilder {
    /// Create a new builder with default values.
    pub fn new() -> Self {
        Self {
            source_type: SourceType::Rulebook,
            source_name: String::new(),
            source_id: None,
            page: None,
            section: None,
            chapter: None,
            paragraph: None,
            excerpt: None,
            confidence: 0.0,
        }
    }

    /// Create a builder for a rulebook citation.
    ///
    /// # Arguments
    /// * `source_name` - Name of the rulebook (e.g., "Player's Handbook", "DMG")
    pub fn from_rulebook(source_name: impl Into<String>) -> Self {
        Self {
            source_type: SourceType::Rulebook,
            source_name: source_name.into(),
            confidence: 0.9, // Rulebooks have high base confidence
            ..Default::default()
        }
    }

    /// Create a builder for a flavour source (setting book, lore document).
    ///
    /// # Arguments
    /// * `source_name` - Name of the flavour source
    pub fn from_flavour_source(source_name: impl Into<String>) -> Self {
        Self {
            source_type: SourceType::FlavourSource,
            source_name: source_name.into(),
            confidence: 0.8, // Flavour sources have good base confidence
            ..Default::default()
        }
    }

    /// Create a builder for an adventure module citation.
    ///
    /// # Arguments
    /// * `source_name` - Name of the adventure module
    pub fn from_adventure(source_name: impl Into<String>) -> Self {
        Self {
            source_type: SourceType::Adventure,
            source_name: source_name.into(),
            confidence: 0.85,
            ..Default::default()
        }
    }

    /// Create a builder for homebrew content citation.
    ///
    /// # Arguments
    /// * `source_name` - Name of the homebrew source
    pub fn from_homebrew(source_name: impl Into<String>) -> Self {
        Self {
            source_type: SourceType::Homebrew,
            source_name: source_name.into(),
            confidence: 0.7, // Homebrew has lower base confidence
            ..Default::default()
        }
    }

    /// Create a builder for a campaign entity citation.
    ///
    /// # Arguments
    /// * `source_name` - Name or description of the campaign entity
    pub fn from_campaign_entity(source_name: impl Into<String>) -> Self {
        Self {
            source_type: SourceType::CampaignEntity,
            source_name: source_name.into(),
            confidence: 1.0, // Campaign entities are canonical within campaign
            ..Default::default()
        }
    }

    /// Create a builder for user input citation.
    ///
    /// # Arguments
    /// * `source_name` - Description of the user input
    pub fn from_user_input(source_name: impl Into<String>) -> Self {
        Self {
            source_type: SourceType::UserInput,
            source_name: source_name.into(),
            confidence: 1.0, // User input is authoritative
            ..Default::default()
        }
    }

    /// Set the source ID (e.g., Meilisearch document ID).
    pub fn source_id(mut self, id: impl Into<String>) -> Self {
        self.source_id = Some(id.into());
        self
    }

    /// Set the page number.
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Set the section name.
    pub fn section(mut self, section: impl Into<String>) -> Self {
        self.section = Some(section.into());
        self
    }

    /// Set the chapter name.
    pub fn chapter(mut self, chapter: impl Into<String>) -> Self {
        self.chapter = Some(chapter.into());
        self
    }

    /// Set the paragraph number within the section.
    pub fn paragraph(mut self, paragraph: u32) -> Self {
        self.paragraph = Some(paragraph);
        self
    }

    /// Set an excerpt from the source material.
    ///
    /// The excerpt should be a relevant quote or summary that supports the citation.
    pub fn excerpt(mut self, excerpt: impl Into<String>) -> Self {
        self.excerpt = Some(excerpt.into());
        self
    }

    /// Alias for `excerpt` - more intuitive for some contexts.
    pub fn with_excerpt(self, excerpt: impl Into<String>) -> Self {
        self.excerpt(excerpt)
    }

    /// Set the confidence score (0.0 to 1.0).
    ///
    /// Higher confidence indicates stronger match to source material.
    /// - 0.95+: Canonical (direct quote or exact match)
    /// - 0.75-0.94: Derived (strong inference from source)
    /// - 0.50-0.74: Unverified (possible match, needs review)
    /// - <0.50: Creative (low confidence, likely AI invention)
    pub fn confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Alias for `confidence` - more intuitive for some contexts.
    pub fn with_confidence(self, confidence: f64) -> Self {
        self.confidence(confidence)
    }

    /// Build the location from current page/section/chapter/paragraph settings.
    fn build_location(&self) -> Option<SourceLocation> {
        if self.page.is_some()
            || self.section.is_some()
            || self.chapter.is_some()
            || self.paragraph.is_some()
        {
            Some(SourceLocation {
                page: self.page,
                section: self.section.clone(),
                chapter: self.chapter.clone(),
                paragraph: self.paragraph,
            })
        } else {
            None
        }
    }

    /// Build the Citation object.
    pub fn build(self) -> Citation {
        // Build location first to avoid partial move issues
        let location = self.build_location();

        Citation {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: self.source_type,
            source_id: self.source_id,
            source_name: self.source_name,
            location,
            excerpt: self.excerpt,
            confidence: self.confidence,
        }
    }

    /// Build a SourceCitationRecord for database storage.
    pub fn build_record(self) -> SourceCitationRecord {
        self.build().to_record()
    }
}

/// Convenience functions for common citation patterns.
impl CitationBuilder {
    /// Create a citation from a search result.
    ///
    /// # Arguments
    /// * `source_name` - Document name from search result
    /// * `source_id` - Document ID from search index
    /// * `score` - Search relevance score (will be converted to confidence)
    pub fn from_search_result(
        source_name: impl Into<String>,
        source_id: impl Into<String>,
        score: f32,
    ) -> Self {
        // Convert search score to confidence (search scores are typically 0-1)
        let confidence = (score as f64).clamp(0.0, 1.0);

        Self {
            source_type: SourceType::Rulebook, // Default, can be changed
            source_name: source_name.into(),
            source_id: Some(source_id.into()),
            confidence,
            ..Default::default()
        }
    }

    /// Create a citation with page reference (common pattern).
    ///
    /// # Arguments
    /// * `source_name` - Name of the source
    /// * `page` - Page number
    pub fn rulebook_page(source_name: impl Into<String>, page: u32) -> Self {
        Self::from_rulebook(source_name).page(page)
    }

    /// Create a citation with chapter reference.
    ///
    /// # Arguments
    /// * `source_name` - Name of the source
    /// * `chapter` - Chapter name or number
    pub fn rulebook_chapter(source_name: impl Into<String>, chapter: impl Into<String>) -> Self {
        Self::from_rulebook(source_name).chapter(chapter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_rulebook() {
        let citation = CitationBuilder::from_rulebook("Player's Handbook")
            .page(42)
            .section("Combat")
            .excerpt("When you take the Attack action...")
            .confidence(0.95)
            .build();

        assert_eq!(citation.source_type, SourceType::Rulebook);
        assert_eq!(citation.source_name, "Player's Handbook");
        assert_eq!(citation.confidence, 0.95);

        let location = citation.location.expect("Should have location");
        assert_eq!(location.page, Some(42));
        assert_eq!(location.section, Some("Combat".to_string()));
        assert_eq!(citation.excerpt, Some("When you take the Attack action...".to_string()));
    }

    #[test]
    fn test_from_flavour_source() {
        let citation = CitationBuilder::from_flavour_source("Sword Coast Adventurer's Guide")
            .chapter("Chapter 2: The Sword Coast")
            .excerpt("The Sword Coast stretches from...")
            .build();

        assert_eq!(citation.source_type, SourceType::FlavourSource);
        assert_eq!(citation.source_name, "Sword Coast Adventurer's Guide");
        assert_eq!(citation.confidence, 0.8); // Default flavour source confidence

        let location = citation.location.expect("Should have location");
        assert_eq!(location.chapter, Some("Chapter 2: The Sword Coast".to_string()));
    }

    #[test]
    fn test_from_adventure() {
        let citation = CitationBuilder::from_adventure("Lost Mine of Phandelver")
            .page(15)
            .section("Wave Echo Cave")
            .build();

        assert_eq!(citation.source_type, SourceType::Adventure);
        assert_eq!(citation.confidence, 0.85);
    }

    #[test]
    fn test_from_homebrew() {
        let citation = CitationBuilder::from_homebrew("Custom Monster Manual")
            .confidence(0.6)
            .build();

        assert_eq!(citation.source_type, SourceType::Homebrew);
        assert_eq!(citation.confidence, 0.6);
    }

    #[test]
    fn test_from_campaign_entity() {
        let citation = CitationBuilder::from_campaign_entity("NPC: Gundren Rockseeker")
            .build();

        assert_eq!(citation.source_type, SourceType::CampaignEntity);
        assert_eq!(citation.confidence, 1.0);
    }

    #[test]
    fn test_from_user_input() {
        let citation = CitationBuilder::from_user_input("GM decision during session 3")
            .build();

        assert_eq!(citation.source_type, SourceType::UserInput);
        assert_eq!(citation.confidence, 1.0);
    }

    #[test]
    fn test_source_id() {
        let citation = CitationBuilder::from_rulebook("PHB")
            .source_id("meili-doc-12345")
            .build();

        assert_eq!(citation.source_id, Some("meili-doc-12345".to_string()));
    }

    #[test]
    fn test_confidence_clamping() {
        let citation_high = CitationBuilder::new()
            .confidence(1.5)
            .build();
        assert_eq!(citation_high.confidence, 1.0);

        let citation_low = CitationBuilder::new()
            .confidence(-0.5)
            .build();
        assert_eq!(citation_low.confidence, 0.0);
    }

    #[test]
    fn test_no_location_when_empty() {
        let citation = CitationBuilder::from_rulebook("Generic Source")
            .build();

        assert!(citation.location.is_none());
    }

    #[test]
    fn test_full_location() {
        let citation = CitationBuilder::from_rulebook("Complete Guide")
            .chapter("Chapter 5")
            .section("Advanced Rules")
            .page(123)
            .paragraph(3)
            .build();

        let location = citation.location.expect("Should have location");
        assert_eq!(location.chapter, Some("Chapter 5".to_string()));
        assert_eq!(location.section, Some("Advanced Rules".to_string()));
        assert_eq!(location.page, Some(123));
        assert_eq!(location.paragraph, Some(3));
    }

    #[test]
    fn test_from_search_result() {
        let citation = CitationBuilder::from_search_result(
            "Monster Manual",
            "meili-123",
            0.87,
        )
        .page(45)
        .build();

        assert_eq!(citation.source_name, "Monster Manual");
        assert_eq!(citation.source_id, Some("meili-123".to_string()));
        assert!((citation.confidence - 0.87).abs() < 0.001);
    }

    #[test]
    fn test_convenience_methods() {
        let citation1 = CitationBuilder::rulebook_page("PHB", 42).build();
        assert_eq!(citation1.location.as_ref().unwrap().page, Some(42));

        let citation2 = CitationBuilder::rulebook_chapter("DMG", "Chapter 3: Creating Adventures").build();
        assert_eq!(
            citation2.location.as_ref().unwrap().chapter,
            Some("Chapter 3: Creating Adventures".to_string())
        );
    }

    #[test]
    fn test_build_record() {
        let record = CitationBuilder::from_rulebook("PHB")
            .page(100)
            .excerpt("Test excerpt")
            .confidence(0.9)
            .build_record();

        assert_eq!(record.source_type, "rulebook");
        assert_eq!(record.source_name, "PHB");
        assert_eq!(record.confidence, 0.9);
        assert!(record.location.is_some());
        assert_eq!(record.excerpt, Some("Test excerpt".to_string()));
    }

    #[test]
    fn test_with_aliases() {
        let citation = CitationBuilder::from_rulebook("Test")
            .with_excerpt("An excerpt")
            .with_confidence(0.75)
            .build();

        assert_eq!(citation.excerpt, Some("An excerpt".to_string()));
        assert_eq!(citation.confidence, 0.75);
    }
}
