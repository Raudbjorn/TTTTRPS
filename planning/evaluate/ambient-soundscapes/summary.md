# Ambient Soundscapes - Feature Evaluation

## Status: PROPOSED

**Priority:** HIGH - Natural extension of voice system, proven immersion pattern

## Overview

Add ambient audio layers (background music, environmental sounds, sound effects) that complement the existing voice system. Soundscapes adapt to scene context, combat state, and DM narration.

## Why This Feature

- **Immersion multiplier**: Voice is point-source; ambience is environmental
- **Proven pattern**: VTT platforms (Foundry, Roll20) with ambience get strong user engagement
- **Existing foundation**: Rodio audio backend already supports multi-sink playback
- **Low friction**: Audio files are commoditized; implementation is the blocker

## Feature Scope

### Tier 1: Manual Soundscapes (MVP)
- Soundscape library browser
- Play/pause/volume per layer (music, ambience, SFX)
- Crossfade between tracks
- Presets: tavern, forest, dungeon, combat, town, ocean
- Import custom audio files

### Tier 2: Context-Aware Automation
- Auto-switch based on combat state (exploration -> combat music)
- Scene tags in session notes trigger soundscapes
- DM narration keyword detection (LLM identifies "you enter the tavern")
- Smooth transitions with configurable fade duration

### Tier 3: Dynamic Mixing
- Layered soundscapes (base ambience + weather overlay + time-of-day variation)
- Randomized SFX triggers (occasional bird chirps, distant thunder)
- Combat intensity scaling (music tempo/layers increase with danger)
- Spatial audio (future: different sounds from map directions)

## Technical Approach

### Audio Architecture
```
┌─────────────────────────────────────────┐
│              Audio Manager              │
├─────────────┬─────────────┬─────────────┤
│   Music     │  Ambience   │    SFX      │
│   Sink      │    Sink     │   Sink      │
├─────────────┴─────────────┴─────────────┤
│              Voice Sink                 │
│           (existing TTS)                │
└─────────────────────────────────────────┘
```

**Existing:** Voice sink via Rodio (voice/manager.rs)
**New:** 3 additional sinks for music, ambience, SFX

### Audio Sources

**Option A: Bundled Packs (Recommended for MVP)**
- Include CC0/royalty-free packs in app bundle
- ~50-100MB of compressed audio covers basics
- No external deps, works offline

**Option B: Streaming Services**
- Syrinscape, Tabletop Audio APIs
- Higher quality, larger library
- Requires subscription/API costs

**Option C: Local Library + AI Generation**
- User imports their own files
- AI music generation via MusicGen/AudioCraft (experimental)

### Implementation Plan

### Phase 1: Audio Sink Infrastructure
- Extend Rodio setup with named sinks (music, ambience, sfx)
- Per-sink volume control
- Crossfade utility function
- Tauri commands: `play_audio`, `stop_audio`, `set_volume`, `crossfade`

### Phase 2: Soundscape Library
- Asset directory structure: `assets/audio/{music,ambience,sfx}/`
- Soundscape presets as JSON (name, tracks, layers, tags)
- Library browser UI component
- Favorite/recent soundscapes

### Phase 3: Session Integration
- Soundscape selector in session panel
- Quick-switch buttons for common scenes
- Combat state triggers music change
- Persist active soundscape in session state

### Phase 4: Smart Automation (Stretch)
**Dependencies:** Requires NPC Memory System (for context tracking) and Session Recap Generation (transcript processing/chunking infrastructure) to be implemented first.

- LLM context analysis for scene detection
- Narration text -> soundscape suggestion (reuses transcript processing from Session Recap)
- User confirms or overrides
- Learning from corrections

**Sequencing note:** Implement Session Recap Generation before this phase, as it provides the transcript processing, chunking, and context extraction infrastructure needed for scene detection.

## Data Structures

```rust
pub struct Soundscape {
    pub id: String,
    pub name: String,
    pub layers: Vec<AudioLayer>,
    pub tags: Vec<String>,  // "combat", "tavern", "forest"
    pub transition: TransitionStyle,
}

pub struct AudioLayer {
    pub sink: AudioSink,  // Music, Ambience, SFX
    pub tracks: Vec<AudioTrack>,
    pub volume: f32,
    pub loop_mode: LoopMode,
}

pub enum AudioSink {
    Music,
    Ambience,
    Sfx,
    Voice,  // existing
}

pub struct AudioTrack {
    pub path: PathBuf,
    pub title: String,
    pub duration_secs: u32,
    pub fade_in: f32,
    pub fade_out: f32,
}
```

## UI Components

- `SoundscapePanel.rs` - Library browser + playback controls
- `AudioMixer.rs` - Per-sink volume sliders
- `QuickSoundBar.rs` - Compact in-session controls
- Settings: Default volumes, auto-play preferences

## Audio Asset Sources (CC0/Royalty-Free)

- **Tabletop Audio** (tabletop-audio.com) - TTRPG-specific, 10-min loops
- **Freesound.org** - SFX, ambience samples
- **Incompetech** (Kevin MacLeod) - Music, CC-BY
- **BBC Sound Effects** - High quality ambience
- **OpenGameArt** - Game-oriented audio

## Effort Estimate

| Phase | Complexity | Notes |
|-------|------------|-------|
| Audio Sinks | Low | Rodio already used |
| Soundscape Library | Medium | Asset curation takes time |
| Session Integration | Low | UI + state connection |
| Smart Automation | High | LLM integration |

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Audio licensing issues | Stick to CC0, document sources |
| Bundle size bloat | Compress audio, optional download packs |
| Jarring transitions | Always crossfade, never hard cut |
| CPU/memory with many tracks | Limit concurrent tracks, stream from disk |

## Success Metrics

- Audio playback latency < 200ms
- 70%+ sessions use at least one soundscape
- User satisfaction: "more immersive" feedback

## Related Features

- Voice system (existing) - Same audio backend
- Combat tracker (existing) - Combat music triggers
- Session notes (existing) - Scene tags for automation
- Personality system (existing) - DM persona voice + matching ambience

## Recommendation

**High value, medium effort.** The audio backend exists; this is primarily asset curation + UI work. Start with manual soundscapes (Tier 1), add automation later. Consider bundling a small "starter pack" of 10-15 essential soundscapes.
