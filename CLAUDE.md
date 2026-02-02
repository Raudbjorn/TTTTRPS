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
| `commands.rs` | All Tauri IPC command handlers (~3000+ lines) |
| `core/llm/` | LLM provider implementations (Claude, Gemini, OpenAI, Ollama) |
| `core/llm_router.rs` | Multi-provider routing with cost tracking and failover |
| `core/search_client.rs` | Meilisearch client for hybrid search (BM25 + vector) |
| `core/meilisearch_pipeline.rs` | Document ingestion pipeline using kreuzberg |
| `core/session_manager.rs` | Campaign session state, combat tracker, conditions |
| `ingestion/kreuzberg_extractor.rs` | Document extraction (PDF, EPUB, DOCX, images) with OCR fallback |
| `ingestion/chunker.rs` | Semantic text chunking with TTRPG-aware splitting |
| `database/` | SQLite with SQLx migrations |

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

- **Database**: `~/.local/share/ttrpg-assistant/ttrpg_assistant.db`
- **Meilisearch**: `~/.local/share/ttrpg-assistant/meilisearch/`
- **API Keys**: System keyring via `keyring` crate

## External Dependencies

- **Meilisearch**: Embedded, auto-started by the app
- **Tesseract OCR**: Required for scanned PDF extraction (`tesseract` + `pdftoppm`)
- **pdfinfo**: Used for PDF page count estimation

## LLM Providers

Configured in Settings, stored in system keyring:
- Claude (Anthropic) - claude-3-5-sonnet, claude-3-haiku
- Gemini (Google) - gemini-1.5-pro, gemini-1.5-flash
- OpenAI - gpt-4o, gpt-4-turbo
- Ollama (local) - no API key required

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
