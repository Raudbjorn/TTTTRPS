# Requirements: Voice Module Command Extraction

## Introduction

The TTRPG Assistant (Sidecar DM) application provides comprehensive voice synthesis capabilities for Game Masters, enabling them to give unique voices to NPCs, generate ambient audio, and create immersive audio experiences during tabletop RPG sessions. Currently, approximately 60+ voice-related Tauri commands are scattered across `commands_legacy.rs` (~1,400 lines of voice code), while a partial skeleton structure exists in `src-tauri/src/commands/voice/`.

This extraction effort aims to complete the migration of all voice-related commands from the monolithic `commands_legacy.rs` into the organized domain module structure at `commands/voice/`. The extraction will improve code maintainability, reduce cognitive load when navigating the codebase, enable independent testing of voice functionality, and establish clear boundaries between voice subsystems.

The voice system comprises eight major functional areas:
1. **Synthesis** - Core TTS operations and audio generation
2. **Configuration** - Provider setup and voice settings management
3. **Providers** - Multi-provider support (ElevenLabs, OpenAI, Piper, Coqui, etc.)
4. **Presets** - Built-in DM voice personas and templates
5. **Profiles** - User-created voice profiles linked to NPCs
6. **Cache** - Audio cache management for performance
7. **Queue** - Basic voice queue for sequential playback (legacy)
8. **Synthesis Queue** - Advanced priority queue with batch pre-generation

## Requirements

### Requirement 1: Voice Synthesis Command Extraction

**User Story:** As a developer, I want all voice synthesis commands extracted to `commands/voice/synthesis.rs`, so that TTS functionality is isolated and independently maintainable.

#### Acceptance Criteria
1. WHEN a developer opens `commands/voice/synthesis.rs` THEN the file SHALL contain all voice synthesis commands:
   - `play_tts` - Synthesize and play audio immediately
   - `list_openai_voices` - List available OpenAI TTS voices
   - `list_openai_tts_models` - List OpenAI TTS models (tts-1, tts-1-hd)
   - `list_elevenlabs_voices` - Fetch ElevenLabs voice catalog
   - `list_available_voices` - List voices from all configured providers
   - `transcribe_audio` - Speech-to-text transcription (if present)

2. WHEN `play_tts` is invoked THEN system SHALL:
   - Use `tokio::task::spawn_blocking` for file I/O operations
   - Release state locks before performing async operations
   - Use `rodio` for audio playback in a blocking task
   - Return `Ok(())` on successful playback or `Err(String)` on failure

3. IF audio data exceeds 10MB THEN system SHALL stream playback rather than loading entirely into memory.

4. WHEN synthesis fails due to provider error THEN system SHALL return a descriptive error message including:
   - Provider name
   - Error type (rate limit, quota, network, invalid voice)
   - Suggested recovery action

---

### Requirement 2: Voice Configuration Command Extraction

**User Story:** As a developer, I want all voice configuration commands extracted to `commands/voice/config.rs`, so that provider setup and settings management is centralized.

#### Acceptance Criteria
1. WHEN a developer opens `commands/voice/config.rs` THEN the file SHALL contain:
   - `configure_voice` - Update voice provider configuration
   - `get_voice_config` - Retrieve current voice configuration
   - `detect_voice_providers` - Detect available local TTS providers
   - `list_all_voices` - Aggregate voice list from all providers

2. WHEN `configure_voice` stores an API key THEN system SHALL:
   - Store the key in the system credential manager via `state.credentials.store_secret`
   - Mask the key in the persisted `voice_config.json` (use empty string or `********`)
   - Never write plaintext API keys to disk files

3. WHEN `get_voice_config` retrieves configuration THEN system SHALL:
   - Return configuration with API keys masked as `********`
   - Never expose actual API key values to the frontend

4. WHEN `detect_voice_providers` runs THEN system SHALL:
   - Check local endpoints for Ollama, Chatterbox, GPT-SoVITS, XTTS-v2, Fish Speech, Dia, Coqui
   - Return availability status, version info, and error messages for each provider
   - Complete detection within 5 seconds (timeout per provider)

---

### Requirement 3: Voice Provider Installation Command Extraction

**User Story:** As a developer, I want all voice provider installation commands extracted to `commands/voice/providers.rs`, so that provider setup and model downloads are managed in one place.

#### Acceptance Criteria
1. WHEN a developer opens `commands/voice/providers.rs` THEN the file SHALL contain:
   - `check_voice_provider_installations` - Check status of all local providers
   - `check_voice_provider_status` - Check status of a specific provider
   - `install_voice_provider` - Install a voice provider (Piper, Coqui)
   - `list_downloadable_piper_voices` - List Piper voices from Hugging Face
   - `get_popular_piper_voices` - Get recommended Piper voices (no network)
   - `download_piper_voice` - Download a Piper voice model

2. WHEN `get_models_dir()` is called THEN system SHALL:
   - Return a deterministic path (`~/.local/share/ttrpg-assistant/voice/piper`)
   - NOT fall back to current working directory (`.`)
   - Return `Result<PathBuf, Error>` to propagate failures gracefully

3. WHEN `download_piper_voice` downloads a model THEN system SHALL:
   - Show download progress (bytes downloaded, percentage, ETA)
   - Support cancellation mid-download
   - Verify file integrity after download
   - Clean up partial downloads on failure or cancellation

---

### Requirement 4: Voice Preset Command Extraction

**User Story:** As a developer, I want all voice preset commands extracted to `commands/voice/presets.rs`, so that built-in DM persona management is isolated.

#### Acceptance Criteria
1. WHEN a developer opens `commands/voice/presets.rs` THEN the file SHALL contain:
   - `list_voice_presets` - List all built-in DM voice presets
   - `list_voice_presets_by_tag` - Filter presets by tag
   - `get_voice_preset` - Get a specific preset by ID

2. WHEN `list_voice_presets` is called THEN system SHALL return all presets from `core::voice::get_dm_presets()` with:
   - Preset ID, name, provider type, voice ID
   - Metadata: gender, age range, personality traits, tags, description

3. WHEN filtering by tag THEN system SHALL perform case-insensitive matching.

---

### Requirement 5: Voice Profile Command Extraction

**User Story:** As a developer, I want all voice profile commands extracted to `commands/voice/profiles.rs`, so that user-created voice profiles and NPC linkage is managed independently.

#### Acceptance Criteria
1. WHEN a developer opens `commands/voice/profiles.rs` THEN the file SHALL contain:
   - `create_voice_profile` - Create a new voice profile
   - `link_voice_profile_to_npc` - Associate profile with an NPC
   - `get_npc_voice_profile` - Get profile linked to an NPC
   - `search_voice_profiles` - Search profiles by query
   - `get_voice_profiles_by_gender` - Filter profiles by gender
   - `get_voice_profiles_by_age` - Filter profiles by age range

2. WHEN `link_voice_profile_to_npc` is called THEN system SHALL:
   - Fetch the NPC record from the database
   - Update the NPC's JSON data to include `voice_profile_id`
   - Persist the updated NPC record
   - Return `Err` if NPC not found

3. WHEN searching profiles THEN system SHALL:
   - Perform case-insensitive matching on name, personality traits, tags, and description
   - Return all matching profiles as a `Vec<VoiceProfile>`

---

### Requirement 6: Audio Cache Command Extraction

**User Story:** As a developer, I want all audio cache commands extracted to `commands/voice/cache.rs`, so that cache management is centralized.

#### Acceptance Criteria
1. WHEN a developer opens `commands/voice/cache.rs` THEN the file SHALL contain:
   - `get_audio_cache_stats` - Get comprehensive cache statistics
   - `get_audio_cache_size` - Get current cache size info
   - `clear_audio_cache` - Clear all cached audio
   - `clear_audio_cache_by_tag` - Clear cache entries by tag
   - `prune_audio_cache` - Remove entries older than specified age
   - `list_audio_cache_entries` - List all cache entries with metadata

2. WHEN `get_audio_cache_stats` is called THEN system SHALL return:
   - Hit/miss counts and cache hit rate
   - Current and maximum cache size in bytes
   - Entry counts by audio format
   - Average entry size
   - Oldest entry age

3. WHEN `prune_audio_cache` is called with `max_age_seconds` THEN system SHALL:
   - Remove all entries older than the specified age
   - Return the count of removed entries
   - NOT remove entries currently being played

---

### Requirement 7: Basic Voice Queue Command Extraction

**User Story:** As a developer, I want basic voice queue commands extracted to `commands/voice/queue.rs`, so that legacy queue functionality is preserved.

#### Acceptance Criteria
1. WHEN a developer opens `commands/voice/queue.rs` THEN the file SHALL contain:
   - `queue_voice` - Add text to voice queue
   - `get_voice_queue` - List queued voice items
   - `cancel_voice` - Remove item from queue
   - `process_voice_queue` (internal) - Background queue processor

2. WHEN `queue_voice` is called THEN system SHALL:
   - Add item to queue with status `Pending`
   - Trigger background processing if not already running
   - Use an `is_processing` flag to prevent multiple concurrent processors
   - Return the `QueuedVoice` item immediately

3. WHEN `process_voice_queue` processes an item THEN system SHALL:
   - Acquire selection and status update in a single write lock (prevent race conditions)
   - Mark item as `Processing` before releasing lock
   - Perform synthesis without holding state locks
   - Mark item as `Playing` during playback
   - Mark item as `Completed` or `Failed` after playback

---

### Requirement 8: Advanced Synthesis Queue Command Extraction

**User Story:** As a developer, I want all synthesis queue commands extracted to `commands/voice/synthesis_queue.rs`, so that the advanced priority queue system is independently maintainable.

#### Acceptance Criteria
1. WHEN a developer opens `commands/voice/synthesis_queue.rs` THEN the file SHALL contain:
   - `submit_synthesis_job` - Submit a job to the priority queue
   - `submit_synthesis_batch` - Submit multiple jobs as a batch
   - `get_synthesis_job` - Get job by ID
   - `get_synthesis_job_status` - Get job status
   - `get_synthesis_job_progress` - Get job progress
   - `cancel_synthesis_job` - Cancel a specific job
   - `cancel_all_synthesis_jobs` - Cancel all jobs
   - `pregen_session_voices` - Pre-generate voice audio for a session
   - `get_synthesis_queue_stats` - Get queue statistics
   - `pause_synthesis_queue` - Pause processing
   - `resume_synthesis_queue` - Resume processing
   - `is_synthesis_queue_paused` - Check pause state
   - `list_pending_synthesis_jobs` - List pending jobs
   - `list_processing_synthesis_jobs` - List processing jobs
   - `list_synthesis_job_history` - List completed/failed/cancelled jobs
   - `list_synthesis_jobs_by_session` - Filter by session
   - `list_synthesis_jobs_by_npc` - Filter by NPC
   - `list_synthesis_jobs_by_tag` - Filter by tag
   - `clear_synthesis_job_history` - Clear job history
   - `get_synthesis_queue_length` - Get total active jobs

2. WHEN `submit_synthesis_job` is called THEN system SHALL:
   - Parse provider string to `VoiceProviderType`
   - Parse priority string to `JobPriority` (immediate/high/normal/low/batch)
   - Create job with all metadata (session_id, npc_id, campaign_id, tags)
   - Return the created `SynthesisJob`

3. WHEN the queue emits events THEN system SHALL use Tauri event channels:
   - `synthesis:job-submitted` - Job added to queue
   - `synthesis:job-started` - Job processing started
   - `synthesis:job-progress` - Job progress updated
   - `synthesis:job-completed` - Job completed successfully
   - `synthesis:job-failed` - Job failed
   - `synthesis:job-cancelled` - Job cancelled
   - `synthesis:queue-stats` - Queue statistics updated
   - `synthesis:queue-paused` - Queue paused
   - `synthesis:queue-resumed` - Queue resumed

---

### Requirement 9: Module Organization and Re-exports

**User Story:** As a developer, I want the voice module to have clear organization with proper re-exports, so that consumers can import commands from a single location.

#### Acceptance Criteria
1. WHEN a developer opens `commands/voice/mod.rs` THEN the file SHALL:
   - Declare all submodules: `config`, `providers`, `synthesis`, `queue`, `presets`, `profiles`, `cache`, `synthesis_queue`
   - Re-export all public commands from each submodule
   - Include module-level documentation describing the voice subsystem

2. WHEN commands are registered in `main.rs` THEN the registration SHALL:
   - Use full module paths (`commands::voice::play_tts`)
   - NOT rely on `pub use` re-exports (due to `#[tauri::command]` proc macro limitations)

3. WHEN building the application THEN system SHALL:
   - Compile with zero warnings related to voice commands
   - Pass all existing voice-related tests

---

### Requirement 10: Security and Credential Handling

**User Story:** As a user, I want my API keys stored securely and never exposed in logs or files, so that my credentials remain private.

#### Acceptance Criteria
1. WHEN an API key is provided in `configure_voice` THEN system SHALL:
   - Store the key via `state.credentials.store_secret(key_name, key_value)`
   - Never log the actual key value
   - Never write the actual key to `voice_config.json`

2. WHEN `VoiceConfig` is serialized or logged THEN system SHALL:
   - Mask all API keys as `********` or empty string
   - NOT derive `Debug` that exposes secrets (use custom `Debug` impl if needed)

3. WHEN loading configuration at startup THEN system SHALL:
   - Read masked config from disk
   - Retrieve actual secrets from credential manager at runtime
   - Keep secrets only in memory, never in persistent storage

---

## Non-Functional Requirements

### NFR-1: Performance
1. WHEN processing the voice queue THEN system SHALL:
   - Handle at least 10 queued items without degradation
   - Complete synthesis initiation within 100ms of queue_voice call
   - Release state locks before performing network/IO operations

2. WHEN listing voices from providers THEN system SHALL:
   - Cache provider voice lists for 5 minutes
   - Complete list operations within 3 seconds

### NFR-2: Concurrency Safety
1. WHEN multiple queue_voice calls arrive simultaneously THEN system SHALL:
   - Process items sequentially (one at a time)
   - Prevent race conditions in item selection using atomic flags or single-lock patterns
   - Not spawn duplicate queue processors

2. WHEN accessing VoiceManager state THEN system SHALL:
   - Use minimal lock scopes (acquire → operate → release)
   - Prefer `read()` locks when not mutating state
   - Never hold locks across await points involving network calls

### NFR-3: Error Handling
1. WHEN a voice provider returns an error THEN system SHALL:
   - Map provider-specific errors to user-friendly messages
   - Log full error details at ERROR level
   - Return structured error information to the frontend

2. WHEN file operations fail THEN system SHALL:
   - Clean up partial files
   - Release any acquired resources
   - Return specific error describing the failure

### NFR-4: Testability
1. WHEN voice commands are extracted THEN each module SHALL:
   - Be testable in isolation via unit tests
   - Support mocking of external dependencies (providers, filesystem, credentials)
   - Include test coverage for happy path, error cases, and edge cases

### NFR-5: Code Reduction Target
1. AFTER extraction is complete THEN `commands_legacy.rs` SHALL:
   - Have ~1,400 fewer lines related to voice functionality
   - No longer contain any voice-related Tauri commands
   - Retain only non-voice functionality

### NFR-6: Code Quality (from Inspection Results)
1. WHEN building the application THEN system SHALL:
   - Pass all Rust code quality checks (clippy, field shorthand)
   - Use idiomatic Rust patterns (field init shorthand syntax)

2. WHEN documentation contains grammar THEN system SHALL:
   - Use correct article usage ("an" before vowel sounds)
   - Use uncountable nouns correctly (no article for "access")
   - Use correct verb tenses

3. **Specific Issues to Address** (source: IntelliJ inspection):
   - `frontend/src/components/settings/voice.rs:284-287`: Use field init shorthand
     - `length_scale: length_scale` → `length_scale`
     - `noise_scale: noise_scale` → `noise_scale`
     - `noise_w: noise_w` → `noise_w`
     - `sentence_silence: sentence_silence` → `sentence_silence`
   - `src-tauri/src/core/voice/cache.rs:75`: Change "an access" → "access"
   - `src-tauri/src/core/voice/cache.rs:242,245`: Change "a" → "an" before vowel
   - `src-tauri/src/core/voice/install.rs:649`: Use past participle form

---

## Constraints and Assumptions

### Constraints
1. **Tauri Command Registration**: Commands must be registered in `main.rs` using full module paths due to proc-macro behavior.
2. **Backward Compatibility**: All existing frontend code calling voice commands must continue to work without modification.
3. **State Structure**: The `AppState` and `SynthesisQueueState` structures cannot be modified during this extraction.
4. **Dependencies**: No new external crates should be added solely for this extraction.

### Assumptions
1. The existing `core::voice` module provides all necessary types and business logic.
2. The current voice functionality is working correctly and this is purely a structural refactor.
3. Test coverage exists for voice functionality that can validate the extraction.
4. The credential manager (`state.credentials`) is properly initialized before voice commands are called.

---

## Traceability Matrix

| Requirement | Files Affected | Commands Count |
|-------------|----------------|----------------|
| REQ-1: Synthesis | synthesis.rs | 5-6 |
| REQ-2: Configuration | config.rs | 4 |
| REQ-3: Providers | providers.rs | 6 |
| REQ-4: Presets | presets.rs | 3 |
| REQ-5: Profiles | profiles.rs | 6 |
| REQ-6: Cache | cache.rs | 6 |
| REQ-7: Basic Queue | queue.rs | 3 |
| REQ-8: Synthesis Queue | synthesis_queue.rs | 22 |
| REQ-9: Module Org | mod.rs, main.rs | - |
| REQ-10: Security | config.rs, types.rs | - |
| NFR-6: Code Quality | voice.rs (FE), cache.rs, install.rs | - |
| **Total** | **8+ files** | **~55-56** |
