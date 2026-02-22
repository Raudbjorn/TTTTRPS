# 06 — Voice System

**Gap addressed:** #14 (voice system severely simplified)

## Actual Voice Architecture

The codebase has a comprehensive voice system with 11 providers, a priority queue, voice profiles, and caching. The original document's proposal to use `tts` crate + `rodio` would discard most of this functionality.

### Provider Registry

**Cloud providers (API key required):**

| Provider | Config | Notes |
|----------|--------|-------|
| ElevenLabs | API key, model_id | Premium voice cloning |
| Fish Audio | API key, base_url | Cloud speech synthesis |
| OpenAI TTS | API key, model (tts-1/tts-1-hd), voice (alloy/echo/fable/onyx/nova/shimmer) | 6 built-in voices |

**Self-hosted providers (local or remote):**

| Provider | Default Port | Special Features |
|----------|-------------|------------------|
| Piper | local binary | length_scale, noise params, speaker_id |
| Ollama | configurable | base_url, model selection |
| Coqui XTTS-v2 | 5002 | speaker_wav cloning, temperature/top_k/top_p |
| Chatterbox | 8000 | reference_audio, exaggeration, cfg_weight |
| GPT-SoVITS | 9880 | reference audio+text, multi-language |
| Fish Speech | 7860 | reference audio+text |
| Dia | 8003 | voice_id, dialogue_mode (podcast-style) |

**System:**
- System TTS (OS native)
- Disabled (placeholder/testing)

### Synthesis Queue (`core/voice/queue/`)

```
SynthesisJob → PriorityQueue → VoiceSynthesizer → Audio Output
                    │
               QueueEventEmitter
                    │
              TUI Event Bridge
```

- **Priority levels:** Urgent, High, Normal, Low
- **Job states:** Pending → Processing → Playing → Completed/Failed
- **Progress tracking:** percent complete, ETA, batch info
- **Queue config:** max concurrent jobs, batch size, timeout
- **Controls:** pause/resume, cancel individual jobs, clear queue

**QueueEventEmitter trait** (`core/voice/queue/events.rs`):
- `JobSubmittedEvent`, `JobStatusEvent`, `QueueStatsEvent`
- `NoopEmitter` for headless/test mode
- TUI should implement this trait to forward queue events to the UI

### Voice Profiles (`core/voice/profiles.rs`)

```rust
VoiceProfile {
    id, name, provider, voice_id, settings,
    metadata: ProfileMetadata {
        age_range: Child | YoungAdult | Adult | MiddleAged | Elderly,
        gender: Male | Female | Neutral | NonBinary,
        personality_traits: Vec<String>,
        linked_npc_ids: Vec<String>,
        description, tags,
    },
    is_preset, timestamps
}
```

13+ built-in DM personality presets (Gruff, Cheerful, Mysterious, etc.).

### Voice Profile ↔ NPC Integration

- Each NPC can have a linked voice profile
- Profile selection based on NPC archetype, culture, personality
- Voice synthesis uses profile settings when generating NPC dialogue

### TUI Requirements

1. **Provider configuration** — per-provider setup forms (API keys, endpoints, self-hosted URLs)
2. **Provider detection** — `detection.rs` probes which providers are available
3. **Profile manager** — create/edit/delete custom voice profiles with metadata
4. **Preset browser** — select from 13+ built-in DM presets
5. **NPC voice linker** — assign profiles to NPCs
6. **Queue monitor** — live view of pending/processing/completed jobs with progress bars
7. **Queue controls** — pause, resume, cancel, clear
8. **Voice preview** — test synthesis with selected profile before committing
9. **Cache management** — view cache stats, manual cleanup
10. **Piper model downloads** — `download.rs` supports downloading voice models
