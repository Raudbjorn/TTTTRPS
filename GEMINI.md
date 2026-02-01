# TTRPG Assistant (Sidecar DM) - Developer Context

This file provides comprehensive context for AI agents working on the TTRPG Assistant codebase.

## Project Overview

TTRPG Assistant is a local-first, AI-powered desktop application for Game Masters. It integrates LLM capabilities from multiple providers with campaign management tools.

**LLM Providers:**
*   **OAuth-based (via `gate` module):** Claude, Gemini, Copilot (implemented)
*   **API-key based:** OpenAI, Ollama - configured in Settings with API keys

**Core Technologies:**
*   **Backend:** Rust (Tauri v2.x)
*   **Frontend:** Rust (Leptos v0.7 WASM)
*   **Database:** SQLite (via `sqlx`)
*   **Search:** Meilisearch (embedded)
*   **Document Processing:** `kreuzberg` (PDF/EPUB extraction)
*   **Authentication:** `gate` module (Unified OAuth for Claude/Gemini/Copilot)
*   **Design System:** Shadcn-UI inspired components (Tailwind CSS) & Phosphor Icons (`phosphor-leptos`)

## Architecture

### Directory Structure

*   `src-tauri/` - Rust Backend
    *   `src/commands.rs` - Tauri IPC command handlers (API surface).
    *   `src/gate/` - Unified OAuth authentication system (Claude & Gemini; Copilot planned).
    *   `src/core/` - Business logic:
        *   `llm/` - LLM provider implementations.
        *   `search/` - Hybrid search (BM25 + Vector).
        *   `session/` - Campaign session state management.
    *   `src/ingestion/` - Document parsing and chunking pipelines.
    *   `src/database/` - SQLx database models and queries.
*   `frontend/` - Leptos Frontend
    *   `src/app.rs` - Main application entry point and routing.
    *   `src/bindings/` - Modular Tauri IPC bindings (`ai`, `campaign`, `world`, etc.).
    *   `src/components/` - UI components.
        *   `design_system/` - Reusable primitives (Button, Card) following Shadcn patterns.
    *   `src/services/` - Frontend state management and business logic.
    *   `src/utils/` - Shared utilities (formatting, helpers).
*   `assets/` - Static assets and default configuration.
*   `planning/` - Design documents and task tracking.

### Key Modules

*   **Gate (`src-tauri/src/gate/`)**: Handles OAuth flows for external LLM providers.
    *   `client.rs`: Main entry point (`GateClient`).
    *   `auth/flow.rs`: Generic OAuth flow orchestrator.
    *   `providers/`: Provider-specific implementations (`claude.rs`, `gemini.rs`; `copilot.rs` planned).
*   **Core (`src-tauri/src/core/`)**:
    *   `llm_router.rs`: Routes requests to the appropriate model/provider.
    *   `meilisearch_pipeline.rs`: Manages the indexing of ingested content.
*   **Frontend Bindings (`frontend/src/bindings/`)**:
    *   `mod.rs`: Re-exports all sub-modules for backward compatibility.
    *   `core.rs`: Low-level IPC and FFI.
    *   `ai.rs`, `campaign.rs`, `world.rs`, etc.: Domain-specific command wrappers.

## Building and Running

The project uses a unified build script: `build.sh`.

*   **Development:**
    ```bash
    ./build.sh dev
    ```
    Starts the Tauri development window with hot-reloading for the frontend.

*   **Build Release:**
    ```bash
    ./build.sh build --release
    ```
    Produces optimized binaries in `src-tauri/target/release/bundle/`.

*   **Testing:**
    ```bash
    ./build.sh test
    ```
    Runs both backend (Rust) and frontend (WASM) tests.

*   **Check/Lint:**
    ```bash
    ./build.sh check
    ./build.sh lint
    ```

## Development Conventions

### Rust (Backend)
*   **Async/Await:** Heavy usage of `tokio` runtime. Most core functions are `async`.
*   **Error Handling:** Custom `Error` types in modules, generally using `thiserror`. `anyhow` is used in some top-level handlers.
*   **State Management:** Tauri's `State<'_, T>` is used to pass global state (Database, Search Client) to commands.
*   **Database:** Compile-time checked queries with `sqlx`. Migrations are in `src-tauri/migrations/`.

### Leptos (Frontend)
*   **Signals:** State is managed via Leptos signals (`ReadSignal`, `WriteSignal`, `RwSignal`).
*   **Components:** Functional components using the `#[component]` macro.
*   **Styling:** Tailwind CSS (v4) via `input.css`. Classes are applied directly in `view!` macros.
*   **Icons:** Use `phosphor-leptos` components (e.g., `<Icon icon=TRASH />`). Avoid raw SVGs.

### Commit Strategy
*   Prefer small, atomic commits.
*   Run `./build.sh check` before committing to ensure no compilation errors.
