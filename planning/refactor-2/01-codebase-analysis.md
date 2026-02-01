# Codebase Analysis

## Overview

This document analyzes `src-tauri/src/commands_legacy.rs` (8303 lines, ~310 Tauri commands) to inform the modularization refactoring.

---

## 1. File Structure Summary

| Section | Lines | Description |
|---------|-------|-------------|
| Imports | 1-100 | Module imports and utility fn |
| OAuth State Types | 128-1033 | Claude, Gemini, Copilot gate states |
| AppState | 1034-1210 | Central state struct (30+ fields) |
| DB Helper Macro | 1211-1227 | Database access macro |
| Request/Response Types | 1229-1267 | Shared DTOs |
| Commands | 1268-8303 | All Tauri commands |

---

## 2. Already Extracted Modules

### 2.1 `commands/voice/` (8 submodules, ~400 lines)
- `config.rs` - configure_voice, get_voice_config, detect_voice_providers
- `providers.rs` - Voice provider management
- `synthesis.rs` - play_tts, list_*_voices
- `queue.rs` - Voice queue management
- `presets.rs` - Voice presets
- `profiles.rs` - Voice profiles CRUD
- `cache.rs` - Audio cache
- `synthesis_queue.rs` - Queue processing

### 2.2 `commands/oauth/` (4 files, ~300 lines)
- `common.rs` - Shared OAuth types
- `claude.rs` - Claude OAuth flow (6 commands)
- `gemini.rs` - Gemini OAuth flow (5 commands)
- `copilot.rs` - Copilot OAuth flow (6 commands)

### 2.3 `commands/archetype/` (5 files, ~350 lines)
- `types.rs` - Request/Response types
- `crud.rs` - Archetype CRUD
- `vocabulary.rs` - Vocabulary banks
- `setting_packs.rs` - Setting packs
- `resolution.rs` - Archetype resolution

---

## 3. Command Inventory (310 commands)

| Domain | Count | Lines Est. |
|--------|-------|-----------|
| LLM/Intelligence | ~25 | ~700 |
| Campaign | ~25 | ~500 |
| Session | ~35 | ~600 |
| Combat | ~20 | ~400 |
| NPC | ~35 | ~800 |
| Personality | ~35 | ~600 |
| Search/Library | ~40 | ~1000 |
| World State | ~15 | ~300 |
| Relationships | ~10 | ~200 |
| Timeline | ~6 | ~150 |
| Generation | ~10 | ~300 |
| Usage/Credentials | ~10 | ~200 |
| Audit | ~7 | ~200 |
| System | ~10 | ~150 |

---

## 4. AppState Fields (30+)

### LLM Configuration
- `llm_client: RwLock<Option<LLMClient>>`
- `llm_config: RwLock<Option<LLMConfig>>`
- `llm_router: AsyncRwLock<LLMRouter>`
- `llm_manager: Arc<AsyncRwLock<LLMManager>>`

### Campaign & Session
- `campaign_manager: CampaignManager`
- `session_manager: SessionManager`
- `npc_store: NPCStore`

### Credentials & Voice
- `credentials: CredentialManager`
- `voice_manager: Arc<AsyncRwLock<VoiceManager>>`

### Search & Ingestion
- `sidecar_manager: Arc<SidecarManager>`
- `search_client: Arc<SearchClient>`
- `ingestion_pipeline: Arc<MeilisearchPipeline>`

### Personality System
- `personality_store: Arc<PersonalityStore>`
- `personality_manager: Arc<PersonalityApplicationManager>`
- `template_store: Arc<SettingTemplateStore>`
- `blend_rule_store: Arc<BlendRuleStore>`
- `personality_blender: Arc<PersonalityBlender>`
- `contextual_personality_manager: Arc<ContextualPersonalityManager>`

### Database
- `database: Database`

### Campaign Extensions
- `version_manager: VersionManager`
- `world_state_manager: WorldStateManager`
- `relationship_manager: RelationshipManager`
- `location_manager: LocationManager`

### Document Extraction
- `extraction_settings: AsyncRwLock<ExtractionSettings>`

### OAuth Providers
- `claude: Arc<ClaudeState>`
- `gemini: Arc<GeminiState>`
- `copilot: Arc<CopilotState>`

### Archetype Registry
- `archetype_registry: AsyncRwLock<Option<Arc<ArchetypeRegistry>>>`
- `vocabulary_manager: AsyncRwLock<Option<Arc<VocabularyBankManager>>>`
- `setting_pack_loader: Arc<SettingPackLoader>`

---

## 5. Additional State Types

Managed separately via `app.manage()`:

| State Type | Purpose |
|------------|---------|
| `UsageTrackerState` | LLM usage tracking |
| `SearchAnalyticsState` | Search query analytics |
| `AuditLoggerState` | Security audit logging |
| `SynthesisQueueState` | Voice synthesis queue |
| `NativeFeaturesState` | Drag-drop, dialogs |

---

## 6. Common Patterns

### Async vs Sync Commands
- **Async**: Database ops, LLM calls, file I/O, Meilisearch
- **Sync**: In-memory ops, simple transformations

### Error Handling
- Most commands use `.map_err(|e| e.to_string())`
- `CommandError` exists in `commands/error.rs` but underutilized

### State Access
- Read: `state.field.read().await` for RwLock
- Write: `state.field.write().await` for RwLock
- Direct: `state.field.method()` for non-locked fields

### Manager Delegation
Most commands delegate to stateful managers:
```rust
state.campaign_manager.get_campaign(&id)
state.session_manager.start_combat(&session_id)
```

---

## 7. Infrastructure

### Existing Support Files
- `commands/mod.rs` - Re-exports all modules
- `commands/error.rs` - CommandError enum
- `commands/macros.rs` - State access macros

### Macros Available
```rust
read_state!($state, $field)  // $state.$field.read().await
write_state!($state, $field) // $state.$field.write().await
with_db!($db_state, |$db| $body)  // Database access
```

---

## 8. Key Observations

1. **Monolithic Growth**: File grew organically, commands scattered by feature delivery order
2. **State Coupling**: Most commands access 1-3 AppState fields
3. **Type Proliferation**: Many inline request/response types
4. **Error Inconsistency**: Mix of `.map_err()` and `?` operator
5. **Test Coverage**: Limited unit tests for command logic
6. **Documentation**: Sparse doc comments on commands
