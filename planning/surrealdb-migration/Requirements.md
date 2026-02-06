# Requirements: SurrealDB Vector Storage Migration

## Overview

Replace `meilisearch-lib` with SurrealDB as the unified storage backend for the TTRPG Assistant. This migration enables graph relationships between entities (NPCs, campaigns, documents), native vector search with metadata filtering, and a single embedded database for all application data.

---

## Motivation

### Current Pain Points with Meilisearch-lib

1. **Dual Storage Complexity**: SQLite stores relational data (campaigns, NPCs, sessions) while Meilisearch stores search indexes and vectors, requiring synchronization
2. **Limited Graph Capabilities**: Cannot express relationships between entities (e.g., "NPC X is allied with NPC Y", "Document A references Document B")
3. **Vector-Metadata Isolation**: Meilisearch treats vectors as a search feature; filtering on metadata during vector search is limited
4. **RAG Pipeline Lock-in**: Meilisearch's chat API requires specific LLM configuration that conflicts with our existing multi-provider LLM router

### SurrealDB Advantages

1. **Unified Storage**: Single database for documents, vectors, relations, and metadata
2. **Native Graph Relations**: First-class support for record links (`->relates_to->`, `<-referenced_by<-`)
3. **Hybrid Search Built-in**: BM25 full-text + HNSW vector search with `search::linear()` fusion
4. **Embedded Rust SDK**: No external process (like meilisearch-lib), supports RocksDB persistence
5. **Metadata-Aware Vector Search**: Filter vectors by any field during KNN search
6. **Schema Flexibility**: Schemaless or strict per-table, supports computed fields

---

## User Stories

### US-1: Unified Document Storage
**As a** Game Master importing rulebooks,
**I want** all document data (chunks, vectors, metadata, relationships) stored in one place,
**So that** I can query across all data without joining separate systems.

### US-2: Graph-Based Campaign Knowledge
**As a** Game Master building campaign notes,
**I want** to link NPCs to locations, factions to conflicts, and sessions to events,
**So that** I can traverse relationships when preparing sessions ("Who are the allies of this NPC?").

### US-3: Metadata-Filtered Vector Search
**As a** user searching for combat rules,
**I want** to search semantically within a specific rulebook or content category,
**So that** I get relevant results without noise from unrelated sources.

### US-4: Hybrid Search
**As a** user asking "How does flanking work?",
**I want** the search to combine exact term matches with semantic similarity,
**So that** I find relevant content whether I use precise terminology or describe concepts.

### US-5: Simplified Architecture
**As a** developer maintaining the codebase,
**I want** a single embedded database dependency,
**So that** I don't manage synchronization between SQLite and Meilisearch.

### US-6: No External Processes
**As a** user installing the application,
**I want** everything bundled in a single binary,
**So that** I don't need to download or manage separate services.

### US-7: Preserve RAG Functionality
**As a** user asking questions about my rulebooks,
**I want** AI-powered Q&A with source citations,
**So that** I can get answers grounded in my indexed content.

### US-8: Preserve Chat History Context
**As a** user returning to a previous chat,
**I want** the chat context and history preserved,
**So that** I can continue conversations seamlessly.

---

## Functional Requirements

### FR-1: Embedded SurrealDB Integration

#### FR-1.1: Database Initialization
- WHEN application starts THEN system SHALL initialize SurrealDB with RocksDB storage at `~/.local/share/ttrpg-assistant/surrealdb/`
- IF database path does not exist THEN system SHALL create required directories
- WHEN initialization fails THEN system SHALL display error and degrade gracefully to read-only mode

#### FR-1.2: Schema Management
- WHEN database initializes THEN system SHALL define required tables, analyzers, and indexes
- WHEN schema version changes THEN system SHALL apply migrations automatically
- IF migration fails THEN system SHALL report error with recovery guidance

#### FR-1.3: Connection Pooling
- WHEN application starts THEN system SHALL create a shared SurrealDB connection wrapped in `Arc<Surreal<RocksDb>>`
- WHEN Tauri command executes THEN system SHALL reuse the shared connection
- WHEN application shuts down THEN system SHALL flush pending writes and close cleanly

### FR-2: Vector Storage and Search

#### FR-2.1: Embedding Storage
- WHEN document chunk is ingested THEN system SHALL store vector embedding as `array<float>` field
- WHEN embedding is stored THEN system SHALL index with HNSW for fast similarity search
- WHEN embedder configuration changes THEN system SHALL support re-embedding existing documents

#### FR-2.2: Vector Search (KNN)
- WHEN user performs vector search THEN system SHALL use `<|K,COSINE|>` operator for KNN
- WHEN search includes filters THEN system SHALL apply WHERE clauses before KNN
- WHEN results are returned THEN system SHALL include `vector::distance::knn()` score

#### FR-2.3: HNSW Index Configuration
- WHEN index is created THEN system SHALL configure:
  - `DIMENSION`: Match embedding model (768 for nomic-embed-text, 1536 for OpenAI)
  - `DIST`: COSINE for semantic similarity
  - `EFC`: 150 (construction quality)
  - `M`: 12 (number of connections)

### FR-3: Full-Text Search

#### FR-3.1: Analyzer Configuration
- WHEN index is created THEN system SHALL define custom analyzers for TTRPG content:
  - `ttrpg_analyzer`: Tokenizers (class, blank, punct) + Filters (lowercase, ascii, snowball(english))
  - Support for game-specific terminology

#### FR-3.2: Full-Text Index
- WHEN document chunk is stored THEN system SHALL index `content` field for full-text search
- WHEN full-text index is created THEN system SHALL use BM25 ranking with HIGHLIGHTS enabled
- WHEN user searches text THEN system SHALL use `@@` operator for full-text matching

#### FR-3.3: Search Highlighting
- WHEN search matches content THEN system SHALL return highlighted snippets via `search::highlight()`
- WHEN displaying results THEN system SHALL show match context with configurable delimiters

### FR-4: Hybrid Search (Vector + Full-Text Fusion)

#### FR-4.1: Dual-Mode Search Execution
- WHEN user performs hybrid search THEN system SHALL execute both:
  1. KNN vector search with semantic embedding
  2. BM25 full-text search on content
- WHEN both searches complete THEN system SHALL fuse results with `search::linear()`

#### FR-4.2: Score Fusion
- WHEN fusing results THEN system SHALL apply configurable weights:
  - Default: 60% semantic (vector), 40% keyword (BM25)
  - User-configurable per search context
- WHEN normalizing scores THEN system SHALL support `minmax` or `zscore` normalization

#### FR-4.3: Semantic Ratio Compatibility
- WHEN configured with `semantic_ratio` THEN system SHALL interpret as:
  - `semantic_ratio = 0.0`: Full-text only
  - `semantic_ratio = 1.0`: Vector only
  - `semantic_ratio = 0.5`: Equal weight
- WHEN ratio is applied THEN system SHALL convert to `search::linear()` weights

### FR-5: Graph Relations

#### FR-5.1: Record Links
- WHEN storing entities THEN system SHALL support record link fields (e.g., `campaign_id: campaign:xyz`)
- WHEN querying relations THEN system SHALL support graph traversal (`->`, `<-`, `<->`)
- WHEN deleting records THEN system SHALL handle cascading based on relation type

#### FR-5.2: TTRPG Entity Relations
- WHEN storing NPC THEN system SHALL support links to:
  - `campaign` (belongs_to)
  - `faction[]` (member_of)
  - `location` (current_location)
  - `npc[]` (allied_with, hostile_to, knows)
- WHEN storing document chunk THEN system SHALL support links to:
  - `library_item` (source_document)
  - `chunk[]` (references, continues_from)
  - `campaign` (associated_with)

#### FR-5.3: Graph Queries
- WHEN user queries "allies of NPC X" THEN system SHALL traverse `->allied_with->npc` relation
- WHEN user queries "documents referencing page 42" THEN system SHALL traverse `<-references<-chunk`

### FR-6: Data Migration from SQLite + Meilisearch

#### FR-6.1: SQLite Data Migration
- WHEN migration runs THEN system SHALL transfer all SQLite tables to SurrealDB:
  - `campaigns` → `campaign` table
  - `npcs` → `npc` table
  - `sessions` → `session` table
  - `chat_messages` → `chat_message` table
  - `library_documents` → `library_item` table
- WHEN migrating THEN system SHALL preserve all relationships via record links

#### FR-6.2: Meilisearch Index Migration
- WHEN migration runs THEN system SHALL transfer all Meilisearch documents:
  - `ttrpg_rules` → `chunk` table with `content_type = "rules"`
  - `ttrpg_fiction` → `chunk` table with `content_type = "fiction"`
  - `session_notes` → `chunk` table with `content_type = "session_notes"`
  - `homebrew` → `chunk` table with `content_type = "homebrew"`
- WHEN migrating THEN system SHALL preserve embeddings if dimension matches

#### FR-6.3: Migration Safety
- WHEN migration starts THEN system SHALL create backup of existing data
- WHEN migration is interrupted THEN system SHALL support resumption
- WHEN migration completes THEN system SHALL verify record counts match

### FR-7: RAG Pipeline Rebuild

#### FR-7.1: Context Retrieval
- WHEN RAG query is received THEN system SHALL:
  1. Execute hybrid search on relevant tables
  2. Retrieve top N chunks with metadata
  3. Format with Liquid templates (preserve existing templates)
  4. Return context for LLM consumption

#### FR-7.2: LLM Integration (Custom, Not SurrealDB's)
- WHEN generating response THEN system SHALL use existing LLM router (not SurrealDB ML)
- WHEN streaming THEN system SHALL emit chunks via Tauri events
- WHEN response completes THEN system SHALL include source citations

#### FR-7.3: Source Citations
- WHEN returning RAG response THEN system SHALL include:
  - Record IDs of contributing chunks
  - Source document titles
  - Page numbers where available

### FR-8: Tauri Command Compatibility

#### FR-8.1: Search Commands
- WHEN `search` command is called THEN system SHALL return compatible response format
- WHEN `hybrid_search` command is called THEN system SHALL use SurrealDB hybrid search
- WHEN `get_suggestions` command is called THEN system SHALL query SurrealDB for completions

#### FR-8.2: Document Commands
- WHEN `ingest_document` is called THEN system SHALL use SurrealDB for storage
- WHEN `delete_document` is called THEN system SHALL cascade delete related chunks
- WHEN `get_library_items` is called THEN system SHALL query SurrealDB

#### FR-8.3: RAG Commands
- WHEN `rag_query` is called THEN system SHALL use SurrealDB for retrieval + LLM router for generation
- WHEN `rag_query_stream` is called THEN system SHALL stream via existing event pattern
- WHEN `configure_rag` is called THEN system SHALL update embedder/search configuration

---

## Non-Functional Requirements

### NFR-1: Performance

#### NFR-1.1: Startup Time
- Database initialization SHALL complete in < 3 seconds on SSD
- Schema definition SHALL complete in < 1 second

#### NFR-1.2: Search Latency
- Full-text search SHALL return results in < 100ms for < 100K documents
- Vector KNN search SHALL return results in < 200ms for < 100K vectors
- Hybrid search SHALL return results in < 300ms

#### NFR-1.3: Ingestion Throughput
- Document chunk insertion SHALL process > 100 chunks/second
- Vector embedding storage SHALL not add > 10% overhead vs. text-only

#### NFR-1.4: Memory Usage
- Idle memory footprint SHALL be < 200MB
- HNSW index SHALL use < 1GB for 100K 768-dim vectors

### NFR-2: Reliability

#### NFR-2.1: Data Durability
- All writes SHALL be persisted before command returns
- RocksDB WAL SHALL ensure crash recovery
- Backup/restore SHALL be supported via file copy

#### NFR-2.2: Error Handling
- All SurrealDB errors SHALL be mapped to application error types
- Connection failures SHALL trigger graceful degradation
- Query timeouts SHALL be configurable with sensible defaults

#### NFR-2.3: Graceful Degradation
- IF vector index is unavailable THEN system SHALL fall back to full-text only
- IF embedding provider is unavailable THEN system SHALL queue for later processing

### NFR-3: Compatibility

#### NFR-3.1: Platform Support
- Integration SHALL work on Linux (x86_64, aarch64)
- Integration SHALL work on Windows (x86_64)
- Integration SHALL work on macOS (x86_64, aarch64)

#### NFR-3.2: Data Format Compatibility
- Existing Tauri command response types SHALL remain unchanged
- Frontend bindings SHALL not require modification
- Chat history format SHALL remain compatible

### NFR-4: Maintainability

#### NFR-4.1: Single Dependency
- System SHALL use only `surrealdb` crate (with RocksDB feature)
- System SHALL NOT require separate SQLite dependency after migration
- System SHALL NOT require meilisearch-lib after migration

#### NFR-4.2: Schema Evolution
- System SHALL support schema migrations without data loss
- System SHALL version schemas for forward/backward compatibility

---

## Constraints

### C-1: No External Processes
- System SHALL NOT spawn or manage any external database processes
- All database operations SHALL be in-process library calls

### C-2: Backward Compatibility (During Transition)
- Existing chat history SHALL be preserved during migration
- Existing campaign data SHALL be preserved during migration
- Frontend SHALL not require changes

### C-3: Embedding Provider Independence
- System SHALL continue supporting multiple embedding providers (Ollama, OpenAI, Copilot)
- Vector dimensions SHALL be configurable per provider
- Re-embedding SHALL be supported when switching providers

### C-4: Single Binary Distribution
- Application SHALL remain a single binary (plus assets)
- No runtime downloads or plugin installations required

---

## Assumptions

### A-1: SurrealDB Rust SDK Stability
- SurrealDB embedded mode is production-ready
- RocksDB storage engine is stable

### A-2: Embedding Model Consistency
- Most users use 768-dimensional embeddings (nomic-embed-text)
- Re-embedding on dimension change is acceptable

### A-3: Existing Data Volume
- Average user: < 100 documents, < 50K chunks, < 10 campaigns
- Power user: < 1000 documents, < 500K chunks, < 100 campaigns

### A-4: Migration Window
- Users can accept a one-time migration on version upgrade
- Migration can run during first app launch after upgrade

---

## Glossary

| Term | Definition |
|------|------------|
| **SurrealDB** | Multi-model database supporting document, graph, and vector storage |
| **HNSW** | Hierarchical Navigable Small World - algorithm for approximate KNN |
| **Record Link** | SurrealDB's native reference type (e.g., `campaign:abc123`) |
| **BM25** | Best Matching 25 - probabilistic text ranking algorithm |
| **Hybrid Search** | Combining vector similarity and text relevance scores |
| **RocksDB** | Embedded key-value store used by SurrealDB for persistence |
| **search::linear()** | SurrealQL function for fusing multiple search result sets |
| **Semantic Ratio** | Weight balance between keyword (0.0) and vector (1.0) search |

---

## Dependencies and Risks

### Dependencies
1. `surrealdb` crate with `kv-rocksdb` feature
2. Existing embedding providers (unchanged)
3. Existing LLM router (unchanged)

### Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| SurrealDB Rust SDK bugs | Medium | High | Extensive testing, fallback to 1.x stable |
| Migration data loss | Low | Critical | Pre-migration backup, validation |
| Performance regression | Medium | Medium | Benchmark before/after, index tuning |
| HNSW memory pressure | Low | Medium | Configure M/EFC conservatively, monitor |
| Schema migration complexity | Medium | Medium | Version schemas, test migrations |

---

## Success Criteria

- [ ] All existing search functionality works with SurrealDB backend
- [ ] Hybrid search returns comparable quality results to Meilisearch
- [ ] Graph queries enable new campaign knowledge features
- [ ] Migration completes successfully for test datasets
- [ ] No increase in startup time (< 3 seconds)
- [ ] No increase in search latency (< 300ms hybrid)
- [ ] Single database dependency (SQLite and Meilisearch removed)
- [ ] All tests pass with SurrealDB backend
