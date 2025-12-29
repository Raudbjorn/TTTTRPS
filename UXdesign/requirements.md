# UX Requirements Specification

## Document Info
- **Version**: 2.0.0
- **Branch**: `feature/ux-overhaul`
- **Last Updated**: 2025-12-29

---

## Core Design Philosophy

### Inspiration Sources
| Source | Metaphor | Application |
|--------|----------|-------------|
| **Spotify** | Campaigns = Albums, Sessions = Tracks, Settings = Genres | Browse campaigns like a music library, play sessions like songs |
| **Slack** | NPC Conversations = DM Channels, Game Log = Threads | Chat with NPCs like Slack conversations, threaded replies |
| **Obsidian** | Campaign = Vault, Entities = Notes, Relationships = Links | Knowledge graph for NPCs/locations/plots, vault-centric navigation |

### Guiding Principles
1. **Immersion First**: UI adapts to the setting, never breaks the mood
2. **Contextual Density**: Show what's relevant now, progressive disclosure for depth
3. **Muscle Memory**: Familiar patterns from tools users already love
4. **Zero Friction**: Common actions within 1-2 clicks

---

## Functional Requirements

### REQ-001: Campaign Hub (Album View)
**Priority**: Critical | **Type**: Frontend

Campaign selection as a music library experience:
- Campaign cards with cover art/generated imagery
- Hover reveals quick stats (sessions played, active NPCs, last played)
- Grid/list toggle view
- Genre/setting badges with theme preview
- "Now Playing" indicator for active campaigns
- Quick-launch button to resume last session

### REQ-002: Session Timeline (Track List)
**Priority**: Critical | **Type**: Frontend

Sessions displayed as a playlist within each campaign:
- Grouped by status: **Planned** → **Current** → **Past**
- Session number as track number
- Duration/playtime shown for completed sessions
- Hover reveals session summary
- Click to open session workspace
- Drag-and-drop reorder for planned sessions
- Visual distinction: Past (muted), Current (glowing), Planned (dashed outline)

### REQ-003: NPC Conversations (Slack Channels)
**Priority**: Critical | **Type**: Frontend + Backend

NPCs as sidebar channels with persistent conversation history:
- NPC avatar + name in sidebar (Slack DM style)
- Conversation history persisted per NPC across sessions
- Unread indicator when NPC has pending responses
- Typing indicator during LLM generation
- Reply threading for multi-turn dialogue
- Pin important exchanges to top
- Right-click context menu: Edit, Voice, Relationships, Stats

**Backend Requirements** (see tasks.md):
- B1: NPC conversation persistence model
- B2: Thread/reply data structure
- B3: Unread state tracking

### REQ-004: Dynamic Theme System
**Priority**: Critical | **Type**: Frontend + Backend

UI visually adapts to campaign setting with **theme interpolation**:

#### Base Themes
| ID | Setting Examples | Vibe | Key Elements |
|----|------------------|------|--------------|
| `fantasy` | D&D, Pathfinder, Warhammer Fantasy | Warm, arcane | Glassmorphism, gold accents, serif headers, parchment textures |
| `cosmic` | Call of Cthulhu, Delta Green, Kult | Dread, wrongness | Non-euclidean borders, eye motifs, sickly greens, grain overlay |
| `terminal` | Mothership, Alien RPG, Traveller | Cold, industrial | CRT scanlines, monospace, amber/green phosphor, glitch effects |
| `noir` | Delta Green, Night's Black Agents | 90s office, paranoia | Typewriter fonts, manila/tan palette, redacted text effect, fluorescent flicker |
| `neon` | Cyberpunk RED, Shadowrun, The Sprawl | Electric, chrome | Neon gradients, sharp angles, RGB highlights, hologram effects |

#### Theme Interpolation
For settings that blend genres, themes can interpolate:
```
Delta Green = cosmic(0.5) + noir(0.5)
Shadowrun = neon(0.7) + fantasy(0.3)
Custom = sliders for manual blend
```

**Backend Requirements**:
- B4: Theme weights stored per campaign (`{fantasy: 0.3, cosmic: 0.7}`)
- B5: Setting-to-theme mapping configuration

### REQ-005: Personality Manager (Artist Profiles)
**Priority**: High | **Type**: Frontend

Voice/personality configuration as artist discography:
- Grid of personality "albums" (extracted from source documents)
- Cards show: name, source doc, key traits preview
- Click opens detail modal:
  - Speech patterns and example phrases
  - Linked NPCs using this personality
  - Voice provider/ID configuration
  - Test voice button
- Drag personality onto NPC to assign
- Import from document button

### REQ-006: Voice Synthesis Controls (Media Bar)
**Priority**: High | **Type**: Frontend

Persistent Spotify-style media bar at bottom:
- Play/pause/stop controls
- Voice queue indicator (pending outputs)
- Volume slider
- Provider badge (ElevenLabs, OpenAI, etc.)
- "Now Speaking" shows current NPC/character
- Mute toggle with visual state
- Waveform visualization during playback

### REQ-007: Live Transcription
**Priority**: Medium | **Type**: Frontend + Backend

Speech-to-text capture for session notes:
- Microphone toggle in media bar
- Visual waveform when active
- Transcribed text flows into session log
- Speaker labels (if diarization supported)
- Enable/disable per session
- Privacy indicator when mic active

**Backend Requirements**:
- B6: Speech-to-text provider integration
- B7: Audio capture and streaming pipeline

### REQ-008: Knowledge Graph (Obsidian-style)
**Priority**: Medium | **Type**: Frontend

Visual relationship map of campaign entities:
- Nodes: NPCs, Locations, Plot Threads, Factions
- Edges: relationships (ally, enemy, knows, visited)
- Click node to open detail panel
- Filter by entity type
- Cluster by faction/region
- Zoom/pan navigation
- Mini-map for large graphs

### REQ-009: Keyboard Navigation
**Priority**: Medium | **Type**: Frontend

Power user shortcuts:
| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl + K` | Command palette (search anything) |
| `Cmd/Ctrl + /` | Toggle NPC sidebar |
| `Cmd/Ctrl + .` | Toggle session sidebar |
| `Space` | Play/pause voice |
| `N` | New session |
| `Escape` | Close modals/panels |
| Arrow keys | Navigate lists |
| `Enter` | Select/open current item |

### REQ-010: Session Workspace Layout
**Priority**: Critical | **Type**: Frontend

Resizable three-panel layout:
1. **Left Rail** (48-64px): Icon nav - Chat, Campaigns, Library, Settings
2. **Context Sidebar** (200-300px): Sessions or NPCs based on view
3. **Main Content** (flex): Chat, notes, combat tracker
4. **Right Panel** (250-350px, optional): NPC details, initiative, dice

### REQ-011: NPC Quick Actions
**Priority**: High | **Type**: Frontend

From NPC sidebar entry:
- **Click**: Open conversation thread
- **Right-click menu**:
  - Edit personality
  - Assign/change voice
  - View relationships graph
  - Generate stat block
  - Quick note
  - Add to initiative
- **Drag**: Reorder priority or drop into initiative tracker

### REQ-012: Session State Indicators
**Priority**: High | **Type**: Frontend

Visual status vocabulary:
| State | Visual Treatment |
|-------|------------------|
| Planned | Dashed outline, future icon, lighter opacity |
| Current/Active | Solid fill, pulsing glow, "LIVE" badge |
| Paused | Dimmed, pause icon overlay |
| Past/Ended | Muted colors, checkmark, duration shown |

### REQ-013: Combat Mode
**Priority**: High | **Type**: Frontend

When combat active, UI adapts:
- Initiative tracker expands prominently
- Current turn combatant highlighted with glow
- HP bars visible on all combatants
- Quick damage/heal number input
- Condition badges (poisoned, stunned, etc.)
- Combat event log
- Exit combat requires confirmation
- Theme intensifies (redder tints, faster animations)

### REQ-014: Search & Command Palette
**Priority**: Medium | **Type**: Frontend

Global spotlight search:
- `Cmd/Ctrl + K` opens palette
- Search: NPCs, sessions, notes, documents, commands
- Filter by type with pill toggles
- Recent searches remembered
- Fuzzy matching for typos
- Results preview on hover

### REQ-015: Responsive Collapse
**Priority**: Medium | **Type**: Frontend

Adaptive sidebar behavior:
- `>1400px`: All panels visible
- `1200-1400px`: One sidebar at a time
- `<1200px`: Sidebars overlay (drawer mode)
- `<800px`: Mobile-optimized single column
- Touch-friendly tap targets

---

## Non-Functional Requirements

### NFR-001: Performance
- Theme transitions: < 200ms
- Message render: < 50ms
- Voice playback start: < 500ms
- Navigation: < 100ms

### NFR-002: Accessibility
- ARIA labels on all interactive elements
- Full keyboard navigation
- Visible focus indicators
- WCAG AA color contrast
- Reduced motion option

### NFR-003: Persistence
- Theme saved per campaign
- Sidebar state remembered
- Last session auto-restored on launch
- Scroll positions preserved

### NFR-004: Extensibility
- Theme system accepts custom CSS overrides
- Color palette injectable via settings
- Font stack customizable per theme

---

## Out of Scope (Phase 1)
- Mobile native app
- Multi-user collaborative sessions
- Plugin/extension system
- Custom theme builder GUI
- Ambient audio/music player (beyond voice TTS)
