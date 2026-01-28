//! Meilisearch Pipeline Data Models
//!
//! This module contains the data transfer objects (DTOs) used by the
//! two-phase ingestion pipeline:
//! - Phase 1: Extract raw pages to `<slug>-raw` index
//! - Phase 2: Create semantic chunks in `<slug>` index
//!
//! These models are serializable for Meilisearch storage and include
//! TTRPG-specific metadata for enhanced search relevance.

use crate::ingestion::ttrpg::{
    TTRPGClassifier, CrossReferenceExtractor, ContentModeClassifier, DiceExtractor,
};
use std::path::Path;

// ============================================================================
// Classification Context (Shared Classifiers)
// ============================================================================

/// Holds all classifiers for v2 enhanced TTRPG content analysis.
///
/// Creating classifiers once and reusing them improves performance when
/// processing many chunks, as the regex patterns are compiled only once.
pub struct ClassificationContext {
    pub classifier: TTRPGClassifier,
    pub mode_classifier: ContentModeClassifier,
    pub cross_ref_extractor: CrossReferenceExtractor,
    pub dice_extractor: DiceExtractor,
}

impl Default for ClassificationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ClassificationContext {
    /// Create a new classification context with all classifiers initialized.
    pub fn new() -> Self {
        Self {
            classifier: TTRPGClassifier::new(),
            mode_classifier: ContentModeClassifier::new(),
            cross_ref_extractor: CrossReferenceExtractor::new(),
            dice_extractor: DiceExtractor::new(),
        }
    }

    /// Classify content and return structured results for a chunk.
    pub fn classify_content(&self, content: &str, page: u32) -> ClassificationResult {
        let classified = self.classifier.classify(content, page);
        let mode_result = self.mode_classifier.classify(content);
        let cross_refs: Vec<String> = self
            .cross_ref_extractor
            .extract(content)
            .iter()
            .map(|r| format!("{}:{}", r.ref_type.as_str(), r.ref_target))
            .collect();
        let dice_result = self.dice_extractor.extract(content);
        let dice_expressions: Vec<String> = dice_result
            .expressions
            .iter()
            .map(|e| e.to_canonical())
            .collect();

        ClassificationResult {
            element_type: classified.element_type.as_str().to_string(),
            classification_confidence: classified.confidence,
            content_mode: mode_result.mode.as_str().to_string(),
            cross_refs,
            dice_expressions,
        }
    }
}

/// Result of classifying content using the ClassificationContext.
pub struct ClassificationResult {
    pub element_type: String,
    pub classification_confidence: f32,
    pub content_mode: String,
    pub cross_refs: Vec<String>,
    pub dice_expressions: Vec<String>,
}

// ============================================================================
// Page Metadata
// ============================================================================

/// Metadata specific to a single page
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PageMetadata {
    /// Character count for this page
    #[serde(default)]
    pub char_count: usize,
    /// Word count for this page
    #[serde(default)]
    pub word_count: usize,
    /// Whether page appears to contain images/figures
    #[serde(default)]
    pub has_images: bool,
    /// Whether page appears to contain tables
    #[serde(default)]
    pub has_tables: bool,
    /// Detected header/title on this page (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_header: Option<String>,
}

impl PageMetadata {
    /// Create page metadata from raw content
    pub fn from_content(content: &str) -> Self {
        let char_count = content.len();
        let word_count = content.split_whitespace().count();

        // Simple heuristics for detecting tables/images
        let content_lower = content.to_lowercase();
        let has_tables = content.contains('|')
            && content.lines().filter(|l| l.contains('|')).count() >= 2
            || content_lower.contains("table ")
            || content
                .lines()
                .any(|l| l.chars().filter(|c| *c == '\t').count() >= 3);

        let has_images = content_lower.contains("[image")
            || content_lower.contains("figure ")
            || content_lower.contains("illustration");

        // Try to detect page header (first non-empty line if it looks like a title)
        let page_header = content
            .lines()
            .find(|l| !l.trim().is_empty())
            .filter(|l| {
                let trimmed = l.trim();
                // Looks like a header if: short, doesn't end with period, has capital letters
                trimmed.len() < 100
                    && !trimmed.ends_with('.')
                    && trimmed.chars().any(|c| c.is_uppercase())
            })
            .map(|s| s.trim().to_string());

        Self {
            char_count,
            word_count,
            has_images,
            has_tables,
            page_header,
        }
    }
}

// ============================================================================
// Raw Document (Page-Level Storage)
// ============================================================================

/// A raw page-level document stored in the `<slug>-raw` index.
///
/// This is the first stage of the two-phase ingestion pipeline.
/// Raw documents preserve the original page structure for provenance tracking.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawDocument {
    /// Deterministic ID: `<slug>-p<page_number>` (e.g., "delta-green-p001")
    pub id: String,
    /// The source slug this document belongs to
    pub source_slug: String,
    /// Raw text content of the page
    pub raw_content: String,
    /// Page number (1-indexed)
    pub page_number: u32,
    /// Page-specific metadata
    #[serde(default)]
    pub page_metadata: PageMetadata,
    /// Timestamp when extracted
    pub extracted_at: String,
}

impl RawDocument {
    /// Create a new raw document from page content
    pub fn new(slug: &str, page_number: u32, content: String) -> Self {
        let id = format!("{}-p{:04}", slug, page_number);
        let page_metadata = PageMetadata::from_content(&content);

        Self {
            id,
            source_slug: slug.to_string(),
            raw_content: content,
            page_number,
            page_metadata,
            extracted_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Generate a deterministic ID for a page
    pub fn make_id(slug: &str, page_number: u32) -> String {
        format!("{}-p{:04}", slug, page_number)
    }
}

// ============================================================================
// Chunked Document (Semantic Chunks)
// ============================================================================

/// A semantic chunk stored in the `<slug>` index.
///
/// This is the second stage of the two-phase ingestion pipeline.
/// Chunks reference their source raw documents for page number attribution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChunkedDocument {
    /// Deterministic ID: `<slug>-c<chunk_index>` (e.g., "delta-green-c001")
    pub id: String,
    /// The source slug this chunk belongs to
    pub source_slug: String,
    /// Semantic chunk content (may span multiple pages)
    pub content: String,
    /// IDs of raw documents this chunk was derived from
    /// Used for page number attribution in search results
    pub source_raw_ids: Vec<String>,
    /// Page range this chunk spans (derived from source_raw_ids)
    pub page_start: u32,
    pub page_end: u32,
    /// Chunk index within this source
    pub chunk_index: u32,
    /// Timestamp when chunked
    pub chunked_at: String,

    // TTRPG-specific metadata (populated during chunking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_system_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mechanic_type: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_keywords: Vec<String>,

    // =========================================================================
    // Enhanced TTRPG Metadata (v2 - semantic chunking improvements)
    // =========================================================================

    /// Element type classification (stat_block, random_table, spell, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_type: Option<String>,

    /// Full section hierarchy path (e.g., "Chapter 1 > Monsters > Goblins")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_path: Option<String>,

    /// Numeric section depth (0 = root, 1 = chapter, 2 = section, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_depth: Option<u32>,

    /// Parent section titles for breadcrumb navigation
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parent_sections: Vec<String>,

    /// Cross-references detected in this chunk (serialized JSON array)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cross_refs: Vec<String>,

    /// Content mode: crunch, fluff, mixed, example, optional, fiction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_mode: Option<String>,

    /// Extracted dice expressions (e.g., ["2d6", "1d20+5"])
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dice_expressions: Vec<String>,

    /// Classification confidence score (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification_confidence: Option<f32>,

    /// Context-injected content for embeddings (section path + type prefix)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_content: Option<String>,
}

impl ChunkedDocument {
    /// Create a new chunked document
    pub fn new(
        slug: &str,
        chunk_index: u32,
        content: String,
        source_raw_ids: Vec<String>,
    ) -> Self {
        let id = format!("{}-c{:04}", slug, chunk_index);

        // Extract page range from source IDs (format: slug-pNNNN)
        let pages: Vec<u32> = source_raw_ids
            .iter()
            .filter_map(|id| id.rsplit_once("-p").and_then(|(_, num)| num.parse().ok()))
            .collect();

        let page_start = pages.iter().copied().min().unwrap_or(1);
        let page_end = pages.iter().copied().max().unwrap_or(1);

        Self {
            id,
            source_slug: slug.to_string(),
            content,
            source_raw_ids,
            page_start,
            page_end,
            chunk_index,
            chunked_at: chrono::Utc::now().to_rfc3339(),
            book_title: None,
            game_system: None,
            game_system_id: None,
            content_category: None,
            section_title: None,
            mechanic_type: None,
            semantic_keywords: Vec::new(),
            // v2 enhanced metadata
            element_type: None,
            section_path: None,
            section_depth: None,
            parent_sections: Vec::new(),
            cross_refs: Vec::new(),
            content_mode: None,
            dice_expressions: Vec::new(),
            classification_confidence: None,
            embedding_content: None,
        }
    }

    /// Generate a deterministic ID for a chunk
    pub fn make_id(slug: &str, chunk_index: u32) -> String {
        format!("{}-c{:04}", slug, chunk_index)
    }

    /// Set TTRPG metadata from extracted metadata
    pub fn with_ttrpg_metadata(mut self, metadata: &TTRPGMetadata) -> Self {
        self.book_title = metadata.book_title.clone();
        self.game_system = metadata.game_system.clone();
        self.game_system_id = metadata.game_system_id.clone();
        self.content_category = metadata.content_category.clone();
        self
    }

    /// Apply enhanced classification and metadata extraction to this chunk.
    ///
    /// This uses the v2 semantic chunking modules to enrich the chunk with:
    /// - Element type classification (stat block, table, spell, etc.)
    /// - Content mode (crunch/fluff/mixed)
    /// - Cross-references (page, chapter, section links)
    /// - Dice expressions
    /// - Context-injected embedding content
    ///
    /// Note: Creates new classifiers for each call. For batch processing,
    /// prefer `with_classification_context()` which accepts a shared context.
    pub fn with_enhanced_classification(self) -> Self {
        let ctx = ClassificationContext::new();
        self.with_classification_context(&ctx)
    }

    /// Apply enhanced classification using a shared context.
    ///
    /// This is more efficient than `with_enhanced_classification()` when
    /// processing multiple chunks, as the classifiers are created once
    /// and reused across all chunks.
    pub fn with_classification_context(mut self, ctx: &ClassificationContext) -> Self {
        let result = ctx.classify_content(&self.content, self.page_start);

        self.element_type = Some(result.element_type);
        self.classification_confidence = Some(result.classification_confidence);
        self.content_mode = Some(result.content_mode);
        self.cross_refs = result.cross_refs;
        self.dice_expressions = result.dice_expressions;

        // Generate context-injected embedding content
        self.embedding_content = Some(self.generate_embedding_content());

        self
    }

    /// Set section hierarchy information
    pub fn with_section_hierarchy(
        mut self,
        path: Option<String>,
        depth: Option<u32>,
        parents: Vec<String>,
    ) -> Self {
        self.section_path = path;
        self.section_depth = depth;
        self.parent_sections = parents;
        self
    }

    /// Generate context-injected content for better embeddings.
    ///
    /// Prepends structured context (section path, element type, game system)
    /// to the chunk content to improve semantic search relevance.
    pub fn generate_embedding_content(&self) -> String {
        let mut context_parts = Vec::new();

        // Add section path if available
        if let Some(ref path) = self.section_path {
            context_parts.push(format!("[Section: {}]", path));
        } else if let Some(ref title) = self.section_title {
            context_parts.push(format!("[Section: {}]", title));
        }

        // Add element type
        if let Some(ref elem_type) = self.element_type {
            context_parts.push(format!("[Type: {}]", elem_type));
        }

        // Add game system
        if let Some(ref system) = self.game_system {
            context_parts.push(format!("[System: {}]", system));
        }

        // Add content mode if not generic
        if let Some(ref mode) = self.content_mode {
            if mode != "mixed" {
                context_parts.push(format!("[Mode: {}]", mode));
            }
        }

        // Combine context with content
        if context_parts.is_empty() {
            self.content.clone()
        } else {
            format!("{} {}", context_parts.join(" "), self.content)
        }
    }
}

// ============================================================================
// TTRPG Metadata
// ============================================================================

/// TTRPG-specific metadata extracted from documents
#[derive(Debug, Clone, Default)]
pub struct TTRPGMetadata {
    /// Human-readable book title (derived from filename)
    pub book_title: Option<String>,
    /// Detected game system display name
    pub game_system: Option<String>,
    /// Detected game system machine ID
    pub game_system_id: Option<String>,
    /// Content category (rulebook, adventure, setting, supplement, bestiary)
    pub content_category: Option<String>,
    /// Genre/theme
    pub genre: Option<String>,
    /// Publisher (if detected)
    pub publisher: Option<String>,
}

impl TTRPGMetadata {
    /// Extract TTRPG metadata from file path and content
    pub fn extract(path: &Path, content: &str, source_type: &str) -> Self {
        use crate::ingestion::ttrpg::game_detector::detect_game_system_with_confidence;

        let mut metadata = Self::default();

        // Extract book title from filename
        if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
            metadata.book_title = Some(Self::clean_book_title(filename));
        }

        // Detect game system from content
        let detection = detect_game_system_with_confidence(content);
        if detection.confidence >= 0.3 {
            metadata.game_system = Some(detection.system.display_name().to_string());
            metadata.game_system_id = Some(detection.system.as_str().to_string());
            metadata.genre = Some(detection.system.genre().to_string());
        }

        // Determine content category from source_type or detection
        metadata.content_category = Self::detect_content_category(source_type, content);

        // Try to detect publisher from content
        metadata.publisher = Self::detect_publisher(content);

        metadata
    }

    /// Clean up a filename to produce a readable book title
    fn clean_book_title(filename: &str) -> String {
        let cleaned = filename
            // Remove common prefixes/suffixes
            .trim_start_matches(|c: char| c.is_ascii_digit() || c == '_' || c == '-' || c == '.')
            // Replace underscores and hyphens with spaces
            .replace(['_', '-'], " ")
            // Remove multiple spaces
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        // Title case the result
        cleaned
            .split(' ')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Detect content category from source_type and content analysis
    fn detect_content_category(source_type: &str, content: &str) -> Option<String> {
        let content_lower = content.to_lowercase();

        // First check source_type
        match source_type {
            "rules" | "rulebook" => return Some("rulebook".to_string()),
            "fiction" | "story" => return Some("fiction".to_string()),
            "adventure" | "module" => return Some("adventure".to_string()),
            "setting" | "worldbook" => return Some("setting".to_string()),
            "bestiary" | "monster" => return Some("bestiary".to_string()),
            _ => {}
        }

        // Content-based detection
        let bestiary_indicators = [
            "stat block",
            "hit points",
            "armor class",
            "challenge rating",
            "creature",
            "monster manual",
        ];
        let adventure_indicators = [
            "adventure",
            "module",
            "campaign",
            "scenario",
            "encounter",
            "dungeon",
        ];
        let setting_indicators = [
            "setting",
            "world",
            "history",
            "geography",
            "faction",
            "timeline",
        ];
        let rulebook_indicators = [
            "rules",
            "character creation",
            "combat",
            "skills",
            "spellcasting",
            "gameplay",
        ];

        let bestiary_score: usize = bestiary_indicators
            .iter()
            .filter(|i| content_lower.contains(*i))
            .count();
        let adventure_score: usize = adventure_indicators
            .iter()
            .filter(|i| content_lower.contains(*i))
            .count();
        let setting_score: usize = setting_indicators
            .iter()
            .filter(|i| content_lower.contains(*i))
            .count();
        let rulebook_score: usize = rulebook_indicators
            .iter()
            .filter(|i| content_lower.contains(*i))
            .count();

        let max_score = bestiary_score
            .max(adventure_score)
            .max(setting_score)
            .max(rulebook_score);

        if max_score >= 2 {
            if bestiary_score == max_score {
                return Some("bestiary".to_string());
            } else if adventure_score == max_score {
                return Some("adventure".to_string());
            } else if setting_score == max_score {
                return Some("setting".to_string());
            } else if rulebook_score == max_score {
                return Some("rulebook".to_string());
            }
        }

        // Default based on source_type
        match source_type {
            "document" | "pdf" => Some("supplement".to_string()),
            _ => None,
        }
    }

    /// Detect publisher from content (basic pattern matching)
    fn detect_publisher(content: &str) -> Option<String> {
        let content_lower = content.to_lowercase();

        // Publisher patterns (lowercase for matching)
        let publishers = [
            ("wizards of the coast", "Wizards of the Coast"),
            ("wotc", "Wizards of the Coast"),
            ("paizo", "Paizo Inc."),
            ("chaosium", "Chaosium Inc."),
            ("arc dream", "Arc Dream Publishing"),
            ("evil hat", "Evil Hat Productions"),
            ("free league", "Free League Publishing"),
            ("fria ligan", "Free League Publishing"),
            ("monte cook games", "Monte Cook Games"),
            ("modiphius", "Modiphius Entertainment"),
            ("pinnacle", "Pinnacle Entertainment"),
            ("steve jackson games", "Steve Jackson Games"),
            ("mongoose publishing", "Mongoose Publishing"),
            ("white wolf", "White Wolf Publishing"),
            ("onyx path", "Onyx Path Publishing"),
            ("fantasy flight", "Fantasy Flight Games"),
            ("tuesday knight", "Tuesday Knight Games"),
            ("kobold press", "Kobold Press"),
        ];

        for (pattern, publisher) in publishers {
            if content_lower.contains(pattern) {
                return Some(publisher.to_string());
            }
        }

        None
    }
}

// ============================================================================
// Configuration
// ============================================================================

/// Chunking configuration
#[derive(Debug, Clone)]
pub struct PipelineChunkConfig {
    /// Target chunk size in characters
    pub chunk_size: usize,
    /// Overlap between chunks in characters
    pub chunk_overlap: usize,
    /// Minimum chunk size (don't create tiny chunks)
    pub min_chunk_size: usize,
}

impl Default for PipelineChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1000,
            chunk_overlap: 200,
            min_chunk_size: 100,
        }
    }
}

/// Pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Chunking settings
    pub chunk_config: PipelineChunkConfig,
    /// Default source type if not specified
    pub default_source_type: String,
    /// Extraction settings (provider selection, OCR, etc.)
    pub extraction_settings: crate::ingestion::ExtractionSettings,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            chunk_config: PipelineChunkConfig::default(),
            default_source_type: "document".to_string(),
            extraction_settings: crate::ingestion::ExtractionSettings::default(),
        }
    }
}

impl PipelineConfig {
    /// Create a config with Claude extraction provider
    pub fn with_claude_extraction() -> Self {
        Self {
            chunk_config: PipelineChunkConfig::default(),
            default_source_type: "document".to_string(),
            extraction_settings: crate::ingestion::ExtractionSettings::with_claude(),
        }
    }
}

// ============================================================================
// Pipeline Results
// ============================================================================

/// Result of extracting raw pages (phase 1 of two-phase pipeline)
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// Generated slug for this source
    pub slug: String,
    /// Human-readable source name
    pub source_name: String,
    /// Index where raw pages were stored
    pub raw_index: String,
    /// Number of pages extracted
    pub page_count: usize,
    /// Total characters extracted
    pub total_chars: usize,
    /// Detected TTRPG metadata
    pub ttrpg_metadata: TTRPGMetadata,
}

/// Result of chunking from raw pages (phase 2 of two-phase pipeline)
#[derive(Debug, Clone)]
pub struct ChunkingResult {
    /// Source slug
    pub slug: String,
    /// Index where chunks were stored
    pub chunks_index: String,
    /// Number of chunks created
    pub chunk_count: usize,
    /// Number of raw pages consumed
    pub pages_consumed: usize,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Raw Document Tests
    // ========================================================================

    #[test]
    fn test_raw_document_id_generation() {
        assert_eq!(RawDocument::make_id("delta-green", 1), "delta-green-p0001");
        assert_eq!(RawDocument::make_id("delta-green", 42), "delta-green-p0042");
        assert_eq!(
            RawDocument::make_id("delta-green", 999),
            "delta-green-p0999"
        );
    }

    #[test]
    fn test_raw_document_creation() {
        let content = "Chapter 1: Character Creation";
        let doc = RawDocument::new("phb", 5, content.to_string());

        assert_eq!(doc.id, "phb-p0005");
        assert_eq!(doc.source_slug, "phb");
        assert_eq!(doc.page_number, 5);
        assert_eq!(doc.raw_content, content);
        assert_eq!(doc.page_metadata.char_count, content.len());
        assert_eq!(doc.page_metadata.word_count, 4);
    }

    #[test]
    fn test_page_metadata_detection() {
        // Test table detection
        let table_content = "Name | HP | AC\nGoblin | 7 | 15\nOrc | 15 | 13";
        let meta = PageMetadata::from_content(table_content);
        assert!(meta.has_tables);

        // Test image detection
        let image_content = "See Figure 1.2 for details";
        let meta = PageMetadata::from_content(image_content);
        assert!(meta.has_images);

        // Test header detection
        let header_content = "Chapter 3: Combat\n\nThis section describes...";
        let meta = PageMetadata::from_content(header_content);
        assert_eq!(meta.page_header, Some("Chapter 3: Combat".to_string()));
    }

    // ========================================================================
    // Chunked Document Tests
    // ========================================================================

    #[test]
    fn test_chunked_document_id_generation() {
        assert_eq!(
            ChunkedDocument::make_id("delta-green", 0),
            "delta-green-c0000"
        );
        assert_eq!(
            ChunkedDocument::make_id("delta-green", 42),
            "delta-green-c0042"
        );
    }

    #[test]
    fn test_chunked_document_page_range() {
        let source_ids = vec![
            "delta-green-p0005".to_string(),
            "delta-green-p0006".to_string(),
            "delta-green-p0007".to_string(),
        ];

        let chunk = ChunkedDocument::new(
            "delta-green",
            0,
            "Some content spanning multiple pages".to_string(),
            source_ids,
        );

        assert_eq!(chunk.page_start, 5);
        assert_eq!(chunk.page_end, 7);
        assert_eq!(chunk.source_raw_ids.len(), 3);
    }

    #[test]
    fn test_chunked_document_single_page() {
        let source_ids = vec!["phb-p0042".to_string()];

        let chunk = ChunkedDocument::new("phb", 10, "Content from single page".to_string(), source_ids);

        assert_eq!(chunk.page_start, 42);
        assert_eq!(chunk.page_end, 42);
        assert_eq!(chunk.id, "phb-c0010");
    }

    #[test]
    fn test_chunked_document_with_metadata() {
        let metadata = TTRPGMetadata {
            book_title: Some("Player's Handbook".to_string()),
            game_system: Some("D&D 5th Edition".to_string()),
            game_system_id: Some("dnd5e".to_string()),
            content_category: Some("rulebook".to_string()),
            ..Default::default()
        };

        let chunk = ChunkedDocument::new(
            "phb",
            0,
            "Content".to_string(),
            vec!["phb-p0001".to_string()],
        )
        .with_ttrpg_metadata(&metadata);

        assert_eq!(chunk.book_title, Some("Player's Handbook".to_string()));
        assert_eq!(chunk.game_system, Some("D&D 5th Edition".to_string()));
        assert_eq!(chunk.game_system_id, Some("dnd5e".to_string()));
        assert_eq!(chunk.content_category, Some("rulebook".to_string()));
    }
}
