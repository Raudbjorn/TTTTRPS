# Requirements: TTRPG Document Parsing & Embedding System

## Introduction

This document specifies requirements for enhancing the existing `ttrpg-assistant` Rust/Tauri application's document parsing and embedding pipeline to provide TTRPG-specific content understanding. The current system uses `lopdf` for PDF extraction, Meilisearch for hybrid search, and SQLite for persistence. This enhancement focuses on domain-specific improvements: stat block detection, random table parsing, critical attribute extraction, and TTRPG-aware chunking.

The core problem this system solves is transforming dense, visually-designed rulebook content into semantically-rich, retrievable chunks that preserve game-mechanical precision while enabling natural language queries. Standard embedding approaches fail on TTRPG content because they blur attribute specificity (e.g., treating "fire damage" and "cold damage" as semantically equivalent) and lose structural context (e.g., which rules belong to which character class).

### Existing Infrastructure

The enhancement builds on:
- **PDF Parsing**: `src-tauri/src/ingestion/pdf_parser.rs` using `lopdf`
- **EPUB Parsing**: `src-tauri/src/ingestion/epub_parser.rs`
- **DOCX Parsing**: `src-tauri/src/ingestion/docx_parser.rs`
- **Chunking**: `src-tauri/src/ingestion/chunker.rs` with sentence/paragraph awareness
- **Search**: `src-tauri/src/core/search_client.rs` + `meilisearch_chat.rs`
- **Query Expansion**: `src-tauri/src/core/query_expansion.rs`
- **Database**: SQLite via `sqlx` in `src-tauri/src/database/`

## Requirements

### Requirement 1: Enhanced PDF Parsing for Complex Layouts

**User Story:** As a content ingestion pipeline, I want to extract structured content from TTRPG PDFs with complex multi-column layouts, so that the full semantic richness of the source material is preserved.

#### Acceptance Criteria

1. WHEN the system receives a PDF with multi-column layout THEN it SHALL detect column boundaries and extract text in logical reading order.
2. WHEN the PDF contains boxed or shaded regions (sidebars, callouts) THEN the system SHALL detect these as distinct elements.
3. WHEN the PDF contains tables THEN the system SHALL extract table structure preserving row/column relationships.
4. WHEN the current `lopdf` extraction produces garbled output THEN the system SHALL fall back to alternative extraction methods (pdf-extract crate or external tools).
5. WHEN parsing fails or produces low-confidence results THEN the system SHALL log detailed diagnostics and return partial results where possible.
6. WHEN the PDF is password-protected THEN the system SHALL accept a password parameter and handle decryption.

### Requirement 2: TTRPG Element Detection & Classification

**User Story:** As a document processor, I want to automatically detect and classify TTRPG-specific content elements, so that downstream processing can apply appropriate handling for each element type.

#### Acceptance Criteria

1. WHEN the system encounters a stat block (creature stat block pattern with AC, HP, ability scores) THEN it SHALL classify it as `StatBlock` and extract structured fields.
2. WHEN the system encounters a random table (dice notation like d6, d20, d100, 2d6) THEN it SHALL classify it as `RandomTable` with parsed probability ranges.
3. WHEN the system encounters boxed/shaded read-aloud text THEN it SHALL classify as `ReadAloudText`.
4. WHEN the system encounters sidebar content THEN it SHALL classify as `Sidebar`.
5. WHEN the system encounters section headers THEN it SHALL detect hierarchy level (h1-h6 equivalent).
6. WHEN the system encounters spell or item descriptions with game-mechanical format THEN it SHALL classify appropriately.
7. WHEN element detection confidence is below threshold (configurable, default 0.7) THEN the system SHALL fall back to `GenericText` classification.

### Requirement 3: Stat Block Structured Extraction

**User Story:** As a TTRPG content processor, I want to parse stat blocks into structured data, so that creature/NPC information is queryable by specific attributes.

#### Acceptance Criteria

1. WHEN parsing a stat block THEN the system SHALL extract: name, creature type, size, alignment.
2. WHEN parsing a stat block THEN the system SHALL extract: armor class, hit points (with dice notation), speeds (walk, fly, swim, etc.).
3. WHEN parsing a stat block THEN the system SHALL extract: all six ability scores with modifiers.
4. WHEN parsing a stat block THEN the system SHALL extract: saving throws, skills, damage resistances/immunities, condition immunities.
5. WHEN parsing a stat block THEN the system SHALL extract: senses, languages, challenge rating.
6. WHEN parsing a stat block THEN the system SHALL extract: traits, actions, reactions, legendary actions as structured lists.
7. WHEN a field cannot be parsed THEN the system SHALL record the raw text and flag for review.

### Requirement 4: Random Table Extraction

**User Story:** As a TTRPG content processor, I want to parse random tables with probability distributions, so that table results are searchable and rollable.

#### Acceptance Criteria

1. WHEN parsing a random table THEN the system SHALL detect the dice notation (d4, d6, d8, d10, d12, d20, d100, 2d6, etc.).
2. WHEN parsing a random table THEN the system SHALL extract roll ranges (e.g., "1-3", "4-6", "01-65") and map to probabilities.
3. WHEN parsing a random table THEN the system SHALL extract the result text for each roll range.
4. WHEN a table spans multiple pages THEN the system SHALL detect continuation and merge into a single logical table.
5. WHEN a table contains nested sub-tables THEN the system SHALL maintain the hierarchical relationship.

### Requirement 5: TTRPG-Aware Chunking

**User Story:** As an embedding generation system, I want to chunk content while respecting TTRPG document structure, so that each chunk contains semantically complete and contextually rich information.

#### Acceptance Criteria

1. WHEN chunking content THEN the system SHALL respect TTRPG element boundaries (never split a stat block, table, or boxed text mid-element).
2. WHEN a section is below minimum chunk size (configurable, default 256 tokens) THEN the system SHALL merge with adjacent sibling sections.
3. WHEN a section exceeds maximum chunk size (configurable, default 1024 tokens) THEN the system SHALL split at natural TTRPG boundaries.
4. WHEN creating chunks THEN the system SHALL include hierarchical context as chunk metadata, specifically:
   - `section_path`: Full hierarchy path (e.g., "Chapter 1 > Monsters > Goblins")
   - `parent_sections`: List of parent section titles excluding current section
5. WHEN chunking stat blocks THEN the system SHALL keep the complete stat block as a single unit, only splitting if it exceeds 2x max chunk size.
6. WHEN chunking random tables THEN the system SHALL keep complete tables together with full header context.
7. WHEN content is chunked THEN overlapping context (configurable, default 10-15%) SHALL be included for cross-chunk coherence.

### Requirement 6: Critical Attribute Extraction

**User Story:** As a metadata enrichment system, I want to extract game-specific attributes from TTRPG content, so that precise attribute-based filtering is possible in Meilisearch.

#### Acceptance Criteria

1. WHEN processing content THEN the system SHALL extract damage types (fire, cold, lightning, radiant, necrotic, etc.) as filterable attributes.
2. WHEN processing content THEN the system SHALL extract creature types (humanoid, undead, dragon, fiend, etc.) as filterable attributes.
3. WHEN processing content THEN the system SHALL extract alignment values as filterable attributes.
4. WHEN processing content THEN the system SHALL extract rarity values (common, uncommon, rare, very rare, legendary) as filterable attributes.
5. WHEN processing content THEN the system SHALL extract level/CR ranges as numeric filterable attributes.
6. WHEN processing content THEN the system SHALL extract size categories as filterable attributes.
7. WHEN extracting attributes THEN the system SHALL normalize to canonical forms (e.g., "Str" → "strength").
8. WHEN extracting attributes THEN the system SHALL assign confidence scores (0.0-1.0) based on match type:
   - Exact word-boundary match: 1.0
   - Pattern/regex match: 0.7-0.9
   - Inferred from context: 0.5-0.7
9. WHEN attributes have low confidence (below configurable threshold, default 0.7) THEN they SHALL be used for soft scoring only, not hard filtering.

### Requirement 7: Antonym Relationship Mapping

**User Story:** As a search system, I want to penalize semantically opposite attributes in search results, so that queries for "fire damage" don't return "cold damage" content.

#### Acceptance Criteria

1. WHEN the system builds the attribute index THEN it SHALL maintain antonym mappings (fire↔cold, radiant↔necrotic, lawful↔chaotic, good↔evil).
2. WHEN a query contains a damage type THEN results containing the antonym damage type SHALL receive a penalty score.
3. WHEN a query contains an alignment component THEN results containing the opposite alignment SHALL receive a penalty score.
4. WHEN indexing to Meilisearch THEN antonym data SHALL be stored as searchable/filterable attributes.
5. WHEN expanding a query THEN the system SHALL include antonym terms with negative weight hints for downstream scoring.

### Requirement 8: Meilisearch Index Enhancement

**User Story:** As a search integration, I want to populate Meilisearch with TTRPG-enriched metadata, so that hybrid search can leverage both semantic and attribute filtering.

#### Acceptance Criteria

1. WHEN indexing a chunk THEN the system SHALL include all extracted critical attributes as filterable fields.
2. WHEN indexing a stat block THEN the system SHALL include structured creature data in the document payload.
3. WHEN indexing a random table THEN the system SHALL include the table title and dice notation as searchable fields.
4. WHEN indexing content THEN the system SHALL include element type (stat_block, random_table, read_aloud, etc.) as a filterable field.
5. WHEN indexing content THEN the system SHALL include source document ID, page range, section hierarchy (`section_path`, `parent_sections`), game system, and attribute confidence scores as metadata.

### Requirement 9: Query-Time Attribute Extraction

**User Story:** As a query processor, I want to extract hard constraints from user queries, so that attribute filtering can be applied during Meilisearch search.

#### Acceptance Criteria

1. WHEN a query contains damage type keywords THEN the system SHALL extract them as filter constraints.
2. WHEN a query contains creature type keywords THEN the system SHALL extract them as filter constraints.
3. WHEN a query contains level/CR specifications ("level 5", "CR 10+") THEN the system SHALL parse as numeric filter constraints.
4. WHEN a query contains rarity specifications THEN the system SHALL extract as filter constraints.
5. WHEN a query contains negation ("not undead", "without fire damage") THEN the system SHALL extract as exclusion filters.
6. WHEN the query mentions specific named entities ("fireball", "Tiamat") THEN the system SHALL flag for exact-match boosting.
7. WHEN negation filters are extracted THEN they SHALL be applied as hard pre-filters in Meilisearch (excluding documents before scoring).

### Requirement 10: Enhanced Ranking with Attribute Scoring

**User Story:** As a retrieval system, I want to adjust search result rankings based on attribute match/mismatch, so that results respect both semantic relevance and attribute constraints.

#### Acceptance Criteria

1. WHEN an exact attribute match is found THEN the system SHALL apply a boost to the result score.
2. WHEN an antonym of a query constraint appears in a result THEN the system SHALL apply a penalty.
3. WHEN re-ranking results THEN the system SHALL compute a combined score from Meilisearch similarity and attribute scoring.
4. WHEN returning results THEN the system SHALL include score breakdown for debugging (semantic score, keyword score, attribute match bonus, antonym penalty, exact match boost, final score).
5. WHEN combining dense (vector) and sparse (keyword) search results THEN the system SHALL use Reciprocal Rank Fusion (RRF) with configurable k parameter (default 60).
6. WHEN applying attribute constraints THEN the system SHALL distinguish between hard filters (pre-filter, excludes documents) and soft penalties (post-filter, reduces score).

### Requirement 11: Source Material Management

**User Story:** As a content manager, I want to track source materials and their processing status, so that the knowledge base can be maintained incrementally.

#### Acceptance Criteria

1. WHEN a document is ingested THEN the system SHALL record: filename, file hash (BLAKE3), processing timestamp, and status in SQLite.
2. WHEN a previously-ingested document is submitted THEN the system SHALL detect duplicate by hash and skip or offer update options.
3. WHEN a document is re-processed THEN the system SHALL update existing entries rather than creating duplicates.
4. WHEN documents are deleted THEN the system SHALL cascade-delete associated chunks from Meilisearch.

### Requirement 12: Game System Vocabulary Support

**User Story:** As a multi-system TTRPG assistant, I want to support game-system-specific vocabularies, so that D&D, Pathfinder, Call of Cthulhu, etc. are handled appropriately.

#### Acceptance Criteria

1. WHEN processing D&D 5e content THEN the system SHALL use D&D 5e vocabulary (ability scores, damage types, conditions).
2. WHEN processing Pathfinder 2e content THEN the system SHALL use PF2e vocabulary (traits, actions, proficiency).
3. WHEN processing other systems THEN the system SHALL support pluggable vocabulary definitions.
4. WHEN the game system is not specified THEN the system SHALL attempt auto-detection based on content patterns.

## Non-Functional Requirements

### Performance

1. WHEN processing a 300-page PDF THEN the system SHALL complete parsing and chunking within 60 seconds on reference hardware.
2. WHEN indexing to Meilisearch THEN the system SHALL process at least 100 chunks per second.
3. WHEN querying with filters THEN the system SHALL return results within 200ms.
4. WHEN applying RRF fusion and attribute re-ranking THEN the system SHALL add no more than 50ms latency to base query time.

### Reliability

1. WHEN parsing fails for a specific element THEN the system SHALL continue processing remaining content.
2. WHEN Meilisearch is temporarily unavailable THEN the system SHALL queue chunks in SQLite for later indexing, with automatic retry on exponential backoff (max 5 retries, 1s/2s/4s/8s/16s delays).
3. WHEN external services fail THEN the system SHALL provide clear error messages in the Tauri frontend.

### Integration

1. All new modules SHALL integrate with existing Tauri command handlers in `commands.rs`.
2. All new data structures SHALL implement `Serialize`/`Deserialize` for frontend communication.
3. All database operations SHALL use the existing `sqlx` connection pool.

## Constraints and Assumptions

### Constraints

- Implementation in Rust, extending existing `src-tauri/src/` modules
- Must integrate with existing Meilisearch instance
- Must use existing SQLite database schema (with migrations for new tables)
- Must work within Tauri's async runtime (tokio)

### Assumptions

- Source PDFs are primarily in English
- TTRPG content follows common Western layout conventions
- Users have legitimate access to the source materials being processed
- Meilisearch is running locally (managed by the application)
