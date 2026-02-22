//! Cross-Reference Detection Module
//!
//! Detects and extracts cross-references within TTRPG documents, enabling
//! linking between different parts of rulebooks, adventures, and supplements.
//!
//! Cross-references are common in TTRPG content:
//! - Page references: "see page 47", "refer to p. 15"
//! - Chapter references: "see Chapter 3", "in Chapter VII"
//! - Section references: "see the 'Combat' section"
//! - Table references: "see Table 1-3", "roll on Table 5"
//! - Figure references: "see Figure 2", "as shown in Fig. 4"
//!
//! # Example
//!
//! ```ignore
//! use crate::ingestion::ttrpg::cross_reference::{CrossReferenceExtractor, ReferenceType};
//!
//! let extractor = CrossReferenceExtractor::new();
//! let refs = extractor.extract("For more details, see page 47 and refer to Chapter 3.");
//!
//! assert_eq!(refs.len(), 2);
//! assert!(matches!(refs[0].ref_type, ReferenceType::Page));
//! assert!(matches!(refs[1].ref_type, ReferenceType::Chapter));
//! ```

use regex::Regex;
use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// The type of cross-reference detected in the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReferenceType {
    /// Reference to a specific page number (e.g., "see page 47", "p. 15")
    Page,
    /// Reference to a chapter (e.g., "see Chapter 3", "Chapter VII")
    Chapter,
    /// Reference to a named section (e.g., "the 'Combat' section")
    Section,
    /// Reference to a table (e.g., "Table 1-3", "roll on Table 5")
    Table,
    /// Reference to a figure or illustration (e.g., "Figure 2", "Fig. 4")
    Figure,
}

impl ReferenceType {
    /// Get a human-readable name for this reference type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Chapter => "chapter",
            Self::Section => "section",
            Self::Table => "table",
            Self::Figure => "figure",
        }
    }
}

/// A detected cross-reference within document text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossReference {
    /// The type of reference (page, chapter, section, etc.)
    pub ref_type: ReferenceType,
    /// The target of the reference (e.g., "47" for page 47, "3" for Chapter 3)
    pub ref_target: String,
    /// The original matched text from the document
    pub ref_text: String,
    /// Confidence score (0.0 to 1.0) based on pattern specificity
    pub confidence: f32,
    /// Start position in the source text (character offset)
    pub start_offset: usize,
    /// End position in the source text (character offset)
    pub end_offset: usize,
}

impl CrossReference {
    /// Create a new cross-reference.
    pub fn new(
        ref_type: ReferenceType,
        ref_target: String,
        ref_text: String,
        confidence: f32,
        start_offset: usize,
        end_offset: usize,
    ) -> Self {
        Self {
            ref_type,
            ref_target,
            ref_text,
            confidence,
            start_offset,
            end_offset,
        }
    }
}

// ============================================================================
// Extractor
// ============================================================================

/// Compiled regex patterns for cross-reference detection.
struct ReferencePatterns {
    /// Patterns for page references with capture groups for the page number
    page_patterns: Vec<(Regex, f32)>,
    /// Patterns for chapter references with capture groups for the chapter number/name
    chapter_patterns: Vec<(Regex, f32)>,
    /// Patterns for section references with capture groups for the section name
    section_patterns: Vec<(Regex, f32)>,
    /// Patterns for table references with capture groups for the table identifier
    table_patterns: Vec<(Regex, f32)>,
    /// Patterns for figure references with capture groups for the figure number
    figure_patterns: Vec<(Regex, f32)>,
}

impl ReferencePatterns {
    fn new() -> Self {
        Self {
            page_patterns: vec![
                // "see page 47", "refer to page 12", "on page 23"
                (
                    Regex::new(r"(?i)\b(?:see|refer(?:\s+to)?|on|at|from)\s+page\s+(\d+)").unwrap(),
                    0.95,
                ),
                // "page 47" standalone
                (Regex::new(r"(?i)\bpage\s+(\d+)\b").unwrap(), 0.85),
                // "p. 15", "p.42", "pp. 15-20"
                (Regex::new(r"(?i)\bpp?\.\s*(\d+(?:\s*[-–]\s*\d+)?)\b").unwrap(), 0.90),
                // "pages 15-20", "pages 47 and 48"
                (
                    Regex::new(r"(?i)\bpages\s+(\d+(?:\s*[-–]\s*\d+|\s+and\s+\d+)?)\b").unwrap(),
                    0.90,
                ),
            ],
            chapter_patterns: vec![
                // "see Chapter 3", "refer to Chapter VII", "in chapter 5"
                (
                    Regex::new(r"(?i)\b(?:see|refer(?:\s+to)?|in|from)\s+chapter\s+(\d+|[IVXLCDM]+)\b")
                        .unwrap(),
                    0.95,
                ),
                // "Chapter 3" standalone
                (
                    Regex::new(r"(?i)\bchapter\s+(\d+|[IVXLCDM]+)\b").unwrap(),
                    0.85,
                ),
                // "Ch. 3", "Ch.VII"
                (
                    Regex::new(r"(?i)\bch\.\s*(\d+|[IVXLCDM]+)\b").unwrap(),
                    0.80,
                ),
            ],
            section_patterns: vec![
                // "see the 'Combat' section", "refer to the Rules section"
                (
                    Regex::new(
                        r#"(?i)\b(?:see|refer(?:\s+to)?)\s+(?:the\s+)?['"]?([A-Za-z][A-Za-z\s]+?)['"]?\s+section"#,
                    )
                    .unwrap(),
                    0.90,
                ),
                // "the section on Combat", "section on Movement"
                (
                    Regex::new(r#"(?i)\b(?:the\s+)?section\s+on\s+['"]?([A-Za-z][A-Za-z\s]+?)['"]?(?:\s|$|,|\.)"#).unwrap(),
                    0.85,
                ),
                // "under 'Combat'", "under Combat"
                (
                    Regex::new(r#"(?i)\bunder\s+['"]?([A-Za-z][A-Za-z\s]+?)['"]?(?:\s|$|,|\.)"#).unwrap(),
                    0.75,
                ),
            ],
            table_patterns: vec![
                // "see Table 1-3", "refer to Table 5", "roll on Table 2"
                (
                    Regex::new(r"(?i)\b(?:see|refer(?:\s+to)?|roll\s+on|use|consult)\s+table\s+(\d+(?:[-–.]\d+)?)\b")
                        .unwrap(),
                    0.95,
                ),
                // "Table 1-3" standalone
                (
                    Regex::new(r"(?i)\btable\s+(\d+(?:[-–.]\d+)?)\b").unwrap(),
                    0.85,
                ),
                // "Tbl. 5", "Tbl 3-2"
                (
                    Regex::new(r"(?i)\btbl\.?\s*(\d+(?:[-–.]\d+)?)\b").unwrap(),
                    0.80,
                ),
            ],
            figure_patterns: vec![
                // "see Figure 2", "as shown in Figure 4", "refer to Figure 1"
                (
                    Regex::new(r"(?i)\b(?:see|(?:as\s+)?shown\s+in|refer(?:\s+to)?)\s+figure\s+(\d+(?:[-–.]\d+)?)\b")
                        .unwrap(),
                    0.95,
                ),
                // "Figure 2" standalone
                (
                    Regex::new(r"(?i)\bfigure\s+(\d+(?:[-–.]\d+)?)\b").unwrap(),
                    0.85,
                ),
                // "Fig. 4", "Fig 2-1"
                (
                    Regex::new(r"(?i)\bfig\.?\s*(\d+(?:[-–.]\d+)?)\b").unwrap(),
                    0.85,
                ),
            ],
        }
    }
}

/// Extracts cross-references from TTRPG document text.
///
/// The extractor uses regex patterns to identify references to pages, chapters,
/// sections, tables, and figures. Each detected reference includes a confidence
/// score based on the specificity of the matched pattern.
pub struct CrossReferenceExtractor {
    patterns: ReferencePatterns,
    /// Minimum confidence threshold for including a reference
    min_confidence: f32,
}

impl Default for CrossReferenceExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossReferenceExtractor {
    /// Create a new extractor with default settings.
    pub fn new() -> Self {
        Self::with_min_confidence(0.0)
    }

    /// Create an extractor with a custom minimum confidence threshold.
    ///
    /// References below this threshold will be filtered out.
    pub fn with_min_confidence(min_confidence: f32) -> Self {
        Self {
            patterns: ReferencePatterns::new(),
            min_confidence,
        }
    }

    /// Extract all cross-references from the given text.
    ///
    /// # Arguments
    /// * `text` - The document text to search for cross-references
    ///
    /// # Returns
    /// A vector of detected cross-references, sorted by position in the text
    pub fn extract(&self, text: &str) -> Vec<CrossReference> {
        let mut references = Vec::new();

        // Extract each reference type
        self.extract_page_refs(text, &mut references);
        self.extract_chapter_refs(text, &mut references);
        self.extract_section_refs(text, &mut references);
        self.extract_table_refs(text, &mut references);
        self.extract_figure_refs(text, &mut references);

        // Filter by minimum confidence
        references.retain(|r| r.confidence >= self.min_confidence);

        // Sort by position in text
        references.sort_by_key(|r| r.start_offset);

        // Remove duplicates (same position, keep highest confidence)
        self.deduplicate(&mut references);

        references
    }

    /// Extract cross-references and return a summary.
    ///
    /// Useful for getting a quick overview of document connectivity.
    pub fn extract_summary(&self, text: &str) -> ReferenceSummary {
        let refs = self.extract(text);
        ReferenceSummary::from_references(&refs)
    }

    /// Helper method to extract references by type using the provided patterns.
    ///
    /// This consolidates the common extraction logic for all reference types,
    /// reducing code duplication and improving maintainability.
    fn extract_refs_by_type(
        &self,
        text: &str,
        results: &mut Vec<CrossReference>,
        ref_type: ReferenceType,
        patterns: &[(Regex, f32)],
    ) {
        for (pattern, confidence) in patterns {
            for caps in pattern.captures_iter(text) {
                if let (Some(full_match), Some(target)) = (caps.get(0), caps.get(1)) {
                    let target_str = target.as_str().trim().to_string();

                    // Skip very short section names (likely false positives)
                    if ref_type == ReferenceType::Section && target_str.len() < 3 {
                        continue;
                    }

                    results.push(CrossReference::new(
                        ref_type,
                        target_str,
                        full_match.as_str().to_string(),
                        *confidence,
                        full_match.start(),
                        full_match.end(),
                    ));
                }
            }
        }
    }

    fn extract_page_refs(&self, text: &str, results: &mut Vec<CrossReference>) {
        self.extract_refs_by_type(text, results, ReferenceType::Page, &self.patterns.page_patterns);
    }

    fn extract_chapter_refs(&self, text: &str, results: &mut Vec<CrossReference>) {
        self.extract_refs_by_type(text, results, ReferenceType::Chapter, &self.patterns.chapter_patterns);
    }

    fn extract_section_refs(&self, text: &str, results: &mut Vec<CrossReference>) {
        self.extract_refs_by_type(text, results, ReferenceType::Section, &self.patterns.section_patterns);
    }

    fn extract_table_refs(&self, text: &str, results: &mut Vec<CrossReference>) {
        self.extract_refs_by_type(text, results, ReferenceType::Table, &self.patterns.table_patterns);
    }

    fn extract_figure_refs(&self, text: &str, results: &mut Vec<CrossReference>) {
        self.extract_refs_by_type(text, results, ReferenceType::Figure, &self.patterns.figure_patterns);
    }

    /// Remove duplicate and overlapping references, keeping the highest confidence.
    ///
    /// When two references overlap (e.g., "see page 47" and "page 47"), we keep
    /// the one with higher confidence. This handles cases where a more specific
    /// pattern matches a subset of a more general pattern.
    fn deduplicate(&self, refs: &mut Vec<CrossReference>) {
        if refs.is_empty() {
            return;
        }

        // Sort by confidence (descending) so we process high-confidence refs first
        refs.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap()
                .then(a.start_offset.cmp(&b.start_offset))
        });

        // Track which character positions have been claimed
        let mut claimed_ranges: Vec<(usize, usize)> = Vec::new();

        refs.retain(|r| {
            // Check if this reference overlaps with any already-claimed range
            let overlaps = claimed_ranges.iter().any(|(start, end)| {
                // Two ranges overlap if one starts before the other ends
                r.start_offset < *end && r.end_offset > *start
            });

            if overlaps {
                false // Remove this reference (a higher-confidence one already claims this area)
            } else {
                // Claim this range
                claimed_ranges.push((r.start_offset, r.end_offset));
                true
            }
        });

        // Re-sort by position for consistent ordering
        refs.sort_by_key(|r| r.start_offset);
    }
}

// ============================================================================
// Summary
// ============================================================================

/// Summary statistics for cross-references in a document.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReferenceSummary {
    /// Total number of cross-references found
    pub total_count: usize,
    /// Count of page references
    pub page_count: usize,
    /// Count of chapter references
    pub chapter_count: usize,
    /// Count of section references
    pub section_count: usize,
    /// Count of table references
    pub table_count: usize,
    /// Count of figure references
    pub figure_count: usize,
    /// Unique page numbers referenced
    pub unique_pages: Vec<String>,
    /// Unique chapters referenced
    pub unique_chapters: Vec<String>,
    /// Unique tables referenced
    pub unique_tables: Vec<String>,
}

impl ReferenceSummary {
    /// Create a summary from a list of cross-references.
    pub fn from_references(refs: &[CrossReference]) -> Self {
        use std::collections::HashSet;

        let mut summary = Self::default();
        let mut pages: HashSet<String> = HashSet::new();
        let mut chapters: HashSet<String> = HashSet::new();
        let mut tables: HashSet<String> = HashSet::new();

        for r in refs {
            summary.total_count += 1;
            match r.ref_type {
                ReferenceType::Page => {
                    summary.page_count += 1;
                    pages.insert(r.ref_target.clone());
                }
                ReferenceType::Chapter => {
                    summary.chapter_count += 1;
                    chapters.insert(r.ref_target.clone());
                }
                ReferenceType::Section => {
                    summary.section_count += 1;
                }
                ReferenceType::Table => {
                    summary.table_count += 1;
                    tables.insert(r.ref_target.clone());
                }
                ReferenceType::Figure => {
                    summary.figure_count += 1;
                }
            }
        }

        summary.unique_pages = pages.into_iter().collect();
        summary.unique_pages.sort();
        summary.unique_chapters = chapters.into_iter().collect();
        summary.unique_chapters.sort();
        summary.unique_tables = tables.into_iter().collect();
        summary.unique_tables.sort();

        summary
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn extractor() -> CrossReferenceExtractor {
        CrossReferenceExtractor::new()
    }

    // -------------------------------------------------------------------------
    // Page Reference Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_page_ref_see_page() {
        let refs = extractor().extract("For more details, see page 47.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Page);
        assert_eq!(refs[0].ref_target, "47");
        assert!(refs[0].confidence >= 0.9);
    }

    #[test]
    fn test_page_ref_refer_to_page() {
        let refs = extractor().extract("Please refer to page 12 for the full rules.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Page);
        assert_eq!(refs[0].ref_target, "12");
    }

    #[test]
    fn test_page_ref_on_page() {
        let refs = extractor().extract("The table on page 23 shows the modifiers.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Page);
        assert_eq!(refs[0].ref_target, "23");
    }

    #[test]
    fn test_page_ref_standalone() {
        let refs = extractor().extract("(page 99)");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_target, "99");
    }

    #[test]
    fn test_page_ref_abbreviated_p_dot() {
        let refs = extractor().extract("For details, p. 15 has more information.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Page);
        assert_eq!(refs[0].ref_target, "15");
    }

    #[test]
    fn test_page_ref_abbreviated_p_no_space() {
        let refs = extractor().extract("(p.42)");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_target, "42");
    }

    #[test]
    fn test_page_ref_range() {
        let refs = extractor().extract("See pp. 15-20 for the complete rules.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_target, "15-20");
    }

    #[test]
    fn test_page_ref_pages_plural() {
        let refs = extractor().extract("Refer to pages 47 and 48.");
        assert_eq!(refs.len(), 1);
        assert!(refs[0].ref_target.contains("47"));
    }

    #[test]
    fn test_multiple_page_refs() {
        let refs = extractor().extract("See page 10 and page 20 for more.");
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].ref_target, "10");
        assert_eq!(refs[1].ref_target, "20");
    }

    // -------------------------------------------------------------------------
    // Chapter Reference Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_chapter_ref_see_chapter() {
        let refs = extractor().extract("For combat rules, see Chapter 3.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Chapter);
        assert_eq!(refs[0].ref_target, "3");
        assert!(refs[0].confidence >= 0.9);
    }

    #[test]
    fn test_chapter_ref_roman_numeral() {
        let refs = extractor().extract("Refer to Chapter VII for magic items.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Chapter);
        assert_eq!(refs[0].ref_target, "VII");
    }

    #[test]
    fn test_chapter_ref_in_chapter() {
        let refs = extractor().extract("As described in chapter 5, the rules are...");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Chapter);
        assert_eq!(refs[0].ref_target, "5");
    }

    #[test]
    fn test_chapter_ref_standalone() {
        let refs = extractor().extract("(Chapter 2)");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_target, "2");
    }

    #[test]
    fn test_chapter_ref_abbreviated() {
        let refs = extractor().extract("See Ch. 4 for details.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Chapter);
        assert_eq!(refs[0].ref_target, "4");
    }

    // -------------------------------------------------------------------------
    // Section Reference Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_section_ref_quoted() {
        let refs = extractor().extract("See the 'Combat' section for attack rules.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Section);
        assert_eq!(refs[0].ref_target, "Combat");
    }

    #[test]
    fn test_section_ref_unquoted() {
        let refs = extractor().extract("Refer to the Rules section.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Section);
        assert_eq!(refs[0].ref_target, "Rules");
    }

    #[test]
    fn test_section_ref_section_on() {
        let refs = extractor().extract("The section on Movement explains this.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Section);
        assert_eq!(refs[0].ref_target, "Movement");
    }

    #[test]
    fn test_section_ref_under() {
        let refs = extractor().extract("Find this under 'Equipment' in the appendix.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Section);
        assert_eq!(refs[0].ref_target, "Equipment");
    }

    #[test]
    fn test_section_ref_multiword() {
        let refs = extractor().extract("See the 'Character Creation' section.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_target, "Character Creation");
    }

    // -------------------------------------------------------------------------
    // Table Reference Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_table_ref_see_table() {
        let refs = extractor().extract("See Table 1-3 for random encounters.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Table);
        assert_eq!(refs[0].ref_target, "1-3");
        assert!(refs[0].confidence >= 0.9);
    }

    #[test]
    fn test_table_ref_refer_to() {
        let refs = extractor().extract("Refer to Table 5 for treasure.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_target, "5");
    }

    #[test]
    fn test_table_ref_roll_on() {
        let refs = extractor().extract("Roll on Table 2 to determine the result.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Table);
        assert_eq!(refs[0].ref_target, "2");
    }

    #[test]
    fn test_table_ref_consult() {
        let refs = extractor().extract("Consult Table 7-1 below.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_target, "7-1");
    }

    #[test]
    fn test_table_ref_standalone() {
        let refs = extractor().extract("(Table 4)");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_target, "4");
    }

    #[test]
    fn test_table_ref_abbreviated() {
        let refs = extractor().extract("See Tbl. 3 for options.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Table);
        assert_eq!(refs[0].ref_target, "3");
    }

    // -------------------------------------------------------------------------
    // Figure Reference Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_figure_ref_see_figure() {
        let refs = extractor().extract("See Figure 2 for the map layout.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Figure);
        assert_eq!(refs[0].ref_target, "2");
        assert!(refs[0].confidence >= 0.9);
    }

    #[test]
    fn test_figure_ref_as_shown_in() {
        let refs = extractor().extract("As shown in Figure 4, the dungeon has...");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Figure);
        assert_eq!(refs[0].ref_target, "4");
    }

    #[test]
    fn test_figure_ref_shown_in() {
        let refs = extractor().extract("This is shown in Figure 1.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_target, "1");
    }

    #[test]
    fn test_figure_ref_standalone() {
        let refs = extractor().extract("(Figure 3)");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_target, "3");
    }

    #[test]
    fn test_figure_ref_abbreviated_fig() {
        let refs = extractor().extract("See Fig. 4 for reference.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Figure);
        assert_eq!(refs[0].ref_target, "4");
    }

    #[test]
    fn test_figure_ref_abbreviated_no_dot() {
        let refs = extractor().extract("(Fig 2-1)");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_target, "2-1");
    }

    // -------------------------------------------------------------------------
    // Mixed Reference Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_multiple_reference_types() {
        let text = "For more details, see page 47 and refer to Chapter 3. \
                    The random encounter table is in Table 5-1 (see Figure 2 for the map).";
        let refs = extractor().extract(text);

        assert_eq!(refs.len(), 4);

        // Check they're in order of appearance
        assert_eq!(refs[0].ref_type, ReferenceType::Page);
        assert_eq!(refs[1].ref_type, ReferenceType::Chapter);
        assert_eq!(refs[2].ref_type, ReferenceType::Table);
        assert_eq!(refs[3].ref_type, ReferenceType::Figure);
    }

    #[test]
    fn test_complex_document_excerpt() {
        let text = r#"
            COMBAT

            For the full rules on combat, see Chapter 9. The action economy is
            explained on page 189. When determining initiative, refer to the
            'Actions in Combat' section.

            Random encounters can be generated using Table 3-1 (p. 45). For a
            visual representation of the battle grid, see Figure 7-2.
        "#;

        let refs = extractor().extract(text);

        // Should find: Chapter 9, page 189, section "Actions in Combat",
        // Table 3-1, p. 45, Figure 7-2
        assert!(refs.len() >= 5);

        let types: Vec<_> = refs.iter().map(|r| r.ref_type).collect();
        assert!(types.contains(&ReferenceType::Chapter));
        assert!(types.contains(&ReferenceType::Page));
        assert!(types.contains(&ReferenceType::Section));
        assert!(types.contains(&ReferenceType::Table));
        assert!(types.contains(&ReferenceType::Figure));
    }

    // -------------------------------------------------------------------------
    // Edge Cases and Validation
    // -------------------------------------------------------------------------

    #[test]
    fn test_no_references() {
        let refs = extractor().extract("This text has no cross-references at all.");
        assert!(refs.is_empty());
    }

    #[test]
    fn test_case_insensitivity() {
        let refs = extractor().extract("SEE PAGE 10 and see page 20");
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn test_offsets_are_correct() {
        let text = "See page 42 for details.";
        let refs = extractor().extract(text);

        assert_eq!(refs.len(), 1);
        let r = &refs[0];
        assert_eq!(&text[r.start_offset..r.end_offset], r.ref_text);
    }

    #[test]
    fn test_minimum_confidence_filter() {
        let high_conf = CrossReferenceExtractor::with_min_confidence(0.9);
        let low_conf = CrossReferenceExtractor::with_min_confidence(0.5);

        let text = "page 42"; // Lower confidence pattern

        let high_refs = high_conf.extract(text);
        let low_refs = low_conf.extract(text);

        // The standalone "page 42" has confidence 0.85, so should be filtered at 0.9
        assert!(high_refs.is_empty());
        assert!(!low_refs.is_empty());
    }

    #[test]
    fn test_summary_generation() {
        let text = "See page 10, page 20, and page 10 again. Also Chapter 3 and Table 1.";
        let summary = extractor().extract_summary(text);

        assert_eq!(summary.total_count, 5);
        assert_eq!(summary.page_count, 3);
        assert_eq!(summary.chapter_count, 1);
        assert_eq!(summary.table_count, 1);
        assert_eq!(summary.unique_pages.len(), 2); // 10 and 20, deduplicated
    }

    #[test]
    fn test_reference_type_as_str() {
        assert_eq!(ReferenceType::Page.as_str(), "page");
        assert_eq!(ReferenceType::Chapter.as_str(), "chapter");
        assert_eq!(ReferenceType::Section.as_str(), "section");
        assert_eq!(ReferenceType::Table.as_str(), "table");
        assert_eq!(ReferenceType::Figure.as_str(), "figure");
    }

    #[test]
    fn test_default_extractor() {
        let extractor = CrossReferenceExtractor::default();
        let refs = extractor.extract("see page 1");
        assert_eq!(refs.len(), 1);
    }
}
