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

- [ ] **Build Custom Meilisearch Binary**
    - Compile `dev-resources/meilisearch` (bin `meilisearch`) in release mode.
    - Validate features: Ensure `enterprise` or AI-related features are enabled if behind flags (though standard build should suffice for v1.38+).
    - **Deliverable**: `src-tauri/bin/meilisearch-x86_64-unknown-linux-gnu` (and other platform variants).

- [ ] **Configure Tauri Sidecar**
    - Update `src-tauri/tauri.conf.json` to include `"externalBin": ["bin/meilisearch"]`.
    - Create `src-tauri/src/core/sidecar_manager.rs` to handle spawning/killing the process.
    - Ensure unique port configuration (e.g., port 7700 or dynamic) to avoid conflicts.

- [ ] **Add Client Dependency**
    - Add `meilisearch-sdk` to `src-tauri/Cargo.toml`.

## Phase 2: Core Implementation (The "New Way")

- [ ] **Implement Meilisearch Client Wrapper**
    - Create `src-tauri/src/core/search_client.rs`.
    - Implement connection logic (waiting for sidecar health check).
    - Implement `ensure_index(name, settings)` function.
    - Configure Index Settings:
        - Enable Vector Search (embedders).
        - Configure `open-ai` or `ollama` embedder based on user settings (this might require dynamic settings updates).

- [ ] **Phase 2.5: Specialized Indexes & Federated Search**
    - **Concept**: Implement separate indexes for distinct content types to leverage specialized embedders.
    - **Action**: Create distinct indexes:
        - `index_rules`: Uses "Technical/Instructional" embedder (e.g., `text-embedding-3-small`).
        - `index_fiction`: Uses "Narrative/Prose" embedder (e.g., higher dimensional narrative model).
        - `index_chat`: Uses "Conversational" embedder.
    - **Refactor**: Update `search_library` command to use **Meilisearch Multi-Search**.
        - Construct a federated query that targets all relevant indexes simultaneously.
        - Add logic to aggregate and categorize results ("Rules", "Lore", "Chat Logs") in the frontend response.

- [ ] **Refactor Ingestion Pipeline**
    - **Modify**: `src-tauri/src/ingestion/mod.rs`
    - **Action**: Instead of calling `vector_store::add` and `keyword_search::add`, call `search_client::add_documents`.
    - **Optimization**: Send raw text chunks to Meilisearch and let it handle embedding generation (Level 2 Integration).

## Phase 3: Replacement & Cleanup (Breaking Changes)

- [ ] **Replace Query Logic**
    - **Modify**: `src-tauri/src/commands.rs`
        - Update `search_library` command to use `meilisearch_sdk`.
    - **Delete**: `src-tauri/src/core/hybrid_search.rs` (Logic is now handled by Meilisearch `hybrid` search query).

- [ ] **Remove Legacy Vector Store**
    - **Delete**: `src-tauri/src/core/vector_store.rs` (LanceDB implementation).
    - **Clean**: Remove `lancedb` and `arrow` dependencies from `Cargo.toml`.
    - **Action**: Remove `~/.local/share/ttrpg-assistant/lancedb` directory handling.

- [ ] **Remove Legacy Keyword Search**
    - **Delete**: `src-tauri/src/core/keyword_search.rs` (Tantivy implementation).
    - **Clean**: Remove `tantivy` dependency from `Cargo.toml`.

- [ ] **Remove Manual Embedding Pipeline**
    - **Delete**: `src-tauri/src/core/embedding_pipeline.rs` (If we fully move to Meilisearch-managed embeddings).
    - **Note**: Keep `llm.rs` for Chat/NPC generation, but remove embedding-specific logic if no longer needed by app (check if other features use embeddings directly).

## Phase 4: Verification & UI

- [ ] **Update Settings UI**
    - Ensure "Meilisearch" status is visible (Health check).
    - Add "Re-index" button in Library settings (calls Meilisearch dump/import or cleared index).

- [ ] **Verify Features**
    - **Library Search**: Test typo tolerance and semantic search.
    - **Document Ingestion**: Verify PDFs are correctly indexed.
