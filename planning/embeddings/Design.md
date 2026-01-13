# Design: TTRPG Document Parsing & Embedding System

## Overview

This design extends the existing `ttrpg-assistant` Rust/Tauri application with TTRPG-specific document processing capabilities. The design follows the existing patterns in the codebase and integrates with current modules rather than replacing them.

### Existing Code Integration Points

| Existing File | Purpose | Extension Strategy |
|---------------|---------|-------------------|
| `src/ingestion/mod.rs` | Module exports | Add new submodules: `ttrpg`, `layout` |
| `src/ingestion/pdf_parser.rs` | PDF extraction with `lopdf` | Extend with fallback extraction, password support |
| `src/ingestion/chunker.rs` | Sentence-aware chunking | Add TTRPG-aware mode respecting element boundaries |
| `src/core/search_client.rs` | Meilisearch wrapper | Extend `SearchDocument.metadata` with TTRPG attributes |
| `src/core/query_expansion.rs` | Query enhancement | Add attribute extraction, negation parsing, antonym mapping |
| `src/database/` | SQLite via sqlx | Add migration for `ttrpg_source_documents` table |
| `src/commands.rs` | Tauri IPC handlers | Add commands for TTRPG-enhanced ingestion |

### Design Principles

1. **Extend, Don't Replace**: New functionality builds on existing code
2. **Trait-Based Extensibility**: Use traits for pluggable game system vocabularies
3. **Backward Compatibility**: Enhanced processing is opt-in; existing behavior unchanged
4. **Existing Patterns**: Follow the error handling, serialization, and module patterns already in use
5. **Hard Filters + Soft Penalties**: Use pre-filters for exclusions, post-scoring for preferences
6. **Confidence-Based Decisions**: Track extraction confidence to inform filtering strictness

## Architecture

### Module Structure

```
src-tauri/src/
├── ingestion/
│   ├── mod.rs                    # [MODIFY] Add new exports
│   ├── pdf_parser.rs             # [EXTEND] Add fallback extraction, password support
│   ├── chunker.rs                # [EXTEND] Add TTRPGChunker with hierarchy tracking
│   │
│   ├── ttrpg/                    # [NEW] TTRPG-specific processing
│   │   ├── mod.rs                # Module exports
│   │   ├── classifier.rs         # Element type classification
│   │   ├── stat_block.rs         # Stat block parsing
│   │   ├── random_table.rs       # Random table parsing (with multi-page support)
│   │   ├── attribute_extractor.rs # Critical term extraction with confidence
│   │   ├── vocabulary.rs         # Game system vocabularies
│   │   └── game_detector.rs      # [NEW] Auto-detect game system from content
│   │
│   └── layout/                   # [NEW] Layout analysis for complex PDFs
│       ├── mod.rs
│       ├── column_detector.rs    # Multi-column boundary detection
│       ├── region_detector.rs    # Boxed/shaded region detection
│       └── table_extractor.rs    # Table structure extraction
│
├── core/
│   ├── search_client.rs          # [EXTEND] Add TTRPG index config, queue fallback
│   ├── query_expansion.rs        # [EXTEND] Add attribute parsing with negation
│   │
│   └── ttrpg_search/             # [NEW] TTRPG-enhanced search
│       ├── mod.rs
│       ├── query_parser.rs       # [NEW] Extract constraints, negations, named entities
│       ├── attribute_filter.rs   # Build Meilisearch filter strings
│       ├── antonym_scorer.rs     # Antonym penalty logic
│       └── result_ranker.rs      # RRF fusion + combined scoring with breakdown
│
├── database/
│   └── migrations/
│       └── YYYYMMDD_ttrpg_source_documents.sql  # [NEW] Source tracking schema
│
└── commands.rs                   # [EXTEND] Add TTRPG ingestion commands
```

### Data Flow

```
                    INGESTION PIPELINE
                    ──────────────────

PDF File ─────► pdf_parser.rs ─────► ExtractedDocument
     │               │                      │
     │               │ (fallback to         │
     │               │  pdf-extract if      │
     │               │  lopdf fails)        │
     │               ▼                      ▼
     │        ┌─────────────┐        ┌──────────────────┐
     │        │ LayoutAnalyzer │      │ TTRPGClassifier  │
     │        │ - columns      │      │ - Detect stat    │
     │        │ - boxes        │      │   blocks, tables │
     │        │ - tables       │      │ - Confidence     │
     │        └───────┬────────┘      └────────┬─────────┘
     │                │                        │
     │                └────────────┬───────────┘
     │                             ▼
     │                   ClassifiedDocument
     │                   (with layout regions)
     │                             │
     │                             ▼
     │                 ┌───────────────────────┐
     │                 │   TTRPGChunker        │
     │                 │ - Respects elements   │
     │                 │ - Tracks hierarchy    │
     │                 │ - Multi-page merge    │
     │                 └───────────┬───────────┘
     │                             │
     │                             ▼
     │                   ContentChunk[]
     │                   (with section context)
     │                             │
     │                             ▼
     │                 ┌───────────────────────┐
     │                 │  AttributeExtractor   │
     │                 │ + Confidence scores   │
     │                 │ + GameDetector        │
     │                 └───────────┬───────────┘
     │                             │
     │                             ▼
     │                   EnrichedChunk[]
     │                   (TTRPGAttributes)
     │                             │
     ▼                             ▼
┌─────────┐              ┌──────────────────┐
│ BLAKE3  │              │ TTRPGSearchDoc   │
│ Hash    │              │ + score-ready    │
│ + SQLite│              │   metadata       │
└─────────┘              └────────┬─────────┘
                                  │
                                  ▼
                           ┌────────────┐
                           │ IndexQueue │ (retry if Meilisearch down)
                           └─────┬──────┘
                                 │
                                 ▼
                           Meilisearch
                         (filterable attrs)


                    SEARCH PIPELINE
                    ───────────────

User Query ─────► QueryParser
                      │
                      ▼
              ┌───────────────────────────────┐
              │ QueryConstraints              │
              │ - required_attributes         │
              │ - excluded_attributes (NOT)   │
              │ - cr_range                    │
              │ - exact_match_entities        │
              │ - expanded_query (+ antonyms) │
              └───────────────┬───────────────┘
                              │
           ┌──────────────────┼──────────────────┐
           ▼                  ▼                  ▼
    ┌────────────┐     ┌────────────┐     ┌────────────┐
    │   Dense    │     │  Keyword   │     │ Pre-Filter │
    │  (vector)  │     │  (BM25)    │     │ (hard NOT) │
    └─────┬──────┘     └─────┬──────┘     └─────┬──────┘
          │                  │                  │
          └──────────────────┼──────────────────┘
                             ▼
                    ┌─────────────────┐
                    │  RRF Fusion     │
                    │ (reciprocal     │
                    │  rank fusion)   │
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │  ResultRanker   │
                    │ - Attr boost    │
                    │ - Antonym pen   │
                    │ - Score breakdown│
                    └────────┬────────┘
                             │
                             ▼
                   TTRPGSearchResult[]
                   (with ScoreBreakdown)
```

## Component Details

### 0. PDF Parser Extensions (`ingestion/pdf_parser.rs`)

**Purpose**: Extend existing PDF parser with fallback extraction and password support.

```rust
// ADDITIONS to src/ingestion/pdf_parser.rs

use pdf_extract;  // Fallback crate

/// Extended PDF parser with fallback and password support
impl PDFParser {
    /// Extract with automatic fallback if lopdf fails
    pub fn extract_with_fallback(
        path: &Path,
        password: Option<&str>,
    ) -> Result<ExtractedDocument> {
        // Try lopdf first
        match Self::extract_structured_internal(path, password) {
            Ok(doc) => {
                // Validate extraction quality
                if Self::is_extraction_quality_acceptable(&doc) {
                    return Ok(doc);
                }
                log::warn!("lopdf extraction quality low, trying fallback");
            }
            Err(e) => {
                log::warn!("lopdf failed: {}, trying fallback", e);
            }
        }

        // Fallback to pdf-extract
        Self::extract_with_pdf_extract(path)
    }

    /// Check extraction quality (detect garbled output)
    fn is_extraction_quality_acceptable(doc: &ExtractedDocument) -> bool {
        let total_text: String = doc.pages.iter()
            .map(|p| p.text.as_str())
            .collect();

        if total_text.is_empty() {
            return false;
        }

        // Check for high ratio of non-printable/garbled characters
        let printable_ratio = total_text.chars()
            .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_punctuation() || c.is_whitespace())
            .count() as f32 / total_text.len() as f32;

        printable_ratio > 0.85
    }

    /// Fallback extraction using pdf-extract crate
    fn extract_with_pdf_extract(path: &Path) -> Result<ExtractedDocument> {
        let bytes = std::fs::read(path)?;
        let text = pdf_extract::extract_text_from_mem(&bytes)
            .map_err(|e| PDFError::ExtractionError(format!("pdf-extract failed: {}", e)))?;

        // pdf-extract doesn't preserve page boundaries, so we create a single-page doc
        Ok(ExtractedDocument {
            source_path: path.to_string_lossy().to_string(),
            page_count: 1,
            pages: vec![ExtractedPage {
                page_number: 1,
                text: text.clone(),
                paragraphs: text.split("\n\n").map(|s| s.to_string()).collect(),
                headers: vec![],
            }],
            metadata: DocumentMetadata::default(),
        })
    }

    fn extract_structured_internal(path: &Path, password: Option<&str>) -> Result<ExtractedDocument> {
        let doc = if let Some(pwd) = password {
            Document::load_with_password(path, pwd.as_bytes())
                .map_err(|e| PDFError::LoadError(format!("Failed with password: {}", e)))?
        } else {
            Document::load(path)
                .map_err(|e| PDFError::LoadError(e.to_string()))?
        };

        // ... rest of existing extraction logic
        Self::extract_from_document(doc, path)
    }
}
```

### 0.1 Layout Detection (`ingestion/layout/`)

**Purpose**: Detect multi-column layouts, boxed regions, and table structures in PDFs.

```rust
// src/ingestion/layout/mod.rs
pub mod column_detector;
pub mod region_detector;
pub mod table_extractor;

pub use column_detector::ColumnDetector;
pub use region_detector::{RegionDetector, DetectedRegion, RegionType};
pub use table_extractor::{TableExtractor, ExtractedTable};

// src/ingestion/layout/column_detector.rs
use serde::{Deserialize, Serialize};

/// Represents a detected column boundary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnBoundary {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

/// Detects multi-column layouts from text position data
pub struct ColumnDetector {
    /// Minimum gap between columns (in points)
    min_column_gap: f32,
    /// Minimum column width to consider
    min_column_width: f32,
}

impl ColumnDetector {
    pub fn new() -> Self {
        Self {
            min_column_gap: 20.0,
            min_column_width: 100.0,
        }
    }

    /// Detect columns from text with position data
    /// Returns text in logical reading order
    pub fn reorder_text_by_columns(
        &self,
        text_blocks: &[TextBlock],
        page_width: f32,
    ) -> Vec<TextBlock> {
        // Group text blocks by horizontal position
        let columns = self.detect_column_boundaries(text_blocks, page_width);

        if columns.len() <= 1 {
            // Single column - return as-is, sorted by Y position
            let mut sorted = text_blocks.to_vec();
            sorted.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());
            return sorted;
        }

        // Multi-column: sort by column, then by Y within column
        let mut result = Vec::new();
        for col in &columns {
            let mut col_blocks: Vec<_> = text_blocks.iter()
                .filter(|b| b.x >= col.left && b.x < col.right)
                .cloned()
                .collect();
            col_blocks.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());
            result.extend(col_blocks);
        }
        result
    }

    fn detect_column_boundaries(&self, blocks: &[TextBlock], page_width: f32) -> Vec<ColumnBoundary> {
        // Histogram of X positions to find column gaps
        let mut x_positions: Vec<f32> = blocks.iter().map(|b| b.x).collect();
        x_positions.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Find gaps larger than min_column_gap
        let mut gaps = Vec::new();
        for i in 1..x_positions.len() {
            let gap = x_positions[i] - x_positions[i - 1];
            if gap > self.min_column_gap {
                gaps.push((x_positions[i - 1], x_positions[i]));
            }
        }

        // Convert gaps to column boundaries
        let mut columns = Vec::new();
        let mut left = 0.0;
        for (gap_left, gap_right) in gaps {
            if gap_left - left >= self.min_column_width {
                columns.push(ColumnBoundary {
                    left,
                    right: gap_left,
                    top: 0.0,
                    bottom: f32::MAX,
                });
            }
            left = gap_right;
        }
        // Last column
        if page_width - left >= self.min_column_width {
            columns.push(ColumnBoundary {
                left,
                right: page_width,
                top: 0.0,
                bottom: f32::MAX,
            });
        }

        columns
    }
}

#[derive(Debug, Clone)]
pub struct TextBlock {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

// src/ingestion/layout/region_detector.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionType {
    Sidebar,
    Callout,
    ReadAloud,
    Table,
    StatBlock,
    Normal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedRegion {
    pub region_type: RegionType,
    pub content: String,
    pub confidence: f32,
    pub page_number: u32,
    pub bounds: Option<RegionBounds>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Detects boxed/shaded regions in PDF content
pub struct RegionDetector {
    /// Keywords that suggest read-aloud text
    read_aloud_indicators: Vec<String>,
    /// Keywords that suggest sidebar content
    sidebar_indicators: Vec<String>,
}

impl RegionDetector {
    pub fn new() -> Self {
        Self {
            read_aloud_indicators: vec![
                "read aloud".to_string(),
                "boxed text".to_string(),
            ],
            sidebar_indicators: vec![
                "sidebar".to_string(),
                "note:".to_string(),
                "tip:".to_string(),
                "variant:".to_string(),
            ],
        }
    }

    /// Detect regions from text content
    /// Note: Full implementation requires PDF graphics parsing for box/shading detection
    pub fn detect_from_text(&self, text: &str, page_number: u32) -> Vec<DetectedRegion> {
        let mut regions = Vec::new();
        let text_lower = text.to_lowercase();

        // Heuristic: Detect read-aloud by formatting patterns
        // Often in italics or preceded by box indicators
        for indicator in &self.read_aloud_indicators {
            if text_lower.contains(indicator) {
                regions.push(DetectedRegion {
                    region_type: RegionType::ReadAloud,
                    content: text.to_string(),
                    confidence: 0.7,
                    page_number,
                    bounds: None,
                });
            }
        }

        // Detect sidebars
        for indicator in &self.sidebar_indicators {
            if text_lower.starts_with(indicator) {
                regions.push(DetectedRegion {
                    region_type: RegionType::Sidebar,
                    content: text.to_string(),
                    confidence: 0.75,
                    page_number,
                    bounds: None,
                });
            }
        }

        regions
    }
}

// src/ingestion/layout/table_extractor.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedTable {
    pub title: Option<String>,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub page_numbers: Vec<u32>,  // Supports multi-page tables
    pub is_continuation: bool,
}

/// Extracts table structure from PDF content
pub struct TableExtractor {
    /// Patterns that indicate table continuation
    continuation_patterns: Vec<regex::Regex>,
}

impl TableExtractor {
    pub fn new() -> Self {
        Self {
            continuation_patterns: vec![
                regex::Regex::new(r"(?i)^\s*\(continued\)").unwrap(),
                regex::Regex::new(r"(?i)table\s+\d+\s*\(cont").unwrap(),
            ],
        }
    }

    /// Detect if a table continues from previous page
    pub fn is_table_continuation(&self, text: &str) -> bool {
        self.continuation_patterns.iter().any(|re| re.is_match(text))
    }

    /// Merge continuation tables from multiple pages
    pub fn merge_continuation_tables(
        &self,
        tables: Vec<ExtractedTable>,
    ) -> Vec<ExtractedTable> {
        let mut result = Vec::new();
        let mut pending_merge: Option<ExtractedTable> = None;

        for table in tables {
            if table.is_continuation {
                if let Some(ref mut base) = pending_merge {
                    // Append rows to existing table
                    base.rows.extend(table.rows);
                    base.page_numbers.extend(table.page_numbers);
                } else {
                    // Orphan continuation - keep as-is
                    result.push(table);
                }
            } else {
                // Flush pending
                if let Some(merged) = pending_merge.take() {
                    result.push(merged);
                }
                pending_merge = Some(table);
            }
        }

        // Flush final
        if let Some(merged) = pending_merge {
            result.push(merged);
        }

        result
    }
}
```

### 1. TTRPG Classifier (`ingestion/ttrpg/classifier.rs`)

**Purpose**: Classify extracted content into TTRPG element types.

**Integration Point**: Called after `PDFParser::extract_structured()` returns `ExtractedDocument`.

```rust
// src/ingestion/ttrpg/classifier.rs

use serde::{Deserialize, Serialize};

/// TTRPG element types that require special handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TTRPGElementType {
    StatBlock,
    RandomTable,
    ReadAloudText,
    Sidebar,
    SpellDescription,
    ItemDescription,
    ClassFeature,
    SectionHeader,
    GenericText,
}

/// Classification result with confidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedElement {
    pub element_type: TTRPGElementType,
    pub confidence: f32,
    pub content: String,
    pub page_number: u32,
    /// For stat blocks: parsed structured data
    pub structured_data: Option<serde_json::Value>,
}

/// Classifies text content into TTRPG element types
pub struct TTRPGClassifier {
    /// Minimum confidence to classify (below falls back to GenericText)
    min_confidence: f32,
}

impl TTRPGClassifier {
    pub fn new() -> Self {
        Self { min_confidence: 0.7 }
    }

    /// Classify a single text block
    pub fn classify(&self, text: &str, page_number: u32) -> ClassifiedElement {
        // Detection priority: most specific first
        if let Some(stat_block) = self.detect_stat_block(text) {
            return ClassifiedElement {
                element_type: TTRPGElementType::StatBlock,
                confidence: stat_block.confidence,
                content: text.to_string(),
                page_number,
                structured_data: Some(serde_json::to_value(&stat_block.data).unwrap()),
            };
        }

        if let Some(table) = self.detect_random_table(text) {
            return ClassifiedElement {
                element_type: TTRPGElementType::RandomTable,
                confidence: table.confidence,
                content: text.to_string(),
                page_number,
                structured_data: Some(serde_json::to_value(&table.data).unwrap()),
            };
        }

        // ... other detections

        ClassifiedElement {
            element_type: TTRPGElementType::GenericText,
            confidence: 1.0,
            content: text.to_string(),
            page_number,
            structured_data: None,
        }
    }

    /// Detect stat block patterns (AC, HP, ability scores)
    fn detect_stat_block(&self, text: &str) -> Option<DetectionResult<StatBlockData>> {
        // Pattern matching for stat block indicators
        let has_ac = regex::Regex::new(r"(?i)armor\s*class|AC\s*\d").unwrap().is_match(text);
        let has_hp = regex::Regex::new(r"(?i)hit\s*points?|HP\s*\d").unwrap().is_match(text);
        let has_abilities = regex::Regex::new(r"(?i)STR\s+DEX\s+CON|Strength|Dexterity").unwrap().is_match(text);

        if has_ac && has_hp && has_abilities {
            // Parse structured data
            let data = StatBlockData::parse(text);
            Some(DetectionResult {
                confidence: 0.9,
                data,
            })
        } else if (has_ac && has_hp) || (has_hp && has_abilities) {
            Some(DetectionResult {
                confidence: 0.7,
                data: StatBlockData::parse(text),
            })
        } else {
            None
        }
    }

    /// Detect random table patterns (dice notation, numbered rows)
    fn detect_random_table(&self, text: &str) -> Option<DetectionResult<RandomTableData>> {
        let dice_pattern = regex::Regex::new(r"\b\d*d\d+\b").unwrap();
        let range_pattern = regex::Regex::new(r"\b(\d+)[-–](\d+)\b").unwrap();

        if dice_pattern.is_match(text) && range_pattern.is_match(text) {
            Some(DetectionResult {
                confidence: 0.85,
                data: RandomTableData::parse(text),
            })
        } else {
            None
        }
    }
}
```

### 2. Stat Block Parser (`ingestion/ttrpg/stat_block.rs`)

**Purpose**: Parse stat block text into structured data.

```rust
// src/ingestion/ttrpg/stat_block.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed stat block data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatBlockData {
    pub name: String,
    pub creature_type: Option<String>,
    pub size: Option<String>,
    pub alignment: Option<String>,
    pub armor_class: Option<u32>,
    pub armor_type: Option<String>,
    pub hit_points: Option<String>,  // "45 (6d10 + 12)"
    pub hit_dice: Option<String>,
    pub speed: HashMap<String, u32>, // walk: 30, fly: 60
    pub ability_scores: AbilityScores,
    pub saving_throws: HashMap<String, i32>,
    pub skills: HashMap<String, i32>,
    pub damage_resistances: Vec<String>,
    pub damage_immunities: Vec<String>,
    pub damage_vulnerabilities: Vec<String>,
    pub condition_immunities: Vec<String>,
    pub senses: HashMap<String, u32>,
    pub languages: Vec<String>,
    pub challenge_rating: Option<String>,
    pub proficiency_bonus: Option<i32>,
    pub traits: Vec<Feature>,
    pub actions: Vec<Feature>,
    pub reactions: Vec<Feature>,
    pub legendary_actions: Vec<Feature>,
    /// Raw text for fields that couldn't be parsed
    pub unparsed_sections: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AbilityScores {
    pub strength: Option<u32>,
    pub dexterity: Option<u32>,
    pub constitution: Option<u32>,
    pub intelligence: Option<u32>,
    pub wisdom: Option<u32>,
    pub charisma: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub name: String,
    pub description: String,
}

impl StatBlockData {
    pub fn parse(text: &str) -> Self {
        let mut data = StatBlockData::default();

        // Extract name (usually first line, often all caps or bold)
        data.name = Self::extract_name(text);

        // Extract type/size/alignment line
        if let Some((size, creature_type, alignment)) = Self::extract_type_line(text) {
            data.size = Some(size);
            data.creature_type = Some(creature_type);
            data.alignment = Some(alignment);
        }

        // Extract AC
        data.armor_class = Self::extract_ac(text);

        // Extract HP
        data.hit_points = Self::extract_hp(text);

        // Extract ability scores
        data.ability_scores = Self::extract_abilities(text);

        // Extract damage types for indexing
        data.damage_resistances = Self::extract_damage_list(text, r"(?i)damage\s+resist");
        data.damage_immunities = Self::extract_damage_list(text, r"(?i)damage\s+immun");

        // Extract CR
        data.challenge_rating = Self::extract_cr(text);

        data
    }

    fn extract_name(text: &str) -> String {
        text.lines()
            .next()
            .map(|l| l.trim().to_string())
            .unwrap_or_default()
    }

    fn extract_ac(text: &str) -> Option<u32> {
        let re = regex::Regex::new(r"(?i)armor\s*class\s*(\d+)").ok()?;
        re.captures(text)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse().ok())
    }

    fn extract_hp(text: &str) -> Option<String> {
        let re = regex::Regex::new(r"(?i)hit\s*points?\s*(\d+\s*\([^)]+\)|\d+)").ok()?;
        re.captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
    }

    fn extract_abilities(text: &str) -> AbilityScores {
        let mut scores = AbilityScores::default();
        // Pattern: "STR 18 (+4) DEX 14 (+2)..." or "Strength 18 Dexterity 14..."
        let patterns = [
            (r"(?i)str(?:ength)?\s*(\d+)", &mut scores.strength),
            (r"(?i)dex(?:terity)?\s*(\d+)", &mut scores.dexterity),
            (r"(?i)con(?:stitution)?\s*(\d+)", &mut scores.constitution),
            (r"(?i)int(?:elligence)?\s*(\d+)", &mut scores.intelligence),
            (r"(?i)wis(?:dom)?\s*(\d+)", &mut scores.wisdom),
            (r"(?i)cha(?:risma)?\s*(\d+)", &mut scores.charisma),
        ];

        for (pattern, field) in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(cap) = re.captures(text) {
                    *field = cap.get(1).and_then(|m| m.as_str().parse().ok());
                }
            }
        }
        scores
    }

    fn extract_cr(text: &str) -> Option<String> {
        let re = regex::Regex::new(r"(?i)challenge\s*(?:rating)?\s*([\d/]+)").ok()?;
        re.captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
    }

    fn extract_damage_list(text: &str, prefix_pattern: &str) -> Vec<String> {
        // Find line containing the prefix, extract damage types
        let damage_types = ["fire", "cold", "lightning", "thunder", "acid", "poison",
            "necrotic", "radiant", "force", "psychic", "bludgeoning", "piercing", "slashing"];

        let prefix = regex::Regex::new(prefix_pattern).ok();
        if prefix.is_none() { return vec![]; }

        let mut found = Vec::new();
        for line in text.lines() {
            if prefix.as_ref().unwrap().is_match(line) {
                for dtype in &damage_types {
                    if line.to_lowercase().contains(dtype) {
                        found.push(dtype.to_string());
                    }
                }
            }
        }
        found
    }

    fn extract_type_line(text: &str) -> Option<(String, String, String)> {
        // Pattern: "Medium humanoid (elf), neutral good"
        let re = regex::Regex::new(
            r"(?i)(tiny|small|medium|large|huge|gargantuan)\s+(\w+)(?:\s*\([^)]+\))?,?\s*(lawful|neutral|chaotic)?\s*(good|neutral|evil)?"
        ).ok()?;

        re.captures(text).map(|c| {
            let size = c.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let creature_type = c.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            let alignment = format!(
                "{} {}",
                c.get(3).map(|m| m.as_str()).unwrap_or(""),
                c.get(4).map(|m| m.as_str()).unwrap_or("")
            ).trim().to_string();
            (size, creature_type, alignment)
        })
    }
}

impl Default for StatBlockData {
    fn default() -> Self {
        Self {
            name: String::new(),
            creature_type: None,
            size: None,
            alignment: None,
            armor_class: None,
            armor_type: None,
            hit_points: None,
            hit_dice: None,
            speed: HashMap::new(),
            ability_scores: AbilityScores::default(),
            saving_throws: HashMap::new(),
            skills: HashMap::new(),
            damage_resistances: Vec::new(),
            damage_immunities: Vec::new(),
            damage_vulnerabilities: Vec::new(),
            condition_immunities: Vec::new(),
            senses: HashMap::new(),
            languages: Vec::new(),
            challenge_rating: None,
            proficiency_bonus: None,
            traits: Vec::new(),
            actions: Vec::new(),
            reactions: Vec::new(),
            legendary_actions: Vec::new(),
            unparsed_sections: Vec::new(),
        }
    }
}
```

### 3. Critical Attribute Extractor (`ingestion/ttrpg/attribute_extractor.rs`)

**Purpose**: Extract filterable attributes for Meilisearch indexing with confidence scores.

```rust
// src/ingestion/ttrpg/attribute_extractor.rs

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Source of an attribute match
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttributeSource {
    /// Exact term match in text
    ExactMatch,
    /// Pattern/regex match
    PatternMatch,
    /// Inferred from context
    Inferred,
    /// Extracted from structured data (stat block)
    StructuredData,
}

/// A single attribute match with confidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeMatch {
    pub value: String,
    pub confidence: f32,  // 0.0-1.0
    pub source: AttributeSource,
}

impl AttributeMatch {
    pub fn exact(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            confidence: 1.0,
            source: AttributeSource::ExactMatch,
        }
    }

    pub fn pattern(value: impl Into<String>, confidence: f32) -> Self {
        Self {
            value: value.into(),
            confidence,
            source: AttributeSource::PatternMatch,
        }
    }

    pub fn inferred(value: impl Into<String>, confidence: f32) -> Self {
        Self {
            value: value.into(),
            confidence,
            source: AttributeSource::Inferred,
        }
    }
}

/// Extracted TTRPG attributes for a content chunk
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TTRPGAttributes {
    /// Damage types mentioned (fire, cold, lightning, etc.)
    pub damage_types: Vec<AttributeMatch>,
    /// Creature types mentioned (humanoid, undead, dragon, etc.)
    pub creature_types: Vec<AttributeMatch>,
    /// Alignment values (lawful, chaotic, good, evil, neutral)
    pub alignments: Vec<AttributeMatch>,
    /// Rarity values (common, uncommon, rare, very rare, legendary)
    pub rarities: Vec<AttributeMatch>,
    /// Creature sizes (tiny, small, medium, large, huge, gargantuan)
    pub sizes: Vec<AttributeMatch>,
    /// Conditions mentioned (poisoned, paralyzed, frightened, etc.)
    pub conditions: Vec<AttributeMatch>,
    /// Schools of magic (evocation, necromancy, etc.)
    pub spell_schools: Vec<AttributeMatch>,
    /// Numeric CR/level if detected
    pub challenge_rating: Option<f32>,
    pub level: Option<u32>,
    /// Element type from classifier
    pub element_type: String,
    /// Named entities (spell names, creature names, etc.)
    pub named_entities: Vec<String>,
    /// Detected game system (if auto-detected)
    pub detected_game_system: Option<GameSystem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameSystem {
    DnD5e,
    Pathfinder2e,
    CallOfCthulhu,
    Other,
}

impl TTRPGAttributes {
    /// Get high-confidence damage types (for hard filtering)
    pub fn confident_damage_types(&self, min_confidence: f32) -> Vec<&str> {
        self.damage_types.iter()
            .filter(|m| m.confidence >= min_confidence)
            .map(|m| m.value.as_str())
            .collect()
    }

    /// Get all damage type values (for soft matching)
    pub fn all_damage_types(&self) -> Vec<&str> {
        self.damage_types.iter().map(|m| m.value.as_str()).collect()
    }

    /// Convert to flat vectors for Meilisearch indexing
    pub fn to_filterable_fields(&self) -> FilterableFields {
        FilterableFields {
            damage_types: self.damage_types.iter().map(|m| m.value.clone()).collect(),
            creature_types: self.creature_types.iter().map(|m| m.value.clone()).collect(),
            alignments: self.alignments.iter().map(|m| m.value.clone()).collect(),
            rarities: self.rarities.iter().map(|m| m.value.clone()).collect(),
            sizes: self.sizes.iter().map(|m| m.value.clone()).collect(),
            conditions: self.conditions.iter().map(|m| m.value.clone()).collect(),
            spell_schools: self.spell_schools.iter().map(|m| m.value.clone()).collect(),
            challenge_rating: self.challenge_rating,
            level: self.level,
        }
    }
}

/// Flat field representation for Meilisearch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterableFields {
    pub damage_types: Vec<String>,
    pub creature_types: Vec<String>,
    pub alignments: Vec<String>,
    pub rarities: Vec<String>,
    pub sizes: Vec<String>,
    pub conditions: Vec<String>,
    pub spell_schools: Vec<String>,
    pub challenge_rating: Option<f32>,
    pub level: Option<u32>,
}

/// Vocabulary for a specific game system
pub trait GameVocabulary: Send + Sync {
    fn damage_types(&self) -> &[&str];
    fn creature_types(&self) -> &[&str];
    fn conditions(&self) -> &[&str];
    fn spell_schools(&self) -> &[&str];
    fn rarities(&self) -> &[&str];
    fn sizes(&self) -> &[&str];
    fn ability_abbreviations(&self) -> &[(&str, &str)]; // ("Str", "strength")
}

/// D&D 5e vocabulary (default)
pub struct DnD5eVocabulary;

impl GameVocabulary for DnD5eVocabulary {
    fn damage_types(&self) -> &[&str] {
        &["fire", "cold", "lightning", "thunder", "acid", "poison",
          "necrotic", "radiant", "force", "psychic", "bludgeoning",
          "piercing", "slashing"]
    }

    fn creature_types(&self) -> &[&str] {
        &["aberration", "beast", "celestial", "construct", "dragon",
          "elemental", "fey", "fiend", "giant", "humanoid", "monstrosity",
          "ooze", "plant", "undead"]
    }

    fn conditions(&self) -> &[&str] {
        &["blinded", "charmed", "deafened", "exhaustion", "frightened",
          "grappled", "incapacitated", "invisible", "paralyzed", "petrified",
          "poisoned", "prone", "restrained", "stunned", "unconscious"]
    }

    fn spell_schools(&self) -> &[&str] {
        &["abjuration", "conjuration", "divination", "enchantment",
          "evocation", "illusion", "necromancy", "transmutation"]
    }

    fn rarities(&self) -> &[&str] {
        &["common", "uncommon", "rare", "very rare", "legendary", "artifact"]
    }

    fn sizes(&self) -> &[&str] {
        &["tiny", "small", "medium", "large", "huge", "gargantuan"]
    }

    fn ability_abbreviations(&self) -> &[(&str, &str)] {
        &[("str", "strength"), ("dex", "dexterity"), ("con", "constitution"),
          ("int", "intelligence"), ("wis", "wisdom"), ("cha", "charisma")]
    }
}

/// Extracts TTRPG attributes from text with confidence scoring
pub struct AttributeExtractor {
    vocabulary: Box<dyn GameVocabulary>,
    /// Threshold for word boundary matching (higher = more strict)
    boundary_match_confidence: f32,
}

impl AttributeExtractor {
    pub fn new() -> Self {
        Self {
            vocabulary: Box::new(DnD5eVocabulary),
            boundary_match_confidence: 0.9,
        }
    }

    pub fn with_vocabulary(vocabulary: Box<dyn GameVocabulary>) -> Self {
        Self {
            vocabulary,
            boundary_match_confidence: 0.9,
        }
    }

    /// Extract all attributes from text content with confidence scores
    pub fn extract(&self, text: &str) -> TTRPGAttributes {
        let text_lower = text.to_lowercase();
        let mut attrs = TTRPGAttributes::default();

        // Extract damage types with confidence
        attrs.damage_types = self.extract_terms_with_confidence(&text_lower, self.vocabulary.damage_types());

        // Extract creature types
        attrs.creature_types = self.extract_terms_with_confidence(&text_lower, self.vocabulary.creature_types());

        // Extract conditions
        attrs.conditions = self.extract_terms_with_confidence(&text_lower, self.vocabulary.conditions());

        // Extract spell schools
        attrs.spell_schools = self.extract_terms_with_confidence(&text_lower, self.vocabulary.spell_schools());

        // Extract rarities
        attrs.rarities = self.extract_terms_with_confidence(&text_lower, self.vocabulary.rarities());

        // Extract sizes
        attrs.sizes = self.extract_terms_with_confidence(&text_lower, self.vocabulary.sizes());

        // Extract alignments
        attrs.alignments = self.extract_alignments_with_confidence(&text_lower);

        // Extract CR if present
        attrs.challenge_rating = self.extract_cr(&text_lower);

        // Extract level if present
        attrs.level = self.extract_level(&text_lower);

        // Auto-detect game system
        attrs.detected_game_system = detect_game_system(text);

        // Extract named entities (spell names, creature names)
        attrs.named_entities = self.extract_named_entities(text);

        attrs
    }

    fn extract_terms_with_confidence(&self, text: &str, terms: &[&str]) -> Vec<AttributeMatch> {
        let mut matches = Vec::new();

        for term in terms {
            // Check for word-boundary match (higher confidence)
            let boundary_re = regex::Regex::new(&format!(r"\b{}\b", regex::escape(term))).ok();

            if let Some(re) = boundary_re {
                if re.is_match(text) {
                    matches.push(AttributeMatch::exact(*term));
                    continue;
                }
            }

            // Check for substring match (lower confidence)
            if text.contains(*term) {
                matches.push(AttributeMatch::pattern(*term, 0.7));
            }
        }

        matches
    }

    fn extract_alignments_with_confidence(&self, text: &str) -> Vec<AttributeMatch> {
        let mut alignments = Vec::new();
        let ethical = ["lawful", "neutral", "chaotic"];
        let moral = ["good", "neutral", "evil"];

        // Check for explicit alignment patterns (high confidence)
        let alignment_re = regex::Regex::new(
            r"(?i)\b(lawful|neutral|chaotic)\s+(good|neutral|evil)\b"
        ).ok();

        if let Some(re) = alignment_re {
            if re.is_match(text) {
                for cap in re.captures_iter(text) {
                    if let Some(ethical_part) = cap.get(1) {
                        alignments.push(AttributeMatch::exact(ethical_part.as_str().to_lowercase()));
                    }
                    if let Some(moral_part) = cap.get(2) {
                        let val = moral_part.as_str().to_lowercase();
                        if val != "neutral" || !alignments.iter().any(|a| a.value == "neutral") {
                            alignments.push(AttributeMatch::exact(val));
                        }
                    }
                }
                return alignments;
            }
        }

        // Fallback to individual term matching (lower confidence)
        for e in &ethical {
            if text.contains(e) {
                alignments.push(AttributeMatch::pattern(*e, 0.6));
            }
        }
        for m in &moral {
            if text.contains(m) && *m != "neutral" {
                alignments.push(AttributeMatch::pattern(*m, 0.6));
            }
        }
        alignments
    }

    fn extract_cr(&self, text: &str) -> Option<f32> {
        let re = regex::Regex::new(r"(?i)(?:cr|challenge(?:\s+rating)?)\s*([\d/]+)").ok()?;
        re.captures(text).and_then(|c| {
            let cr_str = c.get(1)?.as_str();
            if cr_str.contains('/') {
                let parts: Vec<&str> = cr_str.split('/').collect();
                if parts.len() == 2 {
                    let num: f32 = parts[0].parse().ok()?;
                    let den: f32 = parts[1].parse().ok()?;
                    return Some(num / den);
                }
            }
            cr_str.parse().ok()
        })
    }

    fn extract_level(&self, text: &str) -> Option<u32> {
        let re = regex::Regex::new(r"(?i)(?:level|lvl)\s*(\d+)").ok()?;
        re.captures(text)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse().ok())
    }

    /// Extract potential named entities (spell names, creature names, etc.)
    fn extract_named_entities(&self, text: &str) -> Vec<String> {
        let mut entities = Vec::new();

        // Common spell patterns
        let spell_re = regex::Regex::new(r"\b([A-Z][a-z]+(?:'s)?\s+[A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)\b").ok();
        if let Some(re) = spell_re {
            for cap in re.captures_iter(text) {
                if let Some(m) = cap.get(1) {
                    entities.push(m.as_str().to_string());
                }
            }
        }

        entities
    }
}

/// Auto-detect game system from content patterns
pub fn detect_game_system(text: &str) -> Option<GameSystem> {
    let text_lower = text.to_lowercase();

    // D&D 5e indicators
    let dnd5e_indicators = [
        "armor class", "hit dice", "spell slots", "proficiency bonus",
        "5th edition", "dungeons & dragons", "d&d", "saving throw",
        "ability score", "cantrip", "warlock", "sorcerer",
    ];

    // PF2e indicators
    let pf2e_indicators = [
        "three actions", "activity", "pathfinder", "proficiency rank",
        "expert", "master", "legendary", "focus point", "ancestry",
        "heritage", "general feat",
    ];

    // Call of Cthulhu indicators
    let coc_indicators = [
        "sanity", "mythos", "investigator", "keeper", "luck roll",
        "credit rating", "cthulhu", "lovecraft",
    ];

    let dnd_score: usize = dnd5e_indicators.iter()
        .filter(|ind| text_lower.contains(*ind))
        .count();

    let pf2e_score: usize = pf2e_indicators.iter()
        .filter(|ind| text_lower.contains(*ind))
        .count();

    let coc_score: usize = coc_indicators.iter()
        .filter(|ind| text_lower.contains(*ind))
        .count();

    // Require at least 3 indicators for a confident match
    let max_score = dnd_score.max(pf2e_score).max(coc_score);
    if max_score < 3 {
        return None;
    }

    if dnd_score == max_score {
        Some(GameSystem::DnD5e)
    } else if pf2e_score == max_score {
        Some(GameSystem::Pathfinder2e)
    } else if coc_score == max_score {
        Some(GameSystem::CallOfCthulhu)
    } else {
        Some(GameSystem::Other)
    }
}
```

### 4. Extending the Chunker (`ingestion/chunker.rs`)

**Purpose**: Add TTRPG-aware chunking mode to existing `SemanticChunker` with hierarchy tracking.

```rust
// ADDITIONS to src/ingestion/chunker.rs

use crate::ingestion::ttrpg::{ClassifiedElement, TTRPGElementType};

/// Extended chunk configuration with TTRPG options
#[derive(Debug, Clone)]
pub struct TTRPGChunkConfig {
    /// Base chunking config
    pub base: ChunkConfig,
    /// Never split these element types
    pub atomic_elements: Vec<TTRPGElementType>,
    /// Maximum size for atomic elements (2x base max before forced split)
    pub atomic_max_multiplier: f32,
    /// Overlap percentage for non-atomic content (0.10 = 10%)
    pub overlap_percentage: f32,
    /// Whether to include parent section hierarchy in metadata
    pub include_hierarchy: bool,
}

impl Default for TTRPGChunkConfig {
    fn default() -> Self {
        Self {
            base: ChunkConfig::default(),
            atomic_elements: vec![
                TTRPGElementType::StatBlock,
                TTRPGElementType::RandomTable,
                TTRPGElementType::ReadAloudText,
            ],
            atomic_max_multiplier: 2.0,
            overlap_percentage: 0.12, // 12% overlap
            include_hierarchy: true,
        }
    }
}

/// Section hierarchy tracker
#[derive(Debug, Clone, Default)]
pub struct SectionHierarchy {
    /// Stack of section titles (h1 at index 0, h2 at 1, etc.)
    sections: Vec<String>,
}

impl SectionHierarchy {
    pub fn new() -> Self {
        Self { sections: Vec::new() }
    }

    /// Update hierarchy when a new header is encountered
    pub fn update(&mut self, header: &str, level: usize) {
        // Truncate to current level
        if level < self.sections.len() {
            self.sections.truncate(level);
        }
        // Pad if we skipped levels
        while self.sections.len() < level {
            self.sections.push(String::new());
        }
        // Set current level
        if level < self.sections.len() {
            self.sections[level] = header.to_string();
        } else {
            self.sections.push(header.to_string());
        }
    }

    /// Get full hierarchy path
    pub fn path(&self) -> String {
        self.sections.iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" > ")
    }

    /// Get parent sections (excluding current)
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
}

/// TTRPG-aware chunker wrapper (uses composition instead of extending SemanticChunker)
pub struct TTRPGChunker {
    base_chunker: SemanticChunker,
    config: TTRPGChunkConfig,
}

impl TTRPGChunker {
    pub fn new(config: TTRPGChunkConfig) -> Self {
        Self {
            base_chunker: SemanticChunker::with_config(config.base.clone()),
            config,
        }
    }

    /// Chunk with TTRPG element awareness and hierarchy tracking
    pub fn chunk(
        &self,
        elements: &[ClassifiedElement],
        source_id: &str,
    ) -> Vec<ContentChunk> {
        let mut chunks = Vec::new();
        let mut chunk_index = 0;
        let mut current_content = String::new();
        let mut current_page: Option<u32> = None;
        let mut hierarchy = SectionHierarchy::new();

        let base_config = &self.config.base;
        let atomic_max = (base_config.max_size as f32 * self.config.atomic_max_multiplier) as usize;

        for element in elements {
            // Update hierarchy if this is a section header
            if element.element_type == TTRPGElementType::SectionHeader {
                let level = Self::detect_header_level(&element.content);
                hierarchy.update(&element.content, level);
            }

            let is_atomic = self.config.atomic_elements.contains(&element.element_type);

            if is_atomic {
                // Flush current buffer first
                if !current_content.is_empty() && current_content.len() >= base_config.min_size {
                    let mut chunk = self.create_chunk_with_hierarchy(
                        &current_content,
                        source_id,
                        current_page,
                        &hierarchy,
                        chunk_index,
                    );
                    chunk.chunk_type = "text".to_string();
                    chunks.push(chunk);
                    chunk_index += 1;
                    current_content = self.get_overlap(&current_content, base_config);
                }

                // Add atomic element as its own chunk
                if element.content.len() <= atomic_max {
                    let mut chunk = self.create_chunk_with_hierarchy(
                        &element.content,
                        source_id,
                        Some(element.page_number),
                        &hierarchy,
                        chunk_index,
                    );
                    // Tag with element type
                    chunk.chunk_type = format!("{:?}", element.element_type).to_lowercase();
                    chunk.metadata.insert(
                        "ttrpg_element_type".to_string(),
                        format!("{:?}", element.element_type),
                    );
                    if let Some(ref data) = element.structured_data {
                        chunk.metadata.insert(
                            "structured_data".to_string(),
                            data.to_string(),
                        );
                    }
                    chunks.push(chunk);
                    chunk_index += 1;
                } else {
                    // Forced split of oversized atomic element
                    let sub_chunks = self.split_oversized_element(
                        element,
                        source_id,
                        &hierarchy,
                        &mut chunk_index,
                    );
                    chunks.extend(sub_chunks);
                }
            } else {
                // Non-atomic: accumulate into current buffer
                current_page = Some(element.page_number);
                if !current_content.is_empty() {
                    current_content.push_str("\n\n");
                }
                current_content.push_str(&element.content);

                // Flush if buffer exceeds target
                if current_content.len() >= base_config.target_size {
                    let mut chunk = self.create_chunk_with_hierarchy(
                        &current_content,
                        source_id,
                        current_page,
                        &hierarchy,
                        chunk_index,
                    );
                    chunk.chunk_type = "text".to_string();
                    chunks.push(chunk);
                    chunk_index += 1;
                    current_content = self.get_overlap(&current_content, base_config);
                }
            }
        }

        // Final flush
        if !current_content.is_empty() && current_content.len() >= base_config.min_size {
            let mut chunk = self.create_chunk_with_hierarchy(
                &current_content,
                source_id,
                current_page,
                &hierarchy,
                chunk_index,
            );
            chunk.chunk_type = "text".to_string();
            chunks.push(chunk);
        }

        chunks
    }

    fn create_chunk_with_hierarchy(
        &self,
        content: &str,
        source_id: &str,
        page_number: Option<u32>,
        hierarchy: &SectionHierarchy,
        chunk_index: usize,
    ) -> ContentChunk {
        let mut chunk = ContentChunk {
            id: uuid::Uuid::new_v4().to_string(),
            source_id: source_id.to_string(),
            content: content.trim().to_string(),
            page_number,
            section: if hierarchy.path().is_empty() { None } else { Some(hierarchy.path()) },
            chunk_type: "text".to_string(),
            chunk_index,
            metadata: std::collections::HashMap::new(),
        };

        // Add hierarchy metadata
        if self.config.include_hierarchy {
            let parents = hierarchy.parents();
            if !parents.is_empty() {
                chunk.metadata.insert(
                    "parent_sections".to_string(),
                    parents.join(" > "),
                );
            }
            chunk.metadata.insert(
                "section_path".to_string(),
                hierarchy.path(),
            );
        }

        chunk
    }

    fn get_overlap(&self, content: &str, config: &ChunkConfig) -> String {
        let overlap_size = (content.len() as f32 * self.config.overlap_percentage) as usize;
        let overlap_size = overlap_size.min(config.overlap_size);

        if overlap_size == 0 || content.len() <= overlap_size {
            return String::new();
        }

        let overlap_start = content.len().saturating_sub(overlap_size);
        let overlap_text = &content[overlap_start..];

        // Start at word boundary
        if let Some(space_pos) = overlap_text.find(' ') {
            overlap_text[space_pos + 1..].to_string()
        } else {
            overlap_text.to_string()
        }
    }

    fn detect_header_level(text: &str) -> usize {
        // Heuristic: all caps = h1, title case with "Chapter" = h1, etc.
        let text = text.trim();
        if text.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase()) {
            return 0; // h1
        }
        if text.to_lowercase().starts_with("chapter") {
            return 0; // h1
        }
        if text.to_lowercase().starts_with("appendix") {
            return 0; // h1
        }
        if text.to_lowercase().starts_with("part") {
            return 0; // h1
        }
        // Default to h2
        1
    }

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
        let max_size = self.config.base.max_size;

        for sentence in sentences {
            if current.len() + sentence.len() > max_size && !current.is_empty() {
                let mut chunk = self.create_chunk_with_hierarchy(
                    &current,
                    source_id,
                    Some(element.page_number),
                    hierarchy,
                    *chunk_index,
                );
                chunk.chunk_type = format!("{:?}_part", element.element_type).to_lowercase();
                chunks.push(chunk);
                *chunk_index += 1;
                current.clear();
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(&sentence);
        }

        if !current.is_empty() {
            let mut chunk = self.create_chunk_with_hierarchy(
                &current,
                source_id,
                Some(element.page_number),
                hierarchy,
                *chunk_index,
            );
            chunk.chunk_type = format!("{:?}_part", element.element_type).to_lowercase();
            chunks.push(chunk);
            *chunk_index += 1;
        }

        chunks
    }
}
```

### 5. Extending SearchDocument (`core/search_client.rs`)

**Integration**: Extend the existing `SearchDocument` metadata to include TTRPG attributes.

```rust
// ADDITIONS to src/core/search_client.rs

use crate::ingestion::ttrpg::TTRPGAttributes;

/// Extended document with TTRPG-specific fields for Meilisearch
/// These fields become filterable/sortable attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTRPGSearchDocument {
    /// Base document fields
    #[serde(flatten)]
    pub base: SearchDocument,

    // TTRPG-specific filterable fields
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub damage_types: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub creature_types: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub conditions: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub alignments: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub rarities: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub sizes: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenge_rating: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u32>,

    /// Element type: stat_block, random_table, etc.
    #[serde(default)]
    pub element_type: String,
}

impl TTRPGSearchDocument {
    pub fn from_chunk_and_attributes(
        chunk: &ContentChunk,
        source_doc_id: &str,
        attributes: TTRPGAttributes,
    ) -> Self {
        Self {
            base: SearchDocument {
                id: chunk.id.clone(),
                content: chunk.content.clone(),
                source: source_doc_id.to_string(),
                source_type: "rulebook".to_string(),
                page_number: chunk.page_number,
                chunk_index: Some(chunk.chunk_index as u32),
                campaign_id: None,
                session_id: None,
                created_at: chrono::Utc::now().to_rfc3339(),
                metadata: chunk.metadata.clone(),
            },
            damage_types: attributes.damage_types,
            creature_types: attributes.creature_types,
            conditions: attributes.conditions,
            alignments: attributes.alignments,
            rarities: attributes.rarities,
            sizes: attributes.sizes,
            challenge_rating: attributes.challenge_rating,
            level: attributes.level,
            element_type: attributes.element_type,
        }
    }
}

impl SearchClient {
    /// Configure TTRPG-specific index settings
    pub async fn configure_ttrpg_index(&self, index_name: &str) -> Result<()> {
        let index = self.index(index_name);

        let settings = Settings::new()
            .with_filterable_attributes([
                "damage_types",
                "creature_types",
                "conditions",
                "alignments",
                "rarities",
                "sizes",
                "element_type",
                "challenge_rating",
                "level",
                "source",
                "page_number",
            ])
            .with_sortable_attributes([
                "challenge_rating",
                "level",
                "created_at",
            ])
            .with_searchable_attributes([
                "content",
                "element_type",
                "damage_types",
                "creature_types",
            ]);

        let task = index.set_settings(&settings).await?;
        task.wait_for_completion(&self.client, None, None).await?;

        Ok(())
    }

    /// Add TTRPG documents with attributes
    pub async fn add_ttrpg_documents(
        &self,
        index_name: &str,
        documents: &[TTRPGSearchDocument],
    ) -> Result<()> {
        let index = self.index(index_name);
        let task = index.add_documents(documents, Some("id")).await?;
        task.wait_for_completion(&self.client, None, None).await?;
        Ok(())
    }
}
```

### 6. Query Parser (`core/ttrpg_search/query_parser.rs`)

**Purpose**: Extract constraints, negations, and named entities from user queries.

```rust
// src/core/ttrpg_search/query_parser.rs

use serde::{Deserialize, Serialize};
use crate::ingestion::ttrpg::{GameVocabulary, DnD5eVocabulary};

/// Parsed query with extracted constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryConstraints {
    /// Original query text
    pub original_query: String,
    /// Query text for semantic search (with constraints removed for cleaner embedding)
    pub semantic_query: String,
    /// Expanded query text (with antonym hints appended)
    pub expanded_query: String,
    /// Required attribute values (must be present)
    pub required_attributes: Vec<RequiredAttribute>,
    /// Excluded attribute values (must NOT be present) - from negations
    pub excluded_attributes: Vec<String>,
    /// CR range filter
    pub cr_range: Option<(f32, f32)>,
    /// Level range filter
    pub level_range: Option<(u32, u32)>,
    /// Named entities for exact-match boosting
    pub exact_match_entities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredAttribute {
    pub category: String,  // "damage_type", "creature_type", etc.
    pub value: String,
}

/// Parses user queries to extract TTRPG-specific constraints
pub struct QueryParser {
    vocabulary: Box<dyn GameVocabulary>,
    antonym_mapper: super::antonym_scorer::AntonymMapper,
}

impl QueryParser {
    pub fn new() -> Self {
        Self {
            vocabulary: Box::new(DnD5eVocabulary),
            antonym_mapper: super::antonym_scorer::AntonymMapper::new(),
        }
    }

    /// Parse query and extract all constraints
    pub fn parse(&self, query: &str) -> QueryConstraints {
        let query_lower = query.to_lowercase();
        let mut required = Vec::new();
        let mut excluded = Vec::new();
        let mut semantic_parts = Vec::new();

        // Extract negations first (remove from query for semantic search)
        let negation_re = regex::Regex::new(
            r"(?i)\b(not|without|except|excluding|no)\s+(\w+)"
        ).unwrap();

        let mut cleaned_query = query.to_string();
        for cap in negation_re.captures_iter(&query_lower) {
            if let Some(term) = cap.get(2) {
                let term_str = term.as_str();
                // Check if it's a known attribute
                if self.is_known_attribute(term_str) {
                    excluded.push(term_str.to_string());
                }
                // Remove negation phrase from semantic query
                if let Some(full) = cap.get(0) {
                    cleaned_query = cleaned_query.replace(full.as_str(), "");
                }
            }
        }

        // Extract damage types
        for dtype in self.vocabulary.damage_types() {
            if query_lower.contains(dtype) && !excluded.contains(&dtype.to_string()) {
                required.push(RequiredAttribute {
                    category: "damage_type".to_string(),
                    value: dtype.to_string(),
                });
            }
        }

        // Extract creature types
        for ctype in self.vocabulary.creature_types() {
            if query_lower.contains(ctype) && !excluded.contains(&ctype.to_string()) {
                required.push(RequiredAttribute {
                    category: "creature_type".to_string(),
                    value: ctype.to_string(),
                });
            }
        }

        // Extract CR range
        let cr_range = self.extract_cr_range(&query_lower);

        // Extract level range
        let level_range = self.extract_level_range(&query_lower);

        // Extract named entities (capitalized phrases)
        let entities = self.extract_named_entities(query);

        // Build expanded query with antonym hints
        let expanded = self.build_expanded_query(query, &required, &excluded);

        QueryConstraints {
            original_query: query.to_string(),
            semantic_query: cleaned_query.trim().to_string(),
            expanded_query: expanded,
            required_attributes: required,
            excluded_attributes: excluded,
            cr_range,
            level_range,
            exact_match_entities: entities,
        }
    }

    fn is_known_attribute(&self, term: &str) -> bool {
        self.vocabulary.damage_types().contains(&term) ||
        self.vocabulary.creature_types().contains(&term) ||
        self.vocabulary.conditions().contains(&term)
    }

    fn extract_cr_range(&self, query: &str) -> Option<(f32, f32)> {
        // "CR 5" -> (5.0, 5.0)
        // "CR 5+" -> (5.0, 30.0)
        // "CR 1-5" -> (1.0, 5.0)
        let exact_re = regex::Regex::new(r"(?i)cr\s*(\d+(?:/\d+)?)\s*$").ok()?;
        let range_re = regex::Regex::new(r"(?i)cr\s*(\d+)\s*[-–]\s*(\d+)").ok()?;
        let min_re = regex::Regex::new(r"(?i)cr\s*(\d+)\s*\+").ok()?;

        if let Some(cap) = range_re.captures(query) {
            let min: f32 = cap.get(1)?.as_str().parse().ok()?;
            let max: f32 = cap.get(2)?.as_str().parse().ok()?;
            return Some((min, max));
        }

        if let Some(cap) = min_re.captures(query) {
            let min: f32 = cap.get(1)?.as_str().parse().ok()?;
            return Some((min, 30.0)); // Max possible CR
        }

        if let Some(cap) = exact_re.captures(query) {
            let cr_str = cap.get(1)?.as_str();
            let cr = if cr_str.contains('/') {
                let parts: Vec<&str> = cr_str.split('/').collect();
                parts[0].parse::<f32>().ok()? / parts[1].parse::<f32>().ok()?
            } else {
                cr_str.parse().ok()?
            };
            return Some((cr, cr));
        }

        None
    }

    fn extract_level_range(&self, query: &str) -> Option<(u32, u32)> {
        let exact_re = regex::Regex::new(r"(?i)level\s*(\d+)").ok()?;
        let range_re = regex::Regex::new(r"(?i)level\s*(\d+)\s*[-–]\s*(\d+)").ok()?;

        if let Some(cap) = range_re.captures(query) {
            let min: u32 = cap.get(1)?.as_str().parse().ok()?;
            let max: u32 = cap.get(2)?.as_str().parse().ok()?;
            return Some((min, max));
        }

        if let Some(cap) = exact_re.captures(query) {
            let level: u32 = cap.get(1)?.as_str().parse().ok()?;
            return Some((level, level));
        }

        None
    }

    fn extract_named_entities(&self, query: &str) -> Vec<String> {
        let mut entities = Vec::new();
        let entity_re = regex::Regex::new(
            r"\b([A-Z][a-z]+(?:'s)?\s+[A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)\b"
        ).ok();

        if let Some(re) = entity_re {
            for cap in re.captures_iter(query) {
                if let Some(m) = cap.get(1) {
                    entities.push(m.as_str().to_string());
                }
            }
        }

        entities
    }

    fn build_expanded_query(
        &self,
        original: &str,
        required: &[RequiredAttribute],
        excluded: &[String],
    ) -> String {
        let mut expanded = original.to_string();

        // Add antonym hints for required attributes
        for attr in required {
            if let Some(antonyms) = self.antonym_mapper.get_antonyms(&attr.value) {
                for antonym in antonyms {
                    expanded.push_str(&format!(" NOT {}", antonym));
                }
            }
        }

        expanded
    }
}
```

### 6.1 Antonym Scorer (`core/ttrpg_search/antonym_scorer.rs`)

**Purpose**: Apply penalties for semantically opposite attributes.

```rust
// src/core/ttrpg_search/antonym_scorer.rs

use std::collections::HashMap;

/// Maps attributes to their antonyms for penalty scoring
pub struct AntonymMapper {
    /// Bidirectional antonym pairs
    antonyms: HashMap<String, Vec<String>>,
    /// Penalty multiplier for antonym presence (0.0-1.0)
    penalty_multiplier: f32,
}

impl AntonymMapper {
    pub fn new() -> Self {
        let mut antonyms = HashMap::new();

        // Damage type antonyms
        antonyms.insert("fire".to_string(), vec!["cold".to_string()]);
        antonyms.insert("cold".to_string(), vec!["fire".to_string()]);
        antonyms.insert("radiant".to_string(), vec!["necrotic".to_string()]);
        antonyms.insert("necrotic".to_string(), vec!["radiant".to_string()]);
        antonyms.insert("lightning".to_string(), vec!["thunder".to_string()]); // Loose association

        // Alignment antonyms
        antonyms.insert("lawful".to_string(), vec!["chaotic".to_string()]);
        antonyms.insert("chaotic".to_string(), vec!["lawful".to_string()]);
        antonyms.insert("good".to_string(), vec!["evil".to_string()]);
        antonyms.insert("evil".to_string(), vec!["good".to_string()]);

        Self {
            antonyms,
            penalty_multiplier: 0.1, // Severe penalty
        }
    }

    /// Get antonyms for an attribute
    pub fn get_antonyms(&self, attr: &str) -> Option<&Vec<String>> {
        self.antonyms.get(&attr.to_lowercase())
    }

    /// Check if two attributes are antonyms
    pub fn are_antonyms(&self, a: &str, b: &str) -> bool {
        if let Some(antonyms) = self.get_antonyms(a) {
            return antonyms.iter().any(|ant| ant == &b.to_lowercase());
        }
        false
    }

    /// Calculate penalty for a result based on query attributes
    /// Returns 1.0 for no penalty, < 1.0 for penalty
    pub fn calculate_penalty(
        &self,
        query_attrs: &[String],
        result_attrs: &[String],
    ) -> f32 {
        let result_set: std::collections::HashSet<_> = result_attrs.iter()
            .map(|s| s.to_lowercase())
            .collect();

        for query_attr in query_attrs {
            if let Some(antonyms) = self.get_antonyms(query_attr) {
                for antonym in antonyms {
                    if result_set.contains(antonym) {
                        return self.penalty_multiplier;
                    }
                }
            }
        }

        1.0 // No penalty
    }
}

### 6.2 Result Ranker with RRF (`core/ttrpg_search/result_ranker.rs`)

**Purpose**: Combine dense and sparse search results using Reciprocal Rank Fusion and apply attribute scoring.

```rust
// src/core/ttrpg_search/result_ranker.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Score breakdown for debugging and transparency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// Semantic/vector similarity score (0.0-1.0)
    pub semantic_score: f32,
    /// Keyword/BM25 score (normalized 0.0-1.0)
    pub keyword_score: f32,
    /// Bonus for matching required attributes
    pub attribute_match_bonus: f32,
    /// Penalty for antonym presence (1.0 = no penalty, 0.1 = heavy penalty)
    pub antonym_penalty: f32,
    /// Boost for exact entity match
    pub exact_match_boost: f32,
    /// Final combined score
    pub final_score: f32,
}

/// Configuration for result ranking
#[derive(Debug, Clone)]
pub struct RankingConfig {
    /// RRF constant (k parameter, typically 60)
    pub rrf_k: f32,
    /// Weight for semantic score in final blend (0.0-1.0)
    pub semantic_weight: f32,
    /// Weight for keyword score in final blend (0.0-1.0)
    pub keyword_weight: f32,
    /// Bonus per matching required attribute
    pub attribute_match_bonus: f32,
    /// Boost for exact entity match
    pub exact_match_boost: f32,
    /// Whether to use hard veto for excluded attributes
    pub hard_exclude_veto: bool,
}

impl Default for RankingConfig {
    fn default() -> Self {
        Self {
            rrf_k: 60.0,
            semantic_weight: 0.6,
            keyword_weight: 0.4,
            attribute_match_bonus: 0.15,
            exact_match_boost: 0.2,
            hard_exclude_veto: true,
        }
    }
}

/// Candidate result from a single search
#[derive(Debug, Clone)]
pub struct SearchCandidate {
    pub doc_id: String,
    pub rank: usize,
    pub score: f32,
}

/// Final ranked result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedResult {
    pub doc_id: String,
    pub score_breakdown: ScoreBreakdown,
    /// Whether this result was vetoed (excluded attribute present)
    pub vetoed: bool,
}

/// Ranks and fuses search results
pub struct ResultRanker {
    config: RankingConfig,
    antonym_mapper: super::antonym_scorer::AntonymMapper,
}

impl ResultRanker {
    pub fn new(config: RankingConfig) -> Self {
        Self {
            config,
            antonym_mapper: super::antonym_scorer::AntonymMapper::new(),
        }
    }

    /// Fuse results from dense and sparse searches using RRF
    pub fn fuse_rrf(
        &self,
        dense_results: &[SearchCandidate],
        sparse_results: &[SearchCandidate],
    ) -> HashMap<String, (f32, f32)> {
        let mut fused: HashMap<String, (f32, f32)> = HashMap::new();

        // RRF score: 1 / (k + rank)
        for (i, candidate) in dense_results.iter().enumerate() {
            let rrf_score = 1.0 / (self.config.rrf_k + i as f32 + 1.0);
            fused.entry(candidate.doc_id.clone())
                .and_modify(|(dense, _)| *dense += rrf_score)
                .or_insert((rrf_score, 0.0));
        }

        for (i, candidate) in sparse_results.iter().enumerate() {
            let rrf_score = 1.0 / (self.config.rrf_k + i as f32 + 1.0);
            fused.entry(candidate.doc_id.clone())
                .and_modify(|(_, sparse)| *sparse += rrf_score)
                .or_insert((0.0, rrf_score));
        }

        fused
    }

    /// Rank results with full scoring pipeline
    pub fn rank(
        &self,
        dense_results: &[SearchCandidate],
        sparse_results: &[SearchCandidate],
        constraints: &super::query_parser::QueryConstraints,
        doc_attributes: &HashMap<String, Vec<String>>, // doc_id -> attributes
    ) -> Vec<RankedResult> {
        let fused = self.fuse_rrf(dense_results, sparse_results);
        let mut results = Vec::new();

        let required_values: Vec<String> = constraints.required_attributes
            .iter()
            .map(|a| a.value.clone())
            .collect();

        for (doc_id, (dense_rrf, sparse_rrf)) in fused {
            let doc_attrs = doc_attributes.get(&doc_id)
                .cloned()
                .unwrap_or_default();

            // Check for hard veto
            let mut vetoed = false;
            if self.config.hard_exclude_veto {
                for excluded in &constraints.excluded_attributes {
                    if doc_attrs.iter().any(|a| a.to_lowercase() == excluded.to_lowercase()) {
                        vetoed = true;
                        break;
                    }
                }
            }

            // Calculate component scores
            let semantic_score = dense_rrf;
            let keyword_score = sparse_rrf;

            // Attribute match bonus
            let mut attr_bonus = 0.0;
            for req in &constraints.required_attributes {
                if doc_attrs.iter().any(|a| a.to_lowercase() == req.value.to_lowercase()) {
                    attr_bonus += self.config.attribute_match_bonus;
                }
            }

            // Antonym penalty
            let antonym_penalty = self.antonym_mapper.calculate_penalty(
                &required_values,
                &doc_attrs,
            );

            // Exact match boost
            let mut exact_boost = 0.0;
            for entity in &constraints.exact_match_entities {
                // Would need doc content here; simplified for now
                exact_boost += self.config.exact_match_boost;
            }

            // Combine scores
            let base_score = (self.config.semantic_weight * semantic_score)
                + (self.config.keyword_weight * keyword_score);
            let boosted = base_score + attr_bonus + exact_boost;
            let final_score = if vetoed { 0.0 } else { boosted * antonym_penalty };

            results.push(RankedResult {
                doc_id,
                score_breakdown: ScoreBreakdown {
                    semantic_score,
                    keyword_score,
                    attribute_match_bonus: attr_bonus,
                    antonym_penalty,
                    exact_match_boost: exact_boost,
                    final_score,
                },
                vetoed,
            });
        }

        // Sort by final score descending
        results.sort_by(|a, b| {
            b.score_breakdown.final_score
                .partial_cmp(&a.score_breakdown.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Filter out vetoed results
        results.into_iter()
            .filter(|r| !r.vetoed)
            .collect()
    }
}
```

### 6.3 Index Queue (`core/ttrpg_search/index_queue.rs`)

**Purpose**: Queue chunks for Meilisearch indexing with retry logic when the service is unavailable.

```rust
// src/core/ttrpg_search/index_queue.rs

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, interval};

/// Document pending indexing
#[derive(Debug, Clone)]
pub struct PendingDocument {
    pub id: String,
    pub payload: serde_json::Value,
    pub attempts: u32,
    pub created_at: std::time::Instant,
}

/// Queue for failed/pending index operations
pub struct IndexQueue {
    queue: Arc<Mutex<VecDeque<PendingDocument>>>,
    max_retries: u32,
    retry_delay: Duration,
}

impl IndexQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            max_retries: 5,
            retry_delay: Duration::from_secs(30),
        }
    }

    /// Add a document to the retry queue
    pub fn enqueue(&self, id: String, payload: serde_json::Value) {
        let mut queue = self.queue.lock().unwrap();
        queue.push_back(PendingDocument {
            id,
            payload,
            attempts: 0,
            created_at: std::time::Instant::now(),
        });
        log::info!("Queued document {} for retry (queue size: {})", id, queue.len());
    }

    /// Get next document to retry
    pub fn dequeue(&self) -> Option<PendingDocument> {
        let mut queue = self.queue.lock().unwrap();
        queue.pop_front()
    }

    /// Return a failed document to the queue with incremented attempt count
    pub fn requeue(&self, mut doc: PendingDocument) {
        doc.attempts += 1;
        if doc.attempts < self.max_retries {
            let mut queue = self.queue.lock().unwrap();
            queue.push_back(doc);
        } else {
            log::error!(
                "Document {} exceeded max retries ({}), dropping",
                doc.id, self.max_retries
            );
        }
    }

    pub fn len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
```

### 7. Database Migration

**File**: `src/database/migrations/YYYYMMDD_ttrpg_source_documents.sql`

```sql
-- TTRPG Source Document Tracking
-- file_hash uses BLAKE3 for fast, secure duplicate detection
CREATE TABLE IF NOT EXISTS ttrpg_source_documents (
    id TEXT PRIMARY KEY,
    filename TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_hash TEXT NOT NULL UNIQUE,  -- BLAKE3 hash (64 hex chars)
    file_size INTEGER NOT NULL,
    page_count INTEGER,
    game_system TEXT,  -- 'dnd5e', 'pf2e', 'coc', etc.
    processing_status TEXT NOT NULL DEFAULT 'pending',
    -- Status values: 'pending', 'processing', 'completed', 'failed', 'partial'
    chunk_count INTEGER DEFAULT 0,
    entity_count INTEGER DEFAULT 0,
    error_message TEXT,
    processing_time_ms INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for duplicate detection by hash
CREATE INDEX IF NOT EXISTS idx_source_docs_hash ON ttrpg_source_documents(file_hash);
-- Index for listing by status
CREATE INDEX IF NOT EXISTS idx_source_docs_status ON ttrpg_source_documents(processing_status);

-- TTRPG Extracted Entities (stat blocks, spells, items)
CREATE TABLE IF NOT EXISTS ttrpg_entities (
    id TEXT PRIMARY KEY,
    source_document_id TEXT NOT NULL REFERENCES ttrpg_source_documents(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL,  -- 'stat_block', 'spell', 'item', 'random_table'
    name TEXT NOT NULL,
    page_number INTEGER,
    section_path TEXT,  -- Hierarchical section context
    structured_data TEXT,  -- JSON blob with parsed fields
    meilisearch_id TEXT,  -- Reference to Meilisearch document
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_entities_source ON ttrpg_entities(source_document_id);
CREATE INDEX IF NOT EXISTS idx_entities_type ON ttrpg_entities(entity_type);
CREATE INDEX IF NOT EXISTS idx_entities_name ON ttrpg_entities(name);

-- Index queue for failed Meilisearch operations
CREATE TABLE IF NOT EXISTS ttrpg_index_queue (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL,
    payload TEXT NOT NULL,  -- JSON blob
    attempts INTEGER DEFAULT 0,
    last_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    next_retry_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_queue_retry ON ttrpg_index_queue(next_retry_at);
```

### 7.1 BLAKE3 Hashing Utility

**File**: `src/ingestion/hash.rs`

```rust
// src/ingestion/hash.rs

use std::path::Path;

/// Compute BLAKE3 hash of a file
pub fn hash_file(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    let hash = blake3::hash(&bytes);
    Ok(hash.to_hex().to_string())
}

/// Compute BLAKE3 hash of bytes
pub fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// Check if a file with this hash already exists in the database
pub async fn check_duplicate(
    pool: &sqlx::SqlitePool,
    file_hash: &str,
) -> Result<Option<String>, sqlx::Error> {
    let result = sqlx::query_scalar::<_, String>(
        "SELECT id FROM ttrpg_source_documents WHERE file_hash = ?"
    )
    .bind(file_hash)
    .fetch_optional(pool)
    .await?;

    Ok(result)
}
```

### 8. Tauri Command Handlers (`commands.rs`)

**Integration**: Add commands for TTRPG-enhanced ingestion.

```rust
// ADDITIONS to src/commands.rs

use crate::ingestion::ttrpg::{TTRPGClassifier, AttributeExtractor, TTRPGChunkConfig};
use crate::core::search_client::TTRPGSearchDocument;

/// Ingest a document with TTRPG-specific processing
#[tauri::command]
pub async fn ingest_ttrpg_document(
    state: tauri::State<'_, AppState>,
    file_path: String,
    game_system: Option<String>,
) -> Result<IngestionResult, String> {
    let path = std::path::Path::new(&file_path);

    // 1. Parse document
    let extracted = match path.extension().and_then(|e| e.to_str()) {
        Some("pdf") => PDFParser::extract_structured(path)
            .map_err(|e| e.to_string())?,
        Some("epub") => {
            // Convert EPUB to similar structure
            let epub = EPUBParser::extract(path).map_err(|e| e.to_string())?;
            // ... conversion
            todo!()
        },
        _ => return Err("Unsupported file type".to_string()),
    };

    // 2. Classify elements
    let classifier = TTRPGClassifier::new();
    let mut classified_elements = Vec::new();
    for page in &extracted.pages {
        for para in &page.paragraphs {
            classified_elements.push(classifier.classify(para, page.page_number));
        }
    }

    // 3. Chunk with TTRPG awareness
    let chunker = SemanticChunker::new();
    let ttrpg_config = TTRPGChunkConfig::default();
    let source_id = uuid::Uuid::new_v4().to_string();
    let chunks = chunker.chunk_ttrpg(&classified_elements, &source_id, &ttrpg_config);

    // 4. Extract attributes and prepare documents
    let extractor = AttributeExtractor::new();
    let mut ttrpg_docs = Vec::new();

    for chunk in &chunks {
        let mut attrs = extractor.extract(&chunk.content);
        attrs.element_type = chunk.chunk_type.clone();
        ttrpg_docs.push(TTRPGSearchDocument::from_chunk_and_attributes(
            chunk,
            &source_id,
            attrs,
        ));
    }

    // 5. Index to Meilisearch
    let search_client = state.search_client.lock().await;
    search_client.add_ttrpg_documents(INDEX_RULES, &ttrpg_docs)
        .await
        .map_err(|e| e.to_string())?;

    // 6. Record in SQLite
    // ... database insertion

    Ok(IngestionResult {
        document_id: source_id,
        chunk_count: chunks.len(),
        page_count: extracted.page_count,
    })
}

#[derive(Serialize)]
pub struct IngestionResult {
    pub document_id: String,
    pub chunk_count: usize,
    pub page_count: usize,
}
```

## Dependency Updates

**Additions to `Cargo.toml`**:

```toml
[dependencies]
# Existing dependencies remain...

# Add regex for pattern matching
regex = "1.10"

# BLAKE3 for fast file hashing
blake3 = "1.5"

# pdf-extract as fallback PDF parser
pdf-extract = "0.7"
```

All other dependencies are already present in the existing codebase (`serde`, `sqlx`, `meilisearch-sdk`, `lopdf`, `thiserror`, `tokio`, `uuid`).

## Testing Strategy

### Unit Tests

Located alongside modules following existing pattern:

```rust
// src/ingestion/ttrpg/stat_block.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_stat_block() {
        let text = r#"
Goblin
Small humanoid (goblinoid), neutral evil

Armor Class 15 (leather armor, shield)
Hit Points 7 (2d6)
Speed 30 ft.

STR 8 (-1) DEX 14 (+2) CON 10 (+0) INT 10 (+0) WIS 8 (-1) CHA 8 (-1)
"#;

        let data = StatBlockData::parse(text);
        assert_eq!(data.name, "Goblin");
        assert_eq!(data.armor_class, Some(15));
        assert_eq!(data.ability_scores.strength, Some(8));
        assert_eq!(data.ability_scores.dexterity, Some(14));
    }

    #[test]
    fn test_parse_damage_types() {
        let text = "Damage Immunities fire, poison\nDamage Resistances cold, lightning";
        let data = StatBlockData::parse(text);
        assert!(data.damage_immunities.contains(&"fire".to_string()));
        assert!(data.damage_immunities.contains(&"poison".to_string()));
    }
}
```

### Integration Tests

Located in `src/tests/` following existing pattern:

```rust
// src/tests/ttrpg_ingestion_test.rs

#[tokio::test]
async fn test_full_ttrpg_ingestion_pipeline() {
    // Set up test Meilisearch
    // Ingest sample PDF
    // Verify chunks in Meilisearch
    // Verify attribute filtering works
}
```

## Gap Resolution Summary

This design addresses all identified gaps from the requirements analysis:

| Gap | Requirement | Resolution |
|-----|-------------|------------|
| **PDF Layout Detection** | 1.1-1.4 | Added `layout/` module with `ColumnDetector`, `RegionDetector`, `TableExtractor` |
| **Fallback Extraction** | 1.4 | Added `PDFParser::extract_with_fallback()` using `pdf-extract` crate |
| **Password-Protected PDFs** | 1.6 | Added `password: Option<&str>` parameter to extraction methods |
| **Multi-Page Table Continuation** | 4.4 | Added `TableExtractor::merge_continuation_tables()` |
| **Nested Sub-Tables** | 4.5 | `ExtractedTable` supports hierarchical structure via `rows` nesting |
| **Hierarchical Context in Chunks** | 5.4 | Added `SectionHierarchy` tracker, chunks include `section_path` metadata |
| **Chunk Overlap** | 5.7 | Added `TTRPGChunkConfig.overlap_percentage` (default 12%) |
| **Negation Extraction** | 9.5 | Added `QueryParser` with negation regex, populates `excluded_attributes` |
| **Named Entity Boosting** | 9.6 | Added `exact_match_entities` in `QueryConstraints`, boost in `ResultRanker` |
| **Score Breakdown** | 10.4 | Added `ScoreBreakdown` struct with all component scores |
| **Attribute Confidence Scores** | processing2.md | Added `AttributeMatch` with `confidence: f32` and `AttributeSource` |
| **Game System Auto-Detection** | 12.4 | Added `detect_game_system()` function with indicator pattern matching |
| **BLAKE3 Hashing** | 11.1 | Added `hash.rs` module using `blake3` crate |
| **Hard Filter vs Soft Penalty** | processing2.md | Added `hard_exclude_veto` in `RankingConfig`, pre-filter in `ResultRanker` |
| **RRF Fusion** | processing1-3.md | Added `ResultRanker::fuse_rrf()` with configurable k parameter |
| **Query Expansion with Antonyms** | processing2.md | Added `expanded_query` in `QueryConstraints` with antonym hints |
| **Meilisearch Unavailability Queue** | NFR Reliability 2 | Added `IndexQueue` with retry logic and database table |
| **Chunker Config Mismatch** | Design code | Changed from `impl SemanticChunker` to `TTRPGChunker` wrapper with proper config access |

### Key Architectural Decisions

1. **Composition over Inheritance**: `TTRPGChunker` wraps `SemanticChunker` rather than extending it
2. **Confidence-Based Filtering**: Attributes track confidence scores to enable hard vs soft filtering
3. **Two-Phase Search**: Pre-filter (hard exclusions) + Post-rank (soft penalties/boosts)
4. **Fail-Safe Indexing**: Queue mechanism ensures no data loss when Meilisearch is unavailable
5. **Pluggable Vocabularies**: `GameVocabulary` trait enables multi-system support with auto-detection
