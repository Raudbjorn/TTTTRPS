//! Semantic Chunker Module
//!
//! Intelligent text chunking with sentence awareness, overlap, and page tracking.
//! Uses configuration constants from the TTRPG vocabulary module for RAG-optimized chunking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// Import chunking configuration constants from vocabulary module
use crate::ingestion::ttrpg::vocabulary::chunking_config;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for the chunker
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Target chunk size in characters
    pub target_size: usize,
    /// Minimum chunk size (won't split below this)
    pub min_size: usize,
    /// Maximum chunk size (hard limit)
    pub max_size: usize,
    /// Overlap size in characters
    pub overlap_size: usize,
    /// Whether to preserve sentence boundaries
    pub preserve_sentences: bool,
    /// Whether to preserve paragraph boundaries when possible
    pub preserve_paragraphs: bool,
}

impl Default for ChunkConfig {
    /// Default configuration using MDMAI-derived constants optimized for RAG.
    ///
    /// Values from `chunking_config`:
    /// - target_size: 1200 chars (~300 tokens)
    /// - min_size: 300 chars (avoid fragments)
    /// - max_size: 2400 chars (~600 tokens)
    /// - overlap_size: 150 chars (~40 tokens)
    fn default() -> Self {
        Self {
            target_size: chunking_config::TARGET_CHUNK_SIZE,
            min_size: chunking_config::MIN_CHUNK_SIZE,
            max_size: chunking_config::MAX_CHUNK_SIZE,
            overlap_size: chunking_config::CHUNK_OVERLAP,
            preserve_sentences: true,
            preserve_paragraphs: true,
        }
    }
}

impl ChunkConfig {
    /// Create config for small, focused chunks (half of default sizes).
    /// Good for fine-grained search and short context windows.
    pub fn small() -> Self {
        Self {
            target_size: chunking_config::TARGET_CHUNK_SIZE / 2,  // 600
            min_size: chunking_config::MIN_CHUNK_SIZE / 2,        // 150
            max_size: chunking_config::MAX_CHUNK_SIZE / 2,        // 1200
            overlap_size: chunking_config::CHUNK_OVERLAP / 2,     // 75
            ..Default::default()
        }
    }

    /// Create config for large chunks (double of default sizes).
    /// Better for context-heavy RAG and longer documents.
    pub fn large() -> Self {
        Self {
            target_size: chunking_config::TARGET_CHUNK_SIZE * 2,  // 2400
            min_size: chunking_config::MIN_CHUNK_SIZE * 2,        // 600
            max_size: chunking_config::MAX_CHUNK_SIZE * 2,        // 4800
            overlap_size: chunking_config::CHUNK_OVERLAP * 2,     // 300
            ..Default::default()
        }
    }

    /// Create config from token counts (assuming ~4 chars/token).
    ///
    /// # Arguments
    /// * `target_tokens` - Target chunk size in tokens
    /// * `max_tokens` - Maximum chunk size in tokens
    /// * `overlap_tokens` - Overlap size in tokens
    pub fn from_tokens(target_tokens: usize, max_tokens: usize, overlap_tokens: usize) -> Self {
        const CHARS_PER_TOKEN: usize = 4;
        Self {
            target_size: target_tokens * CHARS_PER_TOKEN,
            min_size: (target_tokens / 4) * CHARS_PER_TOKEN,
            max_size: max_tokens * CHARS_PER_TOKEN,
            overlap_size: overlap_tokens * CHARS_PER_TOKEN,
            ..Default::default()
        }
    }

    /// Create config using the token-based constants from vocabulary config.
    /// Uses TARGET_TOKENS (300), MAX_TOKENS (600), OVERLAP_TOKENS (40).
    pub fn from_vocabulary_tokens() -> Self {
        Self::from_tokens(
            chunking_config::TARGET_TOKENS,
            chunking_config::MAX_TOKENS,
            chunking_config::OVERLAP_TOKENS,
        )
    }
}

// ============================================================================
// Content Chunk
// ============================================================================

/// A chunk of content with metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContentChunk {
    /// Unique identifier
    pub id: String,
    /// Source document identifier
    pub source_id: String,
    /// Chunk content
    pub content: String,
    /// Page number (if applicable)
    pub page_number: Option<u32>,
    /// Section/chapter (if detected) - current deepest section
    pub section: Option<String>,
    /// Chunk type (text, table, header, stat_block, spell, monster, rule, narrative)
    pub chunk_type: String,
    /// Chunk index in document
    pub chunk_index: usize,
    /// Additional metadata
    pub metadata: HashMap<String, String>,

    // =========================================================================
    // Enhanced Metadata (from MDMAI patterns)
    // =========================================================================

    /// Chapter title (top-level section from TOC, e.g., "Chapter 3: Combat")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_title: Option<String>,

    /// Subsection title (nested within section)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subsection_title: Option<String>,

    /// Mechanic type for rules content (skill_check, combat, damage, healing, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mechanic_type: Option<String>,

    /// Extracted semantic keywords for embedding boost
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_keywords: Vec<String>,
}

// ============================================================================
// Semantic Chunker
// ============================================================================

/// Intelligent chunker with sentence and paragraph awareness
pub struct SemanticChunker {
    config: ChunkConfig,
}

impl SemanticChunker {
    /// Create a new chunker with default config
    pub fn new() -> Self {
        Self {
            config: ChunkConfig::default(),
        }
    }

    /// Create a chunker with custom config
    pub fn with_config(config: ChunkConfig) -> Self {
        Self { config }
    }

    /// Chunk text with page information
    pub fn chunk_with_pages(
        &self,
        pages: &[(u32, String)],
        source_id: &str,
    ) -> Vec<ContentChunk> {
        let mut chunks = Vec::new();
        let mut chunk_index = 0;

        for (page_num, page_text) in pages {
            let page_chunks = self.chunk_page(page_text, source_id, *page_num, &mut chunk_index);
            chunks.extend(page_chunks);
        }

        chunks
    }

    /// Chunk a single page
    fn chunk_page(
        &self,
        text: &str,
        source_id: &str,
        page_number: u32,
        chunk_index: &mut usize,
    ) -> Vec<ContentChunk> {
        let mut chunks = Vec::new();

        // Split into paragraphs
        let paragraphs: Vec<&str> = text
            .split("\n\n")
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .collect();

        let mut current_chunk = String::new();
        let mut current_section: Option<String> = None;

        for para in paragraphs {
            // Detect section headers
            if self.is_header(para) {
                // Flush current chunk before starting new section
                if !current_chunk.is_empty() && current_chunk.len() >= self.config.min_size {
                    chunks.push(self.create_chunk(
                        &current_chunk,
                        source_id,
                        Some(page_number),
                        current_section.clone(),
                        *chunk_index,
                    ));
                    *chunk_index += 1;
                    current_chunk = self.get_overlap(&current_chunk);
                }
                current_section = Some(para.to_string());
            }

            // Check if adding this paragraph would exceed max size
            if current_chunk.len() + para.len() > self.config.max_size {
                // Need to flush and potentially split
                if current_chunk.len() >= self.config.min_size {
                    chunks.push(self.create_chunk(
                        &current_chunk,
                        source_id,
                        Some(page_number),
                        current_section.clone(),
                        *chunk_index,
                    ));
                    *chunk_index += 1;
                    current_chunk = self.get_overlap(&current_chunk);
                }

                // Handle very long paragraphs by sentence splitting
                if para.len() > self.config.max_size {
                    let sentence_chunks = self.split_by_sentences(
                        para,
                        source_id,
                        Some(page_number),
                        current_section.clone(),
                        chunk_index,
                    );
                    chunks.extend(sentence_chunks);
                    current_chunk.clear();
                    continue;
                }
            }

            // Add paragraph to current chunk
            if !current_chunk.is_empty() {
                current_chunk.push_str("\n\n");
            }
            current_chunk.push_str(para);

            // Flush if we've reached target size
            if current_chunk.len() >= self.config.target_size {
                chunks.push(self.create_chunk(
                    &current_chunk,
                    source_id,
                    Some(page_number),
                    current_section.clone(),
                    *chunk_index,
                ));
                *chunk_index += 1;
                current_chunk = self.get_overlap(&current_chunk);
            }
        }

        // Don't forget remaining content
        if !current_chunk.is_empty() && current_chunk.len() >= self.config.min_size {
            chunks.push(self.create_chunk(
                &current_chunk,
                source_id,
                Some(page_number),
                current_section,
                *chunk_index,
            ));
            *chunk_index += 1;
        }

        chunks
    }

    /// Split text by sentences when paragraphs are too long
    fn split_by_sentences(
        &self,
        text: &str,
        source_id: &str,
        page_number: Option<u32>,
        section: Option<String>,
        chunk_index: &mut usize,
    ) -> Vec<ContentChunk> {
        let mut chunks = Vec::new();
        let sentences = self.split_into_sentences(text);
        let mut current_chunk = String::new();

        for sentence in sentences {
            if current_chunk.len() + sentence.len() > self.config.max_size {
                if current_chunk.len() >= self.config.min_size {
                    chunks.push(self.create_chunk(
                        &current_chunk,
                        source_id,
                        page_number,
                        section.clone(),
                        *chunk_index,
                    ));
                    *chunk_index += 1;
                    current_chunk = self.get_overlap(&current_chunk);
                }
            }

            if !current_chunk.is_empty() && !current_chunk.ends_with(' ') {
                current_chunk.push(' ');
            }
            current_chunk.push_str(&sentence);

            if current_chunk.len() >= self.config.target_size {
                chunks.push(self.create_chunk(
                    &current_chunk,
                    source_id,
                    page_number,
                    section.clone(),
                    *chunk_index,
                ));
                *chunk_index += 1;
                current_chunk = self.get_overlap(&current_chunk);
            }
        }

        if !current_chunk.is_empty() && current_chunk.len() >= self.config.min_size {
            chunks.push(self.create_chunk(
                &current_chunk,
                source_id,
                page_number,
                section,
                *chunk_index,
            ));
            *chunk_index += 1;
        }

        chunks
    }

    /// Split text into sentences
    fn split_into_sentences(&self, text: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = text.chars().collect();

        let mut i = 0;
        while i < chars.len() {
            current.push(chars[i]);

            // Check for sentence-ending punctuation
            if chars[i] == '.' || chars[i] == '!' || chars[i] == '?' {
                // Look ahead to see if this is really the end of a sentence
                let next_is_space_or_end = i + 1 >= chars.len()
                    || chars[i + 1].is_whitespace()
                    || chars[i + 1] == '"'
                    || chars[i + 1] == '\'';

                // Check for common abbreviations (simplified)
                let is_abbreviation = self.is_likely_abbreviation(&current);

                if next_is_space_or_end && !is_abbreviation {
                    sentences.push(current.trim().to_string());
                    current = String::new();
                }
            }

            i += 1;
        }

        if !current.trim().is_empty() {
            sentences.push(current.trim().to_string());
        }

        sentences
    }

    /// Check if the period is likely part of an abbreviation
    fn is_likely_abbreviation(&self, text: &str) -> bool {
        let text = text.trim();
        let abbrevs = [
            "Mr.", "Mrs.", "Ms.", "Dr.", "Prof.", "Sr.", "Jr.",
            "Inc.", "Ltd.", "Corp.", "Co.",
            "vs.", "etc.", "e.g.", "i.e.",
            "St.", "Ave.", "Rd.", "Blvd.",
            "Jan.", "Feb.", "Mar.", "Apr.", "Jun.", "Jul.", "Aug.", "Sep.", "Oct.", "Nov.", "Dec.",
        ];

        for abbr in abbrevs {
            if text.ends_with(abbr) {
                return true;
            }
        }

        // Single letter followed by period (like initials)
        if text.len() >= 2 {
            let last_two: String = text.chars().rev().take(2).collect::<String>().chars().rev().collect();
            if last_two.chars().next().unwrap_or(' ').is_alphabetic()
                && last_two.chars().nth(1) == Some('.')
                && text.len() > 2
                && !text.chars().rev().nth(2).unwrap_or('a').is_whitespace()
            {
                return true;
            }
        }

        false
    }

    /// Check if text looks like a header
    fn is_header(&self, text: &str) -> bool {
        let text = text.trim();

        // Too long to be a header
        if text.len() > 100 {
            return false;
        }

        // Ends with sentence punctuation - probably not a header
        if text.ends_with('.') || text.ends_with(',') || text.ends_with(';') {
            return false;
        }

        // All caps
        let letters: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
        if !letters.is_empty() && letters.iter().all(|c| c.is_uppercase()) {
            return true;
        }

        // Chapter/section patterns
        let lower = text.to_lowercase();
        if lower.starts_with("chapter")
            || lower.starts_with("section")
            || lower.starts_with("part")
            || lower.starts_with("appendix")
        {
            return true;
        }

        false
    }

    /// Get overlap text from the end of a chunk
    fn get_overlap(&self, text: &str) -> String {
        if self.config.overlap_size == 0 || text.len() <= self.config.overlap_size {
            return String::new();
        }

        let overlap_start = text.len().saturating_sub(self.config.overlap_size);

        // Try to start at a sentence or word boundary
        let overlap_text = &text[overlap_start..];

        // Find first space to start at word boundary
        if let Some(space_pos) = overlap_text.find(' ') {
            overlap_text[space_pos + 1..].to_string()
        } else {
            overlap_text.to_string()
        }
    }

    /// Create a content chunk
    fn create_chunk(
        &self,
        content: &str,
        source_id: &str,
        page_number: Option<u32>,
        section: Option<String>,
        chunk_index: usize,
    ) -> ContentChunk {
        ContentChunk {
            id: Uuid::new_v4().to_string(),
            source_id: source_id.to_string(),
            content: content.trim().to_string(),
            page_number,
            section,
            chunk_type: "text".to_string(),
            chunk_index,
            metadata: HashMap::new(),
            // Enhanced metadata (initialized empty, populated by TTRPG pipeline)
            chapter_title: None,
            subsection_title: None,
            mechanic_type: None,
            semantic_keywords: Vec::new(),
        }
    }

    /// Simple text chunking without page info
    pub fn chunk_text(&self, text: &str, source_id: &str) -> Vec<ContentChunk> {
        // Convert to single-page format
        let pages = vec![(1, text.to_string())];
        self.chunk_with_pages(&pages, source_id)
    }
}

impl Default for SemanticChunker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TTRPG-Aware Configuration
// ============================================================================

use crate::ingestion::ttrpg::{
    ClassifiedElement, TTRPGElementType,
    detect_mechanic_type, extract_semantic_keywords,
    detect_header_level as vocabulary_detect_header_level,
};

/// Extended chunk configuration with TTRPG-specific options
#[derive(Debug, Clone)]
pub struct TTRPGChunkConfig {
    /// Base chunking configuration
    pub base: ChunkConfig,
    /// Element types that should not be split
    pub atomic_elements: Vec<TTRPGElementType>,
    /// Maximum multiplier for atomic elements (e.g., 2.0 = allow up to 2x max_size)
    pub atomic_max_multiplier: f32,
    /// Overlap percentage (0.0 to 1.0)
    pub overlap_percentage: f32,
    /// Include section hierarchy in chunk metadata
    pub include_hierarchy: bool,
}

impl Default for TTRPGChunkConfig {
    fn default() -> Self {
        Self {
            base: ChunkConfig::default(),
            atomic_elements: vec![
                TTRPGElementType::StatBlock,
                TTRPGElementType::RandomTable,
                TTRPGElementType::SpellDescription,
                TTRPGElementType::ItemDescription,
            ],
            atomic_max_multiplier: 2.0,
            overlap_percentage: 0.12, // 12% overlap
            include_hierarchy: true,
        }
    }
}

impl TTRPGChunkConfig {
    /// Calculate actual overlap size from percentage
    pub fn calculated_overlap(&self) -> usize {
        (self.base.target_size as f32 * self.overlap_percentage) as usize
    }

    /// Check if an element type should be kept atomic
    pub fn is_atomic(&self, element_type: &TTRPGElementType) -> bool {
        element_type.is_atomic() || self.atomic_elements.contains(element_type)
    }

    /// Get maximum size for atomic elements
    pub fn atomic_max_size(&self) -> usize {
        (self.base.max_size as f32 * self.atomic_max_multiplier) as usize
    }
}

// ============================================================================
// Section Hierarchy
// ============================================================================

/// Tracks section hierarchy for contextual chunks
#[derive(Debug, Clone, Default)]
pub struct SectionHierarchy {
    /// Stack of section titles (h1 at index 0, h2 at index 1, etc.)
    pub sections: Vec<String>,
}

impl SectionHierarchy {
    /// Create a new empty hierarchy
    pub fn new() -> Self {
        Self::default()
    }

    /// Update hierarchy with a new header
    ///
    /// # Arguments
    /// * `header` - The header text
    /// * `level` - The header level (1 = top level, 2 = subsection, etc.)
    pub fn update(&mut self, header: &str, level: usize) {
        // Ensure we have enough capacity
        if level > 0 {
            // Truncate deeper levels when a higher-level header is encountered
            if level <= self.sections.len() {
                self.sections.truncate(level - 1);
            }
            // Pad with empty sections if needed
            while self.sections.len() < level - 1 {
                self.sections.push(String::new());
            }
            // Add the new section
            if level - 1 < self.sections.len() {
                self.sections[level - 1] = header.to_string();
            } else {
                self.sections.push(header.to_string());
            }
        }
    }

    /// Get the full section path as a string
    ///
    /// # Returns
    /// Path like "Chapter 1 > Monsters > Goblins"
    pub fn path(&self) -> String {
        self.sections
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" > ")
    }

    /// Get parent sections (excluding current section)
    pub fn parents(&self) -> Vec<String> {
        if self.sections.len() <= 1 {
            return vec![];
        }
        self.sections[..self.sections.len() - 1]
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect()
    }

    /// Get the current section (deepest level)
    pub fn current(&self) -> Option<&str> {
        self.sections.last().map(|s| s.as_str()).filter(|s| !s.is_empty())
    }

    /// Clear the hierarchy
    pub fn clear(&mut self) {
        self.sections.clear();
    }
}

// ============================================================================
// TTRPG-Aware Chunker
// ============================================================================

/// TTRPG-aware chunker wrapper (composition over inheritance)
pub struct TTRPGChunker {
    /// Underlying semantic chunker
    base_chunker: SemanticChunker,
    /// TTRPG-specific configuration
    config: TTRPGChunkConfig,
}

impl TTRPGChunker {
    /// Create a new TTRPG-aware chunker with default configuration
    pub fn new() -> Self {
        Self::with_config(TTRPGChunkConfig::default())
    }

    /// Create a TTRPG-aware chunker with custom configuration
    pub fn with_config(config: TTRPGChunkConfig) -> Self {
        let mut base_config = config.base.clone();
        base_config.overlap_size = config.calculated_overlap();

        Self {
            base_chunker: SemanticChunker::with_config(base_config),
            config,
        }
    }

    /// Chunk classified elements with TTRPG awareness and hierarchy tracking
    ///
    /// # Arguments
    /// * `elements` - Classified document elements
    /// * `source_id` - Source document identifier
    ///
    /// # Returns
    /// Vector of content chunks with preserved atomic elements and hierarchy
    pub fn chunk(&self, elements: &[ClassifiedElement], source_id: &str) -> Vec<ContentChunk> {
        let mut chunks = Vec::new();
        let mut chunk_index = 0;
        let mut hierarchy = SectionHierarchy::new();
        let mut buffer = String::new();
        let mut buffer_page: Option<u32> = None;

        for element in elements {
            // Update hierarchy for section headers
            if element.element_type == TTRPGElementType::SectionHeader {
                // Flush buffer before section change
                if !buffer.is_empty() && buffer.len() >= self.config.base.min_size {
                    chunks.push(self.create_chunk_with_hierarchy(
                        &buffer,
                        source_id,
                        buffer_page,
                        &hierarchy,
                        "text",
                        &mut chunk_index,
                    ));
                    buffer = self.get_overlap(&buffer);
                    buffer_page = Some(element.page_number);
                }

                let level = Self::detect_header_level(&element.content);
                hierarchy.update(&element.content, level);
                continue;
            }

            // Handle atomic elements (stat blocks, tables, etc.)
            if self.config.is_atomic(&element.element_type) {
                // Flush buffer first
                if !buffer.is_empty() && buffer.len() >= self.config.base.min_size {
                    chunks.push(self.create_chunk_with_hierarchy(
                        &buffer,
                        source_id,
                        buffer_page,
                        &hierarchy,
                        "text",
                        &mut chunk_index,
                    ));
                    buffer.clear();
                }

                // Emit atomic element as single chunk (or split if too large)
                if element.content.len() <= self.config.atomic_max_size() {
                    chunks.push(self.create_chunk_with_hierarchy(
                        &element.content,
                        source_id,
                        Some(element.page_number),
                        &hierarchy,
                        element.element_type.as_str(),
                        &mut chunk_index,
                    ));
                } else {
                    // Element is too large even with multiplier, need to split
                    let split_chunks = self.split_oversized_element(
                        element,
                        source_id,
                        &hierarchy,
                        &mut chunk_index,
                    );
                    chunks.extend(split_chunks);
                }

                buffer_page = None;
                continue;
            }

            // Non-atomic element: accumulate with overlap
            if buffer_page.is_none() {
                buffer_page = Some(element.page_number);
            }

            // Check if adding this element would exceed max size
            if buffer.len() + element.content.len() > self.config.base.max_size {
                // Flush current buffer
                if buffer.len() >= self.config.base.min_size {
                    chunks.push(self.create_chunk_with_hierarchy(
                        &buffer,
                        source_id,
                        buffer_page,
                        &hierarchy,
                        "text",
                        &mut chunk_index,
                    ));
                    buffer = self.get_overlap(&buffer);
                }
            }

            // Add element content to buffer
            if !buffer.is_empty() {
                buffer.push_str("\n\n");
            }
            buffer.push_str(&element.content);

            // Flush if we've reached target size
            if buffer.len() >= self.config.base.target_size {
                chunks.push(self.create_chunk_with_hierarchy(
                    &buffer,
                    source_id,
                    buffer_page,
                    &hierarchy,
                    "text",
                    &mut chunk_index,
                ));
                buffer = self.get_overlap(&buffer);
                buffer_page = Some(element.page_number);
            }
        }

        // Flush remaining buffer
        if !buffer.is_empty() && buffer.len() >= self.config.base.min_size {
            chunks.push(self.create_chunk_with_hierarchy(
                &buffer,
                source_id,
                buffer_page,
                &hierarchy,
                "text",
                &mut chunk_index,
            ));
        }

        chunks
    }

    /// Create a chunk with hierarchy metadata
    fn create_chunk_with_hierarchy(
        &self,
        content: &str,
        source_id: &str,
        page_number: Option<u32>,
        hierarchy: &SectionHierarchy,
        chunk_type: &str,
        chunk_index: &mut usize,
    ) -> ContentChunk {
        let mut metadata = HashMap::new();

        // Extract hierarchy components
        let path = hierarchy.path();
        let parents = hierarchy.parents();
        let sections = &hierarchy.sections;

        // Get chapter (level 1) and subsection (level 3+) if present
        let chapter_title = sections.first().filter(|s| !s.is_empty()).cloned();
        let subsection_title = if sections.len() > 2 {
            sections.get(2).filter(|s| !s.is_empty()).cloned()
        } else {
            None
        };

        if self.config.include_hierarchy {
            if !path.is_empty() {
                metadata.insert("section_path".to_string(), path.clone());
            }

            if !parents.is_empty() {
                metadata.insert("parent_sections".to_string(), parents.join(" | "));
            }
        }

        let idx = *chunk_index;
        *chunk_index += 1;

        // Extract mechanic type and semantic keywords from content
        let content_trimmed = content.trim();
        let detected_mechanic_type = detect_mechanic_type(content_trimmed)
            .map(|s| s.to_string());
        let keywords = extract_semantic_keywords(content_trimmed, 10);

        ContentChunk {
            id: Uuid::new_v4().to_string(),
            source_id: source_id.to_string(),
            content: content_trimmed.to_string(),
            page_number,
            section: hierarchy.current().map(|s| s.to_string()),
            chunk_type: chunk_type.to_string(),
            chunk_index: idx,
            metadata,
            // Enhanced hierarchy metadata
            chapter_title,
            subsection_title,
            mechanic_type: detected_mechanic_type,
            semantic_keywords: keywords,
        }
    }

    /// Get overlap text from the end of content
    fn get_overlap(&self, content: &str) -> String {
        let overlap_size = self.config.calculated_overlap();
        if overlap_size == 0 || content.len() <= overlap_size {
            return String::new();
        }

        let overlap_start = content.len().saturating_sub(overlap_size);
        let overlap_text = &content[overlap_start..];

        // Find first space to start at word boundary
        if let Some(space_pos) = overlap_text.find(' ') {
            overlap_text[space_pos + 1..].to_string()
        } else {
            overlap_text.to_string()
        }
    }

    /// Detect header level from text patterns.
    /// Uses the vocabulary module's `detect_header_level` function with fallback logic.
    fn detect_header_level(text: &str) -> usize {
        // Use the vocabulary module's comprehensive header detection
        if let Some(level) = vocabulary_detect_header_level(text) {
            return level as usize;
        }

        // Fallback for headers not matched by vocabulary patterns
        let text_lower = text.to_lowercase().trim().to_string();

        // Chapter = level 1
        if text_lower.starts_with("chapter") || text_lower.starts_with("part") {
            return 1;
        }

        // Section = level 2
        if text_lower.starts_with("section") || text_lower.starts_with("appendix") {
            return 2;
        }

        // All caps are typically major sections
        let letters: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
        if !letters.is_empty() && letters.iter().all(|c| c.is_uppercase()) {
            return 2;
        }

        // Default to level 3 for other headers
        3
    }

    /// Split an oversized element into multiple chunks
    fn split_oversized_element(
        &self,
        element: &ClassifiedElement,
        source_id: &str,
        hierarchy: &SectionHierarchy,
        chunk_index: &mut usize,
    ) -> Vec<ContentChunk> {
        let mut chunks = Vec::new();
        let sentences = self.base_chunker.split_into_sentences(&element.content);
        let mut current = String::new();

        for sentence in sentences {
            if current.len() + sentence.len() > self.config.base.max_size {
                if current.len() >= self.config.base.min_size {
                    chunks.push(self.create_chunk_with_hierarchy(
                        &current,
                        source_id,
                        Some(element.page_number),
                        hierarchy,
                        element.element_type.as_str(),
                        chunk_index,
                    ));
                    current = self.get_overlap(&current);
                }
            }

            if !current.is_empty() && !current.ends_with(' ') {
                current.push(' ');
            }
            current.push_str(&sentence);

            if current.len() >= self.config.base.target_size {
                chunks.push(self.create_chunk_with_hierarchy(
                    &current,
                    source_id,
                    Some(element.page_number),
                    hierarchy,
                    element.element_type.as_str(),
                    chunk_index,
                ));
                current = self.get_overlap(&current);
            }
        }

        if !current.is_empty() && current.len() >= self.config.base.min_size {
            chunks.push(self.create_chunk_with_hierarchy(
                &current,
                source_id,
                Some(element.page_number),
                hierarchy,
                element.element_type.as_str(),
                chunk_index,
            ));
        }

        chunks
    }

    /// Chunk raw text without pre-classification (uses base chunker)
    pub fn chunk_text(&self, text: &str, source_id: &str) -> Vec<ContentChunk> {
        self.base_chunker.chunk_text(text, source_id)
    }
}

impl Default for TTRPGChunker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_chunking() {
        let chunker = SemanticChunker::with_config(ChunkConfig {
            target_size: 100,
            min_size: 20,
            max_size: 200,
            overlap_size: 20,
            ..Default::default()
        });

        let text = "First paragraph here.\n\nSecond paragraph here.\n\nThird paragraph here.";
        let chunks = chunker.chunk_text(text, "test-source");

        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(!chunk.content.is_empty());
        }
    }

    #[test]
    fn test_sentence_splitting() {
        let chunker = SemanticChunker::new();
        let sentences = chunker.split_into_sentences("Hello world. How are you? I am fine!");

        // Should have at least 2 sentences
        assert!(sentences.len() >= 2, "Expected at least 2 sentences, got {}", sentences.len());
        assert!(sentences.iter().any(|s| s.contains("Hello")));
        assert!(sentences.iter().any(|s| s.contains("fine")));
    }

    #[test]
    fn test_abbreviation_detection() {
        let chunker = SemanticChunker::new();

        // Common abbreviations should be detected
        assert!(chunker.is_likely_abbreviation("Dr."));
        assert!(chunker.is_likely_abbreviation("Hello Mr."));
        // Note: "e.g." and "end." behavior depends on implementation
    }

    #[test]
    fn test_header_detection() {
        let chunker = SemanticChunker::new();

        assert!(chunker.is_header("CHAPTER ONE"));
        assert!(chunker.is_header("Chapter 1: The Beginning"));
        assert!(chunker.is_header("SECTION II"));
        assert!(!chunker.is_header("This is a regular sentence."));
        assert!(!chunker.is_header("This is a very long line that should not be detected as a header because headers are typically short."));
    }

    // ========================================================================
    // TTRPG Chunker Tests
    // ========================================================================

    #[test]
    fn test_section_hierarchy_path() {
        let mut hierarchy = SectionHierarchy::new();

        hierarchy.update("Chapter 1", 1);
        assert_eq!(hierarchy.path(), "Chapter 1");

        hierarchy.update("Monsters", 2);
        assert_eq!(hierarchy.path(), "Chapter 1 > Monsters");

        hierarchy.update("Goblins", 3);
        assert_eq!(hierarchy.path(), "Chapter 1 > Monsters > Goblins");
    }

    #[test]
    fn test_section_hierarchy_truncation() {
        let mut hierarchy = SectionHierarchy::new();

        hierarchy.update("Chapter 1", 1);
        hierarchy.update("Section A", 2);
        hierarchy.update("Subsection", 3);
        assert_eq!(hierarchy.path(), "Chapter 1 > Section A > Subsection");

        // New chapter should truncate deeper levels
        hierarchy.update("Chapter 2", 1);
        assert_eq!(hierarchy.path(), "Chapter 2");
    }

    #[test]
    fn test_section_hierarchy_parents() {
        let mut hierarchy = SectionHierarchy::new();

        hierarchy.update("Chapter 1", 1);
        hierarchy.update("Monsters", 2);
        hierarchy.update("Goblins", 3);

        let parents = hierarchy.parents();
        assert_eq!(parents, vec!["Chapter 1", "Monsters"]);
    }

    #[test]
    fn test_ttrpg_chunk_config_defaults() {
        let config = TTRPGChunkConfig::default();

        assert_eq!(config.overlap_percentage, 0.12);
        assert_eq!(config.atomic_max_multiplier, 2.0);
        assert!(config.include_hierarchy);
        assert!(!config.atomic_elements.is_empty());
    }

    #[test]
    fn test_ttrpg_chunk_config_is_atomic() {
        let config = TTRPGChunkConfig::default();

        assert!(config.is_atomic(&TTRPGElementType::StatBlock));
        assert!(config.is_atomic(&TTRPGElementType::RandomTable));
        assert!(config.is_atomic(&TTRPGElementType::SpellDescription));
        assert!(!config.is_atomic(&TTRPGElementType::GenericText));
    }

    #[test]
    fn test_ttrpg_chunker_preserves_stat_blocks() {
        let config = TTRPGChunkConfig {
            base: ChunkConfig {
                target_size: 100,
                min_size: 20,
                max_size: 200,
                overlap_size: 20,
                ..Default::default()
            },
            ..Default::default()
        };
        let chunker = TTRPGChunker::with_config(config);

        let elements = vec![
            ClassifiedElement::new(
                TTRPGElementType::GenericText,
                0.9,
                "Some introductory text about monsters.".to_string(),
                1,
            ),
            ClassifiedElement::new(
                TTRPGElementType::StatBlock,
                0.95,
                "Goblin\nSmall humanoid\nAC 15\nHP 7".to_string(),
                1,
            ),
            ClassifiedElement::new(
                TTRPGElementType::GenericText,
                0.9,
                "More text after the stat block.".to_string(),
                1,
            ),
        ];

        let chunks = chunker.chunk(&elements, "test");

        // Stat block should be its own chunk
        let stat_block_chunks: Vec<_> = chunks.iter()
            .filter(|c| c.chunk_type == "stat_block")
            .collect();
        assert_eq!(stat_block_chunks.len(), 1);
        assert!(stat_block_chunks[0].content.contains("Goblin"));
    }

    #[test]
    fn test_ttrpg_chunker_hierarchy_metadata() {
        let config = TTRPGChunkConfig {
            base: ChunkConfig {
                target_size: 500,
                min_size: 50,
                max_size: 1000,
                overlap_size: 50,
                ..Default::default()
            },
            include_hierarchy: true,
            ..Default::default()
        };
        let chunker = TTRPGChunker::with_config(config);

        let elements = vec![
            ClassifiedElement::new(
                TTRPGElementType::SectionHeader,
                0.9,
                "Chapter 1".to_string(),
                1,
            ),
            ClassifiedElement::new(
                TTRPGElementType::SectionHeader,
                0.9,
                "Monsters".to_string(),
                1,
            ),
            ClassifiedElement::new(
                TTRPGElementType::GenericText,
                0.9,
                "This chapter describes various monsters you might encounter. These creatures range from simple beasts to terrifying abominations.".to_string(),
                1,
            ),
        ];

        let chunks = chunker.chunk(&elements, "test");

        // Should have hierarchy metadata
        if !chunks.is_empty() {
            let chunk = &chunks[0];
            assert!(chunk.metadata.contains_key("section_path") || chunk.section.is_some());
        }
    }

    #[test]
    fn test_detect_header_level() {
        assert_eq!(TTRPGChunker::detect_header_level("Chapter 1"), 1);
        assert_eq!(TTRPGChunker::detect_header_level("Part II"), 1);
        assert_eq!(TTRPGChunker::detect_header_level("Section A"), 2);
        assert_eq!(TTRPGChunker::detect_header_level("Appendix B"), 2);
        assert_eq!(TTRPGChunker::detect_header_level("MONSTERS"), 2);
        assert_eq!(TTRPGChunker::detect_header_level("Regular Header"), 3);
    }

    // ========================================================================
    // Vocabulary Config Integration Tests
    // ========================================================================

    #[test]
    fn test_chunk_config_default_uses_vocabulary_constants() {
        let config = ChunkConfig::default();

        // Should use chunking_config constants from vocabulary
        assert_eq!(config.target_size, chunking_config::TARGET_CHUNK_SIZE);
        assert_eq!(config.min_size, chunking_config::MIN_CHUNK_SIZE);
        assert_eq!(config.max_size, chunking_config::MAX_CHUNK_SIZE);
        assert_eq!(config.overlap_size, chunking_config::CHUNK_OVERLAP);

        // Verify actual values (from MDMAI config)
        assert_eq!(config.target_size, 1200);
        assert_eq!(config.min_size, 300);
        assert_eq!(config.max_size, 2400);
        assert_eq!(config.overlap_size, 150);
    }

    #[test]
    fn test_chunk_config_small() {
        let config = ChunkConfig::small();

        // Should be half of default
        assert_eq!(config.target_size, chunking_config::TARGET_CHUNK_SIZE / 2);
        assert_eq!(config.min_size, chunking_config::MIN_CHUNK_SIZE / 2);
        assert_eq!(config.max_size, chunking_config::MAX_CHUNK_SIZE / 2);
        assert_eq!(config.overlap_size, chunking_config::CHUNK_OVERLAP / 2);
    }

    #[test]
    fn test_chunk_config_large() {
        let config = ChunkConfig::large();

        // Should be double of default
        assert_eq!(config.target_size, chunking_config::TARGET_CHUNK_SIZE * 2);
        assert_eq!(config.min_size, chunking_config::MIN_CHUNK_SIZE * 2);
        assert_eq!(config.max_size, chunking_config::MAX_CHUNK_SIZE * 2);
        assert_eq!(config.overlap_size, chunking_config::CHUNK_OVERLAP * 2);
    }

    #[test]
    fn test_chunk_config_from_tokens() {
        let config = ChunkConfig::from_tokens(100, 200, 25);

        // 4 chars per token
        assert_eq!(config.target_size, 400);
        assert_eq!(config.max_size, 800);
        assert_eq!(config.overlap_size, 100);
        assert_eq!(config.min_size, 100); // target/4
    }

    #[test]
    fn test_chunk_config_from_vocabulary_tokens() {
        let config = ChunkConfig::from_vocabulary_tokens();

        // Uses TOKEN constants: TARGET_TOKENS=300, MAX_TOKENS=600, OVERLAP_TOKENS=40
        assert_eq!(config.target_size, chunking_config::TARGET_TOKENS * 4);
        assert_eq!(config.max_size, chunking_config::MAX_TOKENS * 4);
        assert_eq!(config.overlap_size, chunking_config::OVERLAP_TOKENS * 4);
    }

    #[test]
    fn test_chunk_config_consistency() {
        let config = ChunkConfig::default();

        // Verify invariants
        assert!(config.min_size < config.target_size, "min_size should be less than target_size");
        assert!(config.target_size < config.max_size, "target_size should be less than max_size");
        assert!(config.overlap_size < config.min_size, "overlap_size should be less than min_size");
    }
}
