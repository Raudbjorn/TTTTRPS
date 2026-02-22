# 02 — Implemented TUI Baseline

**Gap addressed:** #3 (STALE — document reads as greenfield proposal)

## Current State (Phase 6 complete)

The TUI is fully functional with 4721 LOC across 13 files in `src/tui/`. This is NOT a greenfield project — substantial infrastructure is already working.

### Completed Components

| Component | Files | LOC | Status |
|-----------|-------|-----|--------|
| Elm architecture event loop | `app.rs` | 666 | Working |
| Chat with LLM streaming + markdown | `views/chat.rs`, `widgets/markdown.rs` | 1245 | Working |
| Command palette (nucleo fuzzy) | `views/command_palette.rs` | 666 | Working |
| Settings view (async LLM status) | `views/settings.rs` | 694 | Working |
| Library view (SurrealDB docs) | `views/library.rs` | 462 | Working |
| Campaign/session management | `views/campaign.rs` | 586 | Working |
| Input buffer widget | `widgets/input_buffer.rs` | 157 | Working |
| Event system | `events.rs` | 129 | Working |
| Service injection | `services.rs` | 104 | Working |
| 43 unit tests | across all modules | — | Passing |

### Architecture

```
Terminal Input → EventStream → AppState.handle_event()
                                   |
                 ┌─────────────────┴─────────────────┐
                 |         Three-phase input          |
                 |  1. Command palette (modal)        |
                 |  2. Help modal                     |
                 |  3. Focused view → global fallback |
                 └────────────────────────────────────┘
                                   |
                           AppState.render()
                                   |
                 ┌────────────────┬┴┬───────────────┐
                 │    Tab bar     │ │  Status bar   │
                 ├────────────────┤ ├───────────────┤
                 │ Focused view:  │ │ Notifications │
                 │ Chat/Library/  │ │ (overlay)     │
                 │ Campaign/      │ │               │
                 │ Settings/...   │ │               │
                 └────────────────┴─┴───────────────┘
```

### Navigation

6 views: Chat (1), Library (2), Campaign (3), Settings (4), Generation (5), Personality (6).
Generation and Personality remain placeholder stubs — all others are functional with async data loading.

### Patterns Established

- **Async data loading:** Internal `mpsc::UnboundedChannel` per view — `load()` spawns tokio task, `poll()` does `try_recv()` on tick
- **Input handling:** View returns `bool` (consumed) or enum (action to dispatch)
- **Command palette:** 13 commands, nucleo fuzzy matching, match highlighting, keybinding hints
- **Chat:** Vi-mode (Normal/Insert), LLM streaming, markdown rendering (pulldown-cmark + syntect), message persistence
- **Session switching:** Campaign view selects session → `ChatState::switch_to_session()` loads messages

### What Remains (not yet implemented)

| View | Status | Needs |
|------|--------|-------|
| Generation | Placeholder | Campaign generation wizard, template selection |
| Personality | Placeholder | Profile editor, blend configuration, context rules |
| NPC browsing | Not started | Separate from Campaign — needs own view or sub-view |
| Character sheet | Not started | System-specific character display |
| Location browser | Not started | Location list + connection map |
| Relationship graph | Not started | ASCII graph or tree view |
| Voice controls | Not started | Provider config, queue monitor, profile manager |
| Search interface | Not started | Hybrid search with TTRPG filters |
