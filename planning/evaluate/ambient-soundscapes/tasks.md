# Ambient Soundscapes - Tasks

## Phase 1: Audio Sink Infrastructure

### Backend (Rust)

- [ ] **Extend Rodio audio manager**
  - [ ] Add named sink pairs for crossfade: `music_a`/`music_b`, `ambience_a`/`ambience_b`, `sfx` (voice already exists)
  - [ ] Per-channel volume control (0.0 - 1.0) - controls both sinks in pair
  - [ ] Per-channel mute toggle
  - [ ] Sink state tracking (playing, paused, stopped)
  - [ ] Active sink tracking per channel (which of A/B is currently playing)

- [ ] **Audio playback utilities**
  - [ ] `play_audio(channel, path, loop)` - Start playback on inactive sink
  - [ ] `stop_audio(channel)` - Stop with fade out
  - [ ] `pause_audio(channel)` / `resume_audio(channel)`
  - [ ] `set_volume(channel, level)` - Adjust volume
  - [ ] `crossfade(channel, to_path, duration_ms)` - Fade out active sink while fading in inactive sink with new track, then swap active pointer

- [ ] **Tauri commands**
  - [ ] `play_soundscape_track` - Play on specific sink
  - [ ] `stop_soundscape_track`
  - [ ] `pause_soundscape` / `resume_soundscape`
  - [ ] `set_sink_volume`
  - [ ] `get_audio_state` - Current playback status per sink
  - [ ] `crossfade_track`

- [ ] **Audio file validation**
  - [ ] Supported formats: MP3, OGG, WAV, FLAC
  - [ ] File existence check
  - [ ] Duration detection for UI display

### Frontend (Leptos)

- [ ] **AudioMixer component**
  - [ ] Volume sliders for each sink (music, ambience, sfx, voice)
  - [ ] Mute toggles per sink
  - [ ] Master volume control
  - [ ] Visual level indicators

---

## Phase 2: Soundscape Library

### Backend

- [ ] **Asset directory structure**
  ```
  assets/audio/
  ├── music/
  │   ├── combat/
  │   ├── exploration/
  │   └── tavern/
  ├── ambience/
  │   ├── forest/
  │   ├── dungeon/
  │   └── town/
  └── sfx/
      ├── combat/
      ├── environment/
      └── ui/
  ```

- [ ] **Soundscape preset system**
  - [ ] `Soundscape` struct with layers and tags
  - [ ] JSON preset files in assets
  - [ ] Load presets on startup
  - [ ] User-created custom presets

- [ ] **Built-in presets** (bundle ~10-15 essential soundscapes)
  - [ ] Tavern (background chatter, fire crackling, lute music)
  - [ ] Forest (birds, wind, rustling leaves)
  - [ ] Dungeon (dripping water, distant echoes, torch flicker)
  - [ ] Combat (intense music, no ambience)
  - [ ] Town (crowds, carts, market noise)
  - [ ] Ocean/Ship (waves, seagulls, creaking wood)
  - [ ] Cave (echoes, water drops, low rumble)
  - [ ] Castle (stone echoes, distant voices, torches)
  - [ ] Night camp (crickets, fire, owl)
  - [ ] Storm (thunder, heavy rain, wind)

- [ ] **Tauri commands**
  - [ ] `list_soundscapes` - Get all available presets
  - [ ] `get_soundscape(id)` - Get preset details
  - [ ] `play_soundscape(id)` - Activate a soundscape
  - [ ] `stop_soundscape` - Stop all layers
  - [ ] `create_soundscape` - Save custom preset
  - [ ] `delete_soundscape` - Remove custom preset
  - [ ] `import_audio_file` - Add custom audio to library
    - [ ] Validate file path to prevent path traversal attacks
    - [ ] Copy file into managed `data/audio/custom/` directory (never reference external paths)
    - [ ] Validate file format and size limits before copy
    - [ ] Generate unique filename with content hash

### Frontend

- [ ] **SoundscapePanel component**
  - [ ] Grid/list view of available soundscapes
  - [ ] Category filters (combat, exploration, location)
  - [ ] Search/filter by name or tag
  - [ ] Play/stop buttons per soundscape
  - [ ] Currently playing indicator

- [ ] **SoundscapeCard component**
  - [ ] Thumbnail/icon per soundscape
  - [ ] Name and tags
  - [ ] Quick play button
  - [ ] Favorite toggle

- [ ] **CustomSoundscapeEditor modal**
  - [ ] Name and tags input
  - [ ] Layer configuration (select tracks per sink)
  - [ ] Volume per layer
  - [ ] Preview before saving

- [ ] **AudioLibraryBrowser component**
  - [ ] Browse available audio files
  - [ ] Import custom files
  - [ ] Preview individual tracks
  - [ ] Organize into folders

---

## Phase 3: Session Integration

### Backend

- [ ] **Session state extension**
  - [ ] Add `active_soundscape_id` to session state
  - [ ] Persist soundscape across session save/load
  - [ ] Auto-restore soundscape on session resume

- [ ] **Combat state triggers**
  - [ ] Detect combat start -> suggest combat music
  - [ ] Detect combat end -> revert to previous soundscape
  - [ ] Optional auto-switch setting

### Frontend

- [ ] **QuickSoundBar component** (compact session controls)
  - [ ] Current soundscape indicator
  - [ ] Quick switch dropdown
  - [ ] Volume slider (master)
  - [ ] Mute all button

- [ ] **Session panel integration**
  - [ ] Soundscape selector in session header
  - [ ] "Combat music" quick button
  - [ ] Recent soundscapes list

- [ ] **Settings integration**
  - [ ] Default volumes per sink
  - [ ] Auto-switch on combat toggle
  - [ ] Fade duration preference

---

## Phase 4: Smart Automation (Stretch)

### Backend

- [ ] **Scene detection service**
  - [ ] Analyze DM narration for location keywords
  - [ ] Keyword -> soundscape mapping (tavern, forest, cave, etc.)
  - [ ] Confidence threshold for suggestions

- [ ] **LLM-assisted scene detection** (requires Session Recap infrastructure)
  - [ ] Extract scene context from recent chat
  - [ ] Suggest appropriate soundscape
  - [ ] Learn from user corrections

- [ ] **Tauri commands**
  - [ ] `suggest_soundscape(context)` - Get AI suggestion
  - [ ] `record_soundscape_correction` - User feedback for learning

### Frontend

- [ ] **SoundscapeSuggestion component**
  - [ ] Non-intrusive suggestion banner
  - [ ] "Play suggested" / "Dismiss" buttons
  - [ ] "Don't suggest for this scene" option

- [ ] **Automation settings**
  - [ ] Enable/disable auto-suggestions
  - [ ] Keyword customization
  - [ ] Review suggestion history

---

## Audio Asset Sourcing

- [ ] **Curate starter pack** (~50-100MB compressed)
  - [ ] Source from CC0/royalty-free libraries
  - [ ] Document attribution where required (JSON manifest per track)
  - [ ] Normalize audio levels
  - [ ] Convert to consistent format (OGG recommended)

- [ ] **Attribution display (required for CC-BY)**
  - [ ] Create `AUDIO_CREDITS.md` with all attributions
  - [ ] Add "Audio Credits" section in Settings/About UI
  - [ ] Display artist/source for CC-BY tracks (Incompetech, etc.)
  - [ ] Link to original sources where applicable

- [ ] **Sources to evaluate**
  - [ ] Freesound.org (CC0 ambience/sfx)
  - [ ] Incompetech/Kevin MacLeod (CC-BY music)
  - [ ] OpenGameArt (game audio)
  - [ ] BBC Sound Effects (ambience)

---

## Dependencies

- Rodio (existing) - Audio playback
- Additional format decoders if needed (symphonia)

## Testing

- [ ] Unit tests for volume/crossfade calculations
- [ ] Integration tests for soundscape loading
- [ ] Manual testing of crossfade smoothness
- [ ] Test concurrent playback (all sinks active)
