# Tasks: Embedded Meilisearch with RAG Pipeline

## Task Sequencing Strategy

This integration follows a **replace-and-enhance** approach:

1. **Dependency Setup**: Add meilisearch-lib, configure features (Phase 1)
2. **Sidecar Removal**: Remove process management code (Phase 2)
3. **Client Migration**: Replace SDK calls with lib calls (Phase 3)
4. **RAG Configuration**: Configure chat pipeline for TTRPG (Phase 4)
5. **Testing & Polish**: Integration tests, frontend updates (Phase 5)

**Key Simplification**: We keep meilisearch-lib as-is with full RAG pipeline. No SurrealDB, no custom vector store, no RRF fusion code. The library handles hybrid search and RAG internally.

---

## Phase 1: Dependency Setup (~2 hours)

### Epic 1.1: Workspace Configuration

- [ ] **1.1.1** Add meilisearch-lib as workspace dependency
  - Add path dependency in root Cargo.toml workspace
  - Add dependency in src-tauri/Cargo.toml
  - Verify compilation succeeds
  - _Requirements: FR-1.1, C-4_
  - _Est: 1 hour_

- [ ] **1.1.2** Configure feature flags
  - Enable required features for meilisearch-lib
  - Resolve any dependency conflicts with existing crates
  - Test minimal initialization code compiles
  - _Requirements: FR-1.1_
  - _Est: 1 hour_

---

## Phase 2: Sidecar Removal (~4 hours)

### Epic 2.1: Remove Process Management

- [ ] **2.1.1** Remove SidecarManager
  - Delete `src-tauri/src/core/sidecar_manager.rs`
  - Remove from mod.rs exports
  - Remove from AppState
  - _Requirements: FR-5.1, C-1_
  - _Est: 1 hour_

- [ ] **2.1.2** Remove meilisearch binary download logic
  - Delete binary download/extraction code
  - Remove associated paths and constants
  - Clean up any platform-specific sidecar code
  - _Requirements: FR-5.1, C-1_
  - _Est: 1 hour_

- [ ] **2.1.3** Update health check commands
  - Replace HTTP health checks with lib `health()` call
  - Update frontend health indicators
  - Remove process status monitoring
  - _Requirements: FR-1.1, NFR-2.1_
  - _Est: 1 hour_

- [ ] **2.1.4** Update initialization flow
  - Replace sidecar spawn with `MeilisearchLib::new(config)`
  - Configure database path (`~/.local/share/ttrpg-assistant/meilisearch/`)
  - Emit initialization events to frontend
  - _Requirements: FR-1.1, NFR-1.1_
  - _Est: 1 hour_

---

## Phase 3: Client Migration (~12 hours)

### Epic 3.1: Core Client Replacement

- [ ] **3.1.1** Create EmbeddedSearch wrapper
  - Create `src-tauri/src/core/search/embedded.rs`
  - Wrap `MeilisearchLib` in `Arc` for shared access
  - Implement error conversion to existing error types
  - _Requirements: FR-1.1_
  - _Est: 2 hours_

- [ ] **3.1.2** Update AppState
  - Replace `search_client: Arc<SearchClient>` with `embedded_search: Arc<EmbeddedSearch>`
  - Remove meilisearch-sdk imports
  - Update all state access patterns
  - _Requirements: FR-1.1_
  - _Est: 1 hour_

### Epic 3.2: Index Operations Migration

- [ ] **3.2.1** Migrate index management commands
  - Update `create_index` → `meili.create_index(uid, primary_key)`
  - Update `delete_index` → `meili.delete_index(uid)`
  - Update index listing/stats
  - Use `wait_for_task_async()` for task completion
  - _Requirements: FR-1.2_
  - _Est: 2 hours_

### Epic 3.3: Document Operations Migration

- [ ] **3.3.1** Migrate document CRUD
  - Update `add_documents` → `meili.add_documents(uid, docs, pk)`
  - Update `get_document` → `meili.get_document(uid, doc_id)`
  - Update `delete_document` → `meili.delete_document(uid, doc_id)`
  - Update batch operations
  - _Requirements: FR-1.3_
  - _Est: 2 hours_

- [ ] **3.3.2** Update ingestion pipeline
  - Update `MeilisearchPipeline` to use embedded client
  - Ensure task completion verification
  - Maintain chunking and metadata flow
  - _Requirements: FR-1.3_
  - _Est: 2 hours_

### Epic 3.4: Search Operations Migration

- [ ] **3.4.1** Migrate search commands
  - Convert SDK search builder to `SearchQuery::new(q)`
  - Update filter syntax (should be compatible)
  - Update pagination, attributes, sort
  - _Requirements: FR-1.4_
  - _Est: 2 hours_

- [ ] **3.4.2** Migrate hybrid search (simplified!)
  - Replace raw HTTP hybrid search with native `SearchQuery.with_hybrid()`
  - Configure `HybridQuery::new(semantic_ratio).with_embedder(name)`
  - This is actually EASIER than before
  - _Requirements: FR-1.4, US-5_
  - _Est: 1 hour_

### Epic 3.5: Settings/Embedder Migration

- [ ] **3.5.1** Migrate embedder configuration
  - Replace raw HTTP PATCH with `meili.update_embedders(uid, config)`
  - Keep same JSON structure for Ollama/OpenAI/REST embedders
  - This is EASIER than before (native method vs raw HTTP)
  - _Requirements: FR-4.1_
  - _Est: 1 hour_

- [ ] **3.5.2** Migrate experimental features
  - Replace raw HTTP with `meili.set_features(RuntimeTogglableFeatures)`
  - Enable vector store, score details as needed
  - _Requirements: FR-4.2_
  - _Est: 1 hour_

---

## Phase 4: RAG Configuration (~5 hours)

### Epic 4.1: ChatConfig Setup

- [ ] **4.1.1** Create RAG configuration module
  - Create `src-tauri/src/core/rag/mod.rs`
  - Define `RagConfig` wrapper for `ChatConfig`
  - Load from app settings (API keys from keyring)
  - _Requirements: FR-2.1_
  - _Est: 1 hour_

- [ ] **4.1.2** Configure LLM providers
  - Support Claude (Anthropic) - primary
  - Support GPT-4 (OpenAI) - secondary
  - Support Ollama (local) - offline mode
  - Map to `ChatSource` enum
  - _Requirements: FR-3.1, US-7_
  - _Est: 1 hour_

### Epic 4.2: Index-Specific Configuration

- [ ] **4.2.1** Configure TTRPG index RAG settings
  - Create `ChatIndexConfig` for `ttrpg_rules`, `ttrpg_fiction`, etc.
  - Set descriptions for context (e.g., "D&D 5e rules and mechanics")
  - Configure search params (limit: 8, semantic_ratio: 0.7)
  - _Requirements: FR-2.2_
  - _Est: 1 hour_

- [ ] **4.2.2** Create Liquid templates for documents
  - Template for rules: `Source: {{doc.source}} (p{{doc.page}})\n{{doc.content}}`
  - Template for chunks: Include section hierarchy
  - Template for chat history: Conversation format
  - _Requirements: FR-2.2, US-4_
  - _Est: 1 hour_

### Epic 4.3: RAG Commands

- [ ] **4.3.1** Create RAG Tauri commands
  - `configure_rag(config)` - Update RAG settings
  - `rag_query(index, messages)` - Non-streaming RAG
  - `rag_query_stream(index, messages, stream_id)` - Streaming RAG
  - Wire to `meili.chat_completion()` / `chat_completion_stream()`
  - _Requirements: FR-2.3, FR-2.4, US-3, US-6_
  - _Est: 2 hours_

---

## Phase 5: Testing & Polish (~8 hours)

### Epic 5.1: Integration Testing

- [ ] **5.1.1** Test embedded search operations
  - Index creation/deletion
  - Document add/get/delete
  - Search with filters and pagination
  - _Requirements: NFR-1.2_
  - _Est: 2 hours_

- [ ] **5.1.2** Test hybrid search
  - Verify semantic_ratio affects results
  - Test with Ollama embedder
  - Test fallback when embedder unavailable
  - _Requirements: FR-1.4, NFR-2.2_
  - _Est: 2 hours_

- [ ] **5.1.3** Test RAG pipeline
  - Test `chat_completion()` returns relevant context
  - Test source citations are included
  - Test streaming works end-to-end
  - _Requirements: FR-2.3, FR-2.4, FR-2.5_
  - _Est: 2 hours_

### Epic 5.2: Frontend Updates

- [ ] **5.2.1** Update chat UI for RAG streaming
  - Wire streaming events to chat display
  - Show sources/citations after response
  - Handle errors gracefully
  - _Requirements: US-6, US-4_
  - _Est: 2 hours_

### Epic 5.3: Cleanup

- [ ] **5.3.1** Remove deprecated code
  - Remove meilisearch-sdk from Cargo.toml
  - Remove any remaining HTTP client code for Meilisearch
  - Clean up unused imports
  - _Requirements: C-1_
  - _Est: 1 hour_

- [ ] **5.3.2** Update documentation
  - Update CLAUDE.md with new architecture
  - Document RAG configuration options
  - Remove sidecar documentation
  - _Requirements: NFR-4.1_
  - _Est: 1 hour_

---

## Task Dependencies

```
Phase 1 (Dependencies)
    │
    └── 1.1.1 Add lib ──► 1.1.2 Features
                              │
Phase 2 (Sidecar Removal)     │
    │                         │
    ├── 2.1.1 Remove manager ◄┘
    │       │
    │       └── 2.1.2 Remove download
    │               │
    │               └── 2.1.3 Health checks
    │                       │
    │                       └── 2.1.4 Init flow
    │                               │
Phase 3 (Client Migration)          │
    │                               │
    ├── 3.1.1 EmbeddedSearch ◄──────┘
    │       │
    │       └── 3.1.2 AppState
    │               │
    │       ┌───────┴───────┐
    │       │               │
    │       ▼               ▼
    ├── 3.2 Indexes    3.3 Documents
    │       │               │
    │       └───────┬───────┘
    │               │
    │               ▼
    ├── 3.4 Search ◄┘
    │       │
    │       └── 3.5 Settings/Embedders
    │                       │
Phase 4 (RAG)               │
    │                       │
    ├── 4.1 ChatConfig ◄────┘
    │       │
    │       └── 4.2 Index configs
    │               │
    │               └── 4.3 RAG commands
    │                       │
Phase 5 (Testing)           │
    │                       │
    ├── 5.1 Integration tests ◄┘
    │       │
    │       └── 5.2 Frontend updates
    │               │
    │               └── 5.3 Cleanup
```

---

## Estimated Effort

| Phase | Epic | Est. Hours | Complexity |
|-------|------|------------|------------|
| 1 | 1.1 Dependency Setup | 2 | Low |
| 2 | 2.1 Sidecar Removal | 4 | Low |
| 3 | 3.1 Core Client | 3 | Medium |
| 3 | 3.2 Index Operations | 2 | Low |
| 3 | 3.3 Document Operations | 4 | Medium |
| 3 | 3.4 Search Operations | 3 | Medium |
| 3 | 3.5 Settings/Embedders | 2 | Low |
| 4 | 4.1 ChatConfig Setup | 2 | Low |
| 4 | 4.2 Index Configuration | 2 | Low |
| 4 | 4.3 RAG Commands | 2 | Medium |
| 5 | 5.1 Integration Testing | 6 | Medium |
| 5 | 5.2 Frontend Updates | 2 | Medium |
| 5 | 5.3 Cleanup | 2 | Low |

**Total Estimated**: ~31 hours (~4 working days)

---

## What We're NOT Doing (Complexity Avoided)

| Removed Scope | Hours Saved | Reason |
|---------------|-------------|--------|
| SurrealDB integration | 32 | meilisearch-lib handles vectors natively |
| Custom RRF fusion | 8 | meilisearch-lib handles hybrid search |
| Vector migration tooling | 12 | No separate vector store to migrate |
| Custom RAG pipeline | 20 | meilisearch-lib provides chat_completion() |
| Trait abstractions | 8 | Not needed - single implementation |
| EmbedderManager | 8 | LLM proxy handles embedding requests |

**Complexity Reduction**: ~88 hours removed from original plan

---

## Acceptance Criteria

| Requirement | Test Criteria |
|-------------|---------------|
| FR-1.1 | App starts without external meilisearch process |
| FR-1.2 | Index operations complete with task verification |
| FR-1.3 | Documents can be added, retrieved, deleted |
| FR-1.4 | Hybrid search returns combined keyword+semantic results |
| FR-2.1 | RAG can be configured with different LLM providers |
| FR-2.3 | `rag_query()` returns relevant answers with sources |
| FR-2.4 | `rag_query_stream()` streams tokens to frontend |
| FR-2.5 | Response includes document IDs as sources |
| US-3 | "How does flanking work?" returns DMG reference |
| US-4 | Response cites page numbers |
| US-5 | Hybrid search understands synonyms and concepts |
| US-6 | Streaming shows tokens as they arrive |
| US-7 | Can switch between Claude/GPT-4/Ollama |
| NFR-1.1 | Database init < 2 seconds |
| NFR-1.2 | Keyword search < 50ms |
| NFR-2.2 | Graceful fallback when embedder unavailable |
| C-1 | No external processes spawned |
| C-3 | TTRPG synonyms and spell correction work |

---

## Risk Mitigation

### R-1: meilisearch-lib Compatibility
- Pin to specific git commit for reproducibility
- Test with existing TTRPS index data
- Have rollback plan to SDK if critical issues

### R-2: LLM Provider Availability
- Test all providers during development
- Implement proper error handling for API failures
- Document required API keys and setup

### R-3: Embedder Configuration
- Test Ollama, OpenAI, and REST embedders
- Document configuration format
- Provide diagnostic commands for troubleshooting
