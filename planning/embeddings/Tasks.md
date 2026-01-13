# Tasks: TTRPG Document Parsing & Embedding System

## Implementation Overview

This implementation extends the existing `ttrpg-assistant` Rust/Tauri application. All tasks modify or add to the existing `src-tauri/src/` codebase. The implementation follows a layered approach: first adding new modules, then integrating with existing code.

### File Legend

| Symbol | Meaning |
|--------|---------|
| `[NEW]` | Create new file |
| `[MODIFY]` | Edit existing file |
| `[EXTEND]` | Add functions/structs to existing file |

### Existing Files Reference

```
src-tauri/src/
├── ingestion/
│   ├── mod.rs              # Exports parsers and chunker
│   ├── pdf_parser.rs       # PDFParser, ExtractedDocument, ExtractedPage
│   ├── epub_parser.rs      # EPUBParser, ExtractedEPUB
│   ├── chunker.rs          # SemanticChunker, ChunkConfig, ContentChunk
│   └── ...
├── core/
│   ├── search_client.rs    # SearchClient, SearchDocument, INDEX_RULES
│   ├── query_expansion.rs  # Query enhancement
│   └── ...
├── database/
│   └── (sqlx migrations)
└── commands.rs             # Tauri command handlers
```

---

## Phase 0: PDF Parser Extensions

### Task 0.1: Add fallback extraction with pdf-extract

**Files:**
- `[EXTEND] src/ingestion/pdf_parser.rs`

**Description:**
Extend existing PDF parser with fallback extraction and quality validation.

```rust
impl PDFParser {
    /// Extract with automatic fallback if lopdf fails or produces low-quality output
    pub fn extract_with_fallback(
        path: &Path,
        password: Option<&str>,
    ) -> Result<ExtractedDocument>;

    /// Check extraction quality (detect garbled output)
    fn is_extraction_quality_acceptable(doc: &ExtractedDocument) -> bool;

    /// Fallback extraction using pdf-extract crate
    fn extract_with_pdf_extract(path: &Path) -> Result<ExtractedDocument>;
}
```

**Validation:** Handles garbled PDF output gracefully.
- _Requirements: 1.4, 1.5_

---

### Task 0.2: Add password-protected PDF support

**Files:**
- `[EXTEND] src/ingestion/pdf_parser.rs`

**Description:**
Add password parameter to PDF extraction methods.

```rust
fn extract_structured_internal(
    path: &Path,
    password: Option<&str>,
) -> Result<ExtractedDocument>
```

**Test:** Load password-protected test PDF.
- _Requirements: 1.6_

---

### Task 0.3: Add BLAKE3 file hashing utility

**Files:**
- `[NEW] src/ingestion/hash.rs`

**Description:**
Add file hashing for duplicate detection.

```rust
/// Compute BLAKE3 hash of a file
pub fn hash_file(path: &Path) -> std::io::Result<String>;

/// Compute BLAKE3 hash of bytes
pub fn hash_bytes(bytes: &[u8]) -> String;

/// Check if a file with this hash already exists
pub async fn check_duplicate(pool: &SqlitePool, hash: &str) -> Result<Option<String>>;
```

**Dependencies:** Add `blake3 = "1.5"` to Cargo.toml.
- _Requirements: 11.1, 11.2_

---

## Phase 0.5: Layout Detection Module

### Task 0.5.1: Create layout module structure

**Files:**
- `[NEW] src/ingestion/layout/mod.rs`

**Description:**
Create layout analysis submodule for complex PDF parsing.

```rust
pub mod column_detector;
pub mod region_detector;
pub mod table_extractor;

pub use column_detector::{ColumnDetector, ColumnBoundary, TextBlock};
pub use region_detector::{RegionDetector, DetectedRegion, RegionType};
pub use table_extractor::{TableExtractor, ExtractedTable};
```

---

### Task 0.5.2: Implement ColumnDetector

**Files:**
- `[NEW] src/ingestion/layout/column_detector.rs`

**Description:**
Detect multi-column layouts and reorder text to logical reading order.

**Key Types:**
- `ColumnBoundary` struct (left, right, top, bottom)
- `TextBlock` struct (text, x, y, width, height)
- `ColumnDetector::reorder_text_by_columns()` method

**Logic:**
1. Analyze X positions to find column gaps
2. Group text blocks by column
3. Sort by column, then by Y within column

**Test:** Two-column PDF page reorders correctly.
- _Requirements: 1.1, 1.2_

---

### Task 0.5.3: Implement RegionDetector

**Files:**
- `[NEW] src/ingestion/layout/region_detector.rs`

**Description:**
Detect boxed/shaded regions (sidebars, callouts, read-aloud text).

**Key Types:**
- `RegionType` enum (Sidebar, Callout, ReadAloud, Table, StatBlock, Normal)
- `DetectedRegion` struct with content, confidence, bounds
- `RegionDetector::detect_from_text()` method

**Heuristics:**
- Read-aloud: "read aloud", "boxed text" indicators
- Sidebars: "note:", "tip:", "variant:" prefixes

**Test:** Detect sidebar and read-aloud regions.
- _Requirements: 2.3, 2.4_

---

### Task 0.5.4: Implement TableExtractor

**Files:**
- `[NEW] src/ingestion/layout/table_extractor.rs`

**Description:**
Extract table structure with multi-page continuation support.

**Key Types:**
- `ExtractedTable` struct (title, headers, rows, page_numbers, is_continuation)
- `TableExtractor::is_table_continuation()` - detect continuation patterns
- `TableExtractor::merge_continuation_tables()` - merge across pages

**Patterns:**
- "(continued)" or "Table X (cont" indicate continuation

**Test:** Merge multi-page table correctly.
- _Requirements: 1.3, 4.4, 4.5_

---

### Task 0.5.5: Update ingestion/mod.rs with layout exports

**Files:**
- `[MODIFY] src/ingestion/mod.rs`

**Changes:**
```rust
pub mod layout;

pub use layout::{ColumnDetector, RegionDetector, TableExtractor, DetectedRegion};
```

---

## Phase 1: New TTRPG Module Structure

### Task 1.1: Create TTRPG module directory structure

**Files:**
- `[NEW] src/ingestion/ttrpg/mod.rs`

**Description:**
Create the TTRPG submodule within ingestion with all exports.

```rust
// src/ingestion/ttrpg/mod.rs
pub mod classifier;
pub mod stat_block;
pub mod random_table;
pub mod attribute_extractor;
pub mod vocabulary;
pub mod game_detector;

pub use classifier::{TTRPGClassifier, TTRPGElementType, ClassifiedElement};
pub use stat_block::{StatBlockData, AbilityScores, Feature};
pub use random_table::{RandomTableData, TableEntry};
pub use attribute_extractor::{
    AttributeExtractor, TTRPGAttributes, AttributeMatch, AttributeSource,
    GameVocabulary, FilterableFields,
};
pub use vocabulary::DnD5eVocabulary;
pub use game_detector::{detect_game_system, GameSystem};
```

**Validation:** `cargo check` passes with new module.

---

### Task 1.2: Implement TTRPGElementType enum and classifier

**Files:**
- `[NEW] src/ingestion/ttrpg/classifier.rs`

**Description:**
Implement element classification with regex-based pattern matching.

**Key Types:**
- `TTRPGElementType` enum (StatBlock, RandomTable, ReadAloudText, etc.)
- `ClassifiedElement` struct with confidence score
- `TTRPGClassifier` with `classify()` method

**Patterns to detect:**
- Stat blocks: "Armor Class", "Hit Points", ability score blocks
- Random tables: dice notation (d6, d20) + range patterns (1-3, 4-6)
- Read-aloud text: italic markers, box indicators (if available from parser)

**Test:** Unit tests for each element type detection.
- _Requirements: 2.1-2.7_

---

### Task 1.3: Implement StatBlockData parser

**Files:**
- `[NEW] src/ingestion/ttrpg/stat_block.rs`

**Description:**
Parse stat block text into structured `StatBlockData`.

**Fields to extract:**
- Name (first line)
- Size/type/alignment line
- Armor Class (with armor type)
- Hit Points (with dice notation)
- Speed (walk, fly, swim, etc.)
- Ability scores (STR/DEX/CON/INT/WIS/CHA)
- Saving throws, skills
- Damage resistances/immunities/vulnerabilities
- Condition immunities
- Senses, languages
- Challenge Rating
- Traits, Actions, Reactions, Legendary Actions

**Test:** Parse SRD stat blocks (Goblin, Zombie, Adult Red Dragon).
- _Requirements: 3.1-3.7_

---

### Task 1.4: Implement RandomTableData parser

**Files:**
- `[NEW] src/ingestion/ttrpg/random_table.rs`

**Description:**
Parse random table text into structured `RandomTableData`.

**Extraction:**
- Table title
- Dice notation (d4, d6, 2d6, d100, etc.)
- Roll ranges with probabilities
- Result text for each range

```rust
pub struct RandomTableData {
    pub title: String,
    pub dice_notation: String,
    pub entries: Vec<TableEntry>,
}

pub struct TableEntry {
    pub roll_min: u32,
    pub roll_max: u32,
    pub probability: f32,  // Calculated from dice
    pub result: String,
}
```

**Test:** Parse sample random tables with different dice types.
- _Requirements: 4.1-4.5_

---

### Task 1.5: Implement GameVocabulary trait and D&D 5e vocabulary

**Files:**
- `[NEW] src/ingestion/ttrpg/vocabulary.rs`

**Description:**
Define trait for game system vocabularies and implement D&D 5e.

```rust
pub trait GameVocabulary: Send + Sync {
    fn damage_types(&self) -> &[&str];
    fn creature_types(&self) -> &[&str];
    fn conditions(&self) -> &[&str];
    fn spell_schools(&self) -> &[&str];
    fn rarities(&self) -> &[&str];
    fn sizes(&self) -> &[&str];
    fn ability_abbreviations(&self) -> &[(&str, &str)];
}

pub struct DnD5eVocabulary;
impl GameVocabulary for DnD5eVocabulary { ... }
```

**Test:** Vocabulary completeness tests.
- _Requirements: 12.1-12.4_

---

### Task 1.6: Implement AttributeExtractor with confidence scores

**Files:**
- `[NEW] src/ingestion/ttrpg/attribute_extractor.rs`

**Description:**
Extract filterable TTRPG attributes from text content with confidence scoring.

**Key Types:**
```rust
pub enum AttributeSource {
    ExactMatch,      // Word-boundary match (high confidence)
    PatternMatch,    // Substring match (medium confidence)
    Inferred,        // Context-based inference (low confidence)
    StructuredData,  // From parsed stat block (high confidence)
}

pub struct AttributeMatch {
    pub value: String,
    pub confidence: f32,  // 0.0-1.0
    pub source: AttributeSource,
}

pub struct TTRPGAttributes {
    pub damage_types: Vec<AttributeMatch>,
    pub creature_types: Vec<AttributeMatch>,
    pub alignments: Vec<AttributeMatch>,
    // ... all fields use AttributeMatch for confidence tracking
}
```

**Extracts:**
- Damage types (fire, cold, etc.) with word-boundary detection
- Creature types (humanoid, undead, etc.)
- Alignments (explicit patterns like "lawful good" = high confidence)
- Rarities, sizes, conditions, spell schools
- CR/level numeric values
- Named entities (spell names, creature names)

**Methods:**
- `extract(&self, text: &str) -> TTRPGAttributes`
- `confident_damage_types(&self, min_confidence: f32) -> Vec<&str>` - for hard filtering
- `to_filterable_fields(&self) -> FilterableFields` - flat vectors for Meilisearch

**Test:** Extract from sample stat block, verify confidence levels.
- _Requirements: 6.1-6.7_

---

### Task 1.7: Update ingestion/mod.rs exports

**Files:**
- `[MODIFY] src/ingestion/mod.rs`

**Changes:**
```rust
// Add to existing exports
pub mod ttrpg;
pub mod hash;

pub use ttrpg::{
    TTRPGClassifier, TTRPGElementType, ClassifiedElement,
    StatBlockData, RandomTableData, AttributeExtractor, TTRPGAttributes,
    AttributeMatch, AttributeSource, GameVocabulary, DnD5eVocabulary,
    detect_game_system, GameSystem,
};
pub use hash::{hash_file, hash_bytes, check_duplicate};
```

**Validation:** All new types accessible via `crate::ingestion::*`.

---

### Task 1.8: Implement game system auto-detection

**Files:**
- `[NEW] src/ingestion/ttrpg/game_detector.rs`

**Description:**
Auto-detect game system from content patterns.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameSystem {
    DnD5e,
    Pathfinder2e,
    CallOfCthulhu,
    Other,
}

/// Auto-detect game system from content patterns
pub fn detect_game_system(text: &str) -> Option<GameSystem>;
```

**Indicator Patterns:**
- D&D 5e: "armor class", "hit dice", "spell slots", "proficiency bonus", "cantrip"
- PF2e: "three actions", "ancestry", "heritage", "proficiency rank"
- CoC: "sanity", "mythos", "investigator", "keeper"

**Logic:** Count matching indicators, require ≥3 for confident match.

**Test:** Detect system from sample text snippets.
- _Requirements: 12.4_

---

## Phase 2: Extend Existing Chunker

### Task 2.1: Add TTRPGChunkConfig and SectionHierarchy to chunker.rs

**Files:**
- `[EXTEND] src/ingestion/chunker.rs`

**Description:**
Add TTRPG-aware configuration with hierarchy tracking to existing chunker.

```rust
/// Extended chunk configuration with TTRPG options
#[derive(Debug, Clone)]
pub struct TTRPGChunkConfig {
    pub base: ChunkConfig,
    pub atomic_elements: Vec<TTRPGElementType>,
    pub atomic_max_multiplier: f32,
    pub overlap_percentage: f32,      // Default 0.12 (12%)
    pub include_hierarchy: bool,      // Track section context
}

/// Section hierarchy tracker
#[derive(Debug, Clone, Default)]
pub struct SectionHierarchy {
    sections: Vec<String>,  // Stack of section titles (h1 at index 0, h2 at 1, etc.)
}

impl SectionHierarchy {
    pub fn update(&mut self, header: &str, level: usize);
    pub fn path(&self) -> String;        // "Chapter 1 > Monsters > Goblins"
    pub fn parents(&self) -> Vec<String>; // Excluding current section
}
```

**Test:** Config instantiation and hierarchy path building.
- _Requirements: 5.1-5.7_

---

### Task 2.2: Implement TTRPGChunker wrapper

**Files:**
- `[EXTEND] src/ingestion/chunker.rs`

**Description:**
Create TTRPGChunker wrapper that composes SemanticChunker with TTRPG awareness.

```rust
/// TTRPG-aware chunker wrapper (composition over inheritance)
pub struct TTRPGChunker {
    base_chunker: SemanticChunker,
    config: TTRPGChunkConfig,
}

impl TTRPGChunker {
    pub fn new(config: TTRPGChunkConfig) -> Self;

    /// Chunk with TTRPG element awareness and hierarchy tracking
    pub fn chunk(
        &self,
        elements: &[ClassifiedElement],
        source_id: &str,
    ) -> Vec<ContentChunk>;

    fn create_chunk_with_hierarchy(...) -> ContentChunk;
    fn get_overlap(&self, content: &str, config: &ChunkConfig) -> String;
    fn detect_header_level(text: &str) -> usize;
    fn split_oversized_element(...) -> Vec<ContentChunk>;
}
```

**Logic:**
1. Track section hierarchy as headers are encountered
2. For atomic elements (stat blocks, tables): flush buffer, emit as single chunk
3. For non-atomic: accumulate with overlap, flush at target size
4. Include `section_path` and `parent_sections` in chunk metadata
5. Tag chunks with `element_type` in metadata

**Test:** Chunk document with mixed content, verify:
- Stat blocks not split
- Section hierarchy preserved in metadata
- Overlap percentage applied correctly

- _Requirements: 5.1-5.7_

---

## Phase 3: Extend Search Integration

### Task 3.1: Create TTRPGSearchDocument struct

**Files:**
- `[EXTEND] src/core/search_client.rs`

**Description:**
Add extended document type with TTRPG-specific filterable fields.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTRPGSearchDocument {
    #[serde(flatten)]
    pub base: SearchDocument,
    pub damage_types: Vec<String>,
    pub creature_types: Vec<String>,
    pub conditions: Vec<String>,
    pub alignments: Vec<String>,
    pub rarities: Vec<String>,
    pub sizes: Vec<String>,
    pub challenge_rating: Option<f32>,
    pub level: Option<u32>,
    pub element_type: String,
}
```

**Test:** Serialize/deserialize round-trip.
- _Requirements: 8.1-8.5_

---

### Task 3.2: Add configure_ttrpg_index() to SearchClient

**Files:**
- `[EXTEND] src/core/search_client.rs`

**Description:**
Add method to configure Meilisearch index with TTRPG filterable attributes.

```rust
impl SearchClient {
    pub async fn configure_ttrpg_index(&self, index_name: &str) -> Result<()> {
        let settings = Settings::new()
            .with_filterable_attributes([
                "damage_types", "creature_types", "conditions",
                "alignments", "rarities", "sizes", "element_type",
                "challenge_rating", "level", "source", "page_number",
            ])
            .with_sortable_attributes(["challenge_rating", "level", "created_at"]);
        // ...
    }
}
```

**Test:** Integration test with Meilisearch.
- _Requirements: 8.1-8.5_

---

### Task 3.3: Add add_ttrpg_documents() to SearchClient

**Files:**
- `[EXTEND] src/core/search_client.rs`

**Description:**
Add method to index TTRPG documents with full attribute payload.

**Test:** Add documents, query with filters.
- _Requirements: 8.1-8.5_

---

## Phase 4: TTRPG Search Enhancement

### Task 4.1: Create ttrpg_search module structure

**Files:**
- `[NEW] src/core/ttrpg_search/mod.rs`

**Description:**
Create module for TTRPG-enhanced search logic.

```rust
// src/core/ttrpg_search/mod.rs
pub mod query_parser;
pub mod attribute_filter;
pub mod antonym_scorer;
pub mod result_ranker;
pub mod index_queue;

pub use query_parser::{QueryParser, QueryConstraints, RequiredAttribute};
pub use attribute_filter::AttributeFilter;
pub use antonym_scorer::AntonymMapper;
pub use result_ranker::{ResultRanker, RankingConfig, ScoreBreakdown, RankedResult};
pub use index_queue::IndexQueue;
```

---

### Task 4.2: Implement QueryParser with negation support

**Files:**
- `[NEW] src/core/ttrpg_search/query_parser.rs`

**Description:**
Parse user queries to extract constraints, negations, and named entities.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryConstraints {
    pub original_query: String,
    pub semantic_query: String,       // Cleaned for embedding
    pub expanded_query: String,       // With antonym hints
    pub required_attributes: Vec<RequiredAttribute>,
    pub excluded_attributes: Vec<String>,  // From negations
    pub cr_range: Option<(f32, f32)>,
    pub level_range: Option<(u32, u32)>,
    pub exact_match_entities: Vec<String>,
}

pub struct QueryParser {
    vocabulary: Box<dyn GameVocabulary>,
    antonym_mapper: AntonymMapper,
}

impl QueryParser {
    pub fn parse(&self, query: &str) -> QueryConstraints;
}
```

**Negation patterns:** "not undead", "without fire", "except dragons", "excluding cold"

**Test:** Parse queries with negations and CR ranges.
- _Requirements: 9.1-9.6_

---

### Task 4.3: Implement AntonymMapper

**Files:**
- `[NEW] src/core/ttrpg_search/antonym_scorer.rs`

**Description:**
Map semantic opposites for penalty scoring.

**Antonym pairs:**
- fire ↔ cold
- radiant ↔ necrotic
- lawful ↔ chaotic
- good ↔ evil
- lightning ↔ thunder (loose association)

**Methods:**
- `get_antonyms(&self, attr: &str) -> Option<&Vec<String>>`
- `are_antonyms(&self, a: &str, b: &str) -> bool`
- `calculate_penalty(query_attrs, result_attrs) -> f32` (returns 1.0 for no penalty, 0.1 for heavy)

**Test:** Verify penalties applied correctly.
- _Requirements: 7.1-7.4, 10.2_

---

### Task 4.4: Implement ResultRanker with RRF

**Files:**
- `[NEW] src/core/ttrpg_search/result_ranker.rs`

**Description:**
Combine dense and sparse search results using Reciprocal Rank Fusion.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub semantic_score: f32,
    pub keyword_score: f32,
    pub attribute_match_bonus: f32,
    pub antonym_penalty: f32,
    pub exact_match_boost: f32,
    pub final_score: f32,
}

#[derive(Debug, Clone)]
pub struct RankingConfig {
    pub rrf_k: f32,              // Typically 60
    pub semantic_weight: f32,    // 0.6 default
    pub keyword_weight: f32,     // 0.4 default
    pub attribute_match_bonus: f32,
    pub exact_match_boost: f32,
    pub hard_exclude_veto: bool, // Hard filter for excluded attributes
}

pub struct ResultRanker {
    config: RankingConfig,
    antonym_mapper: AntonymMapper,
}

impl ResultRanker {
    /// Fuse dense and sparse results using RRF
    pub fn fuse_rrf(
        &self,
        dense_results: &[SearchCandidate],
        sparse_results: &[SearchCandidate],
    ) -> HashMap<String, (f32, f32)>;

    /// Full ranking pipeline with score breakdown
    pub fn rank(
        &self,
        dense_results: &[SearchCandidate],
        sparse_results: &[SearchCandidate],
        constraints: &QueryConstraints,
        doc_attributes: &HashMap<String, Vec<String>>,
    ) -> Vec<RankedResult>;
}
```

**Test:** RRF fusion, hard veto filtering, score breakdown.
- _Requirements: 10.1-10.4_

---

### Task 4.5: Implement IndexQueue for retry logic

**Files:**
- `[NEW] src/core/ttrpg_search/index_queue.rs`

**Description:**
Queue chunks for Meilisearch indexing with retry when unavailable.

```rust
pub struct PendingDocument {
    pub id: String,
    pub payload: serde_json::Value,
    pub attempts: u32,
    pub created_at: Instant,
}

pub struct IndexQueue {
    queue: Arc<Mutex<VecDeque<PendingDocument>>>,
    max_retries: u32,
    retry_delay: Duration,
}

impl IndexQueue {
    pub fn enqueue(&self, id: String, payload: serde_json::Value);
    pub fn dequeue(&self) -> Option<PendingDocument>;
    pub fn requeue(&self, doc: PendingDocument);
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}
```

**Test:** Enqueue, dequeue, retry counting.
- _Requirements: NFR Reliability 2_

---

### Task 4.6: Implement AttributeFilter for Meilisearch

**Files:**
- `[NEW] src/core/ttrpg_search/attribute_filter.rs`

**Description:**
Build Meilisearch filter strings from QueryConstraints.

```rust
pub struct AttributeFilter;

impl AttributeFilter {
    /// Build Meilisearch filter string from constraints
    pub fn build_filter_string(constraints: &QueryConstraints) -> String;

    /// Build hard exclusion filters
    pub fn build_exclusion_filters(excluded: &[String]) -> Vec<String>;
}
```

**Example output:** `"NOT damage_types = 'cold' AND challenge_rating >= 5 AND challenge_rating <= 10"`

**Test:** Generate correct filter syntax.
- _Requirements: 9.1-9.6_

---

### Task 4.7: Integrate with existing query_expansion.rs

**Files:**
- `[EXTEND] src/core/query_expansion.rs`

**Description:**
Add TTRPG attribute extraction to existing query expansion.

**Integration point:** Call `QueryParser::parse()` during query processing.

**Test:** End-to-end query with attribute filter applied.
- _Requirements: 9.1-9.6_

---

### Task 4.8: Update core/mod.rs exports

**Files:**
- `[MODIFY] src/core/mod.rs`

**Changes:**
```rust
pub mod ttrpg_search;

pub use ttrpg_search::{
    QueryParser, QueryConstraints, RequiredAttribute,
    AntonymMapper, AttributeFilter,
    ResultRanker, RankingConfig, ScoreBreakdown, RankedResult,
    IndexQueue,
};
```

---

## Phase 5: Database Schema

### Task 5.1: Create SQLite migration for TTRPG tables

**Files:**
- `[NEW] src/database/migrations/20240110_ttrpg_source_documents.sql`

**Description:**
Add tables for tracking ingested TTRPG documents, extracted entities, and index queue.

```sql
-- Source Document Tracking (BLAKE3 hash for duplicate detection)
CREATE TABLE ttrpg_source_documents (
    id TEXT PRIMARY KEY,
    filename TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_hash TEXT NOT NULL UNIQUE,  -- BLAKE3 hash (64 hex chars)
    file_size INTEGER NOT NULL,
    page_count INTEGER,
    game_system TEXT,  -- 'dnd5e', 'pf2e', 'coc', etc.
    processing_status TEXT NOT NULL DEFAULT 'pending',
    chunk_count INTEGER DEFAULT 0,
    entity_count INTEGER DEFAULT 0,
    error_message TEXT,
    processing_time_ms INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_source_docs_hash ON ttrpg_source_documents(file_hash);
CREATE INDEX idx_source_docs_status ON ttrpg_source_documents(processing_status);

-- Extracted Entities (stat blocks, spells, items)
CREATE TABLE ttrpg_entities (
    id TEXT PRIMARY KEY,
    source_document_id TEXT NOT NULL REFERENCES ttrpg_source_documents(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL,  -- 'stat_block', 'spell', 'item', 'random_table'
    name TEXT NOT NULL,
    page_number INTEGER,
    section_path TEXT,  -- Hierarchical section context
    structured_data TEXT,  -- JSON blob
    meilisearch_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_entities_source ON ttrpg_entities(source_document_id);
CREATE INDEX idx_entities_type ON ttrpg_entities(entity_type);
CREATE INDEX idx_entities_name ON ttrpg_entities(name);

-- Index Queue (for Meilisearch retry when unavailable)
CREATE TABLE ttrpg_index_queue (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL,
    payload TEXT NOT NULL,  -- JSON blob
    attempts INTEGER DEFAULT 0,
    last_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    next_retry_at TEXT
);

CREATE INDEX idx_queue_retry ON ttrpg_index_queue(next_retry_at);
```

**Validation:** Migration applies successfully.
- _Requirements: 11.1-11.4, NFR Reliability 2_

---

### Task 5.2: Add sqlx queries for TTRPG tables

**Files:**
- `[NEW] src/database/ttrpg_queries.rs`

**Description:**
Implement CRUD operations for TTRPG source documents and index queue.

**Functions:**
```rust
// Source document operations
pub async fn insert_source_document(...) -> Result<()>;
pub async fn get_source_by_hash(pool: &SqlitePool, hash: &str) -> Result<Option<SourceDoc>>;
pub async fn update_processing_status(pool: &SqlitePool, id: &str, status: &str) -> Result<()>;
pub async fn delete_source_document(pool: &SqlitePool, id: &str) -> Result<()>;

// Entity operations
pub async fn insert_entity(...) -> Result<()>;
pub async fn get_entities_by_source(pool: &SqlitePool, source_id: &str) -> Result<Vec<Entity>>;

// Index queue operations
pub async fn enqueue_for_indexing(pool: &SqlitePool, doc_id: &str, payload: &str) -> Result<()>;
pub async fn get_pending_index_items(pool: &SqlitePool, limit: u32) -> Result<Vec<QueueItem>>;
pub async fn mark_indexed(pool: &SqlitePool, id: &str) -> Result<()>;
pub async fn increment_retry(pool: &SqlitePool, id: &str, error: &str) -> Result<()>;
```

**Test:** Database operations including queue retry logic.
- _Requirements: 11.1-11.4_

---

## Phase 6: Tauri Command Integration

### Task 6.1: Add ingest_ttrpg_document command

**Files:**
- `[EXTEND] src/commands.rs`

**Description:**
Add Tauri command for TTRPG-enhanced document ingestion.

```rust
#[tauri::command]
pub async fn ingest_ttrpg_document(
    state: tauri::State<'_, AppState>,
    file_path: String,
    game_system: Option<String>,
) -> Result<IngestionResult, String>
```

**Flow:**
1. Parse document (PDF/EPUB)
2. Classify elements with `TTRPGClassifier`
3. Chunk with `chunk_ttrpg()`
4. Extract attributes with `AttributeExtractor`
5. Build `TTRPGSearchDocument` list
6. Index to Meilisearch
7. Record in SQLite

**Test:** End-to-end ingestion of sample PDF.
- _Requirements: 1.1-1.6, 2.1-2.7_

---

### Task 6.2: Add search_ttrpg command with attribute filtering

**Files:**
- `[EXTEND] src/commands.rs`

**Description:**
Add Tauri command for TTRPG-aware search with attribute filtering.

```rust
#[tauri::command]
pub async fn search_ttrpg(
    state: tauri::State<'_, AppState>,
    query: String,
    filters: Option<TTRPGFilters>,
) -> Result<Vec<TTRPGSearchResult>, String>
```

**Features:**
- Extract constraints from query
- Build Meilisearch filter string
- Apply antonym penalties to results
- Return ranked results with score breakdown

**Test:** Search with various filter combinations.
- _Requirements: 9.1-9.6, 10.1-10.4_

---

### Task 6.3: Add list_ttrpg_sources command

**Files:**
- `[EXTEND] src/commands.rs`

**Description:**
Add command to list ingested TTRPG source documents.

```rust
#[tauri::command]
pub async fn list_ttrpg_sources(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SourceDocumentInfo>, String>
```

**Test:** List after ingestion.
- _Requirements: 11.5-11.6_

---

### Task 6.4: Add delete_ttrpg_source command

**Files:**
- `[EXTEND] src/commands.rs`

**Description:**
Delete source document and cascade to chunks/embeddings.

**Flow:**
1. Delete from SQLite
2. Delete matching documents from Meilisearch

**Test:** Verify cascade deletion.
- _Requirements: 11.4_

---

### Task 6.5: Register new commands in main.rs

**Files:**
- `[MODIFY] src/main.rs`

**Description:**
Add new commands to Tauri invoke handler.

```rust
.invoke_handler(tauri::generate_handler![
    // Existing commands...
    ingest_ttrpg_document,
    search_ttrpg,
    list_ttrpg_sources,
    delete_ttrpg_source,
])
```

---

## Phase 7: Frontend Integration (Leptos)

### Task 7.1: Add Tauri bindings for TTRPG commands

**Files:**
- `[EXTEND] frontend/src/bindings.rs`

**Description:**
Add TypeScript-like binding wrappers for new Tauri commands.

```rust
pub async fn ingest_ttrpg_document(file_path: &str, game_system: Option<&str>) -> Result<IngestionResult, String> {
    invoke("ingest_ttrpg_document", &IngestArgs { file_path, game_system }).await
}
```

---

### Task 7.2: Add TTRPG types to frontend

**Files:**
- `[NEW] frontend/src/types/ttrpg.rs`

**Description:**
Mirror Rust TTRPG types for frontend use.

---

## Phase 8: Testing

### Task 8.1: Unit tests for stat block parsing

**Files:**
- Tests within `src/ingestion/ttrpg/stat_block.rs`

**Test cases:**
- Basic goblin stat block
- Complex dragon with legendary actions
- Partial/malformed stat block (graceful degradation)

---

### Task 8.2: Unit tests for attribute extraction

**Files:**
- Tests within `src/ingestion/ttrpg/attribute_extractor.rs`

**Test cases:**
- Extract damage types from text
- Extract CR from various formats ("CR 5", "Challenge 10", "CR 1/4")
- Multiple attribute types in single text

---

### Task 8.3: Integration tests for full pipeline

**Files:**
- `[NEW] src/tests/ttrpg_integration.rs`

**Test cases:**
- Ingest PDF → classify → chunk → index → search
- Verify filter queries return correct results
- Verify antonym penalties work

---

### Task 8.4: Property-based tests with proptest

**Files:**
- Tests using `proptest` crate (already in dev-dependencies)

**Test cases:**
- Random text doesn't crash classifier
- Any parsed CR is valid f32

---

## Phase 9: Documentation

### Task 9.1: Add rustdoc comments to all new types

**Files:**
- All new `.rs` files

**Description:**
Add `///` documentation comments for all public types and functions.

---

### Task 9.2: Update README with TTRPG ingestion docs

**Files:**
- `[MODIFY] README.md`

**Description:**
Add section on TTRPG document ingestion and search features.

---

## Implementation Order Summary

```
Phase 0 (PDF Parser Extensions)
├── 0.1 Fallback extraction with pdf-extract
├── 0.2 Password-protected PDF support
└── 0.3 BLAKE3 file hashing utility

Phase 0.5 (Layout Detection)
├── 0.5.1 Create layout/mod.rs
├── 0.5.2 ColumnDetector (multi-column)
├── 0.5.3 RegionDetector (boxes, sidebars)
├── 0.5.4 TableExtractor (multi-page tables)
└── 0.5.5 Update ingestion/mod.rs

Phase 1 (TTRPG Module Structure)
├── 1.1 Create ttrpg/mod.rs
├── 1.2 classifier.rs (TTRPGElementType, classify)
├── 1.3 stat_block.rs (StatBlockData)
├── 1.4 random_table.rs (RandomTableData)
├── 1.5 vocabulary.rs (GameVocabulary trait)
├── 1.6 attribute_extractor.rs (with confidence scores)
├── 1.7 Update ingestion/mod.rs
└── 1.8 game_detector.rs (auto-detect game system)

Phase 2 (Chunker Extension)
├── 2.1 TTRPGChunkConfig + SectionHierarchy
└── 2.2 TTRPGChunker wrapper (composition)

Phase 3 (Search Integration)
├── 3.1 TTRPGSearchDocument
├── 3.2 configure_ttrpg_index()
└── 3.3 add_ttrpg_documents()

Phase 4 (TTRPG Search Enhancement)
├── 4.1 ttrpg_search module structure
├── 4.2 QueryParser (with negation support)
├── 4.3 AntonymMapper
├── 4.4 ResultRanker (with RRF fusion)
├── 4.5 IndexQueue (retry logic)
├── 4.6 AttributeFilter (Meilisearch filters)
├── 4.7 Integrate with query_expansion
└── 4.8 Update core/mod.rs

Phase 5 (Database)
├── 5.1 SQLite migration (with index queue table)
└── 5.2 sqlx queries (including queue operations)

Phase 6 (Commands)
├── 6.1 ingest_ttrpg_document
├── 6.2 search_ttrpg (with score breakdown)
├── 6.3 list_ttrpg_sources
├── 6.4 delete_ttrpg_source
└── 6.5 Register in main.rs

Phase 7 (Frontend)
├── 7.1 Tauri bindings
└── 7.2 Frontend types

Phase 8 (Testing)
├── 8.1 Stat block tests
├── 8.2 Attribute extraction tests (with confidence)
├── 8.3 Integration tests
└── 8.4 Property tests

Phase 9 (Documentation)
├── 9.1 Rustdoc comments
└── 9.2 README update
```

## Minimum Viable Implementation

For a functional MVP, complete in order:

**Core Ingestion (15 tasks):**
1. **Tasks 0.1-0.3**: PDF parser extensions (fallback, password, hashing)
2. **Tasks 1.1-1.8**: Core TTRPG module with game detection
3. **Tasks 2.1-2.2**: TTRPG-aware chunking with hierarchy

**Search Integration (5 tasks):**
4. **Tasks 3.1-3.3**: Meilisearch TTRPGSearchDocument
5. **Tasks 4.1-4.3**: Query parsing and antonym mapping

**Persistence + Commands (4 tasks):**
6. **Task 5.1**: Database migration
7. **Tasks 6.1, 6.5**: Ingestion command and registration

**Total MVP: 24 tasks**

This provides working ingestion with:
- Fallback PDF extraction
- TTRPG element classification
- Attribute extraction with confidence
- Game system auto-detection
- Section hierarchy tracking
- Meilisearch filtering
- Antonym penalty scoring

**Optional enhancements after MVP:**
- Layout detection (Phase 0.5) - for complex multi-column PDFs
- RRF fusion (Task 4.4) - for hybrid dense+sparse search
- Index queue (Task 4.5) - for Meilisearch resilience
- Score breakdown (Task 6.2) - for debugging transparency
