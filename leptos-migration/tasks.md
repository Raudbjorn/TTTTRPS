# Leptos Migration Tasks

This document contains actionable tasks for migrating from Dioxus to Leptos, organized by phase.

## Document Information

| Field | Value |
|-------|-------|
| Version | 1.0.0 |
| Created | 2026-01-01 |
| Status | Draft |

---

## Task Status Legend

| Status | Description |
|--------|-------------|
| `[ ]` | Not Started |
| `[~]` | In Progress |
| `[x]` | Completed |
| `[!]` | Blocked |

---

## Phase 1: Project Setup

### TASK-M001: Setup Leptos Project Structure

**Status:** `[ ]`

**Description:**
Create new Leptos-based frontend alongside existing Dioxus frontend for incremental migration.

**Subtasks:**
- [ ] Create `frontend-leptos/` directory
- [ ] Initialize `Cargo.toml` with Leptos 0.7 dependencies
- [ ] Create `Trunk.toml` build configuration
- [ ] Setup `index.html` entry point
- [ ] Create basic `src/main.rs` with mount
- [ ] Configure Tailwind to include new directory
- [ ] Verify WASM builds correctly
- [ ] Test Trunk dev server works

**Files to Create:**
```
frontend-leptos/
├── Cargo.toml
├── Trunk.toml
├── index.html
├── src/
│   ├── main.rs
│   └── app.rs
└── public/
    └── (symlink to ../frontend/public/)
```

**Acceptance Criteria:**
- `trunk serve` runs without errors
- Basic "Hello World" renders in browser
- WASM bundle generates correctly

---

### TASK-M002: Copy and Adapt Bindings

**Status:** `[ ]`

**Description:**
Copy `bindings.rs` and adapt for Leptos usage. Types should remain identical.

**Subtasks:**
- [ ] Copy `frontend/src/bindings.rs` to `frontend-leptos/src/`
- [ ] Verify all types compile
- [ ] Update imports if needed
- [ ] Add `spawn_local` wrapper if needed
- [ ] Test one IPC call works (e.g., `get_app_version`)

**Files to Create/Modify:**
- `frontend-leptos/src/bindings.rs`

**Acceptance Criteria:**
- All type definitions compile
- At least one Tauri command callable
- Types remain identical to Dioxus version

---

### TASK-M003: Setup Routing

**Status:** `[ ]`

**Description:**
Configure Leptos Router with all 6 routes.

**Subtasks:**
- [ ] Add `leptos_router` dependency
- [ ] Create route enum matching current routes
- [ ] Setup `<Router>` in App component
- [ ] Create placeholder components for each route
- [ ] Verify navigation works between routes
- [ ] Add 404 fallback

**Files to Create/Modify:**
- `frontend-leptos/src/app.rs`
- `frontend-leptos/src/routes.rs`

**Routes to implement:**
```rust
#[derive(Routable)]
pub enum Route {
    #[route("/")]
    Chat,
    #[route("/settings")]
    Settings,
    #[route("/library")]
    Library,
    #[route("/campaigns")]
    Campaigns,
    #[route("/session/:campaign_id")]
    Session { campaign_id: String },
    #[route("/character")]
    CharacterCreator,
}
```

**Acceptance Criteria:**
- All 6 routes accessible
- URL changes update view
- Back/forward navigation works

---

### TASK-M004: Global Theme Context

**Status:** `[ ]`

**Description:**
Implement global theme signal and context provider.

**Subtasks:**
- [ ] Create theme signal in App
- [ ] Provide theme context
- [ ] Apply theme class to body element
- [ ] Verify theme CSS applies

**Files to Create/Modify:**
- `frontend-leptos/src/app.rs`
- `frontend-leptos/src/theme.rs`

**Acceptance Criteria:**
- Theme signal accessible in child components
- Theme class applied to DOM
- CSS theming works

---

## Phase 2: Design System Components

### TASK-M005: Port Button Component

**Status:** `[ ]`

**Description:**
Migrate Button component from Dioxus to Leptos.

**Subtasks:**
- [ ] Create `components/design_system/button.rs`
- [ ] Implement `ButtonVariant` enum
- [ ] Implement `Button` component
- [ ] Support all variants (Primary, Secondary, Danger, Ghost, Outline)
- [ ] Support disabled state
- [ ] Support onClick callback
- [ ] Test all variants render correctly

**Files to Create:**
- `frontend-leptos/src/components/design_system/mod.rs`
- `frontend-leptos/src/components/design_system/button.rs`

**Acceptance Criteria:**
- All 5 variants work
- Disabled state works
- Click events fire

---

### TASK-M006: Port Input Component

**Status:** `[ ]`

**Description:**
Migrate Input component with reactive value binding.

**Subtasks:**
- [ ] Create `input.rs`
- [ ] Implement controlled input with Signal
- [ ] Support placeholder
- [ ] Support disabled state
- [ ] Support on_input callback
- [ ] Test two-way binding works

**Files to Create:**
- `frontend-leptos/src/components/design_system/input.rs`

**Acceptance Criteria:**
- Value binding works
- Input events propagate
- Disabled state works

---

### TASK-M007: Port Card Components

**Status:** `[ ]`

**Description:**
Migrate Card, CardHeader, CardBody components.

**Subtasks:**
- [ ] Create `card.rs`
- [ ] Implement `Card` component
- [ ] Implement `CardHeader` component
- [ ] Implement `CardBody` component
- [ ] Test composition works

**Files to Create:**
- `frontend-leptos/src/components/design_system/card.rs`

**Acceptance Criteria:**
- Components compose correctly
- Styling applies

---

### TASK-M008: Port Badge Component

**Status:** `[ ]`

**Description:**
Migrate Badge component with variants.

**Subtasks:**
- [ ] Create `badge.rs`
- [ ] Implement `BadgeVariant` enum
- [ ] Implement `Badge` component
- [ ] Test all variants

**Files to Create:**
- `frontend-leptos/src/components/design_system/badge.rs`

**Acceptance Criteria:**
- All variants render correctly

---

### TASK-M009: Port Select Component

**Status:** `[ ]`

**Description:**
Migrate Select dropdown component.

**Subtasks:**
- [ ] Create `select.rs`
- [ ] Implement with options Vec
- [ ] Support value binding
- [ ] Support on_change callback
- [ ] Test selection works

**Files to Create:**
- `frontend-leptos/src/components/design_system/select.rs`

**Acceptance Criteria:**
- Options render
- Selection updates signal
- Change events fire

---

### TASK-M010: Port Modal Component

**Status:** `[ ]`

**Description:**
Migrate Modal dialog component.

**Subtasks:**
- [ ] Create `modal.rs`
- [ ] Implement open signal control
- [ ] Implement backdrop click close
- [ ] Implement close button
- [ ] Support title prop
- [ ] Support children content
- [ ] Test open/close cycle

**Files to Create:**
- `frontend-leptos/src/components/design_system/modal.rs`

**Acceptance Criteria:**
- Modal shows/hides based on signal
- Backdrop click closes
- Close button works
- Content renders

---

### TASK-M011: Port Loading Components

**Status:** `[ ]`

**Description:**
Migrate LoadingSpinner and TypingIndicator.

**Subtasks:**
- [ ] Create `loading.rs`
- [ ] Implement `LoadingSpinner`
- [ ] Implement `TypingIndicator`
- [ ] Verify animations work

**Files to Create:**
- `frontend-leptos/src/components/design_system/loading.rs`

**Acceptance Criteria:**
- Spinner animates
- Typing indicator animates

---

### TASK-M012: Port Markdown Component

**Status:** `[ ]`

**Description:**
Migrate Markdown renderer component.

**Subtasks:**
- [ ] Create `markdown.rs`
- [ ] Implement pulldown-cmark integration
- [ ] Use Memo for caching parsed HTML
- [ ] Set inner_html safely
- [ ] Test markdown renders

**Files to Create:**
- `frontend-leptos/src/components/design_system/markdown.rs`

**Acceptance Criteria:**
- Markdown parses correctly
- HTML renders in DOM
- Memo prevents unnecessary re-parsing

---

## Phase 3: Page Components

### TASK-M013: Port Chat Page

**Status:** `[ ]`

**Description:**
Migrate the main Chat page component (~369 LOC).

**Subtasks:**
- [ ] Create `components/chat.rs`
- [ ] Implement message list state
- [ ] Implement input state
- [ ] Implement loading state
- [ ] Port send message logic with `spawn_local`
- [ ] Implement message rendering
- [ ] Add voice output button
- [ ] Add token usage display
- [ ] Test full chat flow

**Files to Create:**
- `frontend-leptos/src/components/chat.rs`

**Dependencies:** TASK-M005, TASK-M006, TASK-M011, TASK-M012

**Acceptance Criteria:**
- Messages display
- Input works
- Send triggers Tauri command
- Response displays
- Voice button works

---

### TASK-M014: Port Settings Page

**Status:** `[ ]`

**Description:**
Migrate the Settings page (~981 LOC). This is the largest component.

**Subtasks:**
- [ ] Create `components/settings.rs`
- [ ] Port LLM provider section
- [ ] Port model selection with async loading
- [ ] Port voice provider section
- [ ] Port voice selection
- [ ] Port theme selector
- [ ] Port Meilisearch status section
- [ ] Port API key management
- [ ] Implement save functionality
- [ ] Test all settings save correctly

**Files to Create:**
- `frontend-leptos/src/components/settings.rs`

**Dependencies:** TASK-M005, TASK-M006, TASK-M009

**Acceptance Criteria:**
- All settings sections render
- Provider selection works
- Model lists load asynchronously
- Settings save to backend
- Theme switching works

---

### TASK-M015: Port Library Page

**Status:** `[ ]`

**Description:**
Migrate the Library/Document page (~457 LOC).

**Subtasks:**
- [ ] Create `components/library.rs`
- [ ] Port document ingestion UI
- [ ] Port file picker integration
- [ ] Port progress events
- [ ] Port search interface
- [ ] Port results display
- [ ] Test document ingestion flow

**Files to Create:**
- `frontend-leptos/src/components/library.rs`

**Dependencies:** TASK-M005, TASK-M006, TASK-M007, TASK-M011

**Acceptance Criteria:**
- File picker works
- Ingestion starts
- Progress displays
- Search works
- Results display

---

### TASK-M016: Port Campaigns Page

**Status:** `[ ]`

**Description:**
Migrate the Campaigns list page (~271 LOC).

**Subtasks:**
- [ ] Create `components/campaigns.rs`
- [ ] Port campaign list loading
- [ ] Port campaign cards
- [ ] Port filtering/status
- [ ] Port create campaign modal
- [ ] Port delete confirmation
- [ ] Implement navigation to session
- [ ] Test CRUD operations

**Files to Create:**
- `frontend-leptos/src/components/campaigns.rs`

**Dependencies:** TASK-M007, TASK-M008, TASK-M010

**Acceptance Criteria:**
- Campaigns list loads
- Cards display correctly
- Create works
- Delete works
- Navigation to session works

---

### TASK-M017: Port Session Page

**Status:** `[ ]`

**Description:**
Migrate the Session page with combat tracking (~452 LOC).

**Subtasks:**
- [ ] Create `components/session.rs`
- [ ] Port campaign data loading
- [ ] Port session management
- [ ] Port combat state display
- [ ] Port combatant list
- [ ] Port initiative order
- [ ] Port damage/heal/condition actions
- [ ] Port session history
- [ ] Test combat flow

**Files to Create:**
- `frontend-leptos/src/components/session.rs`

**Dependencies:** TASK-M007, TASK-M008, TASK-M011

**Acceptance Criteria:**
- Session loads with campaign data
- Combat initiates
- Combatants display
- Turn advancement works
- Damage/heal works
- Conditions apply

---

### TASK-M018: Port Character Creator Page

**Status:** `[ ]`

**Description:**
Migrate the Character Creator page (~284 LOC).

**Subtasks:**
- [ ] Create `components/character.rs`
- [ ] Port system selection
- [ ] Port generation options
- [ ] Port character preview
- [ ] Port generate action
- [ ] Test generation flow

**Files to Create:**
- `frontend-leptos/src/components/character.rs`

**Dependencies:** TASK-M005, TASK-M009, TASK-M012

**Acceptance Criteria:**
- System selection works
- Options configure correctly
- Generation triggers
- Preview displays result

---

### TASK-M019: Port Campaign Detail Components

**Status:** `[ ]`

**Description:**
Migrate campaign detail sub-components.

**Subtasks:**
- [ ] Create `components/campaign_details/mod.rs`
- [ ] Port `session_list.rs`
- [ ] Port `npc_list.rs`
- [ ] Port `personality_manager.rs`
- [ ] Test within campaigns/session context

**Files to Create:**
- `frontend-leptos/src/components/campaign_details/mod.rs`
- `frontend-leptos/src/components/campaign_details/session_list.rs`
- `frontend-leptos/src/components/campaign_details/npc_list.rs`
- `frontend-leptos/src/components/campaign_details/personality_manager.rs`

**Acceptance Criteria:**
- All sub-components render
- Data loads correctly
- Actions work

---

## Phase 4: Integration & Testing

### TASK-M020: Update Tauri Configuration

**Status:** `[ ]`

**Description:**
Update Tauri to use new Leptos frontend.

**Subtasks:**
- [ ] Update `tauri.conf.json` build commands
- [ ] Point `frontendDist` to new location
- [ ] Update dev server port if needed
- [ ] Test `cargo tauri dev` works
- [ ] Test `cargo tauri build` works

**Files to Modify:**
- `src-tauri/tauri.conf.json`

**Acceptance Criteria:**
- Dev mode works
- Production build works
- All IPC commands work

---

### TASK-M021: Integration Testing

**Status:** `[ ]`

**Description:**
Comprehensive testing of migrated application.

**Subtasks:**
- [ ] Test all 6 routes accessible
- [ ] Test chat flow end-to-end
- [ ] Test settings save/load
- [ ] Test document ingestion
- [ ] Test campaign CRUD
- [ ] Test session/combat flow
- [ ] Test character generation
- [ ] Test theme switching
- [ ] Test voice synthesis
- [ ] Verify no console errors

**Acceptance Criteria:**
- All features work as before migration
- No regressions
- No console errors

---

### TASK-M022: Remove Dioxus Frontend

**Status:** `[ ]`

**Description:**
Clean up old Dioxus frontend after migration complete.

**Subtasks:**
- [ ] Rename `frontend-leptos/` to `frontend/`
- [ ] Remove old `frontend/` (or archive)
- [ ] Update any remaining references
- [ ] Update CI/CD if applicable
- [ ] Final verification

**Files to Modify:**
- Directory structure
- `src-tauri/tauri.conf.json`
- Any build scripts

**Acceptance Criteria:**
- Clean project structure
- No Dioxus references remain
- Builds work

---

## Task Dependencies

```
TASK-M001 (Setup)
    └─► TASK-M002 (Bindings)
        └─► TASK-M003 (Routing)
            └─► TASK-M004 (Theme)

TASK-M004 ─┬─► TASK-M005 (Button)
           ├─► TASK-M006 (Input)
           ├─► TASK-M007 (Card)
           ├─► TASK-M008 (Badge)
           ├─► TASK-M009 (Select)
           ├─► TASK-M010 (Modal)
           ├─► TASK-M011 (Loading)
           └─► TASK-M012 (Markdown)

Design System ─┬─► TASK-M013 (Chat)
               ├─► TASK-M014 (Settings)
               ├─► TASK-M015 (Library)
               ├─► TASK-M016 (Campaigns)
               ├─► TASK-M017 (Session)
               ├─► TASK-M018 (Character)
               └─► TASK-M019 (Campaign Details)

Pages ─► TASK-M020 (Tauri Config)
     └─► TASK-M021 (Integration)
         └─► TASK-M022 (Cleanup)
```

---

## Effort Estimates

| Task | Complexity | Estimated Hours |
|------|------------|-----------------|
| TASK-M001 | Low | 2 |
| TASK-M002 | Low | 1 |
| TASK-M003 | Low | 2 |
| TASK-M004 | Low | 1 |
| TASK-M005 | Low | 1 |
| TASK-M006 | Low | 1 |
| TASK-M007 | Low | 0.5 |
| TASK-M008 | Low | 0.5 |
| TASK-M009 | Low | 1 |
| TASK-M010 | Medium | 1.5 |
| TASK-M011 | Low | 0.5 |
| TASK-M012 | Low | 1 |
| TASK-M013 | High | 6 |
| TASK-M014 | High | 10 |
| TASK-M015 | Medium | 5 |
| TASK-M016 | Medium | 4 |
| TASK-M017 | High | 6 |
| TASK-M018 | Medium | 3 |
| TASK-M019 | Medium | 3 |
| TASK-M020 | Low | 1 |
| TASK-M021 | Medium | 4 |
| TASK-M022 | Low | 1 |

**Total Estimated Hours: ~56 hours**

---

## Sprint Suggestions

### Sprint 1: Foundation (8 hours)
- TASK-M001: Project Setup
- TASK-M002: Bindings
- TASK-M003: Routing
- TASK-M004: Theme

### Sprint 2: Design System (8 hours)
- TASK-M005 through TASK-M012

### Sprint 3: Core Pages (21 hours)
- TASK-M013: Chat
- TASK-M014: Settings
- TASK-M015: Library

### Sprint 4: Remaining Pages (16 hours)
- TASK-M016: Campaigns
- TASK-M017: Session
- TASK-M018: Character
- TASK-M019: Campaign Details

### Sprint 5: Integration (6 hours)
- TASK-M020: Tauri Config
- TASK-M021: Integration Testing
- TASK-M022: Cleanup

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Component doesn't translate cleanly | Document in component-mapping.md, ask for help |
| IPC bindings break | Test bindings first (TASK-M002) |
| Build issues | Test Trunk early (TASK-M001) |
| Performance regression | Profile before and after |
| Styling breaks | Keep same CSS, test themes |

---

## Related Documents

- [overview.md](./overview.md) - Migration rationale
- [architecture.md](./architecture.md) - Technical architecture
- [component-mapping.md](./component-mapping.md) - Translation guide

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-01 | Initial task breakdown |
