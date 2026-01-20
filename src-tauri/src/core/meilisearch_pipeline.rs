//! Meilisearch Ingestion Pipeline
//!
//! Handles document parsing, chunking, and indexing into Meilisearch.
//! Supports multiple extraction providers: kreuzberg (local) or Claude API.
//! Includes TTRPG-specific metadata extraction for semantic search.

use crate::core::search_client::{SearchClient, SearchDocument, SearchError, LibraryDocumentMetadata};
use crate::ingestion::kreuzberg_extractor::DocumentExtractor;
use crate::ingestion::claude_extractor::ClaudeDocumentExtractor;
use crate::ingestion::extraction_settings::{ExtractionSettings, TextExtractionProvider};
use crate::ingestion::ttrpg::game_detector::detect_game_system_with_confidence;
use crate::ingestion::ttrpg::{
    detect_mechanic_type, extract_semantic_keywords,
    // v2 enhanced modules
    TTRPGClassifier,
    CrossReferenceExtractor,
    ContentModeClassifier,
    DiceExtractor,
};
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

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
        let cross_refs: Vec<String> = self.cross_ref_extractor
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
// Source Slug Generation
// ============================================================================

/// Maximum length for generated slugs (Meilisearch index name limit is 400)
const MAX_SLUG_LENGTH: usize = 64;

/// Generate a deterministic, filesystem-safe slug from a file path.
///
/// The slug is used as the base name for Meilisearch indexes:
/// - `<slug>-raw` for raw page-level documents
/// - `<slug>` for semantic chunks
///
/// # Rules
/// - Lowercase alphanumeric characters and hyphens only
/// - No consecutive hyphens
/// - No leading/trailing hyphens
/// - Deterministic: same input always produces same output
/// - Truncated to MAX_SLUG_LENGTH characters
///
/// # Examples
/// ```
/// use std::path::Path;
/// use ttrpg_assistant::core::meilisearch_pipeline::generate_source_slug;
/// let slug = generate_source_slug(Path::new("Delta Green - Handler's Guide.pdf"), None);
/// assert_eq!(slug, "delta-green-handlers-guide");
/// ```
pub fn generate_source_slug(path: &Path, title_override: Option<&str>) -> String {
    // Use title override if provided, otherwise extract from filename
    let base_name = title_override
        .map(|s| s.to_string())
        .or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unnamed".to_string());

    slugify(&base_name)
}

/// Convert any string to a clean slug.
///
/// Handles unicode by attempting transliteration of common characters,
/// then falling back to stripping non-ASCII.
pub fn slugify(input: &str) -> String {
    let mut slug = String::with_capacity(input.len());
    let mut last_was_hyphen = true; // Start true to avoid leading hyphen

    for c in input.chars() {
        match c {
            // Direct passthrough for lowercase alphanumeric
            'a'..='z' | '0'..='9' => {
                slug.push(c);
                last_was_hyphen = false;
            }
            // Convert uppercase to lowercase
            'A'..='Z' => {
                slug.push(c.to_ascii_lowercase());
                last_was_hyphen = false;
            }
            // Common separators become hyphens
            ' ' | '-' | '_' | '.' | '/' | '\\' | ':' | ',' | ';' | '(' | ')' | '[' | ']' => {
                if !last_was_hyphen {
                    slug.push('-');
                    last_was_hyphen = true;
                }
            }
            // Transliterate common unicode characters (lowercase and uppercase)
            'á' | 'à' | 'ä' | 'â' | 'ã' | 'å' | 'Á' | 'À' | 'Ä' | 'Â' | 'Ã' | 'Å' => {
                slug.push('a');
                last_was_hyphen = false;
            }
            'é' | 'è' | 'ë' | 'ê' | 'É' | 'È' | 'Ë' | 'Ê' => {
                slug.push('e');
                last_was_hyphen = false;
            }
            'í' | 'ì' | 'ï' | 'î' | 'Í' | 'Ì' | 'Ï' | 'Î' => {
                slug.push('i');
                last_was_hyphen = false;
            }
            'ó' | 'ò' | 'ö' | 'ô' | 'õ' | 'ø' | 'Ó' | 'Ò' | 'Ö' | 'Ô' | 'Õ' | 'Ø' => {
                slug.push('o');
                last_was_hyphen = false;
            }
            'ú' | 'ù' | 'ü' | 'û' | 'Ú' | 'Ù' | 'Ü' | 'Û' => {
                slug.push('u');
                last_was_hyphen = false;
            }
            'ñ' | 'Ñ' => {
                slug.push('n');
                last_was_hyphen = false;
            }
            'ç' | 'Ç' => {
                slug.push('c');
                last_was_hyphen = false;
            }
            'ß' => {
                slug.push_str("ss");
                last_was_hyphen = false;
            }
            'æ' | 'Æ' => {
                slug.push_str("ae");
                last_was_hyphen = false;
            }
            'œ' | 'Œ' => {
                slug.push_str("oe");
                last_was_hyphen = false;
            }
            'þ' | 'Þ' => {
                slug.push_str("th");
                last_was_hyphen = false;
            }
            'ð' | 'Ð' => {
                slug.push('d');
                last_was_hyphen = false;
            }
            // Apostrophes and quotes are dropped (don't become hyphens)
            // ASCII quotes
            '\'' | '"' | '`' => {}
            // Unicode curly quotes (using escape sequences)
            '\u{2018}' | '\u{2019}' | '\u{201C}' | '\u{201D}' => {}
            // Skip other characters
            _ => {}
        }
    }

    // Remove trailing hyphen
    while slug.ends_with('-') {
        slug.pop();
    }

    // Truncate to max length at word boundary if possible
    if slug.len() > MAX_SLUG_LENGTH {
        // Find last hyphen before limit
        if let Some(pos) = slug[..MAX_SLUG_LENGTH].rfind('-') {
            slug.truncate(pos);
        } else {
            slug.truncate(MAX_SLUG_LENGTH);
        }
    }

    // Fallback for empty slugs
    if slug.is_empty() {
        slug = "unnamed".to_string();
    }

    slug
}

/// Generate the raw index name for a source slug
pub fn raw_index_name(slug: &str) -> String {
    format!("{}-raw", slug)
}

/// Generate the chunks index name for a source slug (same as slug)
pub fn chunks_index_name(slug: &str) -> String {
    slug.to_string()
}

// ============================================================================
// Raw Document (Page-Level Storage)
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
        let has_tables = content.contains('|') && content.lines().filter(|l| l.contains('|')).count() >= 2
            || content_lower.contains("table ")
            || content.lines().any(|l| l.chars().filter(|c| *c == '\t').count() >= 3);

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
            .filter_map(|id| {
                id.rsplit_once("-p")
                    .and_then(|(_, num)| num.parse().ok())
            })
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
            .replace('_', " ")
            .replace('-', " ")
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
        let bestiary_indicators = ["stat block", "hit points", "armor class", "challenge rating", "creature", "monster manual"];
        let adventure_indicators = ["adventure", "module", "campaign", "scenario", "encounter", "dungeon"];
        let setting_indicators = ["setting", "world", "history", "geography", "faction", "timeline"];
        let rulebook_indicators = ["rules", "character creation", "combat", "skills", "spellcasting", "gameplay"];

        let bestiary_score: usize = bestiary_indicators.iter().filter(|i| content_lower.contains(*i)).count();
        let adventure_score: usize = adventure_indicators.iter().filter(|i| content_lower.contains(*i)).count();
        let setting_score: usize = setting_indicators.iter().filter(|i| content_lower.contains(*i)).count();
        let rulebook_score: usize = rulebook_indicators.iter().filter(|i| content_lower.contains(*i)).count();

        let max_score = bestiary_score.max(adventure_score).max(setting_score).max(rulebook_score);

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
pub struct ChunkConfig {
    /// Target chunk size in characters
    pub chunk_size: usize,
    /// Overlap between chunks in characters
    pub chunk_overlap: usize,
    /// Minimum chunk size (don't create tiny chunks)
    pub min_chunk_size: usize,
}

impl Default for ChunkConfig {
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
    pub chunk_config: ChunkConfig,
    /// Default source type if not specified
    pub default_source_type: String,
    /// Extraction settings (provider selection, OCR, etc.)
    pub extraction_settings: ExtractionSettings,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            chunk_config: ChunkConfig::default(),
            default_source_type: "document".to_string(),
            extraction_settings: ExtractionSettings::default(),
        }
    }
}

impl PipelineConfig {
    /// Create a config with Claude extraction provider
    pub fn with_claude_extraction() -> Self {
        Self {
            chunk_config: ChunkConfig::default(),
            default_source_type: "document".to_string(),
            extraction_settings: ExtractionSettings::with_claude(),
        }
    }
}

// ============================================================================
// Pipeline Result
// ============================================================================

/// Result of processing a document (legacy single-phase)
#[derive(Debug, Clone)]
pub struct IngestionResult {
    pub source: String,
    pub total_chunks: usize,
    pub stored_chunks: usize,
    pub failed_chunks: usize,
    pub index_used: String,
}

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
// Meilisearch Pipeline
// ============================================================================

pub struct MeilisearchPipeline {
    config: PipelineConfig,
}

impl MeilisearchPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(PipelineConfig::default())
    }

    // ========================================================================
    // Two-Phase Pipeline: Extract → Raw → Chunk
    // ========================================================================

    /// Phase 1: Extract document content and store raw pages in `<slug>-raw` index.
    ///
    /// This creates a per-document index with one document per page, preserving
    /// the original page structure for provenance tracking.
    ///
    /// **Incremental/Resumable**: For OCR-based extraction, pages are written to
    /// Meilisearch as they are processed. If interrupted, the next run will
    /// resume from the last successfully persisted page.
    ///
    /// # Arguments
    /// * `search_client` - Meilisearch client for indexing
    /// * `path` - Path to the document file
    /// * `title_override` - Optional custom title (otherwise derived from filename)
    ///
    /// # Returns
    /// `ExtractionResult` with slug, page count, and detected metadata
    pub async fn extract_to_raw(
        &self,
        search_client: &SearchClient,
        path: &Path,
        title_override: Option<&str>,
    ) -> Result<ExtractionResult, SearchError> {
        // Generate deterministic slug from filename
        let slug = generate_source_slug(path, title_override);
        let raw_index = raw_index_name(&slug);
        let chunks_index = chunks_index_name(&slug);

        let source_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        log::info!(
            "Two-phase ingestion: '{}' → raw='{}', chunks='{}'",
            source_name, raw_index, chunks_index
        );

        // FAIL-FAST: Create both indexes BEFORE expensive extraction
        // This ensures we have somewhere to persist results before doing OCR
        // Also configures sortable attributes needed for incremental extraction
        log::info!("Creating raw index '{}' (if not exists)...", raw_index);
        search_client.ensure_raw_index(&raw_index).await
            .map_err(|e| SearchError::ConfigError(format!(
                "Failed to create raw index '{}': {}. Aborting before extraction.",
                raw_index, e
            )))?;

        log::info!("Creating chunks index '{}' (if not exists)...", chunks_index);
        search_client.ensure_chunks_index(&chunks_index).await
            .map_err(|e| SearchError::ConfigError(format!(
                "Failed to create chunks index '{}': {}. Aborting before extraction.",
                chunks_index, e
            )))?;

        log::info!("Indexes ready. Starting document extraction...");

        // Determine source type from file extension
        let source_type = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .unwrap_or_else(|| "unknown".to_string());

        // Write initial library_metadata entry with status="processing"
        // Uses slug as ID for deterministic upsert behavior
        let initial_metadata = LibraryDocumentMetadata {
            id: slug.clone(),
            name: source_name.clone(),
            source_type: source_type.clone(),
            file_path: Some(path.to_string_lossy().to_string()),
            page_count: 0,
            chunk_count: 0,
            character_count: 0,
            content_index: chunks_index.clone(),
            status: "processing".to_string(),
            error_message: None,
            ingested_at: Utc::now().to_rfc3339(),
            // TTRPG metadata - user-editable, not set during ingestion
            game_system: None,
            setting: None,
            content_type: None,
            publisher: None,
        };

        if let Err(e) = search_client.save_library_document(&initial_metadata).await {
            log::warn!("Failed to create initial library_metadata entry for '{}': {}", slug, e);
            // Continue anyway - this is not fatal
        } else {
            log::info!("Created library_metadata entry '{}' with status=processing", slug);
        }

        // Dispatch based on extraction provider
        match self.config.extraction_settings.text_extraction_provider {
            TextExtractionProvider::ClaudeGate => {
                // Use Claude API for extraction
                log::info!("Using Claude API for extraction of '{}'", source_name);
                return self.extract_to_raw_with_claude(
                    search_client,
                    path,
                    &slug,
                    &raw_index,
                    &chunks_index,
                    &source_name,
                    &source_type,
                ).await;
            }
            TextExtractionProvider::Kreuzberg => {
                // Continue with kreuzberg extraction below
                log::info!("Using Kreuzberg (local) for extraction of '{}'", source_name);
            }
        }

        // First, try fast extraction with kreuzberg WITHOUT OCR (to check text content)
        // Using text_check_only() to avoid triggering the expensive OCR fallback
        let extractor = DocumentExtractor::text_check_only();
        let cb: Option<fn(f32, &str)> = None;
        let extracted = extractor.extract(path, cb)
            .await
            .map_err(|e| SearchError::ConfigError(format!("Document extraction failed: {}", e)))?;

        // Check if we got meaningful text (OCR threshold is typically 5000 chars)
        let is_pdf = extracted.mime_type == "application/pdf";
        let low_text = extracted.char_count < 5000;
        let needs_ocr = is_pdf && low_text;

        if needs_ocr {
            // Use incremental OCR extraction with per-page persistence
            // This writes pages to Meilisearch as they're OCR'd, enabling resumability
            log::info!(
                "Low text ({} chars) detected - using incremental OCR for '{}'",
                extracted.char_count, source_name
            );

            return self.extract_to_raw_incremental(
                search_client,
                path,
                &slug,
                &raw_index,
                &chunks_index,
                &source_name,
                &source_type,
            ).await;
        }

        // Fast path: text extracted successfully, store all pages using helper
        self.store_extracted_content(
            search_client,
            path,
            &slug,
            &raw_index,
            &chunks_index,
            &source_name,
            &source_type,
            extracted,
        ).await
    }

    /// Incremental OCR extraction with per-page persistence for resumability.
    ///
    /// This method:
    /// 1. Queries the raw index for already-extracted pages
    /// 2. Resumes from the last page if partially complete
    /// 3. Writes each page to Meilisearch immediately after OCR
    async fn extract_to_raw_incremental(
        &self,
        search_client: &SearchClient,
        path: &Path,
        slug: &str,
        raw_index: &str,
        chunks_index: &str,
        source_name: &str,
        source_type: &str,
    ) -> Result<ExtractionResult, SearchError> {
        // Query existing pages to find where to resume
        let existing_page_count = self.get_highest_page_number(search_client, raw_index).await;
        let start_page = existing_page_count + 1;

        log::info!(
            "Incremental extraction: {} existing pages, starting from page {}",
            existing_page_count, start_page
        );

        // Get total page count
        let extractor = DocumentExtractor::with_ocr();
        let total_pages = extractor.settings().get_pdf_page_count_sync(path).unwrap_or(0);

        if total_pages == 0 {
            return Err(SearchError::ConfigError(
                "Could not determine PDF page count".to_string()
            ));
        }

        if start_page > total_pages {
            log::info!("All {} pages already extracted, skipping OCR", total_pages);

            // Still need to return metadata - fetch sample from existing pages
            let content_sample = self.get_content_sample(search_client, raw_index).await;
            let ttrpg_metadata = TTRPGMetadata::extract(path, &content_sample, "document");

            // Update library_metadata with status="ready" (already complete)
            let final_metadata = LibraryDocumentMetadata {
                id: slug.to_string(),
                name: source_name.to_string(),
                source_type: source_type.to_string(),
                file_path: Some(path.to_string_lossy().to_string()),
                page_count: total_pages as u32,
                chunk_count: 0,
                character_count: 0,
                content_index: chunks_index.to_string(),
                status: "ready".to_string(),
                error_message: None,
                ingested_at: Utc::now().to_rfc3339(),
                // TTRPG metadata - user-editable, not set during ingestion
                game_system: None,
                setting: None,
                content_type: None,
                publisher: None,
            };

            if let Err(e) = search_client.save_library_document(&final_metadata).await {
                log::warn!("Failed to update library_metadata for '{}': {}", slug, e);
            }

            return Ok(ExtractionResult {
                slug: slug.to_string(),
                source_name: source_name.to_string(),
                raw_index: raw_index.to_string(),
                page_count: total_pages,
                total_chars: 0, // We don't recalculate for resumed extractions
                ttrpg_metadata,
            });
        }

        log::info!(
            "OCR extraction: pages {}-{} of {} for '{}'",
            start_page, total_pages, total_pages, source_name
        );

        // Auto-detect concurrency based on available CPU parallelism
        // Override with TTRPG_OCR_CONCURRENCY env var if needed
        let cpu_count = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4);
        let concurrency = std::env::var("TTRPG_OCR_CONCURRENCY")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or_else(|| {
                // Default: cpu_count / 2, clamped to 2-8
                (cpu_count / 2).max(2).min(8)
            });

        log::info!(
            "OCR concurrency: {} (available parallelism: {}, set TTRPG_OCR_CONCURRENCY to override)",
            concurrency, cpu_count
        );

        let mut total_chars_extracted = 0usize;
        let mut pages_written = 0usize;
        let mut current_page = start_page;
        let index = search_client.get_client().index(raw_index);

        // Process wave by wave with immediate persistence
        while current_page <= total_pages {
            let (wave_pages, next_page) = extractor.extract_one_wave(
                path,
                current_page,
                total_pages,
                concurrency,
            ).await.map_err(|e| SearchError::ConfigError(format!("OCR wave failed: {}", e)))?;

            if wave_pages.is_empty() {
                break;
            }

            // Convert to RawDocuments and write immediately
            let raw_docs: Vec<RawDocument> = wave_pages
                .into_iter()
                .map(|(page_num, content)| {
                    total_chars_extracted += content.len();
                    RawDocument::new(slug, page_num as u32, content)
                })
                .collect();

            let doc_count = raw_docs.len();

            match index.add_documents(&raw_docs, Some("id")).await {
                Ok(task) => {
                    let _ = task.wait_for_completion(
                        &index.client,
                        Some(std::time::Duration::from_millis(100)),
                        Some(std::time::Duration::from_secs(60)),
                    ).await;
                    pages_written += doc_count;
                    log::info!("Wave complete: indexed {} pages (total: {}/{})",
                        doc_count, pages_written + existing_page_count, total_pages);
                }
                Err(e) => {
                    log::error!("Failed to index wave: {}", e);
                    // Continue to next wave - partial progress is still saved
                }
            }

            current_page = next_page;
        }

        // Build result
        let result: Result<usize, crate::ingestion::kreuzberg_extractor::ExtractionError> = Ok(pages_written);

        match result {
            Ok(pages_processed) => {
                let final_page_count = existing_page_count + pages_processed;
                log::info!(
                    "Incremental OCR complete: {} new pages extracted, {} total in index",
                    pages_processed, final_page_count
                );

                // Get content sample for metadata detection
                let content_sample = self.get_content_sample(search_client, raw_index).await;
                let ttrpg_metadata = TTRPGMetadata::extract(path, &content_sample, "document");

                // Update library_metadata with final stats and status="ready"
                let final_metadata = LibraryDocumentMetadata {
                    id: slug.to_string(),
                    name: source_name.to_string(),
                    source_type: source_type.to_string(),
                    file_path: Some(path.to_string_lossy().to_string()),
                    page_count: final_page_count as u32,
                    chunk_count: 0, // Will be updated after chunking phase
                    character_count: total_chars_extracted as u64,
                    content_index: chunks_index.to_string(),
                    status: "ready".to_string(),
                    error_message: None,
                    ingested_at: Utc::now().to_rfc3339(),
                    // TTRPG metadata - user-editable, not set during ingestion
                    game_system: None,
                    setting: None,
                    content_type: None,
                    publisher: None,
                };

                if let Err(e) = search_client.save_library_document(&final_metadata).await {
                    log::warn!("Failed to update library_metadata for '{}': {}", slug, e);
                } else {
                    log::info!("Updated library_metadata '{}' with status=ready", slug);
                }

                Ok(ExtractionResult {
                    slug: slug.to_string(),
                    source_name: source_name.to_string(),
                    raw_index: raw_index.to_string(),
                    page_count: final_page_count,
                    total_chars: total_chars_extracted,
                    ttrpg_metadata,
                })
            }
            Err(e) => {
                // Update library_metadata with status="error"
                let error_metadata = LibraryDocumentMetadata {
                    id: slug.to_string(),
                    name: source_name.to_string(),
                    source_type: source_type.to_string(),
                    file_path: Some(path.to_string_lossy().to_string()),
                    page_count: (existing_page_count + pages_written) as u32,
                    chunk_count: 0,
                    character_count: total_chars_extracted as u64,
                    content_index: chunks_index.to_string(),
                    status: "error".to_string(),
                    error_message: Some(e.to_string()),
                    ingested_at: Utc::now().to_rfc3339(),
                    // TTRPG metadata - user-editable, not set during ingestion
                    game_system: None,
                    setting: None,
                    content_type: None,
                    publisher: None,
                };

                let _ = search_client.save_library_document(&error_metadata).await;

                log::error!("Incremental extraction failed: {}. {} pages may have been saved.", e, pages_written);
                Err(SearchError::ConfigError(format!("Incremental extraction failed: {}", e)))
            }
        }
    }

    /// Extract document using Claude API.
    ///
    /// This uses Claude's vision capabilities for high-quality text extraction,
    /// especially useful for:
    /// - Scanned documents
    /// - Complex layouts (multi-column, mixed text/images)
    /// - Documents with handwritten annotations
    async fn extract_to_raw_with_claude(
        &self,
        search_client: &SearchClient,
        path: &Path,
        slug: &str,
        raw_index: &str,
        chunks_index: &str,
        source_name: &str,
        source_type: &str,
    ) -> Result<ExtractionResult, SearchError> {
        // Check if Claude extraction supports this format
        if !ClaudeDocumentExtractor::<crate::claude_gate::FileTokenStorage>::is_supported(path) {
            log::warn!(
                "Claude extraction does not support '{}', falling back to kreuzberg",
                source_type
            );
            // Fall back to kreuzberg for unsupported formats
            let extractor = DocumentExtractor::with_ocr();
            let cb: Option<fn(f32, &str)> = None;
            let extracted = extractor.extract(path, cb)
                .await
                .map_err(|e| SearchError::ConfigError(format!("Document extraction failed: {}", e)))?;

            return self.store_extracted_content(
                search_client,
                path,
                slug,
                raw_index,
                chunks_index,
                source_name,
                source_type,
                extracted,
            ).await;
        }

        // Create Claude extractor
        let claude_extractor = ClaudeDocumentExtractor::new()
            .map_err(|e| SearchError::ConfigError(format!("Failed to create Claude extractor: {}", e)))?;

        // Check authentication
        let is_authenticated = claude_extractor.is_authenticated().await
            .map_err(|e| SearchError::ConfigError(format!("Claude auth check failed: {}", e)))?;

        if !is_authenticated {
            log::warn!("Claude API not authenticated, falling back to kreuzberg extraction");
            let extractor = DocumentExtractor::with_ocr();
            let cb: Option<fn(f32, &str)> = None;
            let extracted = extractor.extract(path, cb)
                .await
                .map_err(|e| SearchError::ConfigError(format!("Document extraction failed: {}", e)))?;

            return self.store_extracted_content(
                search_client,
                path,
                slug,
                raw_index,
                chunks_index,
                source_name,
                source_type,
                extracted,
            ).await;
        }

        log::info!("Extracting '{}' using Claude API...", source_name);

        // Perform Claude extraction
        let cb: Option<fn(f32, &str)> = None;
        let extracted = claude_extractor.extract(path, cb)
            .await
            .map_err(|e| SearchError::ConfigError(format!("Claude extraction failed: {}", e)))?;

        self.store_extracted_content(
            search_client,
            path,
            slug,
            raw_index,
            chunks_index,
            source_name,
            source_type,
            extracted,
        ).await
    }

    /// Store extracted content into the raw index.
    ///
    /// Shared helper used by both kreuzberg and Claude extraction paths.
    async fn store_extracted_content(
        &self,
        search_client: &SearchClient,
        path: &Path,
        slug: &str,
        raw_index: &str,
        chunks_index: &str,
        source_name: &str,
        source_type: &str,
        extracted: crate::ingestion::kreuzberg_extractor::ExtractedContent,
    ) -> Result<ExtractionResult, SearchError> {
        let total_chars = extracted.char_count;
        let mut page_count = 0;
        let mut raw_documents = Vec::new();

        // Convert extracted pages to RawDocuments
        if let Some(pages) = extracted.pages {
            for page in pages {
                let raw_doc = RawDocument::new(slug, page.page_number as u32, page.content);
                raw_documents.push(raw_doc);
                page_count += 1;
            }
        } else {
            // Single page fallback (no page structure)
            let raw_doc = RawDocument::new(slug, 1, extracted.content.clone());
            raw_documents.push(raw_doc);
            page_count = 1;
        }

        // Combine content sample for metadata detection
        let content_sample: String = raw_documents
            .iter()
            .take(20)
            .map(|d| d.raw_content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // Detect TTRPG metadata
        let ttrpg_metadata = TTRPGMetadata::extract(path, &content_sample, "document");

        log::info!(
            "Extracted {} pages from '{}': system={:?}, category={:?}",
            page_count,
            source_name,
            ttrpg_metadata.game_system,
            ttrpg_metadata.content_category
        );

        // Store raw documents in Meilisearch
        let index = search_client.get_client().index(raw_index);
        let task = index.add_documents(&raw_documents, Some("id")).await
            .map_err(|e| SearchError::MeilisearchError(format!("Failed to add raw documents: {}", e)))?;

        // Wait for indexing to complete
        task.wait_for_completion(
            search_client.get_client(),
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(60)),
        ).await
            .map_err(|e| SearchError::MeilisearchError(format!("Raw indexing failed: {}", e)))?;

        log::info!("Stored {} raw pages in '{}'", page_count, raw_index);

        // Update library_metadata with final stats and status="ready"
        let final_metadata = LibraryDocumentMetadata {
            id: slug.to_string(),
            name: source_name.to_string(),
            source_type: source_type.to_string(),
            file_path: Some(path.to_string_lossy().to_string()),
            page_count: page_count as u32,
            chunk_count: 0, // Will be updated after chunking phase
            character_count: total_chars as u64,
            content_index: chunks_index.to_string(),
            status: "ready".to_string(),
            error_message: None,
            ingested_at: Utc::now().to_rfc3339(),
// TTRPG metadata - user-editable, not set during ingestion
            game_system: None,
            setting: None,
            content_type: None,
            publisher: None,
        };

        if let Err(e) = search_client.save_library_document(&final_metadata).await {
            log::warn!("Failed to update library_metadata for '{}': {}", slug, e);
        } else {
            log::info!("Updated library_metadata '{}' with status=ready", slug);
        }

        Ok(ExtractionResult {
            slug: slug.to_string(),
            source_name: source_name.to_string(),
            raw_index: raw_index.to_string(),
            page_count,
            total_chars,
            ttrpg_metadata,
        })
    }

    /// Query the raw index to find the highest page number already extracted.
    /// Returns 0 if no pages exist.
    async fn get_highest_page_number(&self, search_client: &SearchClient, raw_index: &str) -> usize {
        let index = search_client.get_client().index(raw_index);

        // Fetch documents sorted by page_number descending, limit 1
        // Meilisearch doesn't have a direct "max" query, so we fetch with sort
        let result: Result<meilisearch_sdk::search::SearchResults<RawDocument>, _> = index
            .search()
            .with_query("*")
            .with_sort(&["page_number:desc"])
            .with_limit(1)
            .execute()
            .await;

        match result {
            Ok(results) => {
                if let Some(hit) = results.hits.first() {
                    hit.result.page_number as usize
                } else {
                    0
                }
            }
            Err(e) => {
                log::warn!("Could not query existing pages from '{}': {}", raw_index, e);
                0
            }
        }
    }

    /// Get a content sample from the raw index for metadata detection
    async fn get_content_sample(&self, search_client: &SearchClient, raw_index: &str) -> String {
        let index = search_client.get_client().index(raw_index);

        let result: Result<meilisearch_sdk::search::SearchResults<RawDocument>, _> = index
            .search()
            .with_query("*")
            .with_sort(&["page_number:asc"])
            .with_limit(20)
            .execute()
            .await;

        match result {
            Ok(results) => {
                results.hits
                    .iter()
                    .map(|h| h.result.raw_content.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            }
            Err(_) => String::new()
        }
    }

    /// Phase 2: Create semantic chunks from raw pages and store in `<slug>` index.
    ///
    /// Reads from the `<slug>-raw` index, applies semantic chunking that may span
    /// multiple pages, and stores chunks with provenance tracking (source_raw_ids).
    ///
    /// # Arguments
    /// * `search_client` - Meilisearch client
    /// * `extraction` - Result from `extract_to_raw()`
    ///
    /// # Returns
    /// `ChunkingResult` with chunk count and pages consumed
    pub async fn chunk_from_raw(
        &self,
        search_client: &SearchClient,
        extraction: &ExtractionResult,
    ) -> Result<ChunkingResult, SearchError> {
        let slug = &extraction.slug;
        let raw_index = &extraction.raw_index;
        let chunks_index = chunks_index_name(slug);

        log::info!("Chunking from '{}' to '{}'", raw_index, chunks_index);

        // Ensure chunks index exists with proper settings
        search_client.ensure_chunks_index(&chunks_index).await
            .map_err(|e| SearchError::ConfigError(format!("Failed to create chunks index: {}", e)))?;

        // Fetch all raw documents from the raw index
        let index = search_client.get_client().index(raw_index);
        let raw_docs: Vec<RawDocument> = index
            .get_documents()
            .await
            .map_err(|e| SearchError::MeilisearchError(format!("Failed to fetch raw docs: {}", e)))?
            .results;

        if raw_docs.is_empty() {
            return Err(SearchError::DocumentNotFound(format!(
                "No raw documents found in '{}'", raw_index
            )));
        }

        let pages_consumed = raw_docs.len();

        // Sort by page number
        let mut sorted_docs = raw_docs;
        sorted_docs.sort_by_key(|d| d.page_number);

        // Create chunks with provenance tracking
        let chunks = self.create_chunks_with_provenance(slug, &sorted_docs, &extraction.ttrpg_metadata);

        let chunk_count = chunks.len();

        // Store chunks in Meilisearch
        let chunks_idx = search_client.get_client().index(&chunks_index);
        let task = chunks_idx.add_documents(&chunks, Some("id")).await
            .map_err(|e| SearchError::MeilisearchError(format!("Failed to add chunks: {}", e)))?;

        task.wait_for_completion(
            search_client.get_client(),
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(60)),
        ).await
            .map_err(|e| SearchError::MeilisearchError(format!("Chunk indexing failed: {}", e)))?;

        log::info!(
            "Created {} chunks from {} pages in '{}'",
            chunk_count, pages_consumed, chunks_index
        );

        Ok(ChunkingResult {
            slug: slug.clone(),
            chunks_index,
            chunk_count,
            pages_consumed,
        })
    }

    /// Combined two-phase ingestion: extract + chunk in one call.
    ///
    /// Convenience method that runs both phases sequentially.
    pub async fn ingest_two_phase(
        &self,
        search_client: &SearchClient,
        path: &Path,
        title_override: Option<&str>,
    ) -> Result<(ExtractionResult, ChunkingResult), SearchError> {
        let extraction = self.extract_to_raw(search_client, path, title_override).await?;
        let chunking = self.chunk_from_raw(search_client, &extraction).await?;
        Ok((extraction, chunking))
    }

    /// Create semantic chunks from raw documents with provenance tracking.
    ///
    /// Each chunk records which raw document IDs it was derived from,
    /// enabling page number attribution in search results.
    fn create_chunks_with_provenance(
        &self,
        slug: &str,
        raw_docs: &[RawDocument],
        metadata: &TTRPGMetadata,
    ) -> Vec<ChunkedDocument> {
        let config = &self.config.chunk_config;
        let mut chunks = Vec::new();
        let mut chunk_index = 0u32;

        let mut current_content = String::new();
        let mut current_source_ids: Vec<String> = Vec::new();

        for doc in raw_docs {
            let doc_content = doc.raw_content.trim();
            if doc_content.is_empty() {
                continue;
            }

            // Check if adding this page would exceed max chunk size
            let would_exceed = current_content.len() + doc_content.len() + 1 > config.chunk_size * 2;

            // If we have content and would exceed, save current chunk
            if !current_content.is_empty() && would_exceed {
                // Create chunk from accumulated content with v2 enhanced metadata
                let chunk = ChunkedDocument::new(
                    slug,
                    chunk_index,
                    std::mem::take(&mut current_content),
                    std::mem::take(&mut current_source_ids),
                )
                .with_ttrpg_metadata(metadata)
                .with_enhanced_classification();

                chunks.push(chunk);
                chunk_index += 1;
            }

            // Add page to current accumulation
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(doc_content);
            current_source_ids.push(doc.id.clone());

            // If single page exceeds target, split it into smaller chunks
            while current_content.len() > config.chunk_size {
                // Find a good split point (sentence boundary, paragraph, or forced)
                let split_at = self.find_split_point(&current_content, config.chunk_size);

                let chunk_content = current_content[..split_at].to_string();
                let chunk = ChunkedDocument::new(
                    slug,
                    chunk_index,
                    chunk_content,
                    current_source_ids.clone(), // Same source IDs for split chunks
                )
                .with_ttrpg_metadata(metadata)
                .with_enhanced_classification();

                chunks.push(chunk);
                chunk_index += 1;

                // Keep overlap for continuity
                let overlap_start = split_at.saturating_sub(config.chunk_overlap);
                current_content = current_content[overlap_start..].to_string();
            }
        }

        // Don't forget the last chunk
        if current_content.len() >= config.min_chunk_size {
            let chunk = ChunkedDocument::new(
                slug,
                chunk_index,
                current_content,
                current_source_ids,
            )
            .with_ttrpg_metadata(metadata)
            .with_enhanced_classification();
            chunks.push(chunk);
        }

        chunks
    }

    /// Find a good split point in text, preferring sentence/paragraph boundaries.
    fn find_split_point(&self, text: &str, target: usize) -> usize {
        let search_range = text.get(..target.min(text.len())).unwrap_or(text);

        // Prefer paragraph break
        if let Some(pos) = search_range.rfind("\n\n") {
            if pos > target / 2 {
                return pos + 2;
            }
        }

        // Then sentence boundary
        for pattern in [". ", "! ", "? ", ".\n", "!\n", "?\n"] {
            if let Some(pos) = search_range.rfind(pattern) {
                if pos > target / 2 {
                    return pos + pattern.len();
                }
            }
        }

        // Fallback to word boundary
        if let Some(pos) = search_range.rfind(' ') {
            return pos + 1;
        }

        // Last resort: hard cut
        target.min(text.len())
    }

    // ========================================================================
    // Legacy Single-Phase Pipeline
    // ========================================================================

    /// Process a file and ingest into Meilisearch
    ///
    /// Extracts content, chunks it, detects TTRPG metadata (game system, publisher, etc.),
    /// and indexes with rich semantic information for embedding.
    pub async fn process_file(
        &self,
        search_client: &SearchClient,
        path: &Path,
        source_type: &str,
        campaign_id: Option<&str>,
    ) -> Result<IngestionResult, SearchError> {
        let source_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Extract content and chunks
        let (chunks, full_content) = if DocumentExtractor::is_supported(path) {
            // Use kreuzberg for all supported formats
            let chunks = self.process_with_kreuzberg(path, &source_name).await?;
            // Combine chunks for metadata detection (use first ~10000 chars for efficiency)
            let combined: String = chunks.iter()
                .take(20)
                .map(|(c, _)| c.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            (chunks, combined)
        } else {
            // Fallback for unsupported formats (treat as plain text)
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    let chunks = self.chunk_text(&content, &source_name, None);
                    let sample = content.chars().take(10000).collect::<String>();
                    (chunks, sample)
                }
                Err(e) => {
                    return Err(SearchError::ConfigError(format!(
                        "Cannot read file or unsupported format: {}", e
                    )));
                }
            }
        };

        // Extract TTRPG-specific metadata from path and content
        let ttrpg_metadata = TTRPGMetadata::extract(path, &full_content, source_type);

        log::info!(
            "TTRPG metadata for '{}': system={:?}, category={:?}, publisher={:?}",
            source_name,
            ttrpg_metadata.game_system,
            ttrpg_metadata.content_category,
            ttrpg_metadata.publisher
        );

        // Determine target index
        let index_name = SearchClient::select_index_for_source_type(source_type);

        // Build SearchDocuments with rich TTRPG metadata for semantic embedding
        let now = Utc::now().to_rfc3339();

        // Create shared classification context once for all chunks
        let classification_ctx = ClassificationContext::new();

        let documents: Vec<SearchDocument> = chunks
            .into_iter()
            .enumerate()
            .map(|(i, (content, page))| {
                // Extract mechanic type and semantic keywords from chunk content
                let mechanic_type = detect_mechanic_type(&content).map(|s| s.to_string());
                let semantic_keywords = extract_semantic_keywords(&content, 10);

                // Classify content using shared context (avoids creating classifiers per chunk)
                let classification = classification_ctx.classify_content(&content, page.unwrap_or(1));

                SearchDocument {
                    id: format!("{}-{}", Uuid::new_v4(), i),
                    content: content.clone(),
                    source: source_name.clone(),
                    source_type: source_type.to_string(),
                    page_number: page,
                    chunk_index: Some(i as u32),
                    campaign_id: campaign_id.map(|s| s.to_string()),
                    session_id: None,
                    created_at: now.clone(),
                    metadata: HashMap::new(),
                    // TTRPG-specific embedding metadata
                    book_title: ttrpg_metadata.book_title.clone(),
                    game_system: ttrpg_metadata.game_system.clone(),
                    game_system_id: ttrpg_metadata.game_system_id.clone(),
                    content_category: ttrpg_metadata.content_category.clone(),
                    section_title: None, // TODO: Extract from TOC/section headers
                    genre: ttrpg_metadata.genre.clone(),
                    publisher: ttrpg_metadata.publisher.clone(),
                    // Enhanced metadata (from MDMAI patterns)
                    chunk_type: Some(classification.element_type.clone()),
                    chapter_title: None, // Would be populated by TTRPGChunker with hierarchy
                    subsection_title: None, // Would be populated by TTRPGChunker with hierarchy
                    section_path: None, // Would be populated by TTRPGChunker with hierarchy
                    mechanic_type,
                    semantic_keywords,
                    // v2 enhanced TTRPG metadata
                    element_type: Some(classification.element_type),
                    section_depth: None,
                    parent_sections: Vec::new(),
                    cross_refs: classification.cross_refs,
                    content_mode: Some(classification.content_mode),
                    dice_expressions: classification.dice_expressions,
                    classification_confidence: Some(classification.classification_confidence),
                    embedding_content: None, // Generated on-demand if needed
                }
            })
            .collect();

        let total_chunks = documents.len();

        // Ingest into Meilisearch
        search_client.add_documents(index_name, documents).await?;

        log::info!(
            "Ingested {} chunks from '{}' into index '{}' [{}]",
            total_chunks,
            source_name,
            index_name,
            ttrpg_metadata.game_system.as_deref().unwrap_or("unknown system")
        );

        Ok(IngestionResult {
            source: source_name,
            total_chunks,
            stored_chunks: total_chunks,
            failed_chunks: 0,
            index_used: index_name.to_string(),
        })
    }

    /// Process any document using kreuzberg (PDF, DOCX, EPUB, MOBI, images, etc.)
    ///
    /// Uses kreuzberg's async extraction for non-blocking processing.
    async fn process_with_kreuzberg(&self, path: &Path, source_name: &str) -> Result<Vec<(String, Option<u32>)>, SearchError> {
        // Use kreuzberg with OCR fallback for scanned documents
        let extractor = DocumentExtractor::with_ocr();

        // This is now async and uses proper page config
        let cb: Option<fn(f32, &str)> = None;
        let extracted = extractor.extract(path, cb)
            .await
            .map_err(|e| SearchError::ConfigError(format!("Document extraction failed: {}", e)))?;

        let total_text_len = extracted.char_count;
        let page_count = extracted.page_count;

        log::info!(
            "Extracted content from '{}': {} pages, {} chars",
            source_name, page_count, total_text_len
        );

        let mut all_chunks = Vec::new();

        // If we have distinct pages, chunk them individually to preserve page numbers
        if let Some(pages) = extracted.pages {
            for page in pages {
                // Calculate adaptive chunk size per page if needed, but for now global config is fine
                // or we could compute density per page.

                // For simplicity, use standard chunking on page content.
                // We could use the density logic here too if we cared about mixed-density docs.

                let page_len = page.content.len();
                let adaptive_min_chunk_size = if page_len < 100 { 10 } else { self.config.chunk_config.min_chunk_size };

                let page_chunks = self.chunk_text_adaptive(
                    &page.content,
                    source_name,
                    Some(page.page_number as u32),
                    adaptive_min_chunk_size
                );

                all_chunks.extend(page_chunks);
            }
        } else {
            // Fallback for formats without pages or if extraction failed to split pages
             // Calculate adaptive min_chunk_size based on overall extraction quality
            let avg_chars_per_page = if page_count > 0 {
                total_text_len / page_count
            } else {
                0
            };

            let adaptive_min_chunk_size = if avg_chars_per_page >= 2000 {
                self.config.chunk_config.min_chunk_size
            } else if avg_chars_per_page >= 500 {
                50.min(self.config.chunk_config.min_chunk_size)
            } else if avg_chars_per_page >= 100 {
                20.min(self.config.chunk_config.min_chunk_size)
            } else {
                10
            };

            all_chunks = self.chunk_text_adaptive(
                &extracted.content,
                source_name,
                None,
                adaptive_min_chunk_size,
            );
        }

        // Warn if no content extracted
        if all_chunks.is_empty() && total_text_len > 0 {
            log::warn!(
                "Document '{}' extracted {} chars but produced 0 chunks",
                source_name,
                total_text_len
            );
        }

        Ok(all_chunks)
    }

    /// Process a text file
    fn process_text_file(&self, path: &Path, source_name: &str) -> Result<Vec<(String, Option<u32>)>, SearchError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SearchError::ConfigError(format!("Failed to read file: {}", e)))?;

        Ok(self.chunk_text(&content, source_name, None))
    }



    /// Chunk text content with overlap
    fn chunk_text(&self, text: &str, _source: &str, page_number: Option<u32>) -> Vec<(String, Option<u32>)> {
        self.chunk_text_adaptive(text, _source, page_number, self.config.chunk_config.min_chunk_size)
    }

    /// Chunk text content with overlap and custom min_chunk_size
    fn chunk_text_adaptive(
        &self,
        text: &str,
        _source: &str,
        page_number: Option<u32>,
        min_chunk_size: usize,
    ) -> Vec<(String, Option<u32>)> {
        let config = &self.config.chunk_config;
        let mut chunks = Vec::new();
        let text = text.trim();

        if text.is_empty() {
            return chunks;
        }

        // If text is smaller than chunk size, return as single chunk (even if below min)
        if text.len() <= config.chunk_size {
            // For adaptive mode, accept smaller chunks
            if text.len() >= min_chunk_size || min_chunk_size <= 10 {
                chunks.push((text.to_string(), page_number));
            }
            return chunks;
        }

        // Split into sentences for smarter chunking
        let sentences: Vec<&str> = text
            .split(|c| c == '.' || c == '!' || c == '?')
            .filter(|s| !s.trim().is_empty())
            .collect();

        let mut current_chunk = String::new();

        for sentence in sentences {
            let sentence = sentence.trim();
            let potential_len = current_chunk.len() + sentence.len() + 2; // +2 for ". "

            if potential_len > config.chunk_size && !current_chunk.is_empty() {
                // Save current chunk
                if current_chunk.len() >= min_chunk_size {
                    chunks.push((current_chunk.clone(), page_number));
                }

                // Start new chunk with overlap
                let overlap_start = current_chunk
                    .char_indices()
                    .rev()
                    .take(config.chunk_overlap)
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);

                current_chunk = current_chunk[overlap_start..].to_string();
            }

            if !current_chunk.is_empty() {
                current_chunk.push_str(". ");
            }
            current_chunk.push_str(sentence);
        }

        // Don't forget the last chunk
        if current_chunk.len() >= min_chunk_size {
            chunks.push((current_chunk, page_number));
        }

        chunks
    }

    /// Ingest raw text content directly
    pub async fn ingest_text(
        &self,
        search_client: &SearchClient,
        content: &str,
        source: &str,
        source_type: &str,
        campaign_id: Option<&str>,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<IngestionResult, SearchError> {
        let chunks = self.chunk_text(content, source, None);
        let index_name = SearchClient::select_index_for_source_type(source_type);
        let now = Utc::now().to_rfc3339();

        let documents: Vec<SearchDocument> = chunks
            .into_iter()
            .enumerate()
            .map(|(i, (text, _))| SearchDocument {
                id: format!("{}-{}", Uuid::new_v4(), i),
                content: text,
                source: source.to_string(),
                source_type: source_type.to_string(),
                page_number: None,
                chunk_index: Some(i as u32),
                campaign_id: campaign_id.map(|s| s.to_string()),
                session_id: None,
                created_at: now.clone(),
                metadata: metadata.clone().unwrap_or_default(),
                ..Default::default()
            })
            .collect();

        let total_chunks = documents.len();
        search_client.add_documents(index_name, documents).await?;

        Ok(IngestionResult {
            source: source.to_string(),
            total_chunks,
            stored_chunks: total_chunks,
            failed_chunks: 0,
            index_used: index_name.to_string(),
        })
    }

    /// Ingest chat messages into the chat index
    pub async fn ingest_chat_messages(
        &self,
        search_client: &SearchClient,
        messages: Vec<(String, String)>, // (role, content)
        session_id: &str,
        campaign_id: Option<&str>,
    ) -> Result<IngestionResult, SearchError> {
        let now = Utc::now().to_rfc3339();

        let documents: Vec<SearchDocument> = messages
            .into_iter()
            .enumerate()
            .map(|(i, (role, content))| {
                let mut metadata = HashMap::new();
                metadata.insert("role".to_string(), role);

                SearchDocument {
                    id: format!("{}-{}", session_id, i),
                    content,
                    source: format!("session-{}", session_id),
                    source_type: "chat".to_string(),
                    page_number: None,
                    chunk_index: Some(i as u32),
                    campaign_id: campaign_id.map(|s| s.to_string()),
                    session_id: Some(session_id.to_string()),
                    created_at: now.clone(),
                    metadata,
                    ..Default::default()
                }
            })
            .collect();

        let total = documents.len();
        search_client.add_documents("chat", documents).await?;

        Ok(IngestionResult {
            source: format!("session-{}", session_id),
            total_chunks: total,
            stored_chunks: total,
            failed_chunks: 0,
            index_used: "chat".to_string(),
        })
    }
}

impl Default for MeilisearchPipeline {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Slug Generation Tests
    // ========================================================================

    #[test]
    fn test_slugify_basic() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("simple"), "simple");
        assert_eq!(slugify("UPPERCASE"), "uppercase");
    }

    #[test]
    fn test_slugify_special_characters() {
        assert_eq!(slugify("Delta Green - Handler's Guide"), "delta-green-handlers-guide");
        assert_eq!(slugify("D&D 5th Edition"), "dd-5th-edition");
        assert_eq!(slugify("Call of Cthulhu (7th Ed)"), "call-of-cthulhu-7th-ed");
        assert_eq!(slugify("Monster Manual: Expanded"), "monster-manual-expanded");
    }

    #[test]
    fn test_slugify_unicode() {
        assert_eq!(slugify("Über"), "uber");
        assert_eq!(slugify("naïve"), "naive");
        assert_eq!(slugify("café"), "cafe");
        assert_eq!(slugify("Müller"), "muller");
        assert_eq!(slugify("Ægis"), "aegis");
        assert_eq!(slugify("Þórr"), "thorr");
    }

    #[test]
    fn test_slugify_consecutive_separators() {
        assert_eq!(slugify("hello   world"), "hello-world");
        assert_eq!(slugify("hello---world"), "hello-world");
        assert_eq!(slugify("hello___world"), "hello-world");
        assert_eq!(slugify("hello - world"), "hello-world");
    }

    #[test]
    fn test_slugify_leading_trailing() {
        assert_eq!(slugify("  hello  "), "hello");
        assert_eq!(slugify("--hello--"), "hello");
        assert_eq!(slugify("123_test"), "123-test");
    }

    #[test]
    fn test_slugify_empty_and_special() {
        assert_eq!(slugify(""), "unnamed");
        assert_eq!(slugify("!!!"), "unnamed");
        assert_eq!(slugify("'\""), "unnamed");
    }

    #[test]
    fn test_slugify_long_input() {
        let long_input = "This Is A Very Long Title That Exceeds The Maximum Slug Length And Should Be Truncated At A Word Boundary";
        let slug = slugify(long_input);
        assert!(slug.len() <= MAX_SLUG_LENGTH);
        assert!(!slug.ends_with('-'));
    }

    #[test]
    fn test_slugify_numbers() {
        assert_eq!(slugify("5th Edition"), "5th-edition");
        assert_eq!(slugify("2024"), "2024");
        assert_eq!(slugify("D&D 3.5"), "dd-3-5");
    }

    #[test]
    fn test_generate_source_slug_from_path() {
        use std::path::Path;

        let path = Path::new("/home/user/rpg/Delta Green - Handler's Guide.pdf");
        assert_eq!(generate_source_slug(path, None), "delta-green-handlers-guide");

        let path = Path::new("Monster_Manual_5e.pdf");
        assert_eq!(generate_source_slug(path, None), "monster-manual-5e");
    }

    #[test]
    fn test_generate_source_slug_with_override() {
        use std::path::Path;

        let path = Path::new("file123.pdf");
        assert_eq!(
            generate_source_slug(path, Some("Player's Handbook")),
            "players-handbook"
        );
    }

    #[test]
    fn test_index_name_helpers() {
        assert_eq!(raw_index_name("delta-green"), "delta-green-raw");
        assert_eq!(chunks_index_name("delta-green"), "delta-green");
    }

    #[test]
    fn test_slugify_deterministic() {
        // Same input should always produce same output
        let input = "Delta Green: Handler's Guide (2nd Printing)";
        let slug1 = slugify(input);
        let slug2 = slugify(input);
        let slug3 = slugify(input);
        assert_eq!(slug1, slug2);
        assert_eq!(slug2, slug3);
    }

    // ========================================================================
    // Chunking Tests
    // ========================================================================

    #[test]
    fn test_chunk_text_small() {
        // Use small min_chunk_size to accommodate test text
        let pipeline = MeilisearchPipeline::new(PipelineConfig {
            chunk_config: ChunkConfig {
                chunk_size: 1000,
                chunk_overlap: 200,
                min_chunk_size: 5,  // Allow very small chunks for testing
            },
            ..Default::default()
        });
        let chunks = pipeline.chunk_text("Small text.", "test", None);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].0, "Small text.");
    }

    #[test]
    fn test_chunk_text_with_sentences() {
        let pipeline = MeilisearchPipeline::new(PipelineConfig {
            chunk_config: ChunkConfig {
                chunk_size: 50,
                chunk_overlap: 10,
                min_chunk_size: 10,
            },
            ..Default::default()
        });

        let text = "First sentence. Second sentence. Third sentence. Fourth sentence.";
        let chunks = pipeline.chunk_text(text, "test", Some(1));

        assert!(chunks.len() > 1);
        for (_, page) in &chunks {
            assert_eq!(*page, Some(1));
        }
    }

    // ========================================================================
    // Raw Document Tests
    // ========================================================================

    #[test]
    fn test_raw_document_id_generation() {
        assert_eq!(RawDocument::make_id("delta-green", 1), "delta-green-p0001");
        assert_eq!(RawDocument::make_id("delta-green", 42), "delta-green-p0042");
        assert_eq!(RawDocument::make_id("delta-green", 999), "delta-green-p0999");
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
        assert_eq!(doc.page_metadata.word_count, 4); // "Chapter", "1:", "Character", "Creation"
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
        assert_eq!(ChunkedDocument::make_id("delta-green", 0), "delta-green-c0000");
        assert_eq!(ChunkedDocument::make_id("delta-green", 42), "delta-green-c0042");
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

        let chunk = ChunkedDocument::new(
            "phb",
            10,
            "Content from single page".to_string(),
            source_ids,
        );

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
        ).with_ttrpg_metadata(&metadata);

        assert_eq!(chunk.book_title, Some("Player's Handbook".to_string()));
        assert_eq!(chunk.game_system, Some("D&D 5th Edition".to_string()));
        assert_eq!(chunk.game_system_id, Some("dnd5e".to_string()));
        assert_eq!(chunk.content_category, Some("rulebook".to_string()));
    }
}
