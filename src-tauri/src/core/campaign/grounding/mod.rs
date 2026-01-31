//! Content Grounding Layer
//!
//! Phase 3 of the Campaign Generation Overhaul.
//!
//! This module provides citation management, rulebook linking, content tracking,
//! and flavour/lore search capabilities to ground generated content in indexed
//! source material.
//!
//! ## Components
//!
//! - [`CitationBuilder`] - Fluent API for constructing citations
//! - [`RulebookLinker`] - Reference detection and linking to indexed rulebooks
//! - [`UsageTracker`] - Citation usage tracking with deduplication
//! - [`FlavourSearcher`] - Lore and setting content retrieval
//!
//! ## Architecture
//!
//! The grounding layer sits between the generation pipeline and the search indexes:
//!
//! ```text
//! ┌─────────────────┐     ┌──────────────────┐     ┌────────────────┐
//! │ Generation      │────▶│ Grounding Layer  │────▶│ Meilisearch    │
//! │ Pipeline        │◀────│ (citations,      │◀────│ (rules, lore)  │
//! └─────────────────┘     │  validation)     │     └────────────────┘
//!                         └──────────────────┘
//!                                 │
//!                                 ▼
//!                         ┌──────────────────┐
//!                         │ SQLite           │
//!                         │ (usage tracking) │
//!                         └──────────────────┘
//! ```
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use crate::core::campaign::grounding::{
//!     CitationBuilder, RulebookLinker, UsageTracker, FlavourSearcher
//! };
//!
//! // Build a citation manually
//! let citation = CitationBuilder::from_rulebook("Player's Handbook")
//!     .page(123)
//!     .section("Combat")
//!     .confidence(0.95)
//!     .build();
//!
//! // Detect and validate references in text
//! let linker = RulebookLinker::new(search_client.clone());
//! let refs = linker.find_references("See PHB p.123 for details");
//! let validated = linker.validate_references(&refs).await?;
//!
//! // Track citation usage
//! let tracker = UsageTracker::new(db.clone());
//! tracker.mark_content_used("campaign-123", &citation).await?;
//!
//! // Search for setting lore
//! let searcher = FlavourSearcher::new(search_client.clone());
//! let lore = searcher.search_setting_lore(
//!     "history of Waterdeep",
//!     Some(FlavourFilters::for_setting("Forgotten Realms")),
//!     10
//! ).await?;
//! ```

mod citation_builder;
mod flavour_searcher;
mod rulebook_linker;
mod usage_tracker;

pub use citation_builder::CitationBuilder;
pub use flavour_searcher::{
    FlavourFilters, FlavourResult, FlavourSearchError, FlavourSearcher,
    LocationResult, LocationType, LoreCategory, LoreResult, NameResult, NameType,
};
pub use rulebook_linker::{
    InvalidReference, LinkedContent, ReferenceType, RulebookLinker,
    RulebookReference, ValidatedReference, ValidationReport,
};
pub use usage_tracker::{UsageOptions, UsageResult, UsageSummary, UsageTracker, UsageTrackerError};

/// Grounder trait for content grounding implementations.
///
/// This trait defines the interface for grounding generated content with
/// source citations. Implementations can use different strategies for
/// finding and validating references.
///
/// See Design.md for the full specification.
#[async_trait::async_trait]
pub trait Grounder: Send + Sync {
    /// Ground the given text by finding and linking references to source material.
    ///
    /// # Arguments
    /// * `text` - The text to ground
    /// * `campaign_id` - Optional campaign context for filtering
    ///
    /// # Returns
    /// A result containing grounded content with citations.
    async fn ground(&self, text: &str, campaign_id: Option<&str>) -> Result<GroundedContent, String>;

    /// Validate that all references in the text can be resolved.
    ///
    /// # Arguments
    /// * `text` - The text containing references to validate
    ///
    /// # Returns
    /// A validation report showing which references were found and which were not.
    async fn validate(&self, text: &str) -> Result<ValidationReport, String>;
}

/// Content that has been grounded with citations.
#[derive(Debug, Clone)]
pub struct GroundedContent {
    /// Original text
    pub original_text: String,
    /// Text with reference markers (e.g., "[1]", "[2]")
    pub marked_text: String,
    /// Citations found in the text
    pub citations: Vec<crate::database::Citation>,
    /// Overall confidence in the grounding (0.0 to 1.0)
    pub confidence: f64,
    /// References that could not be grounded
    pub ungrounded_references: Vec<RulebookReference>,
}

/// Combined grounder implementation using RulebookLinker and FlavourSearcher.
pub struct CombinedGrounder {
    linker: RulebookLinker,
    #[allow(dead_code)]
    flavour: FlavourSearcher,
}

impl CombinedGrounder {
    /// Create a new CombinedGrounder.
    pub fn new(linker: RulebookLinker, flavour: FlavourSearcher) -> Self {
        Self { linker, flavour }
    }
}

#[async_trait::async_trait]
impl Grounder for CombinedGrounder {
    async fn ground(&self, text: &str, _campaign_id: Option<&str>) -> Result<GroundedContent, String> {
        // Find references in the text
        let references = self.linker.find_references(text);

        let mut citations = Vec::new();
        let mut ungrounded = Vec::new();
        let mut marked_text = text.to_string();
        let mut citation_num = 1;

        // Process references in reverse order to preserve positions
        for reference in references.iter().rev() {
            // Try to link the reference
            let query = if let Some(term) = &reference.term {
                term.clone()
            } else if let Some(section) = &reference.section {
                section.clone()
            } else {
                reference.raw_text.clone()
            };

            match self.linker.link_to_rulebook(&query, None).await {
                Ok(linked) if !linked.is_empty() => {
                    let best = &linked[0];
                    if best.confidence >= 0.5 {
                        let citation = self.linker.build_citation(reference, Some(best));
                        citations.push(citation);

                        // Add marker after the reference
                        marked_text.insert_str(
                            reference.end_pos.min(marked_text.len()),
                            &format!("[{}]", citation_num),
                        );
                        citation_num += 1;
                    } else {
                        ungrounded.push(reference.clone());
                    }
                }
                _ => {
                    ungrounded.push(reference.clone());
                }
            }
        }

        // Reverse citations to match text order
        citations.reverse();

        // Calculate overall confidence
        let confidence = if citations.is_empty() {
            0.0
        } else {
            citations.iter().map(|c| c.confidence).sum::<f64>() / citations.len() as f64
        };

        Ok(GroundedContent {
            original_text: text.to_string(),
            marked_text,
            citations,
            confidence,
            ungrounded_references: ungrounded,
        })
    }

    async fn validate(&self, text: &str) -> Result<ValidationReport, String> {
        let references = self.linker.find_references(text);
        Ok(self.linker.validate_references(&references).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grounded_content_structure() {
        let content = GroundedContent {
            original_text: "Test text".to_string(),
            marked_text: "Test text[1]".to_string(),
            citations: vec![],
            confidence: 0.8,
            ungrounded_references: vec![],
        };

        assert_eq!(content.original_text, "Test text");
        assert_eq!(content.confidence, 0.8);
    }

    #[test]
    fn test_module_exports() {
        // Verify all public types are accessible
        let _: CitationBuilder = CitationBuilder::new();
        let _: FlavourFilters = FlavourFilters::default();
        let _: UsageOptions = UsageOptions::default();
    }
}
