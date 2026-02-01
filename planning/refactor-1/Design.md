# Design: Codebase Refactoring Overhaul

*Note: For specific execution steps and phase numbering, `Tasks.md` is the canonical source. This document outlines the architectural strategy.*

## Overview

This document describes the technical design for refactoring the TTRPG Assistant codebase to reduce line count, eliminate dead code, and improve maintainability. The design addresses all requirements from Requirements.md.

### Design Goals

1. **Reduce total LOC by ~15%** (~7,700 lines) through dead code removal and deduplication
2. **Break monolithic files into focused modules** with max 1,500 lines each
3. **Eliminate all compiler warnings** (`dead_code`, `unused_variables`, `deprecated`)
4. **Establish patterns that prevent future bloat** through shared infrastructure

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Domain-based command extraction | Commands naturally group by domain (LLM, voice, campaign); reflects user mental model |
| Delete `llm_router.rs` | Analysis confirms it's entirely unused dead code (2,131 lines) |
| Shared OAuth infrastructure | Three providers duplicate identical patterns; generic trait saves ~600 lines |
| Error type consolidation | 400+ `.map_err()` calls can use a single error type |
| Keep `bindings.rs` as-is | Auto-generated file; manual intervention risks drift |
| Preserve function names | Function names (e.g., `get_campaign`) MUST match the command name to avoid breaking frontend bindings. Do not rename to `get` inside a module unless using `#[tauri::command(rename="...")]`. |

---

## Architecture

### Current State

```
src-tauri/src/
├── commands.rs          # 10,679 lines - ALL Tauri commands
├── main.rs              # 31,242 lines
├── backstory_commands.rs # 15,720 lines (already extracted)
├── core/
│   ├── llm/
│   │   └── router.rs    # 2,563 lines - ACTIVE router
│   ├── llm_router.rs    # 2,131 lines - DEAD CODE
│   └── ... (40+ modules)
└── ...
```

### Target State

```
src-tauri/src/
├── commands/
│   ├── mod.rs           # Registration and re-exports (~200 lines)
│   ├── state.rs         # AppState definition (~200 lines)
│   ├── error.rs         # Unified error types (~50 lines)
│   ├── macros.rs        # Helper macros (~30 lines)
│   ├── llm/             # 5 files, ~980 lines total
│   ├── oauth/           # 4 files, ~850 lines total
│   ├── documents/       # 4 files, ~980 lines total
│   ├── campaign/        # 6 files, ~640 lines total
│   ├── session/         # 6 files, ~1,060 lines total
│   ├── npc/             # 4 files, ~580 lines total
│   ├── voice/           # 7 files, ~1,270 lines total
│   ├── personality/     # 4 files, ~900 lines total
│   ├── archetype/       # 4 files, ~1,200 lines total
│   └── ...              # Remaining small modules
├── main.rs              # Reduced (~500 lines)
├── core/
│   ├── llm/
│   │   └── router.rs    # Unchanged (active implementation)
│   └── ...              # llm_router.rs DELETED
└── ...
```

### Component Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Frontend (Leptos)                     │
│                       bindings.rs (auto-gen)                │
└─────────────────────────┬───────────────────────────────────┘
                          │ Tauri IPC
┌─────────────────────────▼───────────────────────────────────┐
│                    commands/mod.rs                          │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           │
│  │  llm/   │ │ oauth/  │ │campaign/│ │ voice/  │  ...      │
│  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘           │
│       │           │           │           │                 │
│  ┌────▼───────────▼───────────▼───────────▼────┐           │
│  │              state.rs (AppState)             │           │
│  └──────────────────┬──────────────────────────┘           │
└─────────────────────┼───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                         core/                                │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐        │
│  │ llm/router   │ │session_mgr   │ │ archetype/   │  ...   │
│  └──────────────┘ └──────────────┘ └──────────────┘        │
└─────────────────────────────────────────────────────────────┘
```

---

## Components and Interfaces

### Component 1: Unified Error Type (`commands/error.rs`)

**Purpose:** Eliminate repetitive `.map_err(|e| e.to_string())` patterns.

**Interface:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("LLM error: {0}")]
    Llm(#[from] crate::core::llm::LLMError),

    #[error("Voice error: {0}")]
    Voice(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("{0}")]
    Other(String),
}

// Tauri commands require Result<T, String>, so we implement Into<String>
impl From<CommandError> for String {
    fn from(e: CommandError) -> String {
        e.to_string()
    }
}
```

*Note: Tauri IPC requires `Result<T, String>` for command returns. The `From<CommandError> for String` impl enables `?` operator with automatic conversion. Alternative: implement `serde::Serialize` on `CommandError` for richer error payloads.*

**Usage:**
```rust
// Before (400+ instances)
.map_err(|e| e.to_string())

// After
.map_err(CommandError::from)?
// or simply
?  // with proper From impls
```

### Component 2: Shared OAuth Infrastructure (`commands/oauth/common.rs`, `commands/oauth/state.rs`)

**Purpose:** Eliminate triplicated OAuth flow logic AND state management for Claude, Gemini, and Copilot.

**Interface (`commands/oauth/common.rs`):**
```rust
#[async_trait]
pub trait OAuthGate: Send + Sync + 'static {
    fn provider_name(&self) -> &'static str;
    fn storage_backend(&self) -> StorageBackend;
    async fn is_authenticated(&self) -> Result<bool, CommandError>;
    async fn get_status(&self) -> Result<OAuthStatus, CommandError>;
    async fn start_oauth(&self) -> Result<OAuthFlowState, CommandError>;
    async fn complete_oauth(&self, code: &str) -> Result<(), CommandError>;
    async fn logout(&self) -> Result<(), CommandError>;
}

// ... StorageBackend and OAuthStatus enums as before ...
```

**State Abstraction (`commands/oauth/state.rs`):**
```rust
/// Generic state manager for any OAuth provider.
/// Handles backend switching, pending state verification, and client access.
pub struct GenericGateState<T: OAuthGate> {
    client: AsyncRwLock<Option<Box<dyn OAuthGate>>>, // Or specific T if we don't need trait objects here
    pending_oauth_state: AsyncRwLock<Option<String>>,
    storage_backend: AsyncRwLock<StorageBackend>,
    factory: Box<dyn Fn(StorageBackend) -> Result<T, CommandError> + Send + Sync>,
}

impl<T: OAuthGate> GenericGateState<T> {
    pub fn new(backend: StorageBackend, factory: impl Fn...) -> Self { ... }
    pub async fn switch_backend(&self, new_backend: StorageBackend) -> Result<(), CommandError> { ... }
    pub async fn start_oauth(&self) -> Result<(String, String), CommandError> {
        // Common logic: get client -> start flow -> store pending state -> return url
    }
    // ... complete_oauth with state verification implemented ONCE here ...
}
```

**Implementation Notes:**
- `OAuthGate` trait abstracts the *provider API operations*.
- `GenericGateState<T>` abstracts the *application state management* (locks, backend switching, flow verification).
- This removes ~300 lines of duplicated state management code found in `commands.rs`.

### Component 3: Command Module Structure (`commands/`)

**Purpose:** Replace 10,679-line monolith with organized domain modules.

**Module Registration Pattern:**
```rust
// commands/mod.rs
pub fn register_commands(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder
        .invoke_handler(tauri::generate_handler![
            // LLM
            llm::configure_llm,
            llm::chat,
            llm::stream_chat,
            // OAuth
            oauth::claude_get_status,
            oauth::claude_start_oauth,
            // ... etc
        ])
}
```

**Domain Modules:** (404 commands total, counts are approximate)

| Module | Commands | Purpose |
|--------|----------|---------|
| `llm/` | ~25 | LLM configuration, chat, streaming, model listing |
| `oauth/` | ~18 | Claude, Gemini, Copilot OAuth flows |
| `documents/` | ~35 | Document ingestion, search, library, extraction settings |
| `campaign/` | ~30 | Campaign CRUD, themes, snapshots, notes, versioning |
| `session/` | ~45 | Sessions, combat, conditions, timeline, notes |
| `npc/` | ~25 | NPC generation, CRUD, vocabulary, conversations |
| `voice/` | ~55 | Voice config, synthesis, queue, presets, profiles, cache |
| `personality/` | ~40 | Personality application, templates, blending, context |
| `archetype/` | ~50 | Archetype CRUD, vocabulary banks, setting packs, resolution |
| `credentials/` | ~10 | API key management |
| `meilisearch/` | ~15 | Meilisearch health and chat integration |
| `audio/` | ~12 | Audio playback controls |
| `theme/` | ~8 | UI theme management |
| `utility/` | ~15 | File dialogs, system utilities |
| `character/` | ~6 | Character generation |
| `location/` | ~15 | Location generation and CRUD |

*Note: Exact counts will be determined during extraction. See commands-analysis.md for detailed breakdown.*

### Component 4: State Access Helpers (`commands/macros.rs`)

**Purpose:** Reduce boilerplate for common state access patterns.

```rust
/// Access read-locked state manager
macro_rules! read_state {
    ($state:expr, $field:ident) => {
        $state.$field.read().await
    };
}

/// Access write-locked state manager
macro_rules! write_state {
    ($state:expr, $field:ident) => {
        $state.$field.write().await
    };
}

/// Execute database operation with connection
macro_rules! with_db {
    ($state:expr, |$conn:ident| $body:expr) => {{
        let db = $state.database.read().await;
        let $conn = db.connection();
        $body
    }};
}
```

---

## Dead Code Elimination

### Confirmed Dead Code (From Analysis)

*Note: Use `cargo clippy` to get current locations as line numbers may shift during refactoring.*

| File | Item | Est. Lines | Action |
|------|------|------------|--------|
| `core/llm_router.rs` | Entire file | 2,131 | DELETE |
| `core/llm/router.rs` | `StreamState` unused fields | ~4 | Prefix `_` or remove |
| `core/llm/providers/claude.rs` | `storage_name()` method | ~10 | DELETE |
| `core/llm/providers/gemini.rs` | `storage_name()` method | ~10 | DELETE |
| `core/llm/providers/copilot.rs` | `storage_name()` method | ~10 | DELETE |
| `core/meilisearch_pipeline.rs` | `process_text_file()` method | ~50 | DELETE |
| `core/voice/providers/coqui.rs` | `TtsRequest` struct | ~20 | DELETE |
| `ingestion/claude_extractor.rs` | `PAGE_EXTRACTION_PROMPT` const | ~10 | DELETE |
| `ingestion/layout/column_detector.rs` | `DEFAULT_MIN_COLUMN_WIDTH` const | ~1 | DELETE |

### Unused Variables (Prefix with `_`)

*Identify via `cargo build` warnings or `cargo clippy`. Common patterns:*

| File | Variables |
|------|-----------|
| `commands.rs` | `system_prompt`, `connection_type`, `description` |
| `core/voice/queue.rs` | `was_pending` |
| `core/search/hybrid.rs` | `query_embedding`, `filter` |
| `ingestion/kreuzberg_extractor.rs` | `expected_pages` |
| `core/personality/application.rs` | `patterns`, `content_type` |
| `core/character_gen/mod.rs` | `options` |
| `core/location_gen.rs` | `rng` |
| `core/query_expansion.rs` | `words` |
| `core/session/conditions.rs` | `n` |

### Deprecated API Usage

| File | Issue | Fix |
|------|-------|-----|
| `frontend/components/button.rs` | `MaybeSignal<T>` | Use `Signal<T>` |
| `frontend/components/select.rs` | `MaybeSignal<T>` | Use `Signal<T>` |
| `commands.rs` | `Shell::open()` deprecated | Use `tauri-plugin-opener` |

---

## Data Flow

### Command Invocation Flow

```
Frontend                 Commands                    Core
   │                        │                         │
   │ invoke("chat", msg)    │                         │
   │───────────────────────>│                         │
   │                        │ state.llm_router        │
   │                        │ .read().await           │
   │                        │────────────────────────>│
   │                        │                         │ router.chat(req)
   │                        │<────────────────────────│
   │                        │                         │
   │ Result<ChatResponse>   │                         │
   │<───────────────────────│                         │
```

### OAuth Flow

```
Frontend                 Commands                OAuth Client
   │                        │                         │
   │ start_oauth()          │                         │
   │───────────────────────>│                         │
   │                        │ OAuthGate::start_oauth()│
   │                        │────────────────────────>│
   │                        │      auth_url           │
   │<───────────────────────│<────────────────────────│
   │                        │                         │
   │ (user completes flow)  │                         │
   │                        │                         │
   │ complete_oauth(code)   │                         │
   │───────────────────────>│                         │
   │                        │ OAuthGate::complete()   │
   │                        │────────────────────────>│
   │                        │        token            │
   │ OAuthStatus            │<────────────────────────│
   │<───────────────────────│                         │
```

---

## Error Handling

| Error Category | HTTP/IPC Equivalent | User Action |
|----------------|---------------------|-------------|
| `NotFound` | 404 | Check input, try again |
| `InvalidInput` | 400 | Correct input format |
| `Database` | 500 | Report bug, check logs |
| `Llm` | 503 | Check provider status |
| `Voice` | 503 | Check voice provider |
| `Other` | 500 | Report bug |

All errors implement `Into<String>` for Tauri IPC compatibility.

---

## Testing Strategy

### Unit Testing

- **Scope:** Individual command functions with mocked state
- **Location:** Co-located in each command module (`#[cfg(test)]`)
- **Coverage Target:** 80% of command logic

```rust
// commands/llm/chat.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chat_returns_response() {
        let state = mock_app_state();
        let result = chat(state, request).await;
        assert!(result.is_ok());
    }
}
```

### Integration Testing

- **Scope:** Full command flow with real state
- **Location:** `tests/integration/commands/`
- **Focus:** OAuth flows, document ingestion, multi-step operations

### Regression Testing

- **Scope:** All 404 existing commands
- **Method:** Automated IPC validation
- **Criteria:** Command names unchanged, signature compatibility maintained

---

## Quality Assurance & Inspections

### Inspection Results Summary

The codebase was analyzed with IDE inspections, identifying ~2,900 issues across 28 categories. The table below prioritizes actionable items:

| Category | Count | Severity | Action |
|----------|-------|----------|--------|
| Rust compiler warnings | ~50 | **High** | Fix in Phase 1 |
| `RsArgumentNaming` | 543 | Low | Suppress (Rust convention in macros/derives) |
| `RsFunctionNaming` | 254 | Low | Suppress (Leptos PascalCase components) |
| `RsFieldNaming` | 181 | Low | Suppress (serde renames, FFI) |
| `CssUnusedSymbol` | 166 | Medium | Investigate in Phase 7 |
| `RsFunctionSyntax` | 140 | Low | IDE parsing false positives |
| `ShellCheck` | 34 | Medium | Fix in Phase 7 |
| `RsStructNaming` | 34 | Low | Suppress (Leptos components) |
| `CssInvalidHtmlTagReference` | 11 | Low | Investigate (custom elements) |
| `DuplicatedCode` | 2 | **High** | Fix in Phase 7 |
| `TomlUnresolvedReference` | 4 | Medium | Fix or document |
| `HtmlRequiredLangAttribute` | 2 | Medium | Fix in Phase 7 |

### Current Rust Compiler Warnings (from `cargo build`)

These warnings MUST be resolved in Phase 1:

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `extraction_settings.rs` | 308 | `cfg(feature = "chunking")` undefined | Add feature to Cargo.toml OR remove cfg |
| `core/character_gen/mod.rs` | 318 | Unused variable `options` | Prefix with `_` |
| `core/location_gen.rs` | 549, 571 | Unused variable `rng` | Prefix with `_` |
| `core/search/hybrid.rs` | 533, 619 | Unused variables `query_embedding`, `filter` | Prefix with `_` |
| `core/search/hybrid.rs` | 705-706 | Dead code fields `keyword_rank`, `semantic_rank` | Remove or use |
| `core/personality/contextual.rs` | 31 | Unused import `BlendedProfile` | Remove import |
| `core/query_expansion.rs` | 195 | Unused variable `words` | Prefix with `_` |
| `core/voice/queue.rs` | 697 | Unused variable `was_pending` | Prefix with `_` |
| `core/voice/queue.rs` | 580 | Dead code field `command_rx` | Remove or use |
| `core/session/conditions.rs` | 353 | Unused variable `n` | Prefix with `_` |
| `core/llm/providers/gemini.rs` | 187 | Dead code method `storage_name()` | Delete method |
| `core/llm/providers/claude.rs` | 181 | Dead code method `storage_name()` | Delete method |
| `core/campaign/meilisearch_client.rs` | 101 | Dead code field `api_key` | Remove or use |

### Cargo Feature Flag Issue

The following feature is referenced but not defined:

```toml
# src-tauri/Cargo.toml - line 308 references:
#[cfg(feature = "chunking")]

# Fix: Add to [features] section:
[features]
chunking = []  # Or remove the cfg attribute if feature is obsolete
```

### Code Inspection Standards

| Inspection ID | Category | Strategy | Rationale |
|---------------|----------|----------|-----------|
| `RsFunctionNaming` | Naming | **Suppress/Ignore** for Components | Leptos components use PascalCase by design; Rust conventions expect snake_case. |
| `RsArgumentNaming` | Naming | **Suppress/Ignore** | Generated by derives and macros; not fixable without breaking serde/tauri. |
| `DuplicatedCode` | Maintenance | **Consolidate** | CSS animations must be deduplicated to single source of truth. |
| `HtmlRequiredLangAttribute` | Accessibility | **Fix** | Ensure `<html>` tags have `lang="en"`. |
| `CssUnusedSymbol` | Cleanup | **Investigate** | Remove unused CSS classes if confirmed unused in dynamic calls. |
| `ShellCheck` | Scripts | **Fix** | Separate declare and assign; add `|| exit` to `cd` commands. |
| `TomlUnresolvedReference` | Config | **Document** | IDE limitation with cross-crate feature resolution; build works. |
| `Markdown*` | Documentation | **Ignore** | IDE false positives on code blocks and relative paths. |

### Deduplication Strategy

**CSS Duplication Found:**
- `frontend/public/effects.css:37` and `frontend/public/themes.css:335` both define `@keyframes grain`
- **Fix:** Extract to `frontend/public/shared-animations.css` and import once

**Rust Logic Deduplication:**
- OAuth storage patterns in Claude/Gemini/Copilot providers
- Extract to `commands/oauth/common.rs` as documented in Components section

### Shell Script Issues (ShellCheck)

**`build.sh` (30+ warnings):**

| Issue Type | Count | Example | Fix |
|------------|-------|---------|-----|
| Declare/assign separation | ~15 | `local x=$(cmd)` | `local x; x=$(cmd)` |
| Unused variables | 4 | `PACKAGE`, `TEST`, `BUILD`, `DIST_DIR` | Remove or export |
| Literal braces | 2 | `${@}` in arithmetic | Quote or escape |
| Exit code pattern | 1 | `if [ $? -eq 0 ]` | `if cmd; then` |

**`mcp-server` scripts (2 warnings):**

| Issue | Fix |
|-------|-----|
| `cd` without error handling | Add `|| exit` after `cd "$PROJECT_ROOT"` |

### IDE False Positives (Ignore)

The following inspection results are false positives and do not require action:

| Category | Count | Reason |
|----------|-------|--------|
| `MarkdownUnresolvedFileReference` | 801 | IDE doesn't follow relative paths in docs |
| `RsFunctionNaming` | 254 | Leptos components use PascalCase by design |
| `RsArgumentNaming` | 543 | Generated by serde/tauri derives |
| `Annotator` | 659 | IDE parsing Rust code blocks in markdown |
| `RsFunctionSyntax` | 140 | IDE parsing code blocks in docs |

---

## Migration Strategy

### Phase 0: Infrastructure Setup
1. Create `commands/` directory
2. Add `error.rs`, `state.rs`, `macros.rs`
3. Verify build passes

### Phase 1: Dead Code Removal
1. Delete `core/llm_router.rs`
2. Remove unused functions/fields from clippy output
3. Prefix unused variables with `_`
4. Target: Zero `dead_code` warnings

### Phase 2: OAuth Extraction
1. Create `oauth/common.rs` with `OAuthGate` trait
2. Extract Claude → `oauth/claude.rs`
3. Extract Gemini → `oauth/gemini.rs`
4. Extract Copilot → `oauth/copilot.rs`
5. Update `main.rs` registrations

### Phase 3: Large Module Extraction
1. Extract archetype commands (largest isolated group)
2. Extract personality commands
3. Extract voice commands
4. Validate after each group

### Phase 4: Core Module Extraction
1. Extract LLM commands
2. Extract document commands
3. Extract session commands

### Phase 5: Entity Management Extraction
1. Extract campaign, NPC, location
2. Extract credentials, audio, theme, utility, meilisearch, character
3. Delete original `commands.rs`

### Phase 6: Remaining Modules and Cleanup
1. Extract credentials commands
2. Extract audio commands
3. Extract theme commands
4. Extract utility commands
5. Extract meilisearch commands
6. Extract character commands
7. Delete original `commands.rs`

### Phase 7: Frontend Cleanup
1. Migrate `MaybeSignal` → `Signal`
2. Fix deprecated `Shell::open` usage
3. Consolidate duplicated CSS
4. Verify bindings and fix inspections

### Phase 8: Final Validation
1. Run full test suite
2. Verify metrics
3. **Automated command name verification:**
   - Capture snapshot of all Tauri command names (functions with `#[tauri::command]`)
   - Compare pre-refactor snapshot to post-refactor command names
   - Fail if any command name changed without `#[tauri::command(rename="...")]`
   - Verification script: `grep -r "#\[tauri::command" src-tauri/src/commands/ | sort`
4. Update documentation
5. Create PR

---

## Success Metrics

| Metric | Before | Target | Validation |
|--------|--------|--------|------------|
| `commands.rs` LOC | 10,679 | 0 (extracted) | `wc -l` |
| Total backend LOC | ~51,500 | <44,000 (~15% reduction) | `tokei` |
| Dead code removed | 0 | ~3,000+ lines | `cargo clippy` |
| Compiler warnings | 50+ | 0 | `cargo build` |
| Max file LOC | 10,679 | <1,500 | `wc -l` |
| Command count | 404 | 404 (unchanged) | IPC test |
| Test pass rate | 100% | 100% | `cargo test` |

*The 15% target (~7,700 lines) comes from: dead llm_router.rs (2,131) + command extraction overhead reduction (~1,500) + dead code cleanup (~1,000) + test consolidation (~500) + deduplication savings (~2,500+).*

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking IPC contract | Medium | High | Never rename commands; add aliases if needed |
| Merge conflicts | High | Medium | Feature branch; frequent rebases |
| Missing edge cases | Medium | Medium | Comprehensive test coverage |
| Performance regression | Low | Medium | Benchmark before/after |

---

## Supporting Analysis Documents

The following analysis documents inform this design and should be consulted for detailed breakdowns:

| Document | Purpose | Key Findings |
|----------|---------|--------------|
| `commands-analysis.md` | Detailed breakdown of 404 commands across 46 groups | OAuth duplication (906 lines), CRUD patterns, error handling (400+ `.map_err` calls) |
| `llm-router-analysis.md` | Analysis of dual router files | `llm_router.rs` is dead code (2,131 lines), OAuth trait abstraction saves ~600 lines |
| `test-consolidation-analysis.md` | Test organization and duplication | 200-1000 lines saveable, shared fixture opportunities |

---

## Dependencies

This refactoring has no external dependencies. It uses existing:
- `thiserror` for error types
- `tokio` for async runtime
- `serde` for serialization
- All existing Tauri patterns

No new crates required.

---

## Appendix: Inspection Results Reference

The following inspection result files in `./inspection-results/` were analyzed for this design:

### High Priority (Action Required)

| File | Issues | Status |
|------|--------|--------|
| `DuplicatedCode_aggregate.json` | 2 CSS duplications | Tasks 7.5 |
| `ShellCheck.json` | 34 shell warnings | Tasks 7.3, 7.4 |
| `HtmlRequiredLangAttribute.json` | 2 missing lang attrs | Task 7.6 |
| `TomlUnresolvedReference.json` | 4 feature refs | Task 1.5 |

### Medium Priority (Investigate)

| File | Issues | Status |
|------|--------|--------|
| `CssUnusedSymbol.json` | 166 unused symbols | Task 7.7 |
| `CssNoGenericFontName.json` | 9 missing fallbacks | Evaluate |
| `CssInvalidHtmlTagReference.json` | 11 custom elements | Document |

### Rust Categorized Inspections (`inspection-results/rust/`)

The `rust/` subdirectory contains categorized inspection results. These are the actionable items from source code:

#### WARNING Level (Should Fix)

| Inspection | File:Line | Issue | Fix |
|------------|-----------|-------|-----|
| `RsSelfConvention` | `core/llm/health.rs:420` | `is_*` method takes `&mut self` | Rename method or change to `&self` |
| `RsSelfConvention` | `core/session/plan_types.rs:704` | `from_*` method takes `&self` | Rename method or remove self parameter |

#### WEAK WARNING Level (Nice to Have)

| Inspection | File:Line | Issue | Fix |
|------------|-----------|-------|-----|
| `RsLift` | `gate/gemini/transport/http.rs:159` | Return can be lifted out of match | Restructure to single return |
| `RsLift` | `gate/copilot/auth/device_flow.rs:224` | Return can be lifted out of match | Restructure to single return |
| `RsLift` | `ingestion/ttrpg/dice_extractor.rs:496` | Return can be lifted out of if | Restructure to single return |
| `RsFieldInitShorthand` | `frontend/src/components/settings/voice.rs:284-287` | Non-shorthand field init (4 fields) | Use shorthand: `{ field }` not `{ field: field }` |
| `HttpUrlsUsage` | `core/sidecar_manager.rs:47` | HTTP URL in source | Intentional (localhost); document |
| `HttpUrlsUsage` | `core/llm/proxy.rs:373` | HTTP URL in source | Intentional (localhost); document |

#### TYPO Level (133 files - Domain Terms)

The `rust/typo/SpellCheckingInspection_part*.toon` files (133 parts) contain spelling warnings for domain-specific terms (TTRPG, API names, etc.). These should be added to a project dictionary rather than "fixed."

#### FALSE POSITIVES in `rust/error/`

| Inspection | Location | Reason to Ignore |
|------------|----------|------------------|
| `RsFunctionCannotHaveSelf` | docs/, planning/done/ | Code blocks in markdown lack impl context |
| `RsInvalidLiteralSuffix` | planning/done/ | Markdown code snippets |
| `RsUnclosedTextLiteral` | planning/done/ | Markdown code snippets |
| `RsFunctionCannotBeVariadic` | planning/done/ | Markdown code snippets |

### Low Priority (Suppress/Ignore)

| File | Issues | Reason |
|------|--------|--------|
| `RsFunctionNaming.json` | 254 | Leptos PascalCase |
| `RsArgumentNaming.json` | 543 | Derive-generated |
| `RsFieldNaming.json` | 181 | Serde renames |
| `RsStructNaming.json` | 34 | Leptos components |
| `MarkdownUnresolvedFileReference.json` | 801 | IDE limitation |
| `Annotator.json` | 659 | Code block parsing |
| `RsFunctionSyntax.json` | 140 | Markdown code blocks |

### Informational Only

| File | Issues | Notes |
|------|--------|-------|
| `GrazieInspection.json` | - | Grammar suggestions |
| `GrazieStyle.json` | - | Style suggestions |
| `SpellCheckingInspection.json` | - | Spelling in docs |
| `JsonSchemaRefReference.json` | 4 | Schema refs |
| `JsonStandardCompliance.json` | 2 | JSON5 vs JSON |
