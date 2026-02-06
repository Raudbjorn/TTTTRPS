# Requirements: Embedded Meilisearch with RAG Pipeline

## Overview

Replace the HTTP-based Meilisearch SDK with `meilisearch-lib` embedded library, eliminating external process management while gaining an integrated RAG pipeline for TTRPG rulebook queries.

---

## User Stories

### US-1: Simplified Deployment
**As a** user installing TTRPG Assistant,
**I want** the application to work without downloading or managing external processes,
**So that** installation is reliable across all platforms without network dependencies.

### US-2: Faster Startup
**As a** user opening the application,
**I want** search and RAG functionality to be immediately available,
**So that** I don't have to wait for an external process to initialize.

### US-3: Rulebook Q&A
**As a** Game Master preparing for a session,
**I want** to ask natural language questions about my indexed rulebooks,
**So that** I can quickly find rules and mechanics without manual searching.

### US-4: Source Citations
**As a** Game Master using AI-generated answers,
**I want** responses to include page numbers and source references,
**So that** I can verify rules and cite sources to my players.

### US-5: Hybrid Search
**As a** user searching for rules or lore,
**I want** search to understand both exact terms and conceptual meaning,
**So that** I find relevant content whether I use exact terms or describe concepts.

### US-6: Streaming Responses
**As a** user asking complex questions,
**I want** to see the AI response stream in real-time,
**So that** I get immediate feedback rather than waiting for the full response.

### US-7: Multi-Provider LLM Support
**As a** user with preferences for different AI providers,
**I want** to choose between Claude, GPT-4, Mistral, or local models,
**So that** I can use my preferred provider or work offline with Ollama.

---

## Functional Requirements

### FR-1: Embedded Library Integration

#### FR-1.1: Initialization
- WHEN application starts THEN system SHALL initialize meilisearch-lib with configured database path
- IF database path does not exist THEN system SHALL create required directories
- WHEN initialization fails THEN system SHALL display error and degrade gracefully

#### FR-1.2: Index Management
- WHEN user ingests first document THEN system SHALL create required indexes
- WHEN index operation completes THEN system SHALL await task completion before returning
- IF index creation fails THEN system SHALL report specific error to user

#### FR-1.3: Document Operations
- WHEN user adds documents THEN system SHALL use meilisearch-lib `add_documents()` method
- WHEN documents are deleted THEN system SHALL use appropriate delete methods
- WHEN document operations complete THEN system SHALL verify task success

#### FR-1.4: Search Operations
- WHEN user performs search THEN system SHALL use meilisearch-lib `search()` method
- WHEN search includes filters THEN system SHALL apply filter expressions
- WHEN hybrid search is enabled THEN system SHALL use configured semantic ratio

---

### FR-2: RAG Pipeline

#### FR-2.1: Chat Configuration
- WHEN application starts THEN system SHALL load ChatConfig from settings
- WHEN user changes LLM provider THEN system SHALL update ChatConfig
- WHEN ChatConfig is invalid THEN system SHALL report configuration error

#### FR-2.2: Index Configuration
- WHEN configuring RAG THEN system SHALL support per-index ChatIndexConfig:
  - `description`: Human-readable description for context
  - `template`: Liquid template for document formatting
  - `max_bytes`: Maximum bytes per document in context
  - `search_params`: Hybrid search parameters (limit, semantic_ratio, embedder)

#### FR-2.3: Chat Completion (Non-Streaming)
- WHEN user sends chat request THEN system SHALL:
  1. Extract query from last user message
  2. Execute hybrid search on specified index
  3. Format results using Liquid template
  4. Build prompt with system message + context + conversation
  5. Call configured LLM provider
  6. Return response with source citations
- WHEN no relevant context found THEN system SHALL indicate insufficient data

#### FR-2.4: Chat Completion (Streaming)
- WHEN user requests streaming response THEN system SHALL:
  1. Execute same retrieval pipeline as non-streaming
  2. Stream LLM response chunks via `chat_completion_stream()`
  3. Include sources in final chunk
- WHEN stream is interrupted THEN system SHALL handle gracefully

#### FR-2.5: Source Citations
- WHEN generating response THEN system SHALL track which documents were used
- WHEN returning response THEN system SHALL include document IDs as sources
- WHEN displaying sources THEN frontend SHALL resolve to human-readable references

---

### FR-3: LLM Provider Support

#### FR-3.1: Provider Configuration
- WHEN configuring LLM THEN system SHALL support:
  - **OpenAI**: api_key, model, org_id, project_id
  - **Anthropic**: api_key, model
  - **Azure OpenAI**: api_key, base_url, api_version, deployment_id
  - **Mistral**: api_key, model
  - **vLLM**: base_url, model

#### FR-3.2: Provider Switching
- WHEN user changes provider THEN system SHALL update ChatConfig.source
- WHEN provider is unavailable THEN system SHALL report connection error
- WHEN API key is invalid THEN system SHALL report authentication error

---

### FR-4: Embedder Configuration

#### FR-4.1: Embedder Setup
- WHEN configuring embeddings THEN system SHALL use `update_embedders()` method
- WHEN embedder is Ollama THEN system SHALL configure REST endpoint
- WHEN embedder is OpenAI THEN system SHALL configure API key and model

#### FR-4.2: Hybrid Search Parameters
- WHEN performing hybrid search THEN system SHALL use configured semantic_ratio
- WHEN semantic_ratio is 0.0 THEN system SHALL perform keyword-only search
- WHEN semantic_ratio is 1.0 THEN system SHALL perform semantic-only search
- WHEN semantic_ratio is 0.5 THEN system SHALL balance keyword and semantic equally

---

### FR-5: Migration from HTTP Client

#### FR-5.1: Sidecar Removal
- WHEN migrating THEN system SHALL remove SidecarManager entirely
- WHEN migrating THEN system SHALL remove meilisearch binary download logic
- WHEN migrating THEN system SHALL remove process health monitoring

#### FR-5.2: Data Compatibility
- WHEN migrating THEN system SHALL preserve existing index data
- IF data format is incompatible THEN system SHALL prompt for re-indexing
- WHEN preserving data THEN system SHALL verify index integrity

#### FR-5.3: API Mapping
- WHEN migrating search calls THEN system SHALL map SDK methods to lib methods
- WHEN migrating settings THEN system SHALL preserve configuration
- WHEN migrating embedders THEN system SHALL use native `update_embedders()`

---

## Non-Functional Requirements

### NFR-1: Performance

#### NFR-1.1: Startup Time
- Database initialization SHALL complete in < 2 seconds on SSD
- Search availability SHALL be signaled within 3 seconds of app launch

#### NFR-1.2: Search Latency
- Keyword search SHALL complete in < 50ms for indexes < 100K documents
- Hybrid search SHALL complete in < 200ms including embedding generation

#### NFR-1.3: RAG Latency
- Context retrieval SHALL complete in < 300ms
- Time-to-first-token for streaming SHALL be < 1 second (network dependent)

### NFR-2: Reliability

#### NFR-2.1: Error Handling
- All operations SHALL return Result types with descriptive errors
- Network failures to LLM providers SHALL be reported with retry guidance
- Index corruption SHALL be detected and reported

#### NFR-2.2: Graceful Degradation
- IF embedder is unavailable THEN system SHALL fall back to keyword-only search
- IF LLM provider is unavailable THEN system SHALL report error, not crash

### NFR-3: Compatibility

#### NFR-3.1: Platform Support
- Integration SHALL work on Linux (x86_64, aarch64)
- Integration SHALL work on Windows (x86_64)
- Integration SHALL work on macOS (x86_64, aarch64)

#### NFR-3.2: Data Compatibility
- meilisearch-lib SHALL be compatible with existing index data format
- Settings and embedder configurations SHALL be preserved

---

## Constraints

### C-1: No External Processes
- System SHALL NOT spawn or manage any external processes for search
- All search and RAG operations SHALL be in-process library calls

### C-2: Backward Compatibility
- Existing SQLite database schema SHALL remain unchanged
- Existing document library metadata SHALL be preserved
- Chat history and campaign data SHALL be unaffected

### C-3: Feature Parity
- All current search features SHALL continue working:
  - TTRPG synonym expansion
  - Spell correction
  - Query suggestions
  - Filters and sorting

### C-4: Single Binary Distribution
- Application SHALL remain a single binary (plus assets)
- No runtime downloads or plugin installations required

---

## Assumptions

### A-1: meilisearch-lib Stability
- meilisearch-lib is assumed stable for production use
- API may evolve, but core functionality is reliable

### A-2: LLM Provider Availability
- At least one LLM provider is configured and accessible
- API keys are valid and have sufficient quota

### A-3: Embedder Availability
- Ollama is available locally for embeddings, OR
- OpenAI API key is configured for cloud embeddings

### A-4: Existing Data Volume
- Average user library: < 100 documents, < 500MB indexed
- Power user library: < 1000 documents, < 5GB indexed

---

## Glossary

| Term | Definition |
|------|------------|
| **RAG** | Retrieval-Augmented Generation - combining search with LLM for Q&A |
| **meilisearch-lib** | Embedded Meilisearch library without HTTP server |
| **Hybrid Search** | Combining keyword (BM25) and semantic (vector) search |
| **Semantic Ratio** | Balance between keyword (0.0) and semantic (1.0) search |
| **ChatConfig** | Configuration for the RAG pipeline including LLM provider |
| **ChatIndexConfig** | Per-index configuration for document formatting in RAG |
| **Liquid Template** | Template language for formatting documents in context |
| **Source Citations** | Document IDs included in RAG responses for verification |
