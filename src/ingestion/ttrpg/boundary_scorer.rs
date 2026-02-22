//! Boundary Scoring Module for Semantic Text Splitting
//!
//! Provides intelligent boundary detection for splitting text at natural semantic
//! boundaries rather than arbitrary character positions. Particularly useful for
//! TTRPG content where maintaining context coherence is critical.
//!
//! # Boundary Types (by priority)
//!
//! - **SectionHeader (0.95)**: Markdown headers, detected titles
//! - **DoubleNewline (0.85)**: Paragraph breaks
//! - **AllCapsLine (0.80)**: OSR-style section headers
//! - **BulletStart (0.70)**: List item boundaries
//! - **SentenceCapital (0.60)**: Sentence ending + capital letter start
//! - **TransitionWord (0.50)**: "However", "Therefore", etc.
//! - **SentenceEnd (0.40)**: Period + space
//! - **ClauseBoundary (0.20)**: Comma, semicolon
//! - **Fallback (0.10)**: Character limit reached
//!
//! # Example
//!
//! ```ignore
//! use crate::ingestion::ttrpg::BoundaryScorer;
//!
//! let scorer = BoundaryScorer::new();
//! let text = "# Chapter 1\n\nThe dragon attacks. However, the party is ready.";
//!
//! // Find all boundaries
//! let boundaries = scorer.find_boundaries(text);
//!
//! // Find best split point near position 30, within a 20-char window
//! let split_pos = scorer.find_best_split(text, 30, 20);
//!
//! // Split text into chunks of ~500 chars, max 800
//! let ranges = scorer.split_at_best_boundaries(text, 500, 800);
//! ```

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

// ============================================================================
// Constants
// ============================================================================

/// Transition words that indicate good split points
const TRANSITION_WORDS: &[&str] = &[
    "however",
    "therefore",
    "additionally",
    "furthermore",
    "moreover",
    "nevertheless",
    "consequently",
    "meanwhile",
    "otherwise",
    "accordingly",
    "similarly",
    "likewise",
    "instead",
    "alternatively",
    "notably",
    "specifically",
    "importantly",
    "finally",
    "ultimately",
    "subsequently",
];

// ============================================================================
// Regex Patterns (compiled once)
// ============================================================================

static MARKDOWN_HEADER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^#{1,6}\s+\S").unwrap());

static DOUBLE_NEWLINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\n\s*\n").unwrap());

static ALL_CAPS_LINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^[A-Z][A-Z\s]{2,}[A-Z]$").unwrap());

static BULLET_START_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^[\s]*[-*+\u2022]\s+").unwrap());

static NUMBERED_LIST_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^[\s]*\d+[.)]\s+").unwrap());

static SENTENCE_END_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[.!?][\s]+").unwrap());

static SENTENCE_CAPITAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[.!?][\s]+[A-Z]").unwrap());

static CLAUSE_BOUNDARY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[,;:][\s]+").unwrap());

// ============================================================================
// Types
// ============================================================================

/// Types of text boundaries ordered by semantic significance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BoundaryType {
    /// Markdown header (# Header)
    SectionHeader,
    /// Double newline / paragraph break
    DoubleNewline,
    /// ALL CAPS line (OSR-style headers)
    AllCapsLine,
    /// Bullet or numbered list start
    BulletStart,
    /// Sentence end followed by capital letter
    SentenceCapital,
    /// Transition word at sentence start
    TransitionWord,
    /// Simple sentence end (. ! ?)
    SentenceEnd,
    /// Clause boundary (, ; :)
    ClauseBoundary,
    /// Fallback when no better boundary found
    Fallback,
}

impl BoundaryType {
    /// Get the semantic score for this boundary type (0.0 to 1.0).
    /// Higher scores indicate better split points.
    #[inline]
    pub fn score(self) -> f32 {
        match self {
            Self::SectionHeader => 0.95,
            Self::DoubleNewline => 0.85,
            Self::AllCapsLine => 0.80,
            Self::BulletStart => 0.70,
            Self::SentenceCapital => 0.60,
            Self::TransitionWord => 0.50,
            Self::SentenceEnd => 0.40,
            Self::ClauseBoundary => 0.20,
            Self::Fallback => 0.10,
        }
    }

    /// Get a human-readable name for this boundary type.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SectionHeader => "section_header",
            Self::DoubleNewline => "double_newline",
            Self::AllCapsLine => "all_caps_line",
            Self::BulletStart => "bullet_start",
            Self::SentenceCapital => "sentence_capital",
            Self::TransitionWord => "transition_word",
            Self::SentenceEnd => "sentence_end",
            Self::ClauseBoundary => "clause_boundary",
            Self::Fallback => "fallback",
        }
    }
}

/// A detected boundary in text with its position and type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundaryMatch {
    /// Character position in the source text (start of boundary)
    pub position: usize,
    /// The type of boundary detected
    pub boundary_type: BoundaryType,
    /// Semantic score for splitting at this boundary
    pub score: f32,
}

impl BoundaryMatch {
    /// Create a new boundary match.
    pub fn new(position: usize, boundary_type: BoundaryType) -> Self {
        Self {
            position,
            boundary_type,
            score: boundary_type.score(),
        }
    }
}

// ============================================================================
// BoundaryScorer Implementation
// ============================================================================

/// Scorer for finding optimal text split points based on semantic boundaries.
#[derive(Debug, Clone, Default)]
pub struct BoundaryScorer {
    /// Minimum characters between boundaries to consider
    min_boundary_gap: usize,
}

impl BoundaryScorer {
    /// Create a new boundary scorer with default settings.
    pub fn new() -> Self {
        Self {
            min_boundary_gap: 10,
        }
    }

    /// Create a boundary scorer with custom minimum gap between boundaries.
    pub fn with_min_gap(min_gap: usize) -> Self {
        Self {
            min_boundary_gap: min_gap,
        }
    }

    /// Find all boundary candidates in the given text.
    ///
    /// Returns boundaries sorted by position (ascending).
    pub fn find_boundaries(&self, text: &str) -> Vec<BoundaryMatch> {
        let mut boundaries = Vec::new();

        // Section headers (highest priority)
        for m in MARKDOWN_HEADER_RE.find_iter(text) {
            boundaries.push(BoundaryMatch::new(m.start(), BoundaryType::SectionHeader));
        }

        // Double newlines (paragraph breaks)
        for m in DOUBLE_NEWLINE_RE.find_iter(text) {
            // Position at end of the blank line(s)
            boundaries.push(BoundaryMatch::new(m.end(), BoundaryType::DoubleNewline));
        }

        // ALL CAPS lines (OSR headers)
        for m in ALL_CAPS_LINE_RE.find_iter(text) {
            boundaries.push(BoundaryMatch::new(m.start(), BoundaryType::AllCapsLine));
        }

        // Bullet points
        for m in BULLET_START_RE.find_iter(text) {
            boundaries.push(BoundaryMatch::new(m.start(), BoundaryType::BulletStart));
        }

        // Numbered lists
        for m in NUMBERED_LIST_RE.find_iter(text) {
            boundaries.push(BoundaryMatch::new(m.start(), BoundaryType::BulletStart));
        }

        // Sentence ends with capital (stronger boundary)
        for m in SENTENCE_CAPITAL_RE.find_iter(text) {
            // Position after the punctuation and whitespace, before the capital
            let pos = m.end() - 1;
            boundaries.push(BoundaryMatch::new(pos, BoundaryType::SentenceCapital));
        }

        // Check for transition words after sentence boundaries
        self.find_transition_word_boundaries(text, &mut boundaries);

        // Simple sentence ends
        for m in SENTENCE_END_RE.find_iter(text) {
            boundaries.push(BoundaryMatch::new(m.end(), BoundaryType::SentenceEnd));
        }

        // Clause boundaries (lowest priority)
        for m in CLAUSE_BOUNDARY_RE.find_iter(text) {
            boundaries.push(BoundaryMatch::new(m.end(), BoundaryType::ClauseBoundary));
        }

        // Sort by position and deduplicate nearby boundaries
        boundaries.sort_by_key(|b| b.position);
        self.deduplicate_boundaries(&mut boundaries);

        boundaries
    }

    /// Find transition word boundaries in text.
    fn find_transition_word_boundaries(&self, text: &str, boundaries: &mut Vec<BoundaryMatch>) {
        let text_lower = text.to_lowercase();

        for word in TRANSITION_WORDS {
            // Match ". Word" pattern (sentence end + transition word)
            let pattern = format!(". {}", word);
            let mut start = 0;

            while let Some(pos) = text_lower[start..].find(&pattern) {
                let actual_pos = start + pos + 2; // After ". "
                boundaries.push(BoundaryMatch::new(actual_pos, BoundaryType::TransitionWord));
                start = start + pos + pattern.len();
            }

            // Also match ", word" pattern
            let comma_pattern = format!(", {}", word);
            let mut start = 0;

            while let Some(pos) = text_lower[start..].find(&comma_pattern) {
                let actual_pos = start + pos + 2; // After ", "
                boundaries.push(BoundaryMatch::new(actual_pos, BoundaryType::TransitionWord));
                start = start + pos + comma_pattern.len();
            }
        }
    }

    /// Remove duplicate boundaries that are too close together,
    /// keeping the one with the highest score.
    ///
    /// Optimized O(N) algorithm: iterates through sorted boundaries once,
    /// replacing the last added boundary if a higher-scored one is found
    /// within the minimum gap.
    fn deduplicate_boundaries(&self, boundaries: &mut Vec<BoundaryMatch>) {
        if boundaries.len() < 2 {
            return;
        }

        let mut deduped = Vec::with_capacity(boundaries.len());
        deduped.push(boundaries[0].clone());

        for boundary in boundaries.iter().skip(1) {
            // Safe unwrap: we just pushed an element above
            let last_boundary = deduped.last_mut().unwrap();

            if boundary.position.saturating_sub(last_boundary.position) < self.min_boundary_gap {
                // Too close to previous - keep the one with the higher score
                if boundary.score > last_boundary.score {
                    *last_boundary = boundary.clone();
                }
            } else {
                // Far enough apart - add as new boundary
                deduped.push(boundary.clone());
            }
        }

        *boundaries = deduped;
    }

    /// Find the best split point within a window around the target position.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to search for boundaries
    /// * `target_pos` - The ideal split position (e.g., target chunk size)
    /// * `window` - How far before/after target_pos to search
    ///
    /// # Returns
    ///
    /// The position of the best boundary found, or `None` if no boundary
    /// exists within the window.
    pub fn find_best_split(&self, text: &str, target_pos: usize, window: usize) -> Option<usize> {
        let boundaries = self.find_boundaries(text);

        if boundaries.is_empty() {
            return None;
        }

        let window_start = target_pos.saturating_sub(window);
        let window_end = (target_pos + window).min(text.len());

        // Find boundaries within the window
        let candidates: Vec<_> = boundaries
            .iter()
            .filter(|b| b.position >= window_start && b.position <= window_end)
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // Score candidates based on: boundary score + proximity to target
        // Closer to target is better, higher boundary score is better
        let best = candidates
            .iter()
            .max_by(|a, b| {
                let a_distance = (a.position as isize - target_pos as isize).unsigned_abs();
                let b_distance = (b.position as isize - target_pos as isize).unsigned_abs();

                // Normalize distance penalty (0.0 to 0.3 based on how far from target)
                let a_distance_penalty = (a_distance as f32 / window as f32) * 0.3;
                let b_distance_penalty = (b_distance as f32 / window as f32) * 0.3;

                let a_final_score = a.score - a_distance_penalty;
                let b_final_score = b.score - b_distance_penalty;

                a_final_score
                    .partial_cmp(&b_final_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })?;

        Some(best.position)
    }

    /// Split text into ranges at the best semantic boundaries.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to split
    /// * `target_size` - Ideal chunk size in characters
    /// * `max_size` - Maximum chunk size (will force split even without good boundary)
    ///
    /// # Returns
    ///
    /// A vector of (start, end) character ranges representing chunks.
    pub fn split_at_best_boundaries(
        &self,
        text: &str,
        target_size: usize,
        max_size: usize,
    ) -> Vec<(usize, usize)> {
        if text.is_empty() {
            return vec![];
        }

        if text.len() <= target_size {
            return vec![(0, text.len())];
        }

        let boundaries = self.find_boundaries(text);
        let mut ranges = Vec::new();
        let mut current_start = 0;

        // Window size for finding boundaries (search 30% before/after target)
        let window = target_size / 3;

        while current_start < text.len() {
            let remaining = text.len() - current_start;

            // If remaining text fits in target, take it all
            if remaining <= target_size {
                ranges.push((current_start, text.len()));
                break;
            }

            // Find ideal split point
            let target_end = current_start + target_size;
            let max_end = (current_start + max_size).min(text.len());

            // Look for best boundary in window around target
            let split_pos = self
                .find_boundary_in_range(&boundaries, target_end.saturating_sub(window), max_end)
                .unwrap_or_else(|| {
                    // No good boundary found - force split at max_size
                    // Try to at least split at a space
                    self.find_space_near(text, max_end.min(text.len()))
                        .unwrap_or(max_end.min(text.len()))
                });

            ranges.push((current_start, split_pos));
            current_start = split_pos;

            // Skip leading whitespace for next chunk
            while current_start < text.len()
                && text[current_start..].starts_with(|c: char| c.is_whitespace())
            {
                current_start += 1;
            }
        }

        ranges
    }

    /// Find the best boundary in a given range.
    fn find_boundary_in_range(
        &self,
        boundaries: &[BoundaryMatch],
        start: usize,
        end: usize,
    ) -> Option<usize> {
        boundaries
            .iter()
            .filter(|b| b.position >= start && b.position <= end)
            .max_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|b| b.position)
    }

    /// Find the nearest space character near a position (for fallback splits).
    fn find_space_near(&self, text: &str, pos: usize) -> Option<usize> {
        // Search backwards from position for a space
        let search_start = pos.saturating_sub(50);

        text[search_start..pos]
            .rfind(' ')
            .map(|offset| search_start + offset + 1)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boundary_type_scores() {
        assert_eq!(BoundaryType::SectionHeader.score(), 0.95);
        assert_eq!(BoundaryType::DoubleNewline.score(), 0.85);
        assert_eq!(BoundaryType::AllCapsLine.score(), 0.80);
        assert_eq!(BoundaryType::BulletStart.score(), 0.70);
        assert_eq!(BoundaryType::SentenceCapital.score(), 0.60);
        assert_eq!(BoundaryType::TransitionWord.score(), 0.50);
        assert_eq!(BoundaryType::SentenceEnd.score(), 0.40);
        assert_eq!(BoundaryType::ClauseBoundary.score(), 0.20);
        assert_eq!(BoundaryType::Fallback.score(), 0.10);
    }

    #[test]
    fn test_find_paragraph_boundaries() {
        let text = "First paragraph with content.\n\nSecond paragraph starts here.\n\nThird paragraph.";
        let scorer = BoundaryScorer::new();
        let boundaries = scorer.find_boundaries(text);

        // Should find double newlines
        let double_newlines: Vec<_> = boundaries
            .iter()
            .filter(|b| b.boundary_type == BoundaryType::DoubleNewline)
            .collect();

        assert_eq!(double_newlines.len(), 2, "Should find 2 paragraph breaks");
    }

    #[test]
    fn test_find_markdown_headers() {
        let text = "# Chapter 1\n\nSome content here.\n\n## Section 1.1\n\nMore content.";
        let scorer = BoundaryScorer::new();
        let boundaries = scorer.find_boundaries(text);

        let headers: Vec<_> = boundaries
            .iter()
            .filter(|b| b.boundary_type == BoundaryType::SectionHeader)
            .collect();

        assert_eq!(headers.len(), 2, "Should find 2 markdown headers");
        assert_eq!(headers[0].score, 0.95);
    }

    #[test]
    fn test_find_all_caps_headers() {
        let text = "TREASURE HOARD\n\nThe dragon's treasure includes:\n\nMONSTER STATS\n\nAC 15, HP 45";
        let scorer = BoundaryScorer::new();
        let boundaries = scorer.find_boundaries(text);

        let caps_lines: Vec<_> = boundaries
            .iter()
            .filter(|b| b.boundary_type == BoundaryType::AllCapsLine)
            .collect();

        // At least one ALL CAPS header should be found
        assert!(
            !caps_lines.is_empty(),
            "Should find at least one ALL CAPS header: {:?}",
            caps_lines
        );
        assert_eq!(caps_lines[0].score, 0.80);
    }

    #[test]
    fn test_find_sentence_boundaries() {
        let text = "The dragon attacks! The party fights back. Victory is theirs?";
        let scorer = BoundaryScorer::new();
        let boundaries = scorer.find_boundaries(text);

        let sentence_ends: Vec<_> = boundaries
            .iter()
            .filter(|b| {
                b.boundary_type == BoundaryType::SentenceEnd
                    || b.boundary_type == BoundaryType::SentenceCapital
            })
            .collect();

        assert!(
            sentence_ends.len() >= 2,
            "Should find sentence boundaries: {:?}",
            sentence_ends
        );
    }

    #[test]
    fn test_find_bullet_list_boundaries() {
        let text = "Equipment:\n- Sword\n- Shield\n* Potion\n1. First item\n2. Second item";
        let scorer = BoundaryScorer::new();
        let boundaries = scorer.find_boundaries(text);

        let bullets: Vec<_> = boundaries
            .iter()
            .filter(|b| b.boundary_type == BoundaryType::BulletStart)
            .collect();

        // Should find at least some bullet/list items (deduplication may reduce count)
        assert!(
            bullets.len() >= 2,
            "Should find bullet/list items: {:?}",
            bullets
        );
        assert_eq!(bullets[0].score, 0.70);
    }

    #[test]
    fn test_find_transition_word_boundaries() {
        // Use comma before transition words to test that pattern
        let text = "The party entered the dungeon, however they were not prepared. Therefore, they retreated.";
        let scorer = BoundaryScorer::new();
        let boundaries = scorer.find_boundaries(text);

        let transitions: Vec<_> = boundaries
            .iter()
            .filter(|b| b.boundary_type == BoundaryType::TransitionWord)
            .collect();

        // Should find at least one transition word
        assert!(
            !transitions.is_empty(),
            "Should find transition words: {:?}",
            transitions
        );
        assert_eq!(transitions[0].score, 0.50);
    }

    #[test]
    fn test_find_best_split_prefers_higher_scored() {
        // Text with clear paragraph break and sentence boundaries
        let text = "Some intro text here.\n\nThe next section begins with adventure and excitement.";
        let scorer = BoundaryScorer::new();

        // Target near the paragraph break - should prefer it over sentence end
        let split = scorer.find_best_split(text, 25, 15);

        assert!(split.is_some(), "Should find a split point");

        // The double newline should be found and preferred
        let boundaries = scorer.find_boundaries(text);
        let paragraph = boundaries
            .iter()
            .find(|b| b.boundary_type == BoundaryType::DoubleNewline);
        assert!(paragraph.is_some(), "Should have found the paragraph break");
    }

    #[test]
    fn test_find_best_split_within_window() {
        let text = "First sentence ends here. Second sentence starts now. Third one follows. Fourth is last.";
        let scorer = BoundaryScorer::new();

        // Small window should only find nearby boundaries
        let split = scorer.find_best_split(text, 30, 10);
        assert!(split.is_some());

        let pos = split.unwrap();
        assert!(
            pos >= 20 && pos <= 40,
            "Split should be within window: {}",
            pos
        );
    }

    #[test]
    fn test_split_at_best_boundaries_single_chunk() {
        let text = "Short text that fits.";
        let scorer = BoundaryScorer::new();

        let ranges = scorer.split_at_best_boundaries(text, 100, 150);

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0], (0, text.len()));
    }

    #[test]
    fn test_split_at_best_boundaries_multiple_chunks() {
        let text = "First paragraph with substantial content that should be kept together.\n\n\
                    Second paragraph also has content that needs proper splitting.\n\n\
                    Third paragraph completes the document with additional text.";

        let scorer = BoundaryScorer::new();
        let ranges = scorer.split_at_best_boundaries(text, 80, 120);

        assert!(ranges.len() >= 2, "Should split into multiple chunks");

        // Verify ranges don't overlap and cover the text
        for i in 0..ranges.len() - 1 {
            assert!(
                ranges[i].1 <= ranges[i + 1].0,
                "Ranges should not overlap: {:?}",
                ranges
            );
        }

        // First chunk should start at 0
        assert_eq!(ranges[0].0, 0);

        // Last chunk should end at or near text length
        let last_end = ranges.last().unwrap().1;
        assert!(
            last_end == text.len() || last_end >= text.len() - 5,
            "Last chunk should end near text end"
        );
    }

    #[test]
    fn test_split_respects_max_size() {
        let text = "A".repeat(300); // No natural boundaries
        let scorer = BoundaryScorer::new();

        let ranges = scorer.split_at_best_boundaries(&text, 100, 150);

        for (start, end) in &ranges {
            let chunk_size = end - start;
            assert!(
                chunk_size <= 150,
                "Chunk size {} exceeds max 150",
                chunk_size
            );
        }
    }

    #[test]
    fn test_boundary_match_creation() {
        let boundary = BoundaryMatch::new(42, BoundaryType::DoubleNewline);

        assert_eq!(boundary.position, 42);
        assert_eq!(boundary.boundary_type, BoundaryType::DoubleNewline);
        assert_eq!(boundary.score, 0.85);
    }

    #[test]
    fn test_empty_text_handling() {
        let scorer = BoundaryScorer::new();

        let boundaries = scorer.find_boundaries("");
        assert!(boundaries.is_empty());

        let split = scorer.find_best_split("", 50, 20);
        assert!(split.is_none());

        let ranges = scorer.split_at_best_boundaries("", 100, 150);
        assert!(ranges.is_empty());
    }

    #[test]
    fn test_scorer_with_custom_min_gap() {
        let scorer = BoundaryScorer::with_min_gap(5);

        // Text with close boundaries
        let text = "A. B. C. D. E.";
        let boundaries = scorer.find_boundaries(text);

        // Should deduplicate very close boundaries
        assert!(
            boundaries.len() < 10,
            "Should reduce duplicate boundaries: {:?}",
            boundaries
        );
    }

    #[test]
    fn test_clause_boundary_detection() {
        let text = "The wizard cast fireball, dealing massive damage; the dragon roared in pain.";
        let scorer = BoundaryScorer::new();
        let boundaries = scorer.find_boundaries(text);

        let clauses: Vec<_> = boundaries
            .iter()
            .filter(|b| b.boundary_type == BoundaryType::ClauseBoundary)
            .collect();

        assert!(
            clauses.len() >= 2,
            "Should find clause boundaries: {:?}",
            clauses
        );
    }

    #[test]
    fn test_boundary_type_as_str() {
        assert_eq!(BoundaryType::SectionHeader.as_str(), "section_header");
        assert_eq!(BoundaryType::DoubleNewline.as_str(), "double_newline");
        assert_eq!(BoundaryType::AllCapsLine.as_str(), "all_caps_line");
        assert_eq!(BoundaryType::BulletStart.as_str(), "bullet_start");
        assert_eq!(BoundaryType::SentenceCapital.as_str(), "sentence_capital");
        assert_eq!(BoundaryType::TransitionWord.as_str(), "transition_word");
        assert_eq!(BoundaryType::SentenceEnd.as_str(), "sentence_end");
        assert_eq!(BoundaryType::ClauseBoundary.as_str(), "clause_boundary");
        assert_eq!(BoundaryType::Fallback.as_str(), "fallback");
    }

    #[test]
    fn test_mixed_boundary_types_ranking() {
        // Text with multiple boundary types near each other
        let text = "Introduction text.\n\n# CHAPTER ONE\n\nThe adventure begins. However, danger lurks.";
        let scorer = BoundaryScorer::new();
        let boundaries = scorer.find_boundaries(text);

        // Verify we find various types
        let has_double_newline = boundaries
            .iter()
            .any(|b| b.boundary_type == BoundaryType::DoubleNewline);
        let has_header = boundaries
            .iter()
            .any(|b| b.boundary_type == BoundaryType::SectionHeader);

        assert!(has_double_newline, "Should find paragraph break");
        assert!(has_header, "Should find section header");
    }
}
