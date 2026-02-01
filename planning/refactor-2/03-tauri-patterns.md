# Tauri Patterns Analysis for Command Module Refactoring

**Date**: 2026-01-25
**Author**: Claude (Opus 4.5)
**Target**: `src-tauri/src/commands_legacy.rs` (8303 lines, 310 commands)

---

## 1. AppState Structure Analysis

### 1.1 Current AppState Definition

Located in `commands_legacy.rs` (lines 1012-1055), the `AppState` struct is the central state container managed by Tauri:

```rust
pub struct AppState {
    // LLM Configuration
    pub llm_client: RwLock<Option<LLMClient>>,
    pub llm_config: RwLock<Option<LLMConfig>>,
    pub llm_router: AsyncRwLock<LLMRouter>,
    pub llm_manager: Arc<AsyncRwLock<LLMManager>>,

    // Campaign & Session Management
    pub campaign_manager: CampaignManager,
    pub session_manager: SessionManager,
    pub npc_store: NPCStore,

    // Credentials & Voice
    pub credentials: CredentialManager,
    pub voice_manager: Arc<AsyncRwLock<VoiceManager>>,

    // Search & Ingestion
    pub sidecar_manager: Arc<SidecarManager>,
    pub search_client: Arc<SearchClient>,
    pub ingestion_pipeline: Arc<MeilisearchPipeline>,

    // Personality System
    pub personality_store: Arc<PersonalityStore>,
    pub personality_manager: Arc<PersonalityApplicationManager>,

    // Database
    pub database: Database,

    // Campaign Extensions (versioning, world state, relationships)
    pub version_manager: VersionManager,
    pub world_state_manager: WorldStateManager,
    pub relationship_manager: RelationshipManager,
    pub location_manager: LocationManager,

    // Document Extraction
    pub extraction_settings: AsyncRwLock<ExtractionSettings>,

    // OAuth Providers
    pub claude: Arc<ClaudeState>,
    pub gemini: Arc<GeminiState>,
    pub copilot: Arc<CopilotState>,

    // Archetype Registry (lazy-initialized after Meilisearch)
    pub archetype_registry: AsyncRwLock<Option<Arc<ArchetypeRegistry>>>,
    pub vocabulary_manager: AsyncRwLock<Option<Arc<VocabularyBankManager>>>,
    pub setting_pack_loader: Arc<SettingPackLoader>,

    // Phase 4: Personality Extensions
    pub template_store: Arc<SettingTemplateStore>,
    pub blend_rule_store: Arc<BlendRuleStore>,
    pub personality_blender: Arc<PersonalityBlender>,
    pub contextual_personality_manager: Arc<ContextualPersonalityManager>,
}
```

### 1.2 Domain State Types (Separate from AppState)

Additional state types managed independently via `app.manage()`:

| State Type | Location | Purpose |
|------------|----------|---------|
| `UsageTrackerState` | commands_legacy.rs:5662 | LLM usage tracking |
| `SearchAnalyticsState` | commands_legacy.rs:5669 | Search query analytics |
| `AuditLoggerState` | commands_legacy.rs:5675 | Security audit logging |
| `SynthesisQueueState` | commands/voice/synthesis_queue.rs:21 | Voice synthesis queue |
| `NativeFeaturesState` | main.rs (native_features module) | Drag-drop, dialogs |

### 1.3 State Field Usage by Domain

| Domain | Fields Used | Command Count |
|--------|-------------|---------------|
| **LLM** | `llm_client`, `llm_config`, `llm_router`, `llm_manager`, `search_client` | ~25 |
| **Campaign** | `campaign_manager`, `database` | ~15 |
| **Session** | `session_manager`, `database` | ~25 |
| **Combat** | `session_manager` | ~12 |
| **NPC** | `npc_store`, `personality_store`, `llm_router` | ~15 |
| **Voice** | `voice_manager` | ~25 (extracted) |
| **Search** | `search_client`, `sidecar_manager`, `ingestion_pipeline` | ~20 |
| **Personality** | `personality_store`, `personality_manager`, `contextual_personality_manager` | ~25 |
| **OAuth** | `claude`, `gemini`, `copilot` | ~17 (extracted) |
| **Archetype** | `archetype_registry`, `vocabulary_manager`, `setting_pack_loader` | ~25 (extracted) |
| **Versioning** | `version_manager`, `world_state_manager`, `relationship_manager` | ~30 |
| **Analytics** | `UsageTrackerState`, `SearchAnalyticsState`, `AuditLoggerState` | ~20 |

---

## 2. Command Patterns Analysis

### 2.1 Sync vs Async Commands

**Async Commands (majority)** - Return `Result<T, String>`:
```rust
#[tauri::command]
pub async fn chat(
    payload: ChatRequestPayload,
    state: State<'_, AppState>,
) -> Result<ChatResponsePayload, String> {
    // Async operations
}
```

**Sync Commands (rare)** - For static data or simple reads:
```rust
#[tauri::command]
pub fn get_llm_config(state: State<'_, AppState>) -> Result<Option<LLMSettings>, String> {
    let config = state.llm_config.read().unwrap();
    Ok(config.as_ref().map(|c| /* conversion */))
}

#[tauri::command]
pub fn list_openai_voices() -> Vec<Voice> {
    crate::core::voice::providers::openai::get_openai_voices()
}
```

**Pattern Guidance**:
- Use `async` when: database access, network calls, file I/O, lock contention expected
- Use sync when: static data, `RwLock::read()` with guaranteed-fast access

### 2.2 State Access Patterns

**Pattern A: Direct `State<'_, AppState>` (most common)**
```rust
#[tauri::command]
pub async fn create_archetype(
    request: CreateArchetypeRequest,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let registry = get_registry(&state).await?;
    // ...
}
```

**Pattern B: Separate Domain State**
```rust
#[tauri::command]
pub async fn get_usage_stats(
    state: State<'_, UsageTrackerState>,
) -> Result<UsageStats, String> {
    Ok(state.tracker.get_stats())
}
```

**Pattern C: Multiple States**
```rust
#[tauri::command]
pub async fn queue_voice(
    request: SynthesisRequest,
    app_state: State<'_, AppState>,
    queue_state: State<'_, SynthesisQueueState>,
) -> Result<String, String> {
    // Uses both states
}
```

**Pattern D: AppHandle for Persistence**
```rust
#[tauri::command]
pub async fn configure_llm(
    settings: LLMSettings,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // Uses app_handle for disk persistence
    save_llm_config_disk(&app_handle, &config);
}
```

### 2.3 Error Handling Patterns

**Pattern: Result<T, String> at IPC Boundary**
```rust
// Internal: typed errors
pub enum McpBridgeError {
    #[error("Failed to spawn subprocess: {0}")]
    SpawnError(#[from] std::io::Error),
}

// At command boundary: convert to String
#[tauri::command]
pub async fn send_mcp_message(...) -> Result<Response, String> {
    internal_send(...).await.map_err(|e| e.to_string())
}
```

**CommandError Type** (`commands/error.rs`):
```rust
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("LLM error: {0}")]
    Llm(String),
    // ... other variants
}

impl From<CommandError> for String {
    fn from(e: CommandError) -> String {
        e.to_string()
    }
}

pub type CommandResult<T> = Result<T, CommandError>;
```

**Current Issue**: CommandError exists but is underutilized. Most commands still use `.map_err(|e| e.to_string())`.

### 2.4 Return Type Patterns

**Simple Types**: Primitives, `String`, `Vec<T>`, `Option<T>`, `bool`
```rust
#[tauri::command]
pub async fn archetype_exists(id: String, state: State<'_, AppState>) -> Result<bool, String>
```

**Custom Response Types**: For complex data
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]  // TypeScript compatibility
pub struct ChatResponsePayload {
    pub content: String,
    pub model: String,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}
```

**Request Types**: Input validation at deserialization
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateArchetypeRequest {
    pub id: String,
    pub display_name: String,
    // ...
}
```

---

## 3. Testability Assessment

### 3.1 Current Limitations

1. **Tight Coupling to AppState**: Commands directly access `State<'_, AppState>`, making unit testing require full AppState construction.

2. **No Trait Abstractions**: Services like `SearchClient`, `VoiceManager` are concrete types, not trait objects.

3. **Database Coupling**: Many commands directly call `state.database.method()` without abstraction layer.

4. **Tauri State Requirement**: Commands require `tauri::State` which needs Tauri runtime context.

### 3.2 Existing Mock Infrastructure

Located in `src-tauri/src/tests/mocks/mod.rs`:

```rust
// Mockall-based trait mocks
#[automock]
#[async_trait]
pub trait LlmClient: Send + Sync {
    fn id(&self) -> String;
    async fn health_check(&self) -> bool;
    async fn chat(&self, messages: Vec<MockChatMessage>, max_tokens: Option<u32>) -> LlmResult<MockChatResponse>;
}

#[automock]
#[async_trait]
pub trait VoiceProvider: Send + Sync { /* ... */ }

#[automock]
#[async_trait]
pub trait SearchClient: Send + Sync { /* ... */ }
```

### 3.3 Recommended Improvements

**A. Extract Business Logic from Commands**

```rust
// Current (untestable):
#[tauri::command]
pub async fn create_campaign(name: String, state: State<'_, AppState>) -> Result<Campaign, String> {
    state.campaign_manager.create(&name).map_err(|e| e.to_string())
}

// Proposed (testable):
pub struct CampaignService {
    manager: Arc<CampaignManager>,
    db: Arc<Database>,
}

impl CampaignService {
    pub async fn create(&self, name: &str) -> Result<Campaign, CampaignError> {
        // Business logic here - fully testable
    }
}

#[tauri::command]
pub async fn create_campaign(name: String, state: State<'_, AppState>) -> Result<Campaign, String> {
    // Thin wrapper - only converts types
    state.campaign_service.create(&name).await.map_err(|e| e.to_string())
}
```

**B. Trait-Based Dependencies**

```rust
// Define trait in core module
#[async_trait]
pub trait CampaignRepository: Send + Sync {
    async fn save(&self, campaign: &Campaign) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Campaign>, RepositoryError>;
}

// Production implementation
pub struct SqliteCampaignRepository { db: Database }

// Test implementation
pub struct MockCampaignRepository { campaigns: HashMap<String, Campaign> }
```

**C. Unit Test Patterns**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::mocks::*;

    #[tokio::test]
    async fn test_create_campaign_validates_name() {
        let mock_repo = MockCampaignRepository::new();
        let service = CampaignService::new(Arc::new(mock_repo));

        let result = service.create("").await;
        assert!(matches!(result, Err(CampaignError::InvalidName(_))));
    }
}
```

---

## 4. Re-export and Registration Concerns

### 4.1 Command Registration in main.rs

Commands are registered via `tauri::generate_handler!` macro:

```rust
.invoke_handler(tauri::generate_handler![
    // LLM Commands
    commands::configure_llm,
    commands::chat,
    commands::stream_chat,
    // ...

    // OAuth Commands (extracted module)
    commands::oauth::claude::claude_get_status,
    commands::oauth::claude::claude_start_oauth,
    // ...

    // Archetype Commands (extracted module)
    commands::archetype::crud::create_archetype,
    commands::archetype::crud::get_archetype,
    // ...
])
```

### 4.2 Module Re-export Pattern

In `commands/mod.rs`:

```rust
// Temporary: Re-export everything from legacy until extraction complete
#[path = "../commands_legacy.rs"]
mod commands_legacy;
pub use commands_legacy::*;

// Re-export extracted modules
pub use oauth::{
    ClaudeState, GeminiState, CopilotState,
    claude_get_status, claude_start_oauth, /* ... */
};

pub use archetype::{
    create_archetype, get_archetype, list_archetypes, /* ... */
};

// Voice uses glob re-export
pub use voice::*;
```

### 4.3 Backward Compatibility Requirements

1. **Frontend TypeScript bindings expect exact command names**
   - Command names must match: `configure_llm` not `llm::configure`
   - Extracted commands retain same names

2. **State types must be available at app.manage() call site**
   - `AppState`, `UsageTrackerState`, etc. must be publicly re-exported

3. **Response types must serialize identically**
   - `#[serde(rename_all = "camelCase")]` must be preserved
   - Field names and types unchanged

### 4.4 Registration Pattern for New Modules

**Module Structure**:
```
commands/
  mod.rs           # Re-exports and glob imports
  error.rs         # CommandError type
  macros.rs        # Helper macros
  campaign/
    mod.rs         # pub use submodules::*
    types.rs       # Request/response types
    crud.rs        # create_campaign, get_campaign, etc.
    notes.rs       # add_campaign_note, etc.
    snapshots.rs   # create_snapshot, etc.
```

**In mod.rs**:
```rust
pub mod campaign;
pub use campaign::*;  // Glob re-export for Tauri macros
```

**In main.rs** (full path for clarity):
```rust
.invoke_handler(tauri::generate_handler![
    commands::campaign::crud::create_campaign,
    commands::campaign::crud::get_campaign,
    // ...
])
```

---

## 5. Type Extraction Needs

### 5.1 Types Currently in commands_legacy.rs

| Type | Lines | Purpose | Destination |
|------|-------|---------|-------------|
| `ChatRequestPayload` | 1208-1216 | Chat input | `commands/llm/types.rs` |
| `ChatResponsePayload` | 1218-1224 | Chat output | `commands/llm/types.rs` |
| `LLMSettings` | 1226-1233 | Config input | `commands/llm/types.rs` |
| `HealthStatus` | 1235-1240 | Health check | `commands/llm/types.rs` |
| `UsageTrackerState` | 5662-5664 | Analytics state | `commands/analytics/state.rs` |
| `SearchAnalyticsState` | 5668-5671 | Analytics state | `commands/analytics/state.rs` |
| `AuditLoggerState` | 5675-5691 | Analytics state | `commands/analytics/state.rs` |

### 5.2 Shared Types Across Modules

These types are used by multiple command domains:

| Type | Used By | Current Location |
|------|---------|------------------|
| `Campaign` | campaign, session, versioning | `core/models.rs` |
| `GameSession` | session, combat, timeline | `core/session_manager.rs` |
| `NPC` | npc, personality, conversation | `core/npc_gen.rs` |
| `PersonalityProfile` | personality, npc, chat | `core/personality/mod.rs` |

**Recommendation**: Keep domain types in `core/` modules; only IPC request/response types go in `commands/*/types.rs`.

### 5.3 Serialization Concerns

**Required Derives**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]  // TypeScript interop
pub struct MyResponse {
    pub my_field: String,  // Serializes as "myField"
}
```

**Optional Field Handling**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MyRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_field: Option<String>,

    #[serde(default)]
    pub with_default: Vec<String>,  // Defaults to empty vec if missing
}
```

---

## 6. Extraction Priority Recommendations

### Phase 1: High-Value, Low-Risk
| Module | Commands | Complexity | Dependencies |
|--------|----------|------------|--------------|
| `campaign/` | 15 | Low | `campaign_manager`, `database` |
| `session/` | 25 | Medium | `session_manager`, `database` |
| `analytics/` | 20 | Low | Separate state types |

### Phase 2: Medium Complexity
| Module | Commands | Complexity | Dependencies |
|--------|----------|------------|--------------|
| `llm/` | 25 | Medium | Multiple state fields |
| `npc/` | 15 | Medium | `npc_store`, `personality_store` |
| `search/` | 20 | Medium | Meilisearch integration |

### Phase 3: Complex / Already Extracted
| Module | Commands | Status |
|--------|----------|--------|
| `voice/` | 25 | Extracted |
| `oauth/` | 17 | Extracted |
| `archetype/` | 25 | Extracted |

---

## 7. Migration Checklist

For each extracted module:

- [ ] Create `commands/{domain}/mod.rs` with submodule declarations
- [ ] Create `commands/{domain}/types.rs` with request/response types
- [ ] Move commands preserving exact function signatures
- [ ] Add `pub use {domain}::*` in `commands/mod.rs`
- [ ] Update `main.rs` invoke_handler with full paths
- [ ] Remove from `commands_legacy.rs`
- [ ] Run `cargo test` to verify compilation
- [ ] Test frontend IPC calls still work
- [ ] Add unit tests for business logic (where extracted)

---

## 8. Code Examples from Extracted Modules

### Example: Archetype Module Structure

```
commands/archetype/
  mod.rs          # Re-exports
  types.rs        # CreateArchetypeRequest, ArchetypeResponse, etc.
  crud.rs         # create_archetype, get_archetype, list_archetypes, etc.
  vocabulary.rs   # create_vocabulary_bank, get_vocabulary_bank, etc.
  setting_packs.rs # load_setting_pack, list_setting_packs, etc.
  resolution.rs   # resolve_archetype, resolve_for_npc, etc.
```

### Example: Voice Module Structure

```
commands/voice/
  mod.rs            # pub use submodules::*
  config.rs         # configure_voice, get_voice_config
  providers.rs      # detect_voice_providers, list_available_voices
  synthesis.rs      # play_tts, list_openai_voices, list_elevenlabs_voices
  queue.rs          # queue_voice, get_voice_queue, cancel_voice
  presets.rs        # voice presets management
  profiles.rs       # voice profiles management
  cache.rs          # get_audio_cache_stats, clear_audio_cache
  synthesis_queue.rs # SynthesisQueueState + queue commands
```

---

## Summary

The refactoring from `commands_legacy.rs` to domain modules follows a proven pattern already demonstrated by the extracted `oauth/`, `archetype/`, and `voice/` modules. Key principles:

1. **Preserve IPC contract**: Command names, request/response types unchanged
2. **Use glob re-exports**: Enable Tauri macro compatibility
3. **Extract types to types.rs**: One file per domain for request/response types
4. **State access unchanged**: Continue using `State<'_, AppState>` pattern
5. **Incremental migration**: Move commands one domain at a time
6. **Test after each extraction**: Verify both Rust compilation and frontend IPC
