# Commands.rs Extraction Analysis

## Current State
- **Total lines**: 10,679
- **Number of commands**: 404 `#[tauri::command]` attributes
- **Public functions (total)**: 428
- **Helper/utility functions**: ~24 non-command functions
- **Imports section**: Lines 1-126
- **State definitions**: Lines 128-1270

## File Structure Overview

| Section | Line Range | Lines | Description |
|---------|------------|-------|-------------|
| Imports | 1-126 | 126 | Module imports and utility fn |
| Claude Gate State | 128-443 | 316 | OAuth client for Claude API |
| Gemini Gate State | 444-750 | 307 | OAuth client for Google Cloud Code |
| Copilot Gate State | 751-1033 | 283 | OAuth client for GitHub Copilot |
| Application State | 1034-1210 | 177 | AppState struct and init |
| Database Helper Macro | 1211-1227 | 17 | DB access macro |
| Request/Response Types | 1229-1267 | 39 | Shared DTOs |
| **Commands (total)** | 1268-10679 | 9412 | All Tauri commands |

---

## Command Groups Identified

### Group 1: LLM Commands (lines 1268-1730, ~462 lines)
Core LLM configuration and chat functionality.

**Commands:**
- `configure_llm` - Configure LLM provider settings
- `get_router_stats` - Get router statistics
- `chat` - Synchronous chat completion
- `check_llm_health` - Health check for LLM
- `get_llm_config` - Get current LLM configuration
- `list_ollama_models` - List available Ollama models
- `list_claude_models` - List available Claude models
- `list_openai_models` - List available OpenAI models
- `list_gemini_models` - List available Gemini models
- `list_openrouter_models` - List available OpenRouter models
- `list_provider_models` - Generic provider model listing

**Helpers:**
- `load_llm_config_disk` - Load LLM config from disk
- `load_voice_config_disk` - Load voice config from disk

### Group 2: LLM Router Commands (lines 1732-1930, ~199 lines)
Multi-provider routing, streaming, and cost tracking.

**Commands:**
- `get_router_health` - Router health status
- `get_router_costs` - Cost tracking data
- `estimate_request_cost` - Cost estimation
- `get_healthy_providers` - List healthy providers
- `set_routing_strategy` - Configure routing strategy
- `run_provider_health_checks` - Force health checks
- `stream_chat` - Streaming chat completion
- `cancel_stream` - Cancel active stream
- `get_active_streams` - List active streams

### Group 3: Document Ingestion Commands (lines 1949-2140, ~192 lines)
Document extraction and ingestion pipeline.

**Commands:**
- `ingest_document` - Single-phase document ingestion
- `ingest_document_two_phase` - Two-phase ingestion with per-document indexes

### Group 4: Search Commands (lines 2143-2420, ~278 lines)
Meilisearch and hybrid search functionality.

**Commands:**
- `search` - Basic search
- `hybrid_search` - BM25 + vector hybrid search
- `get_search_suggestions` - Search suggestions
- `get_search_hints` - Search hints
- `expand_query` - Query expansion with synonyms
- `correct_query` - Spell correction

### Group 5: Voice Configuration Commands (lines 2423-2530, ~108 lines)
Voice provider setup and configuration.

**Commands:**
- `configure_voice` - Configure voice settings
- `get_voice_config` - Get current voice config
- `detect_voice_providers` - Detect available providers
- `check_voice_provider_installations` - Check installations
- `check_voice_provider_status` - Provider status check
- `install_voice_provider` - Install voice provider
- `list_downloadable_piper_voices` - List Piper voices
- `get_popular_piper_voices` - Get popular voices
- `download_piper_voice` - Download Piper voice

### Group 6: Voice Synthesis Commands (lines 2541-2600, ~60 lines)
TTS playback and voice listing.

**Commands:**
- `play_tts` - Play TTS audio
- `list_all_voices` - List all available voices

### Group 7: Meilisearch Commands (lines 2604-2655, ~52 lines)
Meilisearch health and maintenance.

**Commands:**
- `check_meilisearch_health` - Health check
- `reindex_library` - Reindex document library

### Group 8: Character Generation Commands (lines 2656-2675, ~20 lines)
Character generation for TTRPG.

**Commands:**
- `generate_character` - Generate a character

### Group 9: Campaign Commands (lines 2677-2900, ~224 lines)
Campaign CRUD and management.

**Commands:**
- `list_campaigns` - List all campaigns
- `create_campaign` - Create new campaign
- `get_campaign` - Get campaign by ID
- `update_campaign` - Update campaign
- `delete_campaign` - Delete campaign
- `get_campaign_theme` - Get theme settings
- `set_campaign_theme` - Set theme settings
- `get_theme_preset` - Get theme preset
- `create_snapshot` - Create campaign snapshot
- `list_snapshots` - List snapshots
- `restore_snapshot` - Restore from snapshot
- `export_campaign` - Export campaign
- `import_campaign` - Import campaign
- `get_campaign_stats` - Get campaign statistics
- `add_campaign_note` - Add note
- `get_campaign_notes` - Get notes
- `search_campaign_notes` - Search notes
- `generate_campaign_cover` - Generate cover image
- `delete_campaign_note` - Delete note

### Group 10: Session Commands (lines 2904-2965, ~62 lines)
Game session management.

**Commands:**
- `start_session` - Start new session
- `get_session` - Get session by ID
- `get_active_session` - Get active session
- `list_sessions` - List all sessions
- `end_session` - End session

### Group 11: Global Chat Session Commands (lines 2966-3125, ~160 lines)
Persistent LLM chat history management.

**Commands:**
- Multiple chat session CRUD commands
- Message history management
- Session switching

### Group 12: Combat Commands (lines 3127-3260, ~134 lines)
Combat tracker functionality.

**Commands:**
- Combat state management
- Initiative tracking
- Turn management
- Combatant CRUD

### Group 13: Advanced Condition Commands (lines 3261-3375, ~115 lines)
TTRPG condition system (TASK-015).

**Commands:**
- `apply_advanced_condition` - Apply condition to combatant
- `tick_conditions_end_of_turn` - Process end-of-turn effects
- Condition management and queries

### Group 14: Character Generation Enhanced (lines 3376-3398, ~23 lines)
Enhanced character generation (TASK-018).

**Commands:**
- Enhanced generation options
- System-specific generation

### Group 15: NPC Commands (lines 3400-3538, ~139 lines)
NPC CRUD and generation.

**Commands:**
- `generate_npc` - Generate NPC
- `list_npcs` - List NPCs
- NPC CRUD operations

### Group 16: NPC Extensions Commands (lines 3540-3666, ~127 lines)
Vocabulary, names, and dialects for NPCs.

**Commands:**
- Vocabulary bank operations
- Cultural naming rules
- Dialect transformations

### Group 17: Document Ingestion Extended (lines 3668-4386, ~719 lines)
Extended document ingestion features.

**Commands:**
- Document library management
- Index management
- Status tracking

### Group 18: Voice Synthesis Extended (lines 4387-4470, ~84 lines)
Extended voice synthesis features.

**Commands:**
- Voice queue management
- Voice preset operations

### Group 19: Audio Playback Commands (lines 4472-4630, ~159 lines)
Audio player controls.

**Commands:**
- Playback controls (play, pause, stop)
- Volume management
- Audio queue

### Group 20: Credential Commands (lines 4631-4838, ~208 lines)
API key and credential management.

**Commands:**
- Store/retrieve credentials
- Validate API keys
- Clear credentials

### Group 21: Utility Commands (lines 4839-4982, ~144 lines)
General utilities.

**Commands:**
- File dialogs
- Path operations
- System utilities

### Group 22: Voice Preset Commands (lines 4983-5125, ~143 lines)
Voice preset management.

**Commands:**
- Preset CRUD
- Preset application

### Group 23: Voice Commands Extended (lines 5126-5366, ~241 lines)
Extended voice functionality.

**Commands:**
- Voice profile management
- Voice synthesis options

### Group 24: NPC Conversation Commands (lines 5367-5559, ~193 lines)
NPC dialogue and conversation history.

**Commands:**
- Conversation CRUD
- Message history
- Conversation search

### Group 25: Theme Commands (lines 5560-5611, ~52 lines)
UI theme management.

**Commands:**
- Get/set theme
- Theme presets

### Group 26: Voice Queue Commands (lines 5612-5820, ~209 lines)
Voice synthesis queue management.

**Commands:**
- Queue operations
- Priority management
- Analytics (in-memory and DB-backed)

### Group 27: Campaign Versioning Commands (lines 5820-5922, ~103 lines)
Version control for campaigns (TASK-006).

**Commands:**
- Version CRUD
- Diff operations
- Rollback

### Group 28: World State Commands (lines 5923-5980, ~58 lines)
Campaign world state (TASK-007).

**Commands:**
- World state snapshots
- State queries

### Group 29: Voice Profile Commands (lines 5981-6127, ~147 lines)
Voice profile management.

**Commands:**
- Profile CRUD
- Profile assignment

### Group 30: Audio Cache Commands (lines 6128-6242, ~115 lines)
Audio caching system.

**Commands:**
- Cache statistics
- Cache cleanup
- Cache queries

### Group 31: Voice Synthesis Queue Commands (lines 6243-6566, ~324 lines)
Voice synthesis job queue.

**Commands:**
- Job submission
- Job status
- Priority management

### Group 32: Session Timeline Commands (lines 6567-6680, ~114 lines)
Session event timeline.

**Commands:**
- Timeline event CRUD
- Timeline queries

### Group 33: Advanced Condition Commands Extended (lines 6681-6779, ~99 lines)
Extended condition management.

**Commands:**
- Condition application
- Condition removal
- Tick processing

### Group 34: Session Notes Commands (lines 6780-7009, ~230 lines)
Session note management.

**Commands:**
- Note CRUD
- Note categorization
- Note search

### Group 35: Personality Application Commands (lines 7010-7359, ~350 lines)
Personality system for NPCs and narration.

**Commands:**
- Personality application
- Preview generation
- Style management

### Group 36: Location Generation Commands (lines 7360-7689, ~330 lines)
Location generation and management.

**Commands:**
- `generate_location` - Generate location
- Location CRUD
- Connection management
- Inhabitant tracking
- Secret/encounter management

### Group 37: Meilisearch Chat Provider Commands (lines 7690-7862, ~173 lines)
Chat provider integration with Meilisearch.

**Commands:**
- `list_chat_providers` - List providers
- Workspace configuration
- Chat settings

### Group 38: Model Selection Commands (lines 7863-7897, ~35 lines)
AI model selection.

**Commands:**
- `get_model_selection` - Get current selection
- `get_model_selection_for_prompt` - Get selection for prompt
- `set_model_override` - Override model

### Group 39: TTRPG Document Commands (lines 7898-8034, ~137 lines)
TTRPG-specific document management.

**Commands:**
- Document queries by source/type/system
- Document search
- Attribute queries

### Group 40: Extraction Settings Commands (lines 8035-8163, ~129 lines)
Document extraction configuration.

**Commands:**
- `get_extraction_settings` - Get settings
- `save_extraction_settings` - Save settings
- `get_supported_formats` - List formats
- `get_extraction_presets` - List presets
- `check_ocr_availability` - Check OCR

### Group 41: Claude Gate OAuth Commands (lines 8164-8397, ~234 lines)
Claude OAuth flow.

**Commands:**
- `claude_get_status` - Auth status
- `claude_start_oauth` - Start OAuth
- `claude_complete_oauth` - Complete OAuth
- `claude_logout` - Logout
- `claude_set_storage_backend` - Set storage
- `claude_list_models` - List models

### Group 42: Gemini Gate OAuth Commands (lines 8398-8590, ~193 lines)
Gemini OAuth flow.

**Commands:**
- `gemini_get_status` - Auth status
- `gemini_start_oauth` - Start OAuth
- `gemini_complete_oauth` - Complete OAuth
- `gemini_logout` - Logout
- `gemini_set_storage_backend` - Set storage

### Group 43: Copilot Gate OAuth Commands (lines 8591-8900, ~310 lines)
GitHub Copilot device code flow.

**Commands:**
- `start_copilot_auth` - Start device flow
- `poll_copilot_auth` - Poll for token
- `check_copilot_auth` - Check auth status
- `logout_copilot` - Logout
- `get_copilot_usage` - Get usage stats
- `get_copilot_models` - List models

### Group 44: Personality Extension Commands (lines 8901-9452, ~552 lines)
Personality system extensions (Phase 4).

**Subgroups:**
- **Template Operations** (lines 8905-9091): Template CRUD, filtering, preview
- **Blend Rules** (lines 9092-9244): Blend rule CRUD
- **Context Detection** (lines 9245-9327): Gameplay context detection
- **Contextual Personality** (lines 9328-9345): Context-based personality
- **Utility Functions** (lines 9346-9452): Context listing, config

**Commands:**
- Template CRUD and search
- Blend rule management
- Context detection and configuration

### Group 45: Utility Commands Extended (lines 9453-9470, ~18 lines)
Additional utilities.

**Commands:**
- `open_url_in_browser` - Open URL

### Group 46: Archetype Registry Commands (lines 9471-10679, ~1209 lines)
Unified archetype system.

**Subgroups:**
- **Core Archetype CRUD** (lines 9475-9827): Archetype management
- **Vocabulary Bank** (lines 9828-9862): Vocabulary operations
- **Vocabulary Bank CRUD** (lines 9863-10122): Bank management
- **Phrase Operations** (lines 10123-10375): Phrase filtering
- **Setting Pack** (lines 10376-10525): Setting pack management
- **Resolution** (lines 10526-10679): Archetype resolution

**Commands:**
- `create_archetype` - Create archetype
- `get_archetype` - Get archetype
- `list_archetypes` - List archetypes
- `update_archetype` - Update archetype
- `delete_archetype` - Delete archetype
- `resolve_archetype` - Resolve archetype
- Vocabulary bank CRUD
- Setting pack operations

---

## Code Duplication Patterns

### Pattern 1: OAuth Flow Boilerplate
All three OAuth clients (Claude, Gemini, Copilot) follow identical patterns:
- Storage backend enum with `File`, `Keyring`, `Auto` variants
- `new()`, `with_defaults()`, `switch_backend()` methods
- `is_authenticated()`, `get_token_info()`, `logout()` methods
- Command wrappers for each operation

**Lines affected**: ~906 lines (128-443, 444-750, 751-1033)

**Recommendation**: Extract to a generic `OAuthGateClient<T>` trait with blanket implementations.

### Pattern 2: Provider String Parsing
Multiple instances of provider string parsing with match statements:
```rust
match provider.as_str() {
    "claude" | "anthropic" => ...
    "gemini" | "google" => ...
    "openai" => ...
    "ollama" => ...
    _ => Err("Unknown provider")
}
```

**Locations**: ~8 instances across LLM and voice commands

**Recommendation**: Create `ProviderType` enum with `FromStr` impl.

### Pattern 3: State Access Pattern
Repetitive pattern for accessing app state:
```rust
pub async fn some_command(
    state: State<'_, AppState>,
) -> Result<T, String> {
    let manager = state.some_manager.read().await;
    manager.operation().map_err(|e| e.to_string())
}
```

**Recommendation**: Create helper macros or wrapper functions.

### Pattern 4: Error Conversion
Ubiquitous `.map_err(|e| e.to_string())` pattern.

**Locations**: 400+ instances

**Recommendation**: Create `IntoTauriError` trait and use `?` operator with custom error type.

### Pattern 5: CRUD Command Sets
Identical patterns for entity CRUD:
- `create_X`, `get_X`, `list_X`, `update_X`, `delete_X`
- Each with same state access and error handling patterns

**Affected entities**: Campaigns, Sessions, Notes, NPCs, Locations, Archetypes, Vocabulary Banks, etc.

**Recommendation**: Consider macro-based generation or trait-based abstraction.

### Pattern 6: List with Filtering
Multiple commands follow pattern:
```rust
pub async fn list_X_by_Y(
    filter: String,
    state: State<'_, AppState>,
) -> Result<Vec<X>, String> {
    let store = state.x_store.read().await;
    store.filter_by_y(&filter).map_err(|e| e.to_string())
}
```

**Recommendation**: Unified filtering API with query builder.

---

## Proposed Module Structure

```
src-tauri/src/commands/
├── mod.rs              # Re-exports and registration
├── state.rs            # AppState and initialization (~200 lines)
├── types.rs            # Shared request/response types (~100 lines)
├── error.rs            # Error types and conversions (~50 lines)
├── macros.rs           # Helper macros for state access (~30 lines)
│
├── llm/
│   ├── mod.rs          # Re-exports
│   ├── config.rs       # configure_llm, get_llm_config (~150 lines)
│   ├── chat.rs         # chat, stream_chat, cancel_stream (~400 lines)
│   ├── router.rs       # Router commands (~200 lines)
│   ├── models.rs       # Model listing commands (~150 lines)
│   └── health.rs       # Health check commands (~80 lines)
│
├── oauth/
│   ├── mod.rs          # Re-exports and shared traits
│   ├── common.rs       # Shared OAuth types and trait (~100 lines)
│   ├── claude.rs       # Claude Gate commands (~250 lines)
│   ├── gemini.rs       # Gemini Gate commands (~200 lines)
│   └── copilot.rs      # Copilot commands (~300 lines)
│
├── documents/
│   ├── mod.rs          # Re-exports
│   ├── ingestion.rs    # Document ingestion (~400 lines)
│   ├── search.rs       # Search commands (~300 lines)
│   ├── ttrpg.rs        # TTRPG document commands (~150 lines)
│   └── extraction.rs   # Extraction settings (~130 lines)
│
├── campaign/
│   ├── mod.rs          # Re-exports
│   ├── crud.rs         # Campaign CRUD (~150 lines)
│   ├── themes.rs       # Theme commands (~80 lines)
│   ├── snapshots.rs    # Snapshot commands (~100 lines)
│   ├── notes.rs        # Campaign notes (~150 lines)
│   ├── versioning.rs   # Version commands (~100 lines)
│   └── world_state.rs  # World state commands (~60 lines)
│
├── session/
│   ├── mod.rs          # Re-exports
│   ├── crud.rs         # Session CRUD (~100 lines)
│   ├── chat.rs         # Global chat sessions (~160 lines)
│   ├── combat.rs       # Combat commands (~250 lines)
│   ├── conditions.rs   # Condition commands (~200 lines)
│   ├── timeline.rs     # Timeline commands (~120 lines)
│   └── notes.rs        # Session notes (~230 lines)
│
├── npc/
│   ├── mod.rs          # Re-exports
│   ├── generation.rs   # NPC generation (~150 lines)
│   ├── crud.rs         # NPC CRUD (~100 lines)
│   ├── extensions.rs   # Vocabulary, names, dialects (~130 lines)
│   └── conversation.rs # NPC conversation (~200 lines)
│
├── character/
│   ├── mod.rs          # Re-exports
│   └── generation.rs   # Character generation (~50 lines)
│
├── location/
│   ├── mod.rs          # Re-exports
│   ├── generation.rs   # Location generation (~200 lines)
│   └── crud.rs         # Location CRUD (~130 lines)
│
├── voice/
│   ├── mod.rs          # Re-exports
│   ├── config.rs       # Voice configuration (~150 lines)
│   ├── providers.rs    # Provider management (~150 lines)
│   ├── synthesis.rs    # TTS synthesis (~200 lines)
│   ├── queue.rs        # Voice queue (~350 lines)
│   ├── presets.rs      # Voice presets (~150 lines)
│   ├── profiles.rs     # Voice profiles (~150 lines)
│   └── cache.rs        # Audio cache (~120 lines)
│
├── personality/
│   ├── mod.rs          # Re-exports
│   ├── application.rs  # Personality application (~350 lines)
│   ├── templates.rs    # Template commands (~200 lines)
│   ├── blending.rs     # Blend rules (~200 lines)
│   └── context.rs      # Context detection (~150 lines)
│
├── archetype/
│   ├── mod.rs          # Re-exports
│   ├── crud.rs         # Archetype CRUD (~500 lines)
│   ├── vocabulary.rs   # Vocabulary banks (~300 lines)
│   ├── setting_packs.rs # Setting pack commands (~200 lines)
│   └── resolution.rs   # Archetype resolution (~200 lines)
│
├── credentials/
│   ├── mod.rs          # Re-exports
│   └── management.rs   # Credential commands (~210 lines)
│
├── meilisearch/
│   ├── mod.rs          # Re-exports
│   ├── health.rs       # Health commands (~60 lines)
│   └── chat.rs         # Chat provider integration (~180 lines)
│
├── audio/
│   ├── mod.rs          # Re-exports
│   └── playback.rs     # Audio playback commands (~160 lines)
│
├── theme/
│   ├── mod.rs          # Re-exports
│   └── commands.rs     # Theme commands (~60 lines)
│
└── utility/
    ├── mod.rs          # Re-exports
    └── commands.rs     # Utility commands (~180 lines)
```

**Total modules**: ~50 files
**Average module size**: ~180 lines
**Maximum module size**: ~500 lines (archetype/crud.rs)

---

## Extraction Priority (by impact/complexity)

### Phase 1: High Impact, Low Risk (Foundational)

| Priority | Module | Lines | Rationale |
|----------|--------|-------|-----------|
| 1 | `commands/error.rs` | ~50 | Foundation for all error handling; unblocks `.map_err` cleanup |
| 2 | `commands/state.rs` | ~200 | AppState extraction; prerequisite for all modules |
| 3 | `commands/types.rs` | ~100 | Shared DTOs; reduces coupling |
| 4 | `commands/macros.rs` | ~30 | State access helpers; reduces boilerplate |

### Phase 2: High Duplication (Quick Wins)

| Priority | Module | Lines | Rationale |
|----------|--------|-------|-----------|
| 5 | `oauth/common.rs` | ~100 | Dedupe 3 OAuth clients (~600 lines saved) |
| 6 | `oauth/claude.rs` | ~250 | Isolated, well-defined scope |
| 7 | `oauth/gemini.rs` | ~200 | Isolated, well-defined scope |
| 8 | `oauth/copilot.rs` | ~300 | Isolated, well-defined scope |

### Phase 3: Large Self-Contained Groups

| Priority | Module | Lines | Rationale |
|----------|--------|-------|-----------|
| 9 | `archetype/*` | ~1200 | Largest group, recently added, well-isolated |
| 10 | `personality/*` | ~900 | Complex but self-contained |
| 11 | `voice/*` | ~1270 | Large but clearly scoped |

### Phase 4: Core Functionality

| Priority | Module | Lines | Rationale |
|----------|--------|-------|-----------|
| 12 | `llm/*` | ~980 | Core functionality, moderate coupling |
| 13 | `documents/*` | ~980 | Core functionality, moderate coupling |
| 14 | `session/*` | ~1060 | Game session core |

### Phase 5: Campaign and Entity Management

| Priority | Module | Lines | Rationale |
|----------|--------|-------|-----------|
| 15 | `campaign/*` | ~640 | Campaign management |
| 16 | `npc/*` | ~580 | NPC system |
| 17 | `location/*` | ~330 | Location system |

### Phase 6: Utilities and Remaining

| Priority | Module | Lines | Rationale |
|----------|--------|-------|-----------|
| 18 | `credentials/*` | ~210 | Isolated |
| 19 | `audio/*` | ~160 | Isolated |
| 20 | `theme/*` | ~60 | Small, isolated |
| 21 | `utility/*` | ~180 | Catch-all |
| 22 | `meilisearch/*` | ~240 | Integration layer |
| 23 | `character/*` | ~50 | Small, isolated |

---

## Migration Strategy

### Step 1: Create Infrastructure (Phases 1-2)
1. Create `commands/` directory structure
2. Extract error types and macros
3. Move AppState to `state.rs`
4. Update `main.rs` to use new module path
5. Verify compilation

### Step 2: Extract OAuth Modules (Phase 2)
1. Create shared OAuth trait in `oauth/common.rs`
2. Extract Claude, Gemini, Copilot to separate files
3. Update imports and registrations
4. Test OAuth flows

### Step 3: Extract Large Groups (Phases 3-4)
1. One module group at a time
2. Move commands and their helpers together
3. Keep request/response types with commands
4. Update `mod.rs` re-exports
5. Verify tests pass after each group

### Step 4: Extract Remaining (Phases 5-6)
1. Continue systematic extraction
2. Final cleanup and documentation

---

## Estimated Effort

| Phase | Modules | Estimated Time |
|-------|---------|----------------|
| 1 | 4 | 2-3 hours |
| 2 | 4 | 3-4 hours |
| 3 | 3 | 4-6 hours |
| 4 | 3 | 4-6 hours |
| 5 | 3 | 3-4 hours |
| 6 | 6 | 3-4 hours |

**Total**: 19-27 hours of focused refactoring work

---

## Risk Mitigation

1. **Compile after each module extraction** - catch issues early
2. **Run full test suite** - ensure no regressions
3. **Keep original file until phase complete** - easy rollback
4. **Extract one command group at a time** - incremental progress
5. **Document dependencies** - understand coupling before moving

---

## Notes

- Current file has good section organization with `// ===` dividers
- Many commands have inline request/response structs that should stay with commands
- Helper functions should move with their related commands
- Some commands reference shared state types that need to be in `state.rs`
- OAuth state types (lines 128-1033) should move to `oauth/` directory
