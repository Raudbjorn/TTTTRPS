# Tasks: Voice Module Command Extraction

## Implementation Overview

This task breakdown follows a **foundation-first** strategy: update module structure, then extract commands submodule by submodule, fix known issues, and finally clean up. Each task produces working, testable code that doesn't break the build.

**Estimated Scope**: ~1,400 lines moved from `commands_legacy.rs` to 8 submodules in `commands/voice/`.

---

## Implementation Plan

### Phase 1: Module Structure Preparation

- [ ] **1.1 Update `commands/voice/mod.rs` module declarations**
  - Add all 8 submodule declarations (some already exist as placeholders)
  - Add comprehensive module-level documentation
  - Update existing re-exports for already-extracted commands
  - Files: `src-tauri/src/commands/voice/mod.rs`
  - _Requirements: REQ-9_

```rust
// Target structure:
//! Voice Commands Module
//!
//! Provides Tauri commands for voice synthesis, provider management,
//! voice profiles, presets, caching, and queue operations.
//!
//! ## Submodules
//! - `config` - Provider configuration and settings
//! - `synthesis` - Core TTS and audio playback
//! - `providers` - Provider installation and model downloads
//! - `presets` - Built-in DM voice presets
//! - `profiles` - User voice profiles and NPC linkage
//! - `cache` - Audio cache management
//! - `queue` - Basic voice queue (legacy)
//! - `synthesis_queue` - Advanced priority queue system

pub mod config;
pub mod synthesis;
pub mod providers;
pub mod presets;
pub mod profiles;
pub mod cache;
pub mod queue;
pub mod synthesis_queue;
```

- [ ] **1.2 Create shared types module (if needed)**
  - Identify any types needed across multiple voice submodules
  - Create `commands/voice/types.rs` if there are command-specific types
  - Add re-exports to mod.rs
  - Files: `src-tauri/src/commands/voice/types.rs` (new if needed)
  - _Requirements: REQ-9_

---

### Phase 2: Extract Preset Commands

- [ ] **2.1 Extract preset commands to `presets.rs`**
  - Move from `commands_legacy.rs`:
    - `list_voice_presets` (line ~5974)
    - `list_voice_presets_by_tag` (line ~5980)
    - `get_voice_preset` (line ~5986)
  - Add necessary imports from `core::voice::presets`
  - These are synchronous commands (no state required)
  - Remove placeholder comments from existing file
  - Files: `src-tauri/src/commands/voice/presets.rs`
  - _Requirements: REQ-4_

```rust
// Target implementation:
use crate::core::voice::{VoiceProfile, get_dm_presets, get_presets_by_tag, get_preset_by_id};

#[tauri::command]
pub fn list_voice_presets() -> Vec<VoiceProfile> {
    get_dm_presets()
}

#[tauri::command]
pub fn list_voice_presets_by_tag(tag: String) -> Vec<VoiceProfile> {
    get_presets_by_tag(&tag)
}

#[tauri::command]
pub fn get_voice_preset(preset_id: String) -> Option<VoiceProfile> {
    get_preset_by_id(&preset_id)
}
```

- [ ] **2.2 Add re-exports and verify compilation**
  - Add `pub use presets::{list_voice_presets, list_voice_presets_by_tag, get_voice_preset};` to mod.rs
  - Run `cargo check` to verify compilation
  - Files: `src-tauri/src/commands/voice/mod.rs`
  - _Requirements: REQ-9_

---

### Phase 3: Extract Profile Commands

- [ ] **3.1 Extract profile commands to `profiles.rs`**
  - Move from `commands_legacy.rs`:
    - `create_voice_profile` (line ~5992)
    - `link_voice_profile_to_npc` (line ~6023)
    - `get_npc_voice_profile` (line ~6044)
    - `search_voice_profiles` (line ~6062)
    - `get_voice_profiles_by_gender` (line ~6077)
    - `get_voice_profiles_by_age` (line ~6093)
  - Add necessary imports:
    - `crate::core::voice::{VoiceProfile, VoiceProviderType, ProfileMetadata, Gender, AgeRange, get_dm_presets}`
    - `crate::commands::AppState`
    - `tauri::State`
  - Remove placeholder comments from existing file
  - Files: `src-tauri/src/commands/voice/profiles.rs`
  - _Requirements: REQ-5_

- [ ] **3.2 Add re-exports and verify compilation**
  - Add profile command re-exports to mod.rs
  - Run `cargo check`
  - Files: `src-tauri/src/commands/voice/mod.rs`
  - _Requirements: REQ-9_

---

### Phase 4: Extract Cache Commands

- [ ] **4.1 Extract cache commands to `cache.rs`**
  - Move from `commands_legacy.rs`:
    - `get_audio_cache_stats` (line ~6121)
    - `clear_audio_cache_by_tag` (line ~6137)
    - `clear_audio_cache` (line ~6150)
    - `prune_audio_cache` (line ~6166)
    - `list_audio_cache_entries` (line ~6183)
    - `get_audio_cache_size` (line ~6198)
  - Move the `AudioCacheSizeInfo` struct definition (line ~6215)
  - Add necessary imports:
    - `crate::core::voice::{AudioCache, CacheEntry, CacheStats}` (alias as `VoiceCacheStats` if needed)
    - `crate::commands::AppState`
    - `tauri::State`
  - Remove placeholder comments from existing file
  - Files: `src-tauri/src/commands/voice/cache.rs`
  - _Requirements: REQ-6_

- [ ] **4.2 Add re-exports and verify compilation**
  - Add cache command re-exports to mod.rs
  - Run `cargo check`
  - Files: `src-tauri/src/commands/voice/mod.rs`
  - _Requirements: REQ-9_

---

### Phase 5: Extract Synthesis Queue Commands

- [ ] **5.1 Create `synthesis_queue.rs` with helper functions**
  - Create new file `commands/voice/synthesis_queue.rs`
  - Add helper functions (move from commands_legacy.rs):
    - `parse_queue_provider(provider: &str) -> Result<VoiceProviderType, String>`
    - `parse_queue_priority(priority: Option<&str>) -> Result<JobPriority, String>`
  - Add necessary type imports
  - Files: `src-tauri/src/commands/voice/synthesis_queue.rs` (new)
  - _Requirements: REQ-8_

```rust
// Helper implementations:
fn parse_queue_provider(provider: &str) -> Result<VoiceProviderType, String> {
    match provider.to_lowercase().as_str() {
        "elevenlabs" => Ok(VoiceProviderType::ElevenLabs),
        "openai" => Ok(VoiceProviderType::OpenAI),
        "piper" => Ok(VoiceProviderType::Piper),
        // ... other providers
        _ => Err(format!("Unknown provider: {}", provider)),
    }
}

fn parse_queue_priority(priority: Option<&str>) -> Result<JobPriority, String> {
    match priority.map(|s| s.to_lowercase()).as_deref() {
        None | Some("normal") => Ok(JobPriority::Normal),
        Some("immediate") => Ok(JobPriority::Immediate),
        Some("high") => Ok(JobPriority::High),
        Some("low") => Ok(JobPriority::Low),
        Some("batch") => Ok(JobPriority::Batch),
        Some(other) => Err(format!("Unknown priority: {}", other)),
    }
}
```

- [ ] **5.2 Extract synthesis queue commands - Part 1 (Core operations)**
  - Move from `commands_legacy.rs`:
    - `submit_synthesis_job` (line ~6296)
    - `get_synthesis_job` (line ~6340)
    - `get_synthesis_job_status` (line ~6349)
    - `get_synthesis_job_progress` (line ~6358)
    - `cancel_synthesis_job` (line ~6367)
    - `cancel_all_synthesis_jobs` (line ~6379)
  - Move `SynthesisJobRequest` struct if defined in commands_legacy.rs
  - Files: `src-tauri/src/commands/voice/synthesis_queue.rs`
  - _Requirements: REQ-8_

- [ ] **5.3 Extract synthesis queue commands - Part 2 (Batch and session)**
  - Move from `commands_legacy.rs`:
    - `pregen_session_voices` (line ~6390)
    - `submit_synthesis_batch` (line ~6406)
  - Files: `src-tauri/src/commands/voice/synthesis_queue.rs`
  - _Requirements: REQ-8_

- [ ] **5.4 Extract synthesis queue commands - Part 3 (Queue control)**
  - Move from `commands_legacy.rs`:
    - `get_synthesis_queue_stats` (line ~6443)
    - `pause_synthesis_queue` (line ~6451)
    - `resume_synthesis_queue` (line ~6461)
    - `is_synthesis_queue_paused` (line ~6471)
  - Files: `src-tauri/src/commands/voice/synthesis_queue.rs`
  - _Requirements: REQ-8_

- [ ] **5.5 Extract synthesis queue commands - Part 4 (Listing and filtering)**
  - Move from `commands_legacy.rs`:
    - `list_pending_synthesis_jobs` (line ~6479)
    - `list_processing_synthesis_jobs` (line ~6487)
    - `list_synthesis_job_history` (line ~6495)
    - `list_synthesis_jobs_by_session` (line ~6504)
    - `list_synthesis_jobs_by_npc` (line ~6513)
    - `list_synthesis_jobs_by_tag` (line ~6522)
    - `clear_synthesis_job_history` (line ~6531)
    - `get_synthesis_queue_length` (line ~6540)
  - Files: `src-tauri/src/commands/voice/synthesis_queue.rs`
  - _Requirements: REQ-8_

- [ ] **5.6 Add synthesis_queue re-exports and verify compilation**
  - Add all synthesis_queue command re-exports to mod.rs
  - Declare `pub mod synthesis_queue;` in mod.rs
  - Run `cargo check`
  - Files: `src-tauri/src/commands/voice/mod.rs`
  - _Requirements: REQ-9_

---

### Phase 6: Verify Existing Extracted Commands

- [ ] **6.1 Review and update `config.rs`**
  - Verify all 4 commands are present:
    - `configure_voice`
    - `get_voice_config`
    - `detect_voice_providers`
    - `list_all_voices`
  - Verify imports are correct
  - Files: `src-tauri/src/commands/voice/config.rs`
  - _Requirements: REQ-2_

- [ ] **6.2 Review and update `synthesis.rs`**
  - Verify all synthesis commands are present:
    - `play_tts`
    - `list_openai_voices`
    - `list_openai_tts_models`
    - `list_elevenlabs_voices`
    - `list_available_voices`
  - Check if `transcribe_audio` exists and should be included
  - Files: `src-tauri/src/commands/voice/synthesis.rs`
  - _Requirements: REQ-1_

- [ ] **6.3 Review and update `providers.rs`**
  - Verify all 6 provider commands are present:
    - `check_voice_provider_installations`
    - `check_voice_provider_status`
    - `install_voice_provider`
    - `list_downloadable_piper_voices`
    - `get_popular_piper_voices`
    - `download_piper_voice`
  - Files: `src-tauri/src/commands/voice/providers.rs`
  - _Requirements: REQ-3_

- [ ] **6.4 Review and update `queue.rs`**
  - Verify all 3 queue commands are present:
    - `queue_voice`
    - `get_voice_queue`
    - `cancel_voice`
  - Verify `process_voice_queue` helper is present
  - Files: `src-tauri/src/commands/voice/queue.rs`
  - _Requirements: REQ-7_

---

### Phase 7: Fix Known Issues

- [ ] **7.1 Fix `get_models_dir()` fallback in `providers.rs`**
  - Change return type to `Result<PathBuf, String>` or use `std::env::temp_dir()` as fallback
  - Do NOT use `PathBuf::from(".")` as fallback
  - Update callers to handle Result
  - Files: `src-tauri/src/commands/voice/providers.rs`
  - _Requirements: REQ-3, NFR-3_

```rust
// Fix:
fn get_models_dir() -> Result<PathBuf, String> {
    dirs::data_local_dir()
        .map(|p| p.join("ttrpg-assistant/voice/piper"))
        .ok_or_else(|| "Could not determine local data directory".to_string())
}

// Or with temp fallback:
fn get_models_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("ttrpg-assistant/voice/piper")
}
```

- [ ] **7.2 Fix secret persistence in `config.rs`**
  - Ensure `save_voice_config_disk` never writes actual API keys
  - Create masked copy of config before saving
  - Verify `configure_voice` flow: store secret → mask config → save masked
  - Files: `src-tauri/src/commands/voice/config.rs`
  - _Requirements: REQ-2, REQ-10_

```rust
// Fix pattern:
pub async fn configure_voice(...) -> Result<String, String> {
    // 1. Store API key in credential manager (if provided and not masked)
    if let Some(elevenlabs) = config.elevenlabs.clone() {
        if !elevenlabs.api_key.is_empty() && elevenlabs.api_key != "********" {
            state.credentials.store_secret("elevenlabs_api_key", &elevenlabs.api_key)?;
        }
    }

    // 2. Create effective config with secrets for runtime use
    let mut effective_config = config.clone();
    // ... restore secrets from credential manager for empty/masked keys ...

    // 3. Create masked config for disk persistence
    let mut config_for_disk = effective_config.clone();
    if let Some(ref mut elevenlabs) = config_for_disk.elevenlabs {
        elevenlabs.api_key = String::new(); // Mask for disk
    }
    save_voice_config_disk(&app_handle, &config_for_disk);

    // 4. Update VoiceManager with effective config (has real secrets)
    let new_manager = VoiceManager::new(effective_config);
    *state.voice_manager.write().await = new_manager;

    Ok("Voice configuration updated".to_string())
}
```

- [ ] **7.3 Fix queue race condition in `queue.rs`**
  - Add `is_processing: AtomicBool` field to VoiceManager or use other synchronization
  - Use compare_exchange to atomically check-and-set before spawning processor
  - Ensure processor clears flag on completion (normal or error)
  - Files: `src-tauri/src/commands/voice/queue.rs`, possibly `core/voice/manager.rs`
  - _Requirements: REQ-7, NFR-2_

```rust
// Fix pattern in queue_voice:
use std::sync::atomic::{AtomicBool, Ordering};

// In queue_voice, before spawning:
static IS_PROCESSING: AtomicBool = AtomicBool::new(false);

// Only spawn if we can atomically set from false to true
if IS_PROCESSING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
    tauri::async_runtime::spawn(async move {
        // Process queue...
        // On completion/error:
        IS_PROCESSING.store(false, Ordering::SeqCst);
    });
}
```

- [ ] **7.4 Ensure atomic selection and claim in `process_voice_queue`**
  - Move item selection AND status update to single write lock scope
  - Prevent TOCTOU race between read (selection) and write (claim)
  - Files: `src-tauri/src/commands/voice/queue.rs`
  - _Requirements: REQ-7, NFR-2_

```rust
// Fix pattern:
let (item, _) = {
    let mut manager = vm_clone.write().await;
    if manager.is_playing {
        (None, ())
    } else if let Some(item) = manager.get_next_pending() {
        manager.update_status(&item.id, VoiceStatus::Processing);
        (Some(item), ())
    } else {
        (None, ())
    }
};
// Lock released here, proceed with synthesis if item.is_some()
```

- [ ] **7.5 Fix issues from IDE inspection results**
  - **Frontend field shorthand** (`frontend/src/components/settings/voice.rs:284-287`):
    - Change `length_scale: length_scale` → `length_scale`
    - Change `noise_scale: noise_scale` → `noise_scale`
    - Change `noise_w: noise_w` → `noise_w`
    - Change `sentence_silence: sentence_silence` → `sentence_silence`
  - **Grammar in doc comments** (`core/voice/cache.rs`):
    - Line 75: Change "an access" → "access" (uncountable noun)
    - Line 242: Change "a" → "an" before vowel sound
    - Line 245: Change "a" → "an" before vowel sound
  - **Grammar in doc comments** (`core/voice/install.rs`):
    - Line 649: Use past participle form
  - Files: `frontend/src/components/settings/voice.rs`, `src-tauri/src/core/voice/cache.rs`, `src-tauri/src/core/voice/install.rs`
  - _Requirements: REQ-9 (code quality)_
  - _Source: IntelliJ inspection results (RsFieldInitShorthand, GrazieInspection)_

---

### Phase 8: Update Command Registration

- [ ] **8.1 Update `main.rs` command registration**
  - Replace old registrations with full module paths
  - Add all newly extracted commands
  - Group commands by module for readability
  - Files: `src-tauri/src/main.rs`
  - _Requirements: REQ-9_

```rust
// Example registration in main.rs:
.invoke_handler(tauri::generate_handler![
    // ... other commands ...

    // Voice: Config
    commands::voice::configure_voice,
    commands::voice::get_voice_config,
    commands::voice::detect_voice_providers,
    commands::voice::list_all_voices,

    // Voice: Synthesis
    commands::voice::play_tts,
    commands::voice::list_openai_voices,
    commands::voice::list_openai_tts_models,
    commands::voice::list_elevenlabs_voices,
    commands::voice::list_available_voices,

    // Voice: Providers
    commands::voice::check_voice_provider_installations,
    commands::voice::check_voice_provider_status,
    commands::voice::install_voice_provider,
    commands::voice::list_downloadable_piper_voices,
    commands::voice::get_popular_piper_voices,
    commands::voice::download_piper_voice,

    // Voice: Presets
    commands::voice::list_voice_presets,
    commands::voice::list_voice_presets_by_tag,
    commands::voice::get_voice_preset,

    // Voice: Profiles
    commands::voice::create_voice_profile,
    commands::voice::link_voice_profile_to_npc,
    commands::voice::get_npc_voice_profile,
    commands::voice::search_voice_profiles,
    commands::voice::get_voice_profiles_by_gender,
    commands::voice::get_voice_profiles_by_age,

    // Voice: Cache
    commands::voice::get_audio_cache_stats,
    commands::voice::get_audio_cache_size,
    commands::voice::clear_audio_cache,
    commands::voice::clear_audio_cache_by_tag,
    commands::voice::prune_audio_cache,
    commands::voice::list_audio_cache_entries,

    // Voice: Queue (Legacy)
    commands::voice::queue_voice,
    commands::voice::get_voice_queue,
    commands::voice::cancel_voice,

    // Voice: Synthesis Queue
    commands::voice::submit_synthesis_job,
    commands::voice::submit_synthesis_batch,
    commands::voice::get_synthesis_job,
    commands::voice::get_synthesis_job_status,
    commands::voice::get_synthesis_job_progress,
    commands::voice::cancel_synthesis_job,
    commands::voice::cancel_all_synthesis_jobs,
    commands::voice::pregen_session_voices,
    commands::voice::get_synthesis_queue_stats,
    commands::voice::pause_synthesis_queue,
    commands::voice::resume_synthesis_queue,
    commands::voice::is_synthesis_queue_paused,
    commands::voice::list_pending_synthesis_jobs,
    commands::voice::list_processing_synthesis_jobs,
    commands::voice::list_synthesis_job_history,
    commands::voice::list_synthesis_jobs_by_session,
    commands::voice::list_synthesis_jobs_by_npc,
    commands::voice::list_synthesis_jobs_by_tag,
    commands::voice::clear_synthesis_job_history,
    commands::voice::get_synthesis_queue_length,
])
```

- [ ] **8.2 Verify build compiles with new registrations**
  - Run `cargo build`
  - Fix any missing import or registration errors
  - Files: `src-tauri/src/main.rs`
  - _Requirements: REQ-9_

---

### Phase 9: Cleanup

- [ ] **9.1 Remove voice code from `commands_legacy.rs`**
  - Remove all voice command functions (now in voice/ modules)
  - Remove voice-related helper functions
  - Remove voice-related type definitions (if moved)
  - Remove unused voice imports
  - Files: `src-tauri/src/commands_legacy.rs`
  - _Requirements: NFR-5_

- [ ] **9.2 Remove duplicate registrations from `main.rs`**
  - Search for any remaining old-style voice command registrations
  - Remove duplicates
  - Files: `src-tauri/src/main.rs`
  - _Requirements: REQ-9_

- [ ] **9.3 Run clippy and fix warnings**
  - Run `cargo clippy -- -D warnings`
  - Fix all clippy warnings related to voice module
  - Pay attention to: unused imports, dead code, unnecessary clones
  - Files: All voice module files
  - _Requirements: REQ-9_

- [ ] **9.4 Run tests**
  - Run `cargo test` for all voice-related tests
  - Verify no regressions
  - Files: All test files
  - _Requirements: NFR-4_

---

### Phase 10: Documentation and Verification

- [ ] **10.1 Add/update module documentation**
  - Ensure each module has `//!` doc comments explaining purpose
  - Add doc comments to public commands
  - Files: All voice module files
  - _Requirements: REQ-9_

- [ ] **10.2 Verify command count matches expectations**
  - Count commands in each module
  - Verify against Requirements.md traceability matrix
  - Expected: ~56 commands total across 8 modules
  - _Requirements: All_

- [ ] **10.3 Manual testing (optional but recommended)**
  - Test voice configuration in UI
  - Test TTS playback
  - Test queue operations
  - Verify no regressions in frontend functionality
  - _Requirements: All_

---

## Task Summary

| Phase | Tasks | Commands Moved | Est. Lines |
|-------|-------|----------------|------------|
| 1. Module Structure | 2 | 0 | 50 |
| 2. Presets | 2 | 3 | 30 |
| 3. Profiles | 2 | 6 | 120 |
| 4. Cache | 2 | 6 | 100 |
| 5. Synthesis Queue | 6 | 22 | 400 |
| 6. Verify Existing | 4 | 0 (verify) | 0 |
| 7. Fix Issues | 5 | 0 | 90 |
| 8. Registration | 2 | 0 | 100 |
| 9. Cleanup | 4 | 0 | -1,400 |
| 10. Documentation | 3 | 0 | 50 |
| **Total** | **32** | **~37 new** | **-~500** |

**Note**: Some commands are already partially extracted. The extraction removes ~1,400 lines from `commands_legacy.rs` while adding ~940 lines to organized submodules, for a net reduction of ~460 lines. This improves maintainability by organizing ~56 commands across 8 focused submodules.

---

## Dependencies Graph

```
Phase 1 (Module Structure)
    │
    ├──► Phase 2 (Presets) ──────────────────────┐
    ├──► Phase 3 (Profiles) ─────────────────────┤
    ├──► Phase 4 (Cache) ────────────────────────┤
    └──► Phase 5 (Synthesis Queue) ──────────────┤
                                                  │
Phase 6 (Verify Existing) ◄───────────────────────┘
    │
    ▼
Phase 7 (Fix Issues) ──────► Phase 8 (Registration)
                                     │
                                     ▼
                             Phase 9 (Cleanup)
                                     │
                                     ▼
                             Phase 10 (Documentation)
```

Phases 2-5 can be executed in parallel after Phase 1 completes.
