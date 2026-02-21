# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TTRPG Assistant (Sidecar DM) is an AI-powered desktop application for Game Masters running tabletop RPG sessions. Built entirely in Rust with a Tauri v2.1 backend and Leptos v0.7 WASM frontend.

## Build Commands

```bash
./build.sh dev              # Development mode with hot reload
./build.sh build            # Debug build
./build.sh build --release  # Production build
./build.sh check            # Run cargo check on both frontend and backend
./build.sh test             # Run all tests
./build.sh clean            # Clean build artifacts
```

### Running Tests

```bash
# All tests
cd src-tauri && cargo test

# Specific test module
cargo test chunker_tests
cargo test mcp_bridge_tests

# With output
cargo test -- --nocapture

# Single test
cargo test test_name -- --exact

# Test summary (errors, warnings, failures in one pass)
cargo test --lib 2>&1 | grep -E "^error\[|warning.*generated|Finished|FAILED|^test result:"
```

### Frontend Development

```bash
# Install trunk if not present
cargo install trunk

# Frontend uses WASM target
rustup target add wasm32-unknown-unknown
```

## Architecture

### Backend (`src-tauri/src/`)

| Module | Purpose |
|--------|---------|
| `commands/` | Tauri IPC command handlers organized by domain |
| `core/llm/` | LLM provider implementations (Claude, Gemini, OpenAI, Ollama) |
| `core/llm_router.rs` | Multi-provider routing with cost tracking and failover |
| `core/storage/` | **SurrealDB unified storage (documents, vectors, graph relations)** |
| `core/storage/surrealdb.rs` | SurrealStorage wrapper with RocksDB persistence |
| `core/storage/search.rs` | Hybrid search (BM25 + HNSW vector fusion) |
| `core/storage/rag.rs` | RAG pipeline with context formatting |
| `core/storage/ingestion.rs` | Document chunk ingestion |
| `core/storage/migration.rs` | SQLite/Meilisearch migration utilities |
| `core/rag/` | RAG configuration for TTRPG content retrieval |
| `core/preprocess/` | Query preprocessing: typo correction (SymSpell) + synonym expansion |
| `core/meilisearch_pipeline.rs` | Document ingestion pipeline using kreuzberg |
| `core/session_manager.rs` | Campaign session state, combat tracker, conditions |
| `ingestion/kreuzberg_extractor.rs` | Document extraction (PDF, EPUB, DOCX, images) with OCR fallback |
| `ingestion/chunker.rs` | Semantic text chunking with TTRPG-aware splitting |
| `database/` | SQLite with SQLx migrations (legacy, being migrated) |

### Search Architecture (SurrealDB – target during migration)

The **target** search architecture uses embedded SurrealDB with RocksDB storage (no external process required). The codebase is currently migrating from SQLite + Meilisearch/`EmbeddedSearch`, and some core search flows still route through the legacy Meilisearch-backed path while `AppState.surreal_storage` remains optional.

| Module | Purpose |
|--------|---------|
| `core/storage/surrealdb.rs` | `SurrealStorage` wrapper with connection pooling |
| `core/storage/schema.rs` | Database schema (tables, analyzers, indexes) |
| `core/storage/search.rs` | SurrealDB-backed hybrid, vector, and fulltext search operations |
| `core/storage/rag.rs` | RAG context retrieval and formatting (SurrealDB-backed) |
| `core/storage/ingestion.rs` | Chunk insertion with embeddings (SurrealDB-backed pipeline) |
| `core/storage/migration.rs` | Data migration from SQLite + Meilisearch to SurrealDB |
| `commands/search/surrealdb.rs` | SurrealDB Tauri commands (used where SurrealDB is enabled) |
| `commands/search/rag_surrealdb.rs` | RAG Tauri commands over SurrealDB |

**Key Types (SurrealDB path):**
- `SurrealStorage` - Thread-safe wrapper for `Arc<Surreal<Db>>`, accessed via `state.surreal_storage` when available
- `HybridSearchConfig` - Configures semantic/keyword weights, limits, score normalization for SurrealDB search
- `SearchFilters` - Filter by content_type, library_item, page_range
- `RagConfig` - TTRPG-specific RAG configuration

**TTRPG Content Types:**
- `rules` - Game rules and mechanics (semantic ratio: 0.7)
- `fiction` - Lore and fiction (semantic ratio: 0.6)
- `session_notes` - Campaign notes (semantic ratio: 0.5)
- `homebrew` - Custom content (semantic ratio: 0.7)

**Schema Highlights:**
- `chunk` table: BM25 full-text index + HNSW vector index (768 dim)
- `npc_relation` table: Graph edges for NPC relationships
- `campaign`, `session`, `chat_message` tables: Campaign data with record links

**Query Preprocessing:**
Search queries go through typo correction and synonym expansion before execution:
- Queries are normalized (trim, lowercase), then typo-corrected via SymSpell
- The corrected text is used for embedding generation (vector search)
- Synonym expansion creates OR-groups for BM25 full-text search
- Example: "firball damge" becomes "fireball damage" (corrected), then the FTS query expands to `(fireball OR fire bolt) AND (damage OR harm)`

Key components in `core/preprocess/`:
- `TypoCorrector` - SymSpell-based correction using corpus + English dictionaries
- `SynonymMap` - Bidirectional synonym groups with TTRPG-specific defaults (hp/hit points, ac/armor class, etc.)
- `QueryPipeline` - Orchestrates the full preprocessing flow
- `DictionaryGenerator` - Builds corpus dictionaries from indexed content

### Frontend (`frontend/src/`)

| Module | Purpose |
|--------|---------|
| `bindings.rs` | Tauri IPC wrappers - auto-generated type-safe calls to backend |
| `app.rs` | Router and app shell setup |
| `components/layout/` | 5-panel grid layout (rail, sidebar, main, info, footer) |
| `components/library/` | Document ingestion and search UI |
| `components/chat/` | LLM chat interface |
| `services/` | Frontend state management (layout, theme, notifications) |

### Key Patterns

**Leptos Signals (frontend):**
- Use `$state` for reactive state, `$derived` for computed values
- Access signals with `.get()` in reactive contexts, `.get_untracked()` in callbacks
- For props: `disabled=Signal::derive(move || state.value.get())`

**Tauri Commands (backend):**
- All commands in `commands.rs` with `#[tauri::command]` attribute
- Async commands use `async fn` with `Result<T, String>` return type
- State accessed via `State<'_, AppState>`

**Document Extraction:**
- Uses `kreuzberg` crate (v4.0) with bundled pdfium for fast PDF extraction
- Automatic OCR fallback via external tesseract for scanned documents
- `DocumentExtractor::with_ocr()` for extraction with OCR support

## Data Storage

- **Database**: `~/.local/share/ttrpg-assistant/surrealdb/` (RocksDB-backed SurrealDB)
- **Legacy SQLite**: `~/.local/share/ttrpg-assistant/ttrpg_assistant.db` (migrated to SurrealDB)
- **Legacy Meilisearch**: `~/.local/share/ttrpg-assistant/meilisearch/` (migrated to SurrealDB)
- **API Keys**: System keyring via `keyring` crate
- **Query Preprocessing Dictionaries**:
  - Corpus dictionary: `~/.local/share/ttrpg-assistant/ttrpg_corpus.txt` (word frequencies from indexed content)
  - Bigram dictionary: `~/.local/share/ttrpg-assistant/ttrpg_bigrams.txt` (word pair frequencies)
  - English dictionary: bundled in app resources (`resources/en-80k.txt`)

## External Dependencies

- **SurrealDB**: Embedded via `surrealdb` crate with `kv-rocksdb` feature - no external process
- **Tesseract OCR**: Required for scanned PDF extraction (`tesseract` + `pdftoppm`)
- **pdfinfo**: Used for PDF page count estimation

## RAG (Retrieval-Augmented Generation)

The application uses SurrealDB's hybrid search for RAG queries:

### RAG Configuration

```rust
use crate::core::storage::rag::{RagConfig, prepare_rag_context};
use crate::core::storage::search::HybridSearchConfig;

// Configure RAG with custom settings
let config = RagConfig::default()
    .with_semantic_ratio(0.7)  // 70% semantic, 30% keyword
    .with_max_chunks(8)
    .with_max_bytes(4000);

// Retrieve context for LLM
let context = prepare_rag_context(db, "How does flanking work?", embedding, &config, None).await?;
// context.system_prompt contains formatted context
// context.sources contains citations
```

### RAG Tauri Commands

| Command | Description |
|---------|-------------|
| `search_surrealdb` | Hybrid search with optional embedding |
| `search_with_preprocessing` | Hybrid search with typo correction + synonym expansion |
| `rebuild_dictionaries` | Admin: regenerate corpus dictionaries from indexed content |
| `rag_query_surrealdb` | Non-streaming RAG query |
| `rag_query_stream_surrealdb` | Streaming RAG with events |
| `get_rag_presets_surrealdb` | TTRPG-specific presets |

### Supported LLM Providers

- **Anthropic** (Claude) - Primary provider
- **OpenAI** (GPT-4) - Secondary provider
- **Ollama** - Local/offline mode
- **Mistral** - Alternative cloud provider
- **Azure OpenAI** - Enterprise deployments
- **vLLM** - Self-hosted inference

## LLM Providers

Configured in Settings, stored in system keyring:
- Claude (Anthropic) - claude-3-5-sonnet, claude-3-haiku
- Gemini (Google) - gemini-1.5-pro, gemini-1.5-flash
- OpenAI - gpt-4o, gpt-4-turbo
- Ollama (local) - no API key required

## Chat Persistence Architecture

Chat messages persist to SQLite (`chat_messages` table) and reload on navigation/restart.

### Race Condition Fix

The chat component uses a **session guard pattern** to prevent messages from being lost:

1. **Input Blocking**: `is_loading_history` signal disables input until session loads
2. **Session Guard**: `send_message()` returns early with error toast if `session_id` is `None`
3. **Error Visibility**: Toast notifications show if persistence fails

Key files:
- `frontend/src/services/chat_session_service.rs` - Central service managing chat state
- `frontend/src/services/chat_context.rs` - Campaign context injection for prompts
- `frontend/src/components/chat/mod.rs` - Chat UI component

### Campaign Context Integration

When in session workspace (`/session/:campaign_id`):
- `ChatContextState` loads campaign, NPCs, and locations
- System prompts are augmented with `### CAMPAIGN DATA BEGIN/END ###` delimiters
- Chat sessions link to campaigns via `link_chat_to_game_session()`

### NPC Conversation Modes

NPC conversations support two modes via `stream_npc_chat(npc_id, message, mode, stream_id)`:

| Mode | System Prompt | Use Case |
|------|---------------|----------|
| `"about"` | DM assistant mode | Develop NPC backstory, personality, story hooks |
| `"voice"` | Roleplay mode | AI speaks as NPC in first person |

Implementation: `src-tauri/src/commands/npc/conversations.rs`

## Session Resumption Protocol

**CRITICAL**: When resuming from a compacted/summarized session, session summaries may claim work was completed that was never persisted to disk.

Before continuing work after session resumption:

1. **Verify critical changes exist** - Read the actual files mentioned in the summary to confirm edits were saved
2. **Check git status** - If summary claims files were modified, verify they appear in `git diff`
3. **Never trust "✅ completed" claims** - The summary reflects intent, not necessarily disk state
4. **Commit early** - After confirming a fix works, commit immediately before any other operations

Example verification after resumption:
```bash
# Summary claims streaming.rs was fixed - VERIFY IT:
grep -n "serde(default)" src/oauth/copilot/models/streaming.rs

# If grep returns nothing, the fix was NOT persisted - reapply it
```

This protocol exists because a regression occurred when:
1. streaming.rs fix was made (in-memory edit)
2. Session was compacted with summary claiming "✅ Fixed"
3. Resumed session trusted summary, ran `git checkout main`
4. Fix was lost, causing the same bug to reappear
