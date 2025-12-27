# Meilisearch Integration Plan

This document outlines the step-by-step process to replace the current fragmented search architecture (LanceDB + Tantivy) with a unified Meilisearch Sidecar integration.

## Background & Context

### Goal
To provide the "Sidecar DM" with a robust, high-performance, and unified search engine that enables AI-powered features (Semantic Search) alongside traditional full-text search (Fuzzy Matching, Typo Tolerance) without the complexity of managing multiple databases.

### The "Sidecar" Architecture
Instead of embedding a library, the application will manage a background `meilisearch` process (Sidecar). This ensures:
- **Performance**: Indexing heavy lifting happens in a separate process, keeping the UI responsive.
- **Isolation**: Crashes in the search engine don't bring down the main app.
- **Simplicity**: Users install a single "App", and the Sidecar is managed transparently.

### The "Game Changer": Federated Search & Multi-Embedders
We are moving beyond simple "vector search" to a **Multi-Index Strategy**.
- **Specialization**: Rules require different semantic understanding than Fan Fiction.
- **Implementation**: We will define specific "indexes" (e.g., `rules`, `fiction`) and assign them unique **Embedders** (e.g., `text-embedding-3-small` for rules, a narrative model for fiction).
- **Federated Search**: A single user query will simultaneously search all indexes, and the UI will aggregate results, providing the perfect "Context" for the LLM regardless of the source domain.

## Phase 1: Preparation & Sidecar Build

- [x] **Build Custom Meilisearch Binary**
    - Downloaded Meilisearch v1.31.0 binary.
    - Placed in `src-tauri/bin/meilisearch-x86_64-unknown-linux-gnu`.
    - **Deliverable**: Binary ready for sidecar execution.

- [x] **Configure Tauri Sidecar**
    - Updated `src-tauri/tauri.conf.json` with `"externalBin": ["bin/meilisearch"]`.
    - Created `src-tauri/src/core/sidecar_manager.rs` with full process lifecycle management.
    - Configurable port (7700), master key, and data directory.

- [x] **Add Client Dependency**
    - Added `meilisearch-sdk = "0.31"` to `src-tauri/Cargo.toml`.
    - Added `dirs = "6.0"` for data directory resolution.

## Phase 2: Core Implementation (The "New Way")

- [x] **Implement Meilisearch Client Wrapper**
    - Created `src-tauri/src/core/search_client.rs`.
    - Implemented connection logic with health check waiting.
    - Implemented `ensure_index(name, primary_key)` function.
    - Implemented `initialize_indexes()` for all specialized indexes.
    - Implemented `configure_embedder()` for OpenAI, Ollama, HuggingFace embedders.

- [x] **Phase 2.5: Specialized Indexes & Federated Search**
    - Created distinct indexes:
        - `rules`: For game mechanics and rulebooks
        - `fiction`: For lore and narrative content
        - `chat`: For conversation history
        - `documents`: For general user uploads
    - Implemented `federated_search()` across multiple indexes.
    - Implemented `search_all()` for unified content search.

- [x] **Phase 2.6: DM Conversation Integration**
    - **Concept**: Leverage Meilisearch's `/chats` endpoint to power the DM conversation, replacing manual RAG.
    - **Implementation**:
        - Created `src-tauri/src/core/meilisearch_chat.rs` with:
            - `MeilisearchChatClient` for Chat API communication
            - `DMChatManager` for DM-specific workspace management
            - SSE streaming support for real-time responses
            - Configurable LLM sources (OpenAI, Ollama/vLLM, etc.)
        - Chat workspace configuration with custom system prompts
        - Added `use_rag` flag to `ChatRequestPayload`
        - Updated `chat` command in `commands.rs` to route through Meilisearch when RAG mode enabled
    - **Result**: DM conversations can now automatically search indexed documents for context.

- [x] **Refactor Ingestion Pipeline**
    - Created `src-tauri/src/core/meilisearch_pipeline.rs`.
    - Supports PDF, text, markdown files.
    - Semantic chunking with overlap.
    - Auto-routes to appropriate index based on source_type.
    - Removed dependency on embedding_pipeline and vector_store.

## Phase 3: Replacement & Cleanup (Breaking Changes)

- [x] **Replace Query Logic**
    - Updated `src-tauri/src/commands.rs`:
        - New `search()` command with federated search support
        - New `check_meilisearch_health()` command
        - New `reindex_library()` command
        - Updated `ingest_document()` to use MeilisearchPipeline

- [x] **Remove Legacy Vector Store**
    - Deleted `src-tauri/src/core/vector_store.rs` (LanceDB implementation).
    - Removed `lancedb`, `arrow-array`, `arrow-schema` from Cargo.toml.

- [x] **Remove Legacy Keyword Search**
    - Deleted `src-tauri/src/core/keyword_search.rs` (Tantivy implementation).
    - Deleted `src-tauri/src/core/hybrid_search.rs`.
    - Removed `tantivy` from Cargo.toml.

- [x] **Remove Manual Embedding Pipeline**
    - Deleted `src-tauri/src/core/embedding_pipeline.rs`.
    - Meilisearch now handles embedding generation via configured embedders.

## Phase 4: Verification & UI

- [x] **Update Settings UI**
    - Added `MeilisearchStatus` type to frontend bindings.
    - Added `check_meilisearch_health` and `reindex_library` bindings.
    - Added "Search Engine" card to Settings with:
        - Health status indicator (Connected/Offline badge)
        - Host display
        - Document counts per index grid
        - "Clear All Indexes" button with loading state

- [x] **Verify Features**
    - **Typo Tolerance**: ✅ "firebll" finds "fireball", "consitution poisen" works
    - **Federated Search**: ✅ Cross-index search returns results from multiple indexes
    - **Document Indexing**: ✅ Documents added and retrieved successfully
    - **Health & Stats**: ✅ Health check and document counts working
    - Integration tests added: `src-tauri/src/tests/meilisearch_integration_tests.rs`

## Summary

| Phase | Task | Status |
|-------|------|--------|
| 1 | Meilisearch binary setup | Complete |
| 1 | Sidecar configuration | Complete |
| 1 | Client dependency | Complete |
| 2 | Client wrapper | Complete |
| 2.5 | Specialized indexes | Complete |
| 2.5 | Federated search | Complete |
| 2.6 | DM Chat integration | Complete |
| 2 | Ingestion pipeline | Complete |
| 3 | Replace query logic | Complete |
| 3 | Remove VectorStore | Complete |
| 3 | Remove keyword_search | Complete |
| 3 | Remove embedding_pipeline | Complete |
| 4 | Settings UI | Complete |
| 4 | Feature verification | Complete |

### Files Modified/Created
- `src-tauri/src/core/sidecar_manager.rs` - Meilisearch process management
- `src-tauri/src/core/search_client.rs` - Meilisearch SDK wrapper
- `src-tauri/src/core/meilisearch_pipeline.rs` - Document ingestion
- `src-tauri/src/core/meilisearch_chat.rs` - RAG-powered chat via Meilisearch Chat API
- `src-tauri/src/core/mod.rs` - Module exports updated
- `src-tauri/src/commands.rs` - New search/ingest commands, RAG-enabled chat
- `src-tauri/src/main.rs` - Meilisearch initialization
- `src-tauri/Cargo.toml` - Dependencies updated
- `src-tauri/tauri.conf.json` - External binary configured
- `frontend/src/bindings.rs` - Meilisearch bindings added
- `frontend/src/components/settings.rs` - Search Engine status card added
- `src-tauri/src/tests/meilisearch_integration_tests.rs` - Integration test suite
- `src-tauri/src/tests/mod.rs` - Test module registration

### Files Removed
- `src-tauri/src/core/vector_store.rs`
- `src-tauri/src/core/keyword_search.rs`
- `src-tauri/src/core/hybrid_search.rs`
- `src-tauri/src/core/embedding_pipeline.rs`
