# Module Architecture Design

## Document Purpose

Design for refactoring `commands_legacy.rs` (8303 lines, ~310 commands) into domain-focused modules.

## Existing Patterns Analysis

### Successfully Extracted Modules

**1. `commands/voice/` (8 submodules)**
```
voice/
  mod.rs          - Re-exports with glob (pub use submodule::*)
  config.rs       - configure_voice, get_voice_config, detect_voice_providers
  providers.rs    - check_voice_provider_*, install_voice_provider, *_piper_*
  synthesis.rs    - play_tts, list_*_voices
  queue.rs        - queue_voice, get_voice_queue, cancel_voice
  presets.rs      - list_voice_presets, get_voice_preset
  profiles.rs     - CRUD for voice profiles
  cache.rs        - Audio cache management
  synthesis_queue.rs - Queue processing
```

Pattern: Functional decomposition within a single domain. Each file is focused (~30-100 lines) with clear responsibility.

**2. `commands/oauth/` (4 files)**
```
oauth/
  mod.rs          - Explicit re-exports of types AND commands
  common.rs       - Shared OAuth types (GateTokenInfo, GateOAuthFlowState)
  claude.rs       - ClaudeState, 6 commands
  gemini.rs       - GeminiState, 5 commands
  copilot.rs      - CopilotState, 6 commands
```

Pattern: Provider-based decomposition with shared types. Explicit re-exports for type safety.

**3. `commands/archetype/` (5 files)**
```
archetype/
  mod.rs          - Re-exports types and commands
  types.rs        - Request/Response types, helper functions
  crud.rs         - create/get/list/update/delete_archetype
  vocabulary.rs   - Vocabulary bank commands
  setting_packs.rs - Setting pack management
  resolution.rs   - resolve_archetype, caching
```

Pattern: Domain with shared types extracted. Helper functions colocated with types.

### Infrastructure Patterns

**`commands/macros.rs`**
```rust
#[macro_export]
macro_rules! read_state { ($state:expr, $field:ident) => { $state.$field.read().await }; }
macro_rules! write_state { ($state:expr, $field:ident) => { $state.$field.write().await }; }
macro_rules! with_db { ($db_state:expr, |$db:ident| $body:expr) => { ... }; }
```

**`commands/error.rs`**
```rust
#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Database error: {0}")] Database(#[from] sqlx::Error),
    #[error("LLM error: {0}")] Llm(String),
    #[error("Voice error: {0}")] Voice(String),
    #[error("Not found: {0}")] NotFound(String),
    // ...
}
pub type CommandResult<T> = Result<T, CommandError>;
```

---

## Proposed Module Architecture

### Overview

```
src-tauri/src/commands/
  mod.rs                    # Root module with re-exports
  error.rs                  # Unified CommandError (existing)
  macros.rs                 # State access macros (existing)

  # Existing extracted modules
  oauth/                    # OAuth providers (existing)
  voice/                    # Voice synthesis (existing)
  archetype/                # Archetype registry (existing)

  # New domain modules
  llm/                      # LLM configuration and chat
  campaign/                 # Campaign CRUD, snapshots, versioning
  session/                  # Session management, combat, conditions
  npc/                      # NPC CRUD, conversations, indexing
  personality/              # Personality profiles and application
  search/                   # Search, analytics, ingestion
  world/                    # World state, events, calendar, relationships
  generation/               # Character and location generation
  extraction/               # Document extraction settings
  utility/                  # Credentials, theming, misc
```

---

## Module Specifications

### 1. `commands/llm/`

**Purpose**: LLM provider configuration, chat operations, model selection, health checks.

**Submodules**:
```
llm/
  mod.rs                # Re-exports
  types.rs              # ChatRequestPayload, ChatResponsePayload, LLMSettings, HealthStatus
  config.rs             # configure_llm, get_router_stats, load/save config helpers
  chat.rs               # chat, streaming chat operations
  health.rs             # check_llm_health, provider health checks
  models.rs             # list_models_*, get_model_selection, task complexity
  meilisearch.rs        # Meilisearch chat provider commands
```

**Commands (estimated 25)**:
- `configure_llm`, `get_router_stats`
- `chat`, streaming variants
- `check_llm_health`
- `list_models_for_provider`, `get_model_info`, `get_model_selection`
- `get_meilisearch_chat_status`, configure embedders
- `select_model_for_task`, model selection commands

**AppState Dependencies**:
- `llm_config: RwLock<Option<LLMConfig>>`
- `llm_router: AsyncRwLock<LLMRouter>`
- `llm_manager: Arc<AsyncRwLock<LLMManager>>`
- `search_client: Arc<SearchClient>`
- `sidecar_manager: Arc<SidecarManager>`

**Types to Extract**:
- `ChatRequestPayload`, `ChatResponsePayload`
- `LLMSettings`, `HealthStatus`
- `EmbedderConfigRequest`, `SetupEmbeddingsResult`

---

### 2. `commands/campaign/`

**Purpose**: Campaign CRUD, notes, snapshots, versioning, statistics.

**Submodules**:
```
campaign/
  mod.rs                # Re-exports
  types.rs              # Campaign-specific request/response types
  crud.rs               # list/create/get/update/delete_campaign
  notes.rs              # add/get/search/delete_campaign_note
  snapshots.rs          # create/list/restore_snapshot
  versioning.rs         # create/list/get/compare/rollback_campaign_version
  stats.rs              # get_campaign_stats, get_campaign_theme
  export.rs             # export/import_campaign, generate_campaign_cover
```

**Commands (estimated 25)**:
- CRUD: `list_campaigns`, `create_campaign`, `get_campaign`, `update_campaign`, `delete_campaign`
- Notes: `add_campaign_note`, `get_campaign_notes`, `search_campaign_notes`, `delete_campaign_note`
- Snapshots: `create_snapshot`, `list_snapshots`, `restore_snapshot`
- Versioning: `create_campaign_version`, `list_campaign_versions`, `get_campaign_version`, `compare_campaign_versions`, `rollback_campaign`, `delete_campaign_version`, `add_version_tag`, `mark_version_milestone`
- Stats: `get_campaign_stats`, `get_campaign_theme`, `set_campaign_theme`, `get_theme_preset`
- Export: `export_campaign`, `import_campaign`, `generate_campaign_cover`

**AppState Dependencies**:
- `campaign_manager: CampaignManager`
- `session_manager: SessionManager`
- `database: Database`
- `version_manager: VersionManager`

---

### 3. `commands/session/`

**Purpose**: Game session lifecycle, combat tracker, conditions, timeline.

**Submodules**:
```
session/
  mod.rs                # Re-exports
  types.rs              # Combat/condition request types
  lifecycle.rs          # start/get/end_session, create_planned_session
  combat.rs             # start/end_combat, add/remove/update_combatant
  conditions.rs         # add/remove_condition, create_custom_condition
  timeline.rs           # add/update/delete_timeline_event, get_timeline
  notes.rs              # add/get/categorize session notes
  chat.rs               # Global chat session persistence
```

**Commands (estimated 45)**:
- Lifecycle: `start_session`, `get_session`, `get_active_session`, `list_sessions`, `end_session`, `create_planned_session`, `start_planned_session`, `reorder_session`
- Combat: `start_combat`, `end_combat`, `add_combatant`, `remove_combatant`, `update_combatant`, `set_combatant_hp`, `damage_combatant`, `heal_combatant`, `set_combatant_initiative`, `advance_turn`, `get_current_combatant`
- Conditions: `add_condition`, `remove_condition`, `list_conditions`, `list_condition_presets`, `create_custom_condition`, `update_condition_duration`
- Timeline: `add_timeline_event`, `update_timeline_event`, `delete_timeline_event`, `get_session_timeline`, `filter_timeline_events`
- Notes: `add_session_note_with_category`, `get_session_notes`, `categorize_note`, `link_entity_to_note`, `unlink_entity_from_note`
- Chat: `get_or_create_chat_session`, `get_active_chat_session`, `add_chat_message`, `list_chat_messages`, `archive_chat_session`

**AppState Dependencies**:
- `session_manager: SessionManager`
- `database: Database`
- `llm_config` (for note categorization)

---

### 4. `commands/npc/`

**Purpose**: NPC CRUD, conversations, extensions (vocabulary, names, dialects), indexing.

**Submodules**:
```
npc/
  mod.rs                # Re-exports
  types.rs              # NPC request/response types
  crud.rs               # generate/get/list/update/delete_npc, search_npcs
  conversations.rs      # NPC conversation commands
  vocabulary.rs         # load_vocabulary_bank, get_vocabulary_phrase (legacy NPC format)
  names.rs              # load_naming_rules, get_random_name_structure, validate_naming_rules
  dialects.rs           # load_dialect, apply_dialect
  indexing.rs           # initialize_npc_indexes, get_npc_indexes_stats, clear_npc_indexes
```

**Commands (estimated 25)**:
- CRUD: `generate_npc`, `get_npc`, `list_npcs`, `update_npc`, `delete_npc`, `search_npcs`
- Conversations: `start_npc_conversation`, `add_conversation_message`, `get_npc_conversation`, `list_npc_conversations`, `end_npc_conversation`
- Vocabulary: `load_vocabulary_bank`, `get_vocabulary_directory`, `get_vocabulary_phrase`
- Names: `load_naming_rules`, `get_names_directory`, `get_random_name_structure`, `validate_naming_rules`
- Dialects: `load_dialect`, `get_dialects_directory`, `apply_dialect`
- Indexing: `initialize_npc_indexes`, `get_npc_indexes_stats`, `clear_npc_indexes`

**AppState Dependencies**:
- `npc_store: NPCStore`
- `database: Database`
- `search_client: Arc<SearchClient>`

---

### 5. `commands/personality/`

**Purpose**: Personality profiles, application, styling, Phase 4 extensions (templates, blending, context).

**Submodules**:
```
personality/
  mod.rs                # Re-exports
  types.rs              # All personality request/response types
  profiles.rs           # list_personalities, preview_personality, test_personality
  application.rs        # set/get_active_personality, apply_personality_to_text
  context.rs            # get/set_personality_context, scene mood
  styling.rs            # style_npc_dialogue, build_npc_system_prompt, build_narration_prompt
  templates.rs          # TASK-PERS-014: list/filter/search/apply templates
  blending.rs           # TASK-PERS-015: blend rules CRUD
  detection.rs          # TASK-PERS-016: context detection commands
  contextual.rs         # TASK-PERS-017: contextual personality lookup
```

**Commands (estimated 50)**:
- Profiles: `list_personalities`, `preview_personality`, `preview_personality_extended`, `generate_personality_preview`, `test_personality`
- Application: `set_active_personality`, `get_active_personality`, `get_personality_prompt`, `apply_personality_to_text`, `set_personality_active`
- Context: `get_personality_context`, `get_session_personality_context`, `set_personality_context`, `set_narrator_personality`, `assign_npc_personality`, `unassign_npc_personality`, `set_scene_mood`, `set_personality_settings`, `clear_session_personality_context`
- Styling: `style_npc_dialogue`, `build_npc_system_prompt`, `build_narration_prompt`, `get_session_system_prompt`
- Templates (Phase 4): `list_personality_templates`, `filter_templates_by_game_system`, `filter_templates_by_setting`, `search_personality_templates`, `get_template_preview`, `apply_template_to_campaign`, `create_template`, `update_template`, `delete_template`, `create_template_from_personality`
- Blending (Phase 4): `list_blend_rules`, `get_blend_rule`, `create_blend_rule`, `update_blend_rule`, `delete_blend_rule`, `enable_blend_rule`, `disable_blend_rule`, `get_blender_cache_stats`, `clear_blender_cache`
- Detection (Phase 4): `detect_gameplay_context`, `get_context_signals`
- Contextual (Phase 4): `get_contextual_personality`, `get_blended_personality_for_context`

**AppState Dependencies**:
- `personality_store: Arc<PersonalityStore>`
- `personality_manager: Arc<PersonalityApplicationManager>`
- `template_store: Arc<SettingTemplateStore>`
- `blend_rule_store: Arc<BlendRuleStore>`
- `personality_blender: Arc<PersonalityBlender>`
- `contextual_personality_manager: Arc<ContextualPersonalityManager>`
- `llm_config` (for test_personality, apply transformations)

---

### 6. `commands/search/`

**Purpose**: Document search, hybrid search, analytics, Meilisearch management.

**Submodules**:
```
search/
  mod.rs                # Re-exports
  types.rs              # Search request/response types
  basic.rs              # search_documents, correct_query
  hybrid.rs             # hybrid_search, semantic_search, keyword_search
  meilisearch.rs        # check_meilisearch_health, reindex_library
  analytics.rs          # In-memory analytics commands
  analytics_db.rs       # Database-backed analytics commands
  ingestion.rs          # ingest_pdf, get_vector_store_status
```

**Commands (estimated 35)**:
- Basic: `search_documents`, `correct_query`
- Hybrid: `hybrid_search`, `semantic_search`, `keyword_search`, `search_with_filters`
- Meilisearch: `check_meilisearch_health`, `reindex_library`
- Analytics (in-memory): `get_search_analytics`, `get_popular_queries`, `get_cache_stats`, `get_trending_queries`, `get_zero_result_queries`, `get_click_distribution`, `record_search_selection`
- Analytics (DB): `get_search_analytics_db`, `get_popular_queries_db`, `get_cache_stats_db`, `get_trending_queries_db`, `get_zero_result_queries_db`, `get_click_distribution_db`, `record_search_event`, `record_search_selection_db`, `cleanup_search_analytics`
- Ingestion: `ingest_pdf`, `get_vector_store_status`, `configure_meilisearch_embedder`, `setup_ollama_embeddings`

**AppState Dependencies**:
- `search_client: Arc<SearchClient>`
- `ingestion_pipeline: Arc<MeilisearchPipeline>`
- `database: Database`
- `SearchAnalyticsState` (separate state)

---

### 7. `commands/world/`

**Purpose**: World state, events, calendar, locations, entity relationships.

**Submodules**:
```
world/
  mod.rs                # Re-exports
  types.rs              # World state request/response types
  state.rs              # get/update_world_state, custom fields
  events.rs             # add/list/delete_world_event
  calendar.rs           # set/get_in_game_date, advance_date, calendar config
  locations.rs          # set/get_location_state, list_locations, update_location_condition
  relationships.rs      # create/get/update/delete_entity_relationship, get_entity_graph
```

**Commands (estimated 40)**:
- State: `get_world_state`, `update_world_state`, `set_world_custom_field`, `get_world_custom_field`, `list_world_custom_fields`
- Events: `add_world_event`, `list_world_events`, `delete_world_event`, `update_world_event`
- Calendar: `set_in_game_date`, `get_in_game_date`, `advance_in_game_date`, `set_calendar_config`, `get_calendar_config`
- Locations: `set_location_state`, `get_location_state`, `list_locations`, `update_location_condition`
- Relationships: `create_entity_relationship`, `get_entity_relationship`, `update_entity_relationship`, `delete_entity_relationship`, `list_entity_relationships`, `get_relationship_graph`, `get_entity_relationships`, `find_relationship_path`, `get_faction_hierarchy`, `get_entities_by_type`

**AppState Dependencies**:
- `world_state_manager: WorldStateManager`
- `relationship_manager: RelationshipManager`
- `location_manager: LocationManager`

---

### 8. `commands/generation/`

**Purpose**: Character and location procedural generation.

**Submodules**:
```
generation/
  mod.rs                # Re-exports
  types.rs              # Generation options, results
  character.rs          # generate_character, generate_character_advanced
  location.rs           # generate_location, list_location_types
  backstory.rs          # Backstory generation (TASK-019)
  system_info.rs        # list_game_systems, get_system_info
```

**Commands (estimated 12)**:
- Character: `generate_character`, `generate_character_advanced`, `list_game_systems`, `get_system_info`
- Location: `generate_location`, `list_location_types`, `generate_location_details`
- Backstory: `generate_backstory`, `generate_npc_backstory`

**AppState Dependencies**:
- `llm_config` (for AI-enhanced generation)

---

### 9. `commands/extraction/`

**Purpose**: Document extraction settings and OCR configuration.

**Submodules**:
```
extraction/
  mod.rs                # Re-exports
  types.rs              # ExtractionPreset, OcrAvailability
  settings.rs           # get/save_extraction_settings, get_extraction_presets
  ocr.rs                # check_ocr_availability
  formats.rs            # get_supported_formats
  ttrpg.rs              # TTRPG document ingestion jobs
```

**Commands (estimated 10)**:
- Settings: `get_extraction_settings`, `save_extraction_settings`, `get_extraction_presets`
- OCR: `check_ocr_availability`
- Formats: `get_supported_formats`
- TTRPG: `ingest_ttrpg_document`, `get_ttrpg_ingestion_status`, `list_active_ttrpg_ingestion_jobs`

**AppState Dependencies**:
- `extraction_settings: AsyncRwLock<ExtractionSettings>`
- `database: Database`

---

### 10. `commands/utility/`

**Purpose**: Credentials, theming, audio, security audit, miscellaneous.

**Submodules**:
```
utility/
  mod.rs                # Re-exports
  credentials.rs        # store/get/delete_secret
  audio.rs              # play_audio, stop_audio, audio volumes
  security.rs           # Audit logging commands (TASK-024)
  usage.rs              # Usage tracking commands (TASK-022)
  misc.rs               # get_app_info, other utilities
```

**Commands (estimated 25)**:
- Credentials: `store_secret`, `get_secret`, `delete_secret`, `has_secret`
- Audio: `play_audio`, `stop_audio`, `get_audio_volumes`, `set_audio_volumes`
- Security: `get_audit_logs`, `query_audit_logs`, `export_audit_logs`, `get_security_summary`
- Usage: `get_usage_stats`, `get_usage_by_period`, `get_cost_breakdown`, `get_budget_status`, `set_budget_limit`, `get_provider_usage`, `reset_usage_session`
- Misc: `get_app_info`, `open_external_link`

**AppState Dependencies**:
- `credentials: CredentialManager`
- `UsageTrackerState` (separate state)
- `AuditLoggerState` (separate state)

---

## Cross-Cutting Concerns

### State Access Patterns

All modules should use the existing macros:

```rust
use crate::commands::macros::{read_state, write_state, with_db};

// Example usage
let config = read_state!(state, llm_config).clone();
write_state!(state, session_manager).update(...);
with_db!(state.database, |db| db.list_npcs(campaign_id))
```

### Error Handling

Extend `CommandError` as needed:

```rust
#[derive(Debug, Error)]
pub enum CommandError {
    // Existing variants...

    #[error("Campaign error: {0}")]
    Campaign(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Generation error: {0}")]
    Generation(String),

    #[error("World state error: {0}")]
    WorldState(String),
}
```

### Common Types

Consider a `commands/common_types.rs` for shared types:

```rust
// Pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationRequest {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

// ID wrapper for type safety
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityId(pub String);

// Standard timestamps
pub type Timestamp = chrono::DateTime<chrono::Utc>;
```

### Testability Improvements

1. **Pure functions where possible**: Extract business logic to core modules
2. **Dependency injection**: Commands receive state via `State<'_, T>`
3. **Mock-friendly traits**: Use traits for external services

```rust
// In types.rs
#[cfg(test)]
pub fn mock_app_state() -> AppState {
    // Create test fixtures
}

// In each command file
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_campaign() {
        let state = mock_app_state();
        let result = create_campaign("Test".into(), "dnd5e".into(), State::new(&state));
        assert!(result.is_ok());
    }
}
```

---

## Migration Strategy

### Phase 1: Infrastructure (Week 1)
1. Create module directory structure
2. Add types.rs to each module (extract from legacy)
3. Update `commands/mod.rs` with new re-exports

### Phase 2: Low-Dependency Modules (Week 2)
1. Extract `utility/` (credentials, audio, misc)
2. Extract `extraction/` (settings, OCR)
3. Extract `generation/` (character, location)

### Phase 3: Medium-Dependency Modules (Week 3)
1. Extract `campaign/` (depends on versioning)
2. Extract `session/` (depends on campaign)
3. Extract `world/` (relationships, events)

### Phase 4: High-Dependency Modules (Week 4)
1. Extract `llm/` (many dependencies)
2. Extract `npc/` (depends on search, database)
3. Extract `search/` (analytics integration)
4. Extract `personality/` (Phase 4 extensions)

### Phase 5: Cleanup (Week 5)
1. Remove `commands_legacy.rs`
2. Update all imports in `lib.rs`/`main.rs`
3. Verify all re-exports work
4. Run full test suite

---

## File Size Guidelines

| File Type | Target Lines | Max Lines |
|-----------|-------------|-----------|
| mod.rs | 20-50 | 100 |
| types.rs | 100-200 | 400 |
| Command files | 100-200 | 300 |
| Total per module | 300-600 | 1000 |

---

## Checklist for Each Module Extraction

- [ ] Create module directory
- [ ] Create `mod.rs` with submodule declarations
- [ ] Create `types.rs` with request/response types
- [ ] Extract commands to appropriate submodules
- [ ] Add helper functions from legacy file
- [ ] Update `commands/mod.rs` re-exports
- [ ] Verify commands still compile
- [ ] Add `#[cfg(test)]` module with basic tests
- [ ] Remove extracted code from `commands_legacy.rs`
- [ ] Update documentation comments
