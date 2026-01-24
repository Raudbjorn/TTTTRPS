# Design: Voice Module Command Extraction

## Overview

This document provides the technical design for extracting voice-related Tauri commands from `commands_legacy.rs` into the organized `commands/voice/` module structure. The extraction follows a domain-driven approach, grouping related functionality into cohesive submodules while maintaining the existing core business logic in `core/voice/`.

### Design Goals
- **Separation of Concerns**: Each submodule handles one aspect of voice functionality
- **Minimal Lock Scopes**: Release state locks before async operations
- **Security by Design**: API keys never exposed in logs, files, or responses
- **Backward Compatibility**: No changes to Tauri command signatures or behavior
- **Testability**: Each module independently testable with mocked dependencies

### Key Design Decisions

1. **Decision: Module File Structure**
   - **Context**: Need to organize ~56 commands into logical groups
   - **Decision**: Use 8 submodules matching functional areas (synthesis, config, providers, presets, profiles, cache, queue, synthesis_queue)
   - **Rationale**: Mirrors the existing `core/voice/` structure, reduces cognitive load, enables focused maintenance

2. **Decision: Command Registration Strategy**
   - **Context**: `#[tauri::command]` proc-macro doesn't propagate through `pub use` re-exports
   - **Decision**: Use full module paths in `main.rs` for command registration
   - **Rationale**: Required by Tauri's `generate_handler!` macro behavior

3. **Decision: Secret Handling Pattern**
   - **Context**: Need to store API keys securely without exposing them
   - **Decision**: Store via credential manager, mask in all external representations
   - **Rationale**: Defense in depth - even if config file is exposed, no secrets leak

4. **Decision: Queue Race Condition Fix**
   - **Context**: Current code has a TOCTOU race between item selection and status update
   - **Decision**: Use `AtomicBool` for `is_processing` flag, single write lock for selection+claim
   - **Rationale**: Prevents multiple concurrent queue processors, ensures atomic state transitions

---

## Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Frontend (Leptos WASM)                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐      │
│  │ Voice    │  │ TTS      │  │ Provider │  │ Profile  │  │ Queue    │      │
│  │ Settings │  │ Playback │  │ Install  │  │ Manager  │  │ Monitor  │      │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘      │
└───────┼─────────────┼─────────────┼─────────────┼─────────────┼────────────┘
        │ invoke()    │ invoke()    │ invoke()    │ invoke()    │ invoke()
        ▼             ▼             ▼             ▼             ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Tauri IPC Layer (commands/)                          │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                          voice/                                       │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │   │
│  │  │config.rs │ │synthesis │ │providers │ │presets.rs│ │profiles  │  │   │
│  │  │          │ │.rs       │ │.rs       │ │          │ │.rs       │  │   │
│  │  │4 commands│ │6 commands│ │6 commands│ │3 commands│ │6 commands│  │   │
│  │  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘  │   │
│  │       │            │            │            │            │         │   │
│  │  ┌──────────┐ ┌──────────┐                                         │   │
│  │  │cache.rs  │ │queue.rs  │ ┌───────────────────────┐               │   │
│  │  │          │ │          │ │synthesis_queue.rs     │               │   │
│  │  │6 commands│ │3 commands│ │22 commands            │               │   │
│  │  └────┬─────┘ └────┬─────┘ └───────────┬───────────┘               │   │
│  │       │            │                    │                           │   │
│  │  ┌────┴────────────┴────────────────────┴──────────────────────┐   │   │
│  │  │                         mod.rs                               │   │   │
│  │  │              (module declarations + re-exports)              │   │   │
│  │  └──────────────────────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Application State (AppState)                          │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐ │
│  │ voice_manager   │  │ credentials     │  │ database                    │ │
│  │ Arc<AsyncRwLock │  │ CredentialMgr   │  │ Database                    │ │
│  │ <VoiceManager>> │  │                 │  │                             │ │
│  └────────┬────────┘  └────────┬────────┘  └──────────────┬──────────────┘ │
└───────────┼────────────────────┼───────────────────────────┼────────────────┘
            │                    │                           │
            ▼                    ▼                           ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Core Business Logic (core/)                        │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                           voice/                                       │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐     │  │
│  │  │types.rs    │  │manager.rs  │  │providers/  │  │cache.rs    │     │  │
│  │  │VoiceConfig │  │VoiceManager│  │├ openai    │  │AudioCache  │     │  │
│  │  │Voice       │  │synthesize()│  │├ elevenlabs│  │CacheStats  │     │  │
│  │  │VoiceError  │  │list_voices │  │├ piper     │  │            │     │  │
│  │  │OutputFormat│  │play_audio  │  │├ coqui     │  │            │     │  │
│  │  └────────────┘  └────────────┘  │├ ...       │  └────────────┘     │  │
│  │                                   └────────────┘                      │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐     │  │
│  │  │profiles.rs │  │presets.rs  │  │queue.rs    │  │detection.rs│     │  │
│  │  │VoiceProfile│  │get_dm_     │  │SynthesisQue│  │detect_     │     │  │
│  │  │ProfileMeta │  │presets()   │  │SynthesisJob│  │providers() │     │  │
│  │  └────────────┘  └────────────┘  └────────────┘  └────────────┘     │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Component Architecture

The voice module is organized into 8 submodules, each with specific responsibilities:

```
commands/voice/
├── mod.rs              # Module declarations, re-exports, documentation
├── config.rs           # Provider configuration, settings management
├── synthesis.rs        # Core TTS operations, audio generation
├── providers.rs        # Provider installation, model downloads
├── presets.rs          # Built-in DM voice presets
├── profiles.rs         # User voice profiles, NPC linkage
├── cache.rs            # Audio cache management
├── queue.rs            # Basic voice queue (legacy)
└── synthesis_queue.rs  # Advanced priority queue system
```

### Technology Stack

| Layer | Technology | Rationale |
|-------|------------|-----------|
| IPC Commands | Tauri `#[tauri::command]` | Required for frontend-backend communication |
| State Management | `Arc<AsyncRwLock<T>>` | Thread-safe async state access |
| Audio Playback | `rodio` | Cross-platform audio output |
| Async Runtime | Tokio | Tauri's standard async runtime |
| Credential Storage | `keyring` crate | Secure OS-level secret storage |
| Serialization | `serde` | JSON serialization for IPC |

---

## Components and Interfaces

### Component 1: `config.rs` - Voice Configuration

- **Purpose**: Manage voice provider configuration and settings
- **Responsibilities**:
  - Store/retrieve voice configuration
  - Handle API key security (store in credential manager, mask in responses)
  - Detect available voice providers
  - Aggregate voice lists from all providers

- **Interfaces**:
  ```rust
  // Input Types
  VoiceConfig {
      provider: VoiceProviderType,
      cache_dir: Option<PathBuf>,
      default_voice_id: Option<String>,
      elevenlabs: Option<ElevenLabsConfig>,
      // ... other provider configs
  }

  // Output Types
  VoiceProviderDetection {
      providers: Vec<ProviderStatus>,
      detected_at: Option<String>,
  }

  // Dependencies
  - AppState.voice_manager: Arc<AsyncRwLock<VoiceManager>>
  - AppState.credentials: CredentialManager
  - tauri::AppHandle (for config file path)
  ```

- **Implementation Notes**:
  - `configure_voice`: Extract API key → store in credentials → mask in config → save to disk
  - `save_voice_config_disk`: MUST NOT write actual secrets - mask before saving
  - `get_voice_config`: Return config with masked API keys (`********`)

### Component 2: `synthesis.rs` - Voice Synthesis

- **Purpose**: Core TTS operations and audio playback
- **Responsibilities**:
  - Synthesize text to audio
  - Play audio through system output
  - List available voices from providers

- **Interfaces**:
  ```rust
  // Input Types
  SynthesisRequest {
      text: String,
      voice_id: String,
      settings: Option<VoiceSettings>,
      output_format: OutputFormat,
  }

  // Output Types
  SynthesisResult {
      audio_path: PathBuf,
      duration_ms: Option<u64>,
      format: OutputFormat,
      cached: bool,
  }

  Voice {
      id: String,
      name: String,
      provider: String,
      description: Option<String>,
      preview_url: Option<String>,
      labels: Vec<String>,
  }

  // Dependencies
  - AppState.voice_manager: Arc<AsyncRwLock<VoiceManager>>
  ```

- **Implementation Notes**:
  - `play_tts`: Lock scope pattern - acquire lock, create request, release lock, then async operations
  - File I/O must use `tokio::task::spawn_blocking`
  - Audio playback must use `tokio::task::spawn_blocking` with `rodio`

### Component 3: `providers.rs` - Provider Installation

- **Purpose**: Manage local voice provider installation and model downloads
- **Responsibilities**:
  - Check provider installation status
  - Install voice providers (Piper, Coqui)
  - Download voice models from Hugging Face

- **Interfaces**:
  ```rust
  // Input Types
  VoiceProviderType (enum)
  voice_key: String (for model download)
  quality: Option<String>

  // Output Types
  InstallStatus {
      provider: VoiceProviderType,
      installed: bool,
      version: Option<String>,
      path: Option<PathBuf>,
      error: Option<String>,
  }

  AvailablePiperVoice {
      key: String,
      name: String,
      language: String,
      quality: String,
      // ...
  }

  // Dependencies
  - ProviderInstaller from core/voice/install.rs
  - get_models_dir() helper function
  ```

- **Implementation Notes**:
  - `get_models_dir()`: Return `Result<PathBuf>`, NOT fallback to `.`
  - Model directory: `~/.local/share/ttrpg-assistant/voice/piper`
  - Download progress via callbacks (future: Tauri events)

### Component 4: `presets.rs` - Voice Presets

- **Purpose**: Provide built-in DM voice personas
- **Responsibilities**:
  - List all available presets
  - Filter presets by tag
  - Get specific preset by ID

- **Interfaces**:
  ```rust
  // Output Types
  VoiceProfile {
      id: String,
      name: String,
      provider: VoiceProviderType,
      voice_id: String,
      settings: VoiceSettings,
      metadata: ProfileMetadata,
  }

  ProfileMetadata {
      gender: Gender,
      age_range: AgeRange,
      personality_traits: Vec<String>,
      tags: Vec<String>,
      description: Option<String>,
  }

  // Dependencies
  - core::voice::presets::{get_dm_presets, get_presets_by_tag, get_preset_by_id}
  ```

- **Implementation Notes**:
  - These are synchronous commands (no async, no state)
  - Presets are hardcoded in `core/voice/presets.rs`

### Component 5: `profiles.rs` - Voice Profiles

- **Purpose**: Manage user-created voice profiles and NPC associations
- **Responsibilities**:
  - Create voice profiles
  - Link profiles to NPCs
  - Search and filter profiles

- **Interfaces**:
  ```rust
  // Input Types
  name: String
  provider: String
  voice_id: String
  metadata: Option<ProfileMetadata>
  profile_id: String
  npc_id: String
  query: String
  gender: String
  age_range: String

  // Output Types
  VoiceProfile (as defined above)
  Option<String> (profile_id linked to NPC)

  // Dependencies
  - AppState.database: Database
  - core::voice::profiles module
  ```

- **Implementation Notes**:
  - `create_voice_profile`: Parse provider string → create VoiceProfile → return ID
  - `link_voice_profile_to_npc`: Database operation - read NPC → update JSON → save
  - `search_voice_profiles`: Case-insensitive search across multiple fields

### Component 6: `cache.rs` - Audio Cache

- **Purpose**: Manage synthesized audio cache
- **Responsibilities**:
  - Report cache statistics
  - Clear cache (all or by tag)
  - Prune old entries
  - List cache entries

- **Interfaces**:
  ```rust
  // Input Types
  tag: String
  max_age_seconds: i64

  // Output Types
  VoiceCacheStats {
      hit_count: u64,
      miss_count: u64,
      hit_rate: f64,
      current_size_bytes: u64,
      max_size_bytes: u64,
      entry_count: usize,
      // ...
  }

  CacheEntry {
      key: String,
      path: PathBuf,
      size_bytes: u64,
      created_at: DateTime<Utc>,
      last_accessed: DateTime<Utc>,
      access_count: u64,
      tags: Vec<String>,
      format: OutputFormat,
      duration_ms: Option<u64>,
  }

  AudioCacheSizeInfo {
      current_size_bytes: u64,
      max_size_bytes: u64,
      entry_count: usize,
      usage_percent: f64,
  }

  // Dependencies
  - AppState.voice_manager: Arc<AsyncRwLock<VoiceManager>>
  ```

- **Implementation Notes**:
  - Cache operations go through VoiceManager for most functionality
  - `list_audio_cache_entries` may need direct AudioCache access

### Component 7: `queue.rs` - Basic Voice Queue

- **Purpose**: Simple sequential voice playback queue (legacy)
- **Responsibilities**:
  - Add text to queue
  - List queue items
  - Cancel queue items
  - Process queue in background

- **Interfaces**:
  ```rust
  // Input Types
  text: String
  voice_id: Option<String>
  queue_id: String

  // Output Types
  QueuedVoice {
      id: String,
      text: String,
      voice_id: String,
      status: VoiceStatus,
      created_at: String,
  }

  VoiceStatus {
      Pending,
      Processing,
      Playing,
      Completed,
      Failed(String),
  }

  // Dependencies
  - AppState.voice_manager: Arc<AsyncRwLock<VoiceManager>>
  ```

- **Implementation Notes**:
  - **Race Condition Fix**: Use `AtomicBool` for `is_processing` flag
  - Selection and claim must be atomic (single write lock scope)
  - Background processor: `tauri::async_runtime::spawn`

### Component 8: `synthesis_queue.rs` - Advanced Synthesis Queue

- **Purpose**: Priority-based synthesis queue with batch processing and events
- **Responsibilities**:
  - Submit/manage synthesis jobs
  - Priority-based processing
  - Progress tracking and events
  - Batch pre-generation
  - Pause/resume/cancel operations

- **Interfaces**:
  ```rust
  // Input Types
  SynthesisJobRequest {
      text: String,
      profile_id: String,
      voice_id: String,
      provider: String,
      priority: Option<String>,
      session_id: Option<String>,
      npc_id: Option<String>,
      campaign_id: Option<String>,
      tags: Option<Vec<String>>,
  }

  // Output Types
  SynthesisJob {
      id: String,
      text: String,
      profile_id: String,
      provider: VoiceProviderType,
      voice_id: String,
      priority: JobPriority,
      status: JobStatus,
      progress: JobProgress,
      // ... (many fields)
  }

  JobPriority { Immediate, High, Normal, Low, Batch }
  JobStatus { Pending, Processing, Completed, Failed(String), Cancelled }
  JobProgress { progress: f32, stage: String, eta_seconds: Option<u32>, ... }
  QueueStats { total_submitted, pending_count, processing_count, ... }

  // Dependencies
  - SynthesisQueueState.queue: SynthesisQueue
  - tauri::AppHandle (for event emission)
  ```

- **Implementation Notes**:
  - Helper functions: `parse_queue_provider(&str)`, `parse_queue_priority(Option<&str>)`
  - Events emitted via `app_handle.emit(event_name, payload)`
  - Uses separate state: `SynthesisQueueState`

---

## Data Models

### VoiceConfig
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub provider: VoiceProviderType,
    pub cache_dir: Option<PathBuf>,
    pub default_voice_id: Option<String>,
    // Cloud providers
    pub elevenlabs: Option<ElevenLabsConfig>,
    pub fish_audio: Option<FishAudioConfig>,
    pub openai: Option<OpenAIVoiceConfig>,
    pub piper: Option<PiperConfig>,
    // Self-hosted providers
    pub ollama: Option<OllamaConfig>,
    pub chatterbox: Option<ChatterboxConfig>,
    pub gpt_sovits: Option<GptSoVitsConfig>,
    pub xtts_v2: Option<XttsV2Config>,
    pub fish_speech: Option<FishSpeechConfig>,
    pub dia: Option<DiaConfig>,
    pub coqui: Option<CoquiConfig>,
}
```
- **Validation Rules**: Provider type must be valid enum variant
- **Relationships**: Each provider config is optional; only active provider's config is used
- **Storage**: Serialized to `~/.local/share/ttrpg-assistant/voice_config.json`

### VoiceProfile
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProfile {
    pub id: String,
    pub name: String,
    pub provider: VoiceProviderType,
    pub voice_id: String,
    pub settings: VoiceSettings,
    pub metadata: ProfileMetadata,
}
```
- **Validation Rules**: ID must be unique, name non-empty
- **Relationships**: Can be linked to NPCs via database
- **Storage**: Built-in presets in code; user profiles in database

### SynthesisJob
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisJob {
    pub id: String,                          // UUID
    pub text: String,                        // Text to synthesize
    pub profile_id: String,                  // Voice profile reference
    pub provider: VoiceProviderType,         // Target provider
    pub voice_id: String,                    // Provider-specific voice
    pub settings: VoiceSettings,             // Voice settings
    pub output_format: OutputFormat,         // Mp3/Wav/Ogg/Pcm
    pub priority: JobPriority,               // Queue priority
    pub status: JobStatus,                   // Current state
    pub progress: JobProgress,               // Progress info
    pub tags: Vec<String>,                   // Metadata tags
    pub campaign_id: Option<String>,         // Associated campaign
    pub session_id: Option<String>,          // Associated session
    pub npc_id: Option<String>,              // Associated NPC
    pub created_at: DateTime<Utc>,           // Creation timestamp
    pub started_at: Option<DateTime<Utc>>,   // Processing start
    pub completed_at: Option<DateTime<Utc>>, // Completion timestamp
    pub result_path: Option<String>,         // Output audio path
    pub error: Option<String>,               // Error message if failed
    pub retry_count: u32,                    // Current retry attempt
    pub max_retries: u32,                    // Max retries allowed
    pub char_count: usize,                   // Character count for billing
}
```
- **Validation Rules**: Text non-empty, priority valid enum
- **Relationships**: Linked to session/campaign/NPC via optional IDs
- **Storage**: In-memory with optional persistence

---

## API Design

All APIs are Tauri commands invoked via `window.__TAURI__.invoke()`.

### Configuration APIs

#### `configure_voice`
- **Input**: `VoiceConfig`
- **Output**: `Result<String, String>` (success message)
- **Side Effects**: Stores secrets in credential manager, saves config to disk
- **Errors**: Credential storage failure, file write failure

#### `get_voice_config`
- **Input**: None
- **Output**: `Result<VoiceConfig, String>` (with masked secrets)
- **Side Effects**: None
- **Errors**: State access failure

### Synthesis APIs

#### `play_tts`
- **Input**: `text: String, voice_id: String`
- **Output**: `Result<(), String>`
- **Side Effects**: Plays audio through system speakers
- **Errors**: Synthesis failure, playback failure

#### `list_available_voices`
- **Input**: None
- **Output**: `Result<Vec<Voice>, String>`
- **Side Effects**: May cache voice list
- **Errors**: Provider API failure

### Queue APIs

#### `submit_synthesis_job`
- **Input**: `SynthesisJobRequest` fields as separate parameters
- **Output**: `Result<SynthesisJob, String>`
- **Side Effects**: Adds job to queue, emits `synthesis:job-submitted` event
- **Errors**: Queue full, invalid provider/priority

#### `cancel_synthesis_job`
- **Input**: `job_id: String`
- **Output**: `Result<(), String>`
- **Side Effects**: Cancels job, emits `synthesis:job-cancelled` event
- **Errors**: Job not found, invalid state

---

## Error Handling

| Category | Error Type | User Action | Log Level |
|----------|------------|-------------|-----------|
| Validation | Invalid provider string | Fix input, retry | WARN |
| Validation | Invalid priority string | Fix input, retry | WARN |
| Credential | Secret storage failure | Check keyring access | ERROR |
| Provider | API rate limit | Wait and retry | WARN |
| Provider | API quota exceeded | Upgrade plan / wait | ERROR |
| Provider | Network timeout | Check connection, retry | ERROR |
| Provider | Invalid voice ID | Select different voice | ERROR |
| Queue | Queue full | Wait for jobs to complete | WARN |
| Queue | Job not found | Refresh job list | WARN |
| File | Read failure | Check file permissions | ERROR |
| File | Write failure | Check disk space/permissions | ERROR |
| Playback | Audio device unavailable | Check audio settings | ERROR |

### Error Response Format
```rust
// Commands return Result<T, String>
// Error strings follow pattern: "{Context}: {Specific error}"
// Examples:
"Voice synthesis failed: Provider rate limit exceeded"
"Cache operation failed: Permission denied writing to cache directory"
"Queue submission failed: Maximum queue size (100) reached"
```

---

## Testing Strategy

### Unit Testing
- **Scope**: Individual functions within each module
- **Mocking**:
  - `VoiceManager` operations via trait
  - Credential manager via mock implementation
  - Database operations via mock database
- **Coverage Target**: 80% line coverage per module

### Integration Testing
- **Scope**: Command → Core logic → State interaction
- **Approach**:
  - Use real `VoiceManager` with mock providers
  - Test state lock patterns don't deadlock
  - Verify event emission
- **Key Scenarios**:
  - Full synthesis flow: submit → process → complete
  - Queue operations: add → list → cancel
  - Configuration round-trip: set → get → verify masked

### Test Files Location
```
src-tauri/src/commands/voice/
├── config.rs          # Unit tests inline: #[cfg(test)] mod tests
├── synthesis.rs       # Unit tests inline
├── providers.rs       # Unit tests inline
├── presets.rs         # Unit tests inline
├── profiles.rs        # Unit tests inline
├── cache.rs           # Unit tests inline
├── queue.rs           # Unit tests inline
└── synthesis_queue.rs # Unit tests inline

src-tauri/tests/
└── voice_integration.rs  # Integration tests (optional)
```

---

## Security Considerations

### API Key Protection
1. **Storage**: System keyring via `keyring` crate
2. **In-memory**: Keep in VoiceConfig only during operation
3. **Serialization**: Always mask before serializing
4. **Logging**: Never log actual key values

### Implementation Pattern
```rust
// In configure_voice:
if !api_key.is_empty() && api_key != "********" {
    state.credentials.store_secret("elevenlabs_api_key", &api_key)?;
}

// Before saving to disk:
let mut config_for_disk = config.clone();
if let Some(ref mut elevenlabs) = config_for_disk.elevenlabs {
    elevenlabs.api_key = String::new(); // Mask for disk storage
}
save_voice_config_disk(&app_handle, &config_for_disk);

// In get_voice_config:
let mut config = manager.get_config().clone();
if let Some(ref mut elevenlabs) = config.elevenlabs {
    if !elevenlabs.api_key.is_empty() {
        elevenlabs.api_key = "********".to_string();
    }
}
```

### Debug Trait
- Do NOT derive `Debug` on types containing secrets
- If needed, implement custom `Debug` that masks secrets

---

## Migration Strategy

### Phase 1: Prepare Module Structure
1. Update `mod.rs` with all submodule declarations
2. Add proper documentation comments
3. Update re-exports for existing extracted commands

### Phase 2: Extract by Submodule
1. Move commands from `commands_legacy.rs` to target file
2. Add necessary imports
3. Test compilation
4. Verify command still works via frontend

### Phase 3: Fix Known Issues
1. Fix `get_models_dir()` fallback (return Result instead of `.`)
2. Fix secret persistence (mask before saving)
3. Fix queue race condition (atomic flag)

### Phase 4: Update Registration
1. Update `main.rs` command registration
2. Use full module paths
3. Remove old registrations

### Phase 5: Cleanup
1. Remove voice code from `commands_legacy.rs`
2. Remove unused imports
3. Run clippy, fix warnings
4. Run tests

---

## Metrics

| Metric | Before | After | Target |
|--------|--------|-------|--------|
| Voice lines in commands_legacy.rs | ~1,400 | 0 | 0 |
| Voice modules | 8 (partial) | 8 (complete) | 8 |
| Commands extracted | ~20 | ~56 | 56 |
| Compiler warnings (voice) | TBD | 0 | 0 |
| Test coverage | TBD | 80% | 80% |
