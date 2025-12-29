# UX Overhaul Implementation Tasks

## Document Info
- **Version**: 2.0.0
- **Branch**: `feature/ux-overhaul`
- **Last Updated**: 2025-12-29

---

## Task Legend

| Prefix | Type | Description |
|--------|------|-------------|
| `[BE]` | Backend | Rust/Tauri changes (src-tauri/) |
| `[FE]` | Frontend | Dioxus/CSS changes (frontend/) |
| `[REQ-XXX]` | Requirement | Links to requirements.md |

**Priority**: P0 (Critical) → P1 (High) → P2 (Medium) → P3 (Nice-to-have)

---

## Phase 1: Backend Implementation (Rust/Tauri)

### NPC Conversation Persistence
> Supports: REQ-003 (NPC Conversations)

- [ ] **[BE] B1**: Create `NpcConversation` model and database table (P0)
  ```rust
  pub struct NpcConversation {
      pub id: String,
      pub npc_id: String,
      pub campaign_id: String,
      pub messages: Vec<ConversationMessage>,
      pub unread_count: u32,
      pub last_message_at: DateTime<Utc>,
  }
  ```
  - Migration: `npc_conversations` table with foreign keys
  - Files: `src-tauri/src/database/models.rs`, `migrations/`

- [ ] **[BE] B2**: Implement NPC conversation Tauri commands (P0)
  - `list_npc_conversations(campaign_id)` → Vec<NpcConversation>
  - `get_npc_conversation(npc_id)` → NpcConversation
  - `add_npc_message(npc_id, content, role)` → ConversationMessage
  - `mark_npc_read(npc_id)` → ()
  - Files: `src-tauri/src/commands.rs`

- [ ] **[BE] B3**: Add thread/reply support to conversation messages (P1)
  - `parent_message_id` field for threading
  - `list_thread(message_id)` command
  - Files: `src-tauri/src/core/models.rs`

### Session Management
> Supports: REQ-002 (Session Timeline)

- [ ] **[BE] B4**: Add `Planned` session status and creation flow (P0)
  - Extend `SessionStatus` enum: `{ Planned, Active, Paused, Ended }`
  - `create_planned_session(campaign_id, title, notes)` command
  - `start_planned_session(session_id)` transitions Planned → Active
  - Files: `src-tauri/src/core/session_manager.rs`

- [ ] **[BE] B5**: Update `list_sessions` to include full status info (P0)
  - Return `SessionSummary` with status, duration, summary preview
  - Sort: Current first, then Planned by order, then Past by date desc
  - Files: `src-tauri/src/commands.rs`

- [ ] **[BE] B6**: Add session reordering for planned sessions (P2)
  - `reorder_session(session_id, new_position)` command
  - `order` field in session table
  - Files: `src-tauri/src/core/session_manager.rs`

### Dynamic Theme System
> Supports: REQ-004 (Dynamic Theme System)

- [ ] **[BE] B7**: Create `ThemeWeights` model and persistence (P0)
  ```rust
  pub struct ThemeWeights {
      pub fantasy: f32,
      pub cosmic: f32,
      pub terminal: f32,
      pub noir: f32,
      pub neon: f32,
  }
  ```
  - Store in `campaign_settings` as JSON
  - Files: `src-tauri/src/core/models.rs`, `campaign_manager.rs`

- [ ] **[BE] B8**: Implement setting-to-theme mapping (P1)
  - `get_theme_preset(system: &str)` → ThemeWeights
  - Default mappings: "D&D 5e" → fantasy, "Call of Cthulhu" → cosmic, etc.
  - Fallback to fantasy for unknown systems
  - Files: `src-tauri/src/core/theme.rs` (new)

- [ ] **[BE] B9**: Add theme configuration commands (P1)
  - `get_campaign_theme(campaign_id)` → ThemeWeights
  - `set_campaign_theme(campaign_id, weights)` → ()
  - Files: `src-tauri/src/commands.rs`

### Personality Management
> Supports: REQ-005 (Personality Manager)

- [ ] **[BE] B10**: Persist personality profiles to database (P1)
  - Currently in-memory only (`PERSONALITY_STORE`)
  - Migration: `personalities` table
  - CRUD commands: `save_personality`, `delete_personality`, `list_personalities`
  - Files: `src-tauri/src/core/personality.rs`, `commands.rs`

- [ ] **[BE] B11**: Link personalities to NPCs (P1)
  - `personality_id` field on NPC model
  - `assign_personality(npc_id, personality_id)` command
  - Files: `src-tauri/src/core/npc_gen.rs`

### Voice & Transcription
> Supports: REQ-006, REQ-007

- [ ] **[BE] B12**: Add voice queue management (P1)
  - Queue multiple voice synthesis requests
  - `queue_voice(text, voice_id)` → QueuePosition
  - `get_voice_queue()` → Vec<QueuedVoice>
  - `cancel_voice(queue_id)` → ()
  - Files: `src-tauri/src/core/voice/manager.rs`

- [ ] **[BE] B13**: Implement speech-to-text integration (P2)
  - Provider abstraction (Whisper API, local Whisper, browser API)
  - `start_transcription()` → Stream<TranscriptionEvent>
  - `stop_transcription()` → FinalTranscript
  - Files: `src-tauri/src/core/transcription/` (new module)

### Campaign Enhancements
> Supports: REQ-001 (Campaign Hub)

- [ ] **[BE] B14**: Add campaign statistics aggregation (P2)
  - `get_campaign_stats(campaign_id)` → CampaignStats
  - Stats: session_count, npc_count, total_playtime, last_played
  - Files: `src-tauri/src/core/campaign_manager.rs`

- [ ] **[BE] B15**: Generate campaign cover art placeholder (P3)
  - Hash-based generative art (like GitHub identicons)
  - Or integrate with image generation API
  - Files: `src-tauri/src/core/campaign_manager.rs`

---

## Phase 2: Frontend Implementation (Dioxus/Rust)

### Layout Architecture
> Supports: REQ-010 (Session Workspace Layout)

- [ ] **[FE] F1**: Implement App Shell with 4-panel layout (P0)
  - Icon Rail (left, 56px fixed)
  - Context Sidebar (left, 240px, resizable)
  - Main Content (center, flex)
  - Info Panel (right, 300px, toggleable)
  - Files: `frontend/src/components/app_shell.rs` (new)

- [ ] **[FE] F2**: Create Icon Rail navigation component (P0)
  - Icons: Chat, Campaigns, Library, Graph, Settings
  - Active indicator (vertical bar)
  - Tooltips on hover
  - Files: `frontend/src/components/icon_rail.rs` (new)

- [ ] **[FE] F3**: Implement resizable panel system (P1)
  - Drag handles between panels
  - Min/max width constraints
  - Collapse to icon mode
  - Files: `frontend/src/components/resizable_panel.rs` (new)

### Campaign Hub (Album View)
> Supports: REQ-001

- [ ] **[FE] F4**: Redesign Campaign Cards as album covers (P0)
  - Cover art area (placeholder or generated)
  - Genre badge overlay
  - "Now Playing" pulse animation
  - Hover reveals stats
  - Files: `frontend/src/components/campaigns.rs`

- [ ] **[FE] F5**: Add grid/list view toggle for campaigns (P2)
  - Grid: 3-4 columns of cards
  - List: compact rows with more metadata
  - Persist preference
  - Files: `frontend/src/components/campaigns.rs`

### Session List (Track List)
> Supports: REQ-002, REQ-012

- [ ] **[FE] F6**: Refactor SessionList with status grouping (P0)
  - Collapsible sections: Current, Planned, Past
  - Visual differentiation per status (see design.md)
  - Click to select/open session
  - Files: `frontend/src/components/campaign_details/session_list.rs`

- [ ] **[FE] F7**: Add drag-and-drop reordering for planned sessions (P2)
  - Drag handle on planned session items
  - Visual drop indicator
  - Calls `reorder_session` on drop
  - Files: `frontend/src/components/campaign_details/session_list.rs`

- [ ] **[FE] F8**: Implement session status badges and indicators (P1)
  - Pulsing dot for active
  - Dashed border for planned
  - Muted + checkmark for past
  - Files: `frontend/src/components/design_system.rs`

### NPC Sidebar (Slack DMs)
> Supports: REQ-003, REQ-011

- [ ] **[FE] F9**: Redesign NPC list as Slack-style DM list (P0)
  - Avatar + name + unread badge
  - Last message preview
  - Presence/activity indicator
  - Files: `frontend/src/components/campaign_details/npc_list.rs`

- [ ] **[FE] F10**: Create NPC conversation view (P0)
  - Threaded message display
  - Input for new messages
  - Typing indicator during LLM response
  - Files: `frontend/src/components/npc_conversation.rs` (new)

- [ ] **[FE] F11**: Implement NPC context menu (P1)
  - Right-click menu: Edit, Voice, Relationships, Stats, Note
  - Actions call appropriate commands
  - Files: `frontend/src/components/context_menu.rs` (new)

### Personality Manager
> Supports: REQ-005

- [ ] **[FE] F12**: Create personality grid view (P1)
  - Card per personality with name, source, traits preview
  - Click opens detail modal
  - Files: `frontend/src/components/campaign_details/personality_manager.rs`

- [ ] **[FE] F13**: Implement personality detail modal (P1)
  - Speech patterns, example phrases
  - Linked NPCs list
  - Voice configuration
  - Test voice button
  - Files: `frontend/src/components/personality_detail.rs` (new)

- [ ] **[FE] F14**: Add drag-to-assign personality to NPC (P2)
  - Drag personality card onto NPC in sidebar
  - Visual feedback on valid drop target
  - Files: `frontend/src/components/campaign_details/`

### Media Bar
> Supports: REQ-006

- [ ] **[FE] F15**: Create persistent Media Bar component (P0)
  - Fixed bottom, 56px height
  - Play/Pause/Stop controls
  - Progress bar with seek
  - Volume slider
  - Files: `frontend/src/components/media_bar.rs` (new)

- [ ] **[FE] F16**: Add "Now Speaking" display (P1)
  - NPC avatar + name
  - Waveform visualization during playback
  - Files: `frontend/src/components/media_bar.rs`

- [ ] **[FE] F17**: Implement voice queue indicator (P2)
  - Badge showing queue length
  - Click to expand queue list
  - Cancel individual items
  - Files: `frontend/src/components/media_bar.rs`

### Transcription Toggle
> Supports: REQ-007

- [ ] **[FE] F18**: Add transcription toggle to Media Bar (P2)
  - Microphone icon button
  - Visual waveform when active
  - Privacy indicator (red dot)
  - Blocked by: BE B13
  - Files: `frontend/src/components/media_bar.rs`

- [ ] **[FE] F19**: Display transcribed text in session log (P2)
  - Real-time text insertion
  - Speaker labels if available
  - Blocked by: BE B13
  - Files: `frontend/src/components/session.rs`

### Dynamic Theme System
> Supports: REQ-004

- [ ] **[FE] F20**: Implement ThemeProvider context (P0)
  - Reads campaign theme weights
  - Applies CSS class and custom properties
  - Live updates on campaign switch
  - Files: `frontend/src/components/theme_provider.rs` (new)

- [ ] **[FE] F21**: Create theme CSS with all 5 base themes (P0)
  - Fantasy, Cosmic, Terminal, Noir, Neon
  - CSS custom properties per design.md
  - Files: `frontend/public/themes.css`

- [ ] **[FE] F22**: Implement CSS property interpolation for blended themes (P1)
  - JavaScript/WASM interpolation of oklch colors
  - Apply computed values as inline styles
  - Files: `frontend/src/theme/interpolate.rs` (new)

- [ ] **[FE] F23**: Add visual effects (grain, scanlines, glow) (P1)
  - Pseudo-element overlays
  - Intensity controlled by `--effect-*` variables
  - Respect `prefers-reduced-motion`
  - Files: `frontend/public/effects.css` (new)

- [ ] **[FE] F24**: Create theme blend settings UI (P2)
  - 5 sliders for theme weights
  - Auto-normalize to 100%
  - Live preview
  - Save to campaign
  - Files: `frontend/src/components/settings/theme_settings.rs` (new)

### Chat Enhancements
> Supports: REQ-003

- [ ] **[FE] F25**: Update ChatMessage with play button integration (P1)
  - Play button queues to Media Bar
  - Only visible when voice configured
  - Files: `frontend/src/components/chat.rs`

- [ ] **[FE] F26**: Add message threading UI (P2)
  - Thread preview: "3 replies"
  - Click to expand inline
  - Reply action on messages
  - Files: `frontend/src/components/chat.rs`

- [ ] **[FE] F27**: Implement quick actions on messages (P2)
  - Pin, React (emoji), Add Note
  - Hover to reveal action bar
  - Files: `frontend/src/components/chat.rs`

### Command Palette
> Supports: REQ-009, REQ-014

- [ ] **[FE] F28**: Create command palette component (P2)
  - `Cmd+K` to open
  - Fuzzy search across: NPCs, sessions, notes, documents, commands
  - Type filters with pills
  - Recent searches
  - Files: `frontend/src/components/command_palette.rs` (new)

- [ ] **[FE] F29**: Implement keyboard shortcuts (P2)
  - Global listener for shortcuts
  - Configurable mappings
  - Help modal showing all shortcuts
  - Files: `frontend/src/keyboard.rs` (new)

### Combat Mode
> Supports: REQ-013

- [ ] **[FE] F30**: Enhance combat view with theme intensity (P2)
  - Redder tints when in combat
  - Faster pulse animations
  - Initiative tracker expansion
  - Files: `frontend/src/components/session.rs`

### Knowledge Graph
> Supports: REQ-008

- [ ] **[FE] F31**: Create entity graph visualization (P3)
  - Force-directed layout
  - Node types: NPC, Location, Plot, Faction
  - Edge types: ally, enemy, knows, visited
  - Zoom/pan controls
  - Click to open detail
  - Files: `frontend/src/components/graph_view.rs` (new)

### Responsive Design
> Supports: REQ-015

- [ ] **[FE] F32**: Implement responsive breakpoints (P1)
  - ≥1400px: Full layout
  - 1200-1399px: Hide info panel
  - 900-1199px: Collapse sidebar
  - <900px: Drawer mode
  - Files: `frontend/src/components/app_shell.rs`

- [ ] **[FE] F33**: Add sidebar drawer mode for narrow screens (P2)
  - Slide-in overlay
  - Backdrop click to close
  - Swipe gesture (future)
  - Files: `frontend/src/components/drawer.rs` (new)

---

## Phase 3: Polish & Accessibility

- [ ] **[FE] F34**: Audit and add ARIA labels (P1)
  - All interactive elements
  - Live regions for updates
  - Landmark roles
  - Files: All components

- [ ] **[FE] F35**: Implement focus management (P1)
  - Visible focus rings
  - Focus trap in modals
  - Logical tab order
  - Files: `frontend/public/tailwind.css`, components

- [ ] **[FE] F36**: Add `prefers-reduced-motion` support (P2)
  - Disable animations when preferred
  - Fallback to instant transitions
  - Files: `frontend/public/tailwind.css`

- [ ] **[FE] F37**: Color contrast audit (P2)
  - Verify WCAG AA for all themes
  - Fix any failing combinations
  - Files: `frontend/public/themes.css`

---

## Dependency Graph

```
BE B1 ──┬──> FE F9 (NPC list)
BE B2 ──┘    │
             └──> FE F10 (NPC conversation)

BE B4 ──┬──> FE F6 (Session list)
BE B5 ──┘

BE B7 ──┬──> FE F20 (ThemeProvider)
BE B8 ──┤
BE B9 ──┘    └──> FE F21, F22, F23 (Theme CSS)

BE B12 ─────> FE F15, F16, F17 (Media Bar)

BE B13 ─────> FE F18, F19 (Transcription)
```

---

## Milestones

### M1: Core Layout (P0 tasks)
- App shell with 4 panels
- Session list with grouping
- NPC sidebar redesign
- Basic theme switching

### M2: Conversations & Voice (P0-P1)
- NPC conversation persistence
- Media bar with playback
- Play buttons on messages
- Voice queue

### M3: Dynamic Themes (P1)
- All 5 theme definitions
- Theme interpolation
- Visual effects
- Theme settings UI

### M4: Polish (P2-P3)
- Command palette
- Keyboard shortcuts
- Responsive design
- Accessibility audit
- Knowledge graph
