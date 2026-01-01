# UX Overhaul Implementation Tasks

## Document Info
- **Version**: 2.1.0
- **Branch**: `feature/ux-overhaul`
- **Last Updated**: 2025-12-29

## Progress Summary

| Phase | Complete | In Progress | Pending | Total |
|-------|----------|-------------|---------|-------|
| Backend (B1-B15) | 4 | 0 | 11 | 15 |
| Frontend (F1-F37) | 12 | 4 | 21 | 37 |
| **Overall** | **16** | **4** | **32** | **52** |

**Legend**: `[x]` Complete, `[~]` Partial, `[ ]` Pending

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

- [x] **[BE] B1**: Create `NpcConversation` model and database table (P0)
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

- [x] **[BE] B2**: Implement NPC conversation Tauri commands (P0)
  - `list_npc_conversations(campaign_id)` → Vec<NpcConversation>
  - `get_npc_conversation(npc_id, offset?, limit?)` → NpcConversation
  - `add_npc_message(npc_id, content, role)` → ConversationMessage
  - `mark_npc_read(npc_id)` → ()
  - Files: `src-tauri/src/commands.rs`
  - **Design Note**: Support pagination for message history. Long-running campaigns
    may have hundreds of messages per NPC. Use cursor-based pagination with
    `offset`/`limit` parameters on `get_npc_conversation`. Frontend should implement
    infinite scroll or "Load More" pattern.

- [ ] **[BE] B3**: Add thread/reply support to conversation messages (P1)
  - `parent_message_id` field for threading
  - `list_thread(message_id)` command
  - Files: `src-tauri/src/core/models.rs`

### Session Management
> Supports: REQ-002 (Session Timeline)

- [x] **[BE] B4**: Add `Planned` session status and creation flow (P0)
  - Extend `SessionStatus` enum: `{ Planned, Active, Paused, Ended }`
  - `create_planned_session(campaign_id, title, notes)` command
  - `start_planned_session(session_id)` transitions Planned → Active
  - Files: `src-tauri/src/core/session_manager.rs`

- [x] **[BE] B5**: Update `list_sessions` to include full status info (P0)
  - Return `SessionSummary` with status, duration, summary preview
  - Sort: Current first, then Planned by order, then Past by date desc
  - Files: `src-tauri/src/commands.rs`

- [ ] **[BE] B6**: Add session reordering for planned sessions (P2)
  - `reorder_session(session_id, new_position)` command
  - `order` field in session table
  - Files: `src-tauri/src/core/session_manager.rs`

### Dynamic Theme System
> Supports: REQ-004 (Dynamic Theme System)

- [x] **[BE] B7**: Create `ThemeWeights` model and persistence (P0)
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

- [x] **[BE] B8**: Implement setting-to-theme mapping (P1)
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

- [x] **[FE] F1**: Implement App Shell with 4-panel layout (P0)
  - Icon Rail (left, 64px fixed)
  - Context Sidebar (left, 280px, toggleable)
  - Main Content (center, flex)
  - Info Panel (right, 320px, toggleable)
  - Files: `frontend/src/components/layout/main_shell.rs`

- [x] **[FE] F2**: Create Icon Rail navigation component (P0)
  - Icons: Campaigns, Chat, Library, Graph, Settings
  - Active indicator (border-left accent)
  - Tooltips on hover
  - Files: `frontend/src/components/layout/icon_rail.rs`

- [ ] **[FE] F3**: Implement resizable panel system (P1)
  - Drag handles between panels
  - Min/max width constraints
  - Collapse to icon mode
  - Files: `frontend/src/components/resizable_panel.rs` (new)

### Campaign Hub (Album View)
> Supports: REQ-001

- [x] **[FE] F4**: Redesign Campaign Cards as album covers (P0)
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

- [x] **[FE] F6**: Refactor SessionList with status grouping (P0)
  - Sections: CURRENT, PLANNED, HISTORY
  - Visual differentiation per status
  - Click to select/open session
  - Files: `frontend/src/components/campaign_details/session_list.rs`
  - **Note**: Uses mock session grouping logic until backend B5 status field is fully integrated

- [ ] **[FE] F7**: Add drag-and-drop reordering for planned sessions (P2)
  - Drag handle on planned session items
  - Visual drop indicator
  - Calls `reorder_session` on drop
  - Files: `frontend/src/components/campaign_details/session_list.rs`

- [x] **[FE] F8**: Implement session status badges and indicators (P1)
  - Pulsing green dot for active (animate-pulse)
  - Dashed border for planned
  - Muted style for history
  - Files: `frontend/src/components/campaign_details/session_list.rs`

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

- [x] **[FE] F12**: Create personality grid view (P1)
  - Spotify-style grid with album art cards
  - Name, voice provider, source doc preview
  - Play and Edit buttons on hover
  - Files: `frontend/src/components/campaign_details/personality_manager.rs`

- [x] **[FE] F13**: Implement personality detail modal (P1)
  - Name, voice provider fields
  - Source knowledge (RAG) file picker
  - Save/Cancel actions
  - Files: `frontend/src/components/campaign_details/personality_manager.rs`
  - **Note**: Currently uses mock data, needs backend B10 integration

- [ ] **[FE] F14**: Add drag-to-assign personality to NPC (P2)
  - Drag personality card onto NPC in sidebar
  - Visual feedback on valid drop target
  - Files: `frontend/src/components/campaign_details/`

### Media Bar
> Supports: REQ-006

- [x] **[FE] F15**: Create persistent Media Bar component (P0)
  - Fixed bottom, 56px height in grid layout
  - Play/Pause/Skip controls
  - Progress bar scrubber
  - Volume indicator
  - Files: `frontend/src/components/layout/media_bar.rs`
  - **Note**: Skeleton UI only, needs backend B12 voice queue integration

- [~] **[FE] F16**: Add "Now Speaking" display (P1)
  - Shows "SYSTEM" with pulsing indicator
  - Needs NPC avatar + name integration
  - Files: `frontend/src/components/layout/media_bar.rs`

- [ ] **[FE] F17**: Implement voice queue indicator (P2)
  - Badge showing queue length
  - Click to expand queue list
  - Cancel individual items
  - Files: `frontend/src/components/layout/media_bar.rs`

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

- [~] **[FE] F20**: Implement ThemeProvider context (P0)
  - Theme class applied via use_memo in session.rs
  - Maps campaign system to theme class (fantasy, cosmic, terminal, noir, neon)
  - **TODO**: Extract to proper ThemeProvider component with Signal context
  - Files: `frontend/src/components/session.rs:68-122`

- [x] **[FE] F21**: Create theme CSS with all 5 base themes (P0)
  - Fantasy (arcane glassmorphism)
  - Cosmic (eldritch dread)
  - Terminal (nostromo console)
  - Noir (90s office paranoia)
  - Neon (cyberpunk chrome)
  - CSS custom properties per design.md
  - Files: `frontend/public/themes.css`

- [ ] **[FE] F22**: Implement CSS property interpolation for blended themes (P1)
  - JavaScript/WASM interpolation of oklch colors
  - Apply computed values as inline styles
  - **See TODO in session.rs:71-75**
  - Files: `frontend/src/theme/interpolate.rs` (new)

- [x] **[FE] F23**: Add visual effects (grain, scanlines, glow) (P1)
  - `.effect-grain::before` and `.effect-scanlines::after` overlays
  - Intensity controlled by `--effect-grain`, `--effect-scanline`, `--effect-glow`
  - Respects `prefers-reduced-motion`
  - Files: `frontend/public/themes.css:266-420`

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

- [~] **[FE] F34**: Audit and add ARIA labels (P1)
  - Added aria_label to: Add NPC button, damage/heal/remove buttons, voice play button
  - Remaining: All interactive elements need audit
  - **TODO**: Live regions for updates, landmark roles
  - Files: All components

- [~] **[FE] F35**: Implement focus management (P1)
  - Focus styles defined via `:focus` on `.redacted` class
  - **TODO**: Visible focus rings on all buttons, focus trap in modals
  - Files: `frontend/public/themes.css`, components

- [x] **[FE] F36**: Add `prefers-reduced-motion` support (P2)
  - Implemented in themes.css:406-420
  - Disables animations, gradients, and effect overlays
  - Files: `frontend/public/themes.css`

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

### M1: Core Layout (P0 tasks) ✅ COMPLETE
- [x] App shell with 4 panels (F1)
- [x] Icon rail navigation (F2)
- [x] Session list with status grouping (F6, F8)
- [x] Basic theme switching (F20, F21)
- [x] Media bar skeleton (F15)

### M2: Conversations & Voice (P0-P1) 🔄 IN PROGRESS
- [x] NPC conversation persistence (B1, B2)
- [x] Media bar layout (F15)
- [ ] NPC conversation view (F10)
- [ ] Play buttons on messages (F25)
- [ ] Voice queue integration (B12, F17)

### M3: Dynamic Themes (P1) 🔄 IN PROGRESS
- [x] All 5 theme definitions (F21)
- [x] Visual effects (F23)
- [~] Theme detection (F20)
- [ ] Theme interpolation (F22)
- [ ] Theme settings UI (F24)

### M4: Polish (P2-P3) ⏳ PENDING
- [ ] Command palette (F28)
- [ ] Keyboard shortcuts (F29)
- [ ] Responsive design (F32, F33)
- [~] Accessibility (F34, F35)
- [x] prefers-reduced-motion (F36)
- [ ] Knowledge graph (F31)
