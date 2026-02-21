# Tasks: SurrealDB Vector Storage Migration

## Overview

Implementation tasks for migrating from `meilisearch-lib` + SQLite to SurrealDB as the unified storage backend. Tasks are sequenced to enable incremental progress with minimal risk.

**Estimated Total**: ~45-55 hours (~6-7 working days)

---

## Phase 1: Foundation (~8 hours)

### 1.1 Dependency Setup

- [ ] **1.1.1** Add `surrealdb` crate with RocksDB feature to `Cargo.toml`
  - `surrealdb = { version = "2.x", features = ["kv-rocksdb"] }`
  - Verify feature compatibility with existing dependencies
  - _Requirements: FR-1.1_

- [ ] **1.1.2** Create storage module structure
  - `src-tauri/src/core/storage/mod.rs`
  - `src-tauri/src/core/storage/surrealdb.rs`
  - `src-tauri/src/core/storage/error.rs`
  - `src-tauri/src/core/storage/schema.rs`
  - _Requirements: FR-1.1, NFR-4.1_

- [ ] **1.1.3** Implement `SurrealStorage` wrapper
  - `new(db_path: PathBuf)` → initializes RocksDB-backed SurrealDB
  - `db()` → returns reference for queries
  - `clone_db()` → returns Arc for async tasks
  - Unit test: verify creation and basic query
  - _Requirements: FR-1.1, FR-1.3_

### 1.2 Schema Definition

- [ ] **1.2.1** Define TTRPG analyzer for full-text search
  - Tokenizers: class, blank, punct
  - Filters: lowercase, ascii, snowball(english)
  - _Requirements: FR-3.1_

- [ ] **1.2.2** Define core tables (campaign, npc, session, chat_message)
  - Include all fields from current SQLite schema
  - Add record link fields for relations
  - Add datetime fields with defaults
  - _Requirements: FR-6.1, FR-5.2_

- [ ] **1.2.3** Define library_item and chunk tables
  - chunk.content with BM25 full-text index
  - chunk.embedding with HNSW vector index (768 dimensions)
  - Filter indexes (library_item, content_type, page_number)
  - _Requirements: FR-2.1, FR-3.2, FR-2.3_

- [ ] **1.2.4** Define graph relation tables (npc_relation, chunk_reference)
  - in/out record link fields
  - relation_type string field
  - _Requirements: FR-5.1, FR-5.2_

- [ ] **1.2.5** Implement schema application in `SurrealStorage::apply_schema()`
  - Run schema on database init
  - Handle schema version checking (future migrations)
  - Unit test: verify all tables and indexes exist
  - _Requirements: FR-1.2_

---

## Phase 2: Search Implementation (~12 hours)

### 2.1 Vector Search

- [ ] **2.1.1** Implement `vector_search()` function
  - Input: embedding, limit, optional filters
  - Use `<|K,COSINE|>` operator for KNN
  - Return `SearchResult` with distance score
  - Unit test: insert vectors, search, verify ordering
  - _Requirements: FR-2.2_

- [ ] **2.1.2** Support metadata filtering during vector search
  - WHERE clause before KNN operator
  - Filter by: content_type, library_item, page range
  - Unit test: filter by content_type
  - _Requirements: FR-2.2, US-3_

### 2.2 Full-Text Search

- [ ] **2.2.1** Implement `fulltext_search()` function
  - Input: query string, limit, optional filters
  - Use `@@` operator with ttrpg_analyzer
  - Return results with BM25 scores
  - Unit test: insert documents, search, verify ranking
  - _Requirements: FR-3.2_

- [ ] **2.2.2** Implement search highlighting
  - Use `search::highlight()` function
  - Configurable delimiters (default: `<mark></mark>`)
  - Unit test: verify highlight markers in results
  - _Requirements: FR-3.3_

### 2.3 Hybrid Search

- [ ] **2.3.1** Implement `HybridSearchConfig` struct
  - `semantic_weight`, `keyword_weight`
  - `limit`, `min_score`
  - `normalization` (MinMax/ZScore)
  - `from_semantic_ratio()` constructor
  - _Requirements: FR-4.1, FR-4.3_

- [ ] **2.3.2** Implement `hybrid_search()` function
  - Execute vector and fulltext searches in parallel
  - Use `search::linear()` for score fusion
  - Apply min_score threshold
  - Unit test: verify fusion produces combined results
  - _Requirements: FR-4.1, FR-4.2_

- [ ] **2.3.3** Benchmark hybrid search performance
  - Target: < 300ms for 10K documents
  - Compare with current Meilisearch performance
  - Document results
  - _Requirements: NFR-1.2_

---

## Phase 3: Document Ingestion (~8 hours)

### 3.1 Ingestion Pipeline

- [ ] **3.1.1** Implement `ingest_chunks()` function
  - Input: library_item_id, chunks, optional embeddings
  - Transactional insertion (BEGIN/COMMIT)
  - Create chunk records with all metadata
  - Link to library_item via record link
  - Update library_item status to "ready"
  - Unit test: ingest chunks, verify count and links
  - _Requirements: FR-2.1, FR-6.2_

- [ ] **3.1.2** Implement `delete_library_chunks()` function
  - Delete all chunks for a library item
  - Return count of deleted records
  - Unit test: delete and verify empty
  - _Requirements: FR-8.2_

- [ ] **3.1.3** Update `MeilisearchPipeline` to use SurrealDB
  - Replace `meili.add_documents()` with `storage.ingest_chunks()`
  - Preserve two-phase extraction flow
  - Keep embeddings generation unchanged
  - Integration test: full PDF → chunks → searchable
  - _Requirements: FR-6.2_

### 3.2 Library Management

- [ ] **3.2.1** Implement library_item CRUD operations
  - `create_library_item()`, `get_library_item()`, `update_library_item()`, `delete_library_item()`
  - Delete cascades to chunks
  - _Requirements: FR-8.2_

- [ ] **3.2.2** Implement `get_library_items()` with pagination
  - Support status filtering
  - Include chunk count per item
  - _Requirements: FR-8.2_

---

## Phase 4: RAG Pipeline (~8 hours)

### 4.1 Context Retrieval

- [ ] **4.1.1** Implement `RagConfig` struct
  - `search_config: HybridSearchConfig`
  - `max_context_chunks`, `max_context_bytes`
  - `include_sources`
  - _Requirements: FR-7.1_

- [ ] **4.1.2** Implement context formatting with Liquid templates
  - Preserve existing templates from Meilisearch integration
  - Format: `[N] Source (p.X)\nContent`
  - Track sources for citation
  - _Requirements: FR-7.1, FR-7.3_

### 4.2 LLM Integration

- [ ] **4.2.1** Implement `rag_query()` function
  - Execute hybrid search for context
  - Format context with templates
  - Build system prompt
  - Call existing LLM router
  - Return response with sources
  - _Requirements: FR-7.2_

- [ ] **4.2.2** Implement streaming RAG
  - Same context retrieval
  - Stream LLM response chunks
  - Include sources in final chunk
  - _Requirements: FR-7.2, US-6_

- [ ] **4.2.3** Update RAG Tauri commands
  - `rag_query` → use `storage.rag_query()`
  - `rag_query_stream` → use streaming implementation
  - Preserve response format for frontend compatibility
  - _Requirements: FR-8.3_

---

## Phase 5: Data Migration (~10 hours)

### 5.1 SQLite Migration

- [ ] **5.1.1** Implement `MigrationStatus` tracking
  - Phases: NotStarted, BackingUp, MigratingSqlite, MigratingMeilisearch, Validating, Completed, Failed
  - Record counts per table
  - Error collection
  - _Requirements: FR-6.3_

- [ ] **5.1.2** Implement SQLite backup
  - Copy database file to `.backup` directory
  - Verify backup integrity
  - _Requirements: FR-6.3_

- [ ] **5.1.3** Implement `migrate_campaigns()` function
  - Read from SQLite campaigns table
  - Insert into SurrealDB campaign table
  - Preserve IDs for relation mapping
  - _Requirements: FR-6.1_

- [ ] **5.1.4** Implement `migrate_npcs()` function
  - Read from SQLite npcs table
  - Insert with campaign record links
  - _Requirements: FR-6.1_

- [ ] **5.1.5** Implement `migrate_sessions()` function
  - Read from SQLite sessions table
  - Insert with campaign record links
  - _Requirements: FR-6.1_

- [ ] **5.1.6** Implement `migrate_chat_messages()` function
  - Read from SQLite chat_messages table
  - Insert with campaign/npc record links
  - _Requirements: FR-6.1_

- [ ] **5.1.7** Implement `migrate_library_items()` function
  - Read from SQLite library_documents table
  - Map to library_item table
  - _Requirements: FR-6.1_

### 5.2 Meilisearch Migration

- [ ] **5.2.1** Implement `migrate_meilisearch_indexes()` function
  - Read all documents from each index
  - Map index names to content_type:
    - `ttrpg_rules` → "rules"
    - `ttrpg_fiction` → "fiction"
    - `session_notes` → "session_notes"
    - `homebrew` → "homebrew"
  - Preserve embeddings if dimensions match (768)
  - _Requirements: FR-6.2_

- [ ] **5.2.2** Handle dimension mismatch
  - If embeddings are different dimensions, queue for re-embedding
  - Log warning for user
  - _Requirements: FR-6.2, C-3_

### 5.3 Validation

- [ ] **5.3.1** Implement `validate_migration()` function
  - Compare record counts: SQLite vs SurrealDB
  - Compare chunk counts: Meilisearch vs SurrealDB
  - Test sample queries return results
  - _Requirements: FR-6.3_

- [ ] **5.3.2** Implement migration resumption
  - Track progress in metadata table
  - Allow restart from last completed phase
  - _Requirements: FR-6.3_

### 5.4 Migration Command

- [ ] **5.4.1** Implement `run_migration` Tauri command
  - Check if migration needed (old data exists)
  - Show progress to user
  - Handle errors gracefully
  - _Requirements: FR-6.3_

- [ ] **5.4.2** Implement auto-migration on app start
  - Detect first launch after upgrade
  - Run migration before showing main UI
  - Show progress indicator
  - _Requirements: A-4_

---

## Phase 6: Tauri Command Updates (~6 hours)

### 6.1 Search Commands

- [ ] **6.1.1** Update `search` command
  - Replace Meilisearch call with `storage.hybrid_search()`
  - Preserve response format (SearchHit)
  - _Requirements: FR-8.1_

- [ ] **6.1.2** Update `hybrid_search` command
  - Use `HybridSearchConfig::from_semantic_ratio()`
  - Preserve filter handling
  - _Requirements: FR-8.1_

- [ ] **6.1.3** Update `get_suggestions` command
  - Query SurrealDB for prefix matches
  - Preserve response format
  - _Requirements: FR-8.1_

### 6.2 Document Commands

- [ ] **6.2.1** Update `ingest_document` command
  - Use SurrealDB ingestion pipeline
  - Preserve progress events
  - _Requirements: FR-8.2_

- [ ] **6.2.2** Update `delete_document` command
  - Use `storage.delete_library_chunks()`
  - Cascade delete library_item
  - _Requirements: FR-8.2_

- [ ] **6.2.3** Update `get_library_items` command
  - Query SurrealDB library_item table
  - Include chunk counts
  - _Requirements: FR-8.2_

### 6.3 Campaign/NPC Commands

- [ ] **6.3.1** Update campaign CRUD commands
  - Use SurrealDB queries
  - Preserve response formats
  - _Requirements: C-2_

- [ ] **6.3.2** Update NPC CRUD commands
  - Use SurrealDB with record links
  - Support graph relation queries
  - _Requirements: FR-5.2, FR-5.3_

### 6.4 Embedder Commands

- [ ] **6.4.1** Update embedder configuration commands
  - Store config in SurrealDB metadata table
  - Support runtime embedder switching
  - _Requirements: FR-8.3, C-3_

---

## Phase 7: Cleanup & Testing (~5 hours)

### 7.1 Remove Old Dependencies

- [ ] **7.1.1** Remove `meilisearch-lib` from Cargo.toml
  - Remove path dependency
  - Remove related feature flags
  - _Requirements: NFR-4.1_

- [ ] **7.1.2** Remove `meilisearch-sdk` from Cargo.toml
  - Remove SDK calls
  - Remove related imports
  - _Requirements: NFR-4.1_

- [ ] **7.1.3** Remove SQLx SQLite dependency
  - Keep migration code until verified
  - Remove after migration success confirmed
  - _Requirements: NFR-4.1_

- [ ] **7.1.4** Remove old search modules
  - `src-tauri/src/core/search/embedded.rs` (Meilisearch wrapper)
  - `src-tauri/src/core/search/client.rs` (HTTP client)
  - Preserve hybrid.rs logic (copied to SurrealDB search)
  - _Requirements: NFR-4.1_

- [ ] **7.1.5** Remove SidecarManager references
  - Already removed, verify no remaining imports
  - _Requirements: C-1_

### 7.2 Testing

- [ ] **7.2.1** Integration test: Full document ingestion → search
  - Ingest test PDF
  - Verify chunks created
  - Verify hybrid search returns results
  - Verify RAG query works
  - _Requirements: Success criteria_

- [ ] **7.2.2** Integration test: Migration from test fixtures
  - Create SQLite + Meilisearch test data
  - Run migration
  - Verify all records migrated
  - Verify graph relations work
  - _Requirements: FR-6.3_

- [ ] **7.2.3** Performance benchmarks
  - Startup time: < 3 seconds
  - Hybrid search: < 300ms
  - Document ingestion: > 100 chunks/sec
  - Document results
  - _Requirements: NFR-1_

- [ ] **7.2.4** Platform testing
  - Verify Linux (x86_64) build
  - Verify macOS (aarch64) build
  - Verify Windows (x86_64) build
  - _Requirements: NFR-3.1_

### 7.3 Documentation

- [ ] **7.3.1** Update CLAUDE.md with new architecture
  - SurrealDB schema overview
  - New search module locations
  - Migration instructions
  - _Requirements: Documentation_

- [ ] **7.3.2** Update command documentation
  - Any changed parameters
  - New capabilities (graph queries)
  - _Requirements: Documentation_

---

## Task Dependency Graph

```
Phase 1: Foundation
├── 1.1.1 Dependencies
├── 1.1.2 Module structure
├── 1.1.3 SurrealStorage wrapper
│   └── depends on: 1.1.1, 1.1.2
├── 1.2.1-1.2.4 Schema definitions
│   └── depends on: 1.1.3
└── 1.2.5 Schema application
    └── depends on: 1.2.1-1.2.4

Phase 2: Search (depends on Phase 1)
├── 2.1.1-2.1.2 Vector search
├── 2.2.1-2.2.2 Full-text search
└── 2.3.1-2.3.3 Hybrid search
    └── depends on: 2.1, 2.2

Phase 3: Ingestion (depends on Phase 1)
├── 3.1.1-3.1.3 Ingestion pipeline
└── 3.2.1-3.2.2 Library management

Phase 4: RAG (depends on Phase 2)
├── 4.1.1-4.1.2 Context retrieval
├── 4.2.1-4.2.2 LLM integration
│   └── depends on: 4.1
└── 4.2.3 Tauri commands
    └── depends on: 4.2.1-4.2.2

Phase 5: Migration (depends on Phases 1-4)
├── 5.1.1-5.1.7 SQLite migration
├── 5.2.1-5.2.2 Meilisearch migration
├── 5.3.1-5.3.2 Validation
│   └── depends on: 5.1, 5.2
└── 5.4.1-5.4.2 Migration commands
    └── depends on: 5.3

Phase 6: Command Updates (parallel with Phase 5)
├── 6.1.1-6.1.3 Search commands (depends on Phase 2)
├── 6.2.1-6.2.3 Document commands (depends on Phase 3)
├── 6.3.1-6.3.2 Campaign/NPC commands (depends on Phase 1)
└── 6.4.1 Embedder commands (depends on Phase 3)

Phase 7: Cleanup (depends on Phases 5-6)
├── 7.1.1-7.1.5 Remove dependencies
├── 7.2.1-7.2.4 Testing
└── 7.3.1-7.3.2 Documentation
```

---

## Risk Mitigation Tasks

### R-1: SurrealDB SDK Issues

- [ ] **R-1.1** Create fallback branch with current Meilisearch implementation
- [ ] **R-1.2** Test SurrealDB 2.x embedded mode thoroughly before migration
- [ ] **R-1.3** Have rollback procedure documented

### R-2: Migration Data Loss

- [ ] **R-2.1** Implement comprehensive backup before migration
- [ ] **R-2.2** Add dry-run mode for migration validation
- [ ] **R-2.3** Test migration on copy of production data

### R-3: Performance Regression

- [ ] **R-3.1** Benchmark before removing Meilisearch
- [ ] **R-3.2** Tune HNSW parameters (EFC, M) based on benchmarks
- [ ] **R-3.3** Add performance monitoring for search operations

---

## Acceptance Criteria Checklist

- [ ] App starts without spawning external processes
- [ ] Hybrid search returns comparable quality to Meilisearch
- [ ] Graph queries work (e.g., "allies of NPC X")
- [ ] Migration completes successfully for existing data
- [ ] Startup time < 3 seconds
- [ ] Hybrid search latency < 300ms
- [ ] Single `surrealdb` dependency (no SQLite, no Meilisearch)
- [ ] All existing tests pass with SurrealDB backend
- [ ] Frontend unchanged (API compatibility)
- [ ] Works on Linux, Windows, macOS
