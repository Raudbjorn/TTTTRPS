# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TTTTRPS is an AI-powered TUI application for Game Masters running tabletop RPG sessions. Built entirely in Rust with a ratatui terminal interface. Forked from [TTTRPS](https://github.com/Raudbjorn/TTTRPS) (Tauri+Leptos desktop app) — the core engine is shared, the UI layer is terminal-native.

## Build Commands

```bash
cargo run                  # Development mode
cargo build --release      # Production build
cargo check                # Type check
cargo test --lib           # Run unit tests
cargo test                 # Run all tests
```

### Running Tests

```bash
# All tests
cargo test

# Specific test module
cargo test chunker_tests

# With output
cargo test -- --nocapture

# Single test
cargo test test_name -- --exact

# Test summary (errors, warnings, failures in one pass)
cargo test --lib 2>&1 | grep -E "^error\[|warning.*generated|Finished|FAILED|^test result:"
```

## Architecture

### Core (`src/`)

| Module | Purpose |
|--------|---------|
| `main.rs` | TUI entry point (ratatui + crossterm event loop) |
| `lib.rs` | Library root (core, database, ingestion, oauth) |
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
| `core/voice/` | Voice synthesis with priority queue and provider abstraction |
| `core/personality/` | DM personality system with blending and templates |
| `ingestion/kreuzberg_extractor.rs` | Document extraction (PDF, EPUB, DOCX, images) with OCR fallback |
| `ingestion/chunker.rs` | Semantic text chunking with TTRPG-aware splitting |
| `database/` | SQLite with SQLx migrations (legacy, being migrated to SurrealDB) |
| `oauth/` | OAuth flows for Claude, Gemini, and GitHub Copilot |

### Search Architecture (SurrealDB)

Embedded SurrealDB with RocksDB storage (no external process required). Migrating from SQLite + Meilisearch.

| Module | Purpose |
|--------|---------|
| `core/storage/surrealdb.rs` | `SurrealStorage` wrapper with connection pooling |
| `core/storage/schema.rs` | Database schema (tables, analyzers, indexes) |
| `core/storage/search.rs` | Hybrid, vector, and fulltext search operations |
| `core/storage/rag.rs` | RAG context retrieval and formatting |
| `core/storage/ingestion.rs` | Chunk insertion with embeddings |
| `core/storage/migration.rs` | Data migration from SQLite + Meilisearch |

**Key Types:**
- `SurrealStorage` - Thread-safe wrapper for `Arc<Surreal<Db>>`
- `HybridSearchConfig` - Configures semantic/keyword weights, limits, score normalization
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
- `SynonymMap` - Bidirectional synonym groups with TTRPG-specific defaults
- `QueryPipeline` - Orchestrates the full preprocessing flow
- `DictionaryGenerator` - Builds corpus dictionaries from indexed content

### Key Patterns

**Document Extraction:**
- Uses `kreuzberg` crate with bundled pdfium for fast PDF extraction
- Automatic OCR fallback via external tesseract for scanned documents
- `DocumentExtractor::with_ocr()` for extraction with OCR support

**Voice Queue Events:**
- Event emission uses `QueueEventEmitter` trait (defined in `core/voice/queue/events.rs`)
- Implementations can forward events to TUI widgets or log them
- `NoopEmitter` available for headless/test mode

## Data Storage

- **Database**: `~/.local/share/ttrpg-assistant/surrealdb/` (RocksDB-backed SurrealDB)
- **Legacy SQLite**: `~/.local/share/ttrpg-assistant/ttrpg_assistant.db`
- **API Keys**: System keyring via `keyring` crate
- **Query Preprocessing Dictionaries**:
  - Corpus dictionary: `~/.local/share/ttrpg-assistant/ttrpg_corpus.txt`
  - Bigram dictionary: `~/.local/share/ttrpg-assistant/ttrpg_bigrams.txt`
  - English dictionary: bundled in app resources

## External Dependencies

- **SurrealDB**: Embedded via `surrealdb` crate with `kv-rocksdb` feature - no external process
- **Tesseract OCR**: Optional, for scanned PDF extraction
- **pdfinfo**: Used for PDF page count estimation

## LLM Providers

Stored in system keyring:
- Claude (Anthropic) - claude-3-5-sonnet, claude-3-haiku
- Gemini (Google) - gemini-1.5-pro, gemini-1.5-flash
- OpenAI - gpt-4o, gpt-4-turbo
- Ollama (local) - no API key required
- Mistral, Azure OpenAI, vLLM

## NPC Conversation Modes

NPC conversations support two modes:

| Mode | System Prompt | Use Case |
|------|---------------|----------|
| `"about"` | DM assistant mode | Develop NPC backstory, personality, story hooks |
| `"voice"` | Roleplay mode | AI speaks as NPC in first person |

## Session Resumption Protocol

**CRITICAL**: When resuming from a compacted/summarized session, session summaries may claim work was completed that was never persisted to disk.

Before continuing work after session resumption:

1. **Verify critical changes exist** - Read the actual files mentioned in the summary to confirm edits were saved
2. **Check git status** - If summary claims files were modified, verify they appear in `git diff`
3. **Never trust "completed" claims** - The summary reflects intent, not necessarily disk state
4. **Commit early** - After confirming a fix works, commit immediately before any other operations
