# Napkin — TTTTRPS

## Corrections
| Date | Source | What Went Wrong | What To Do Instead |
|------|--------|----------------|-------------------|
| 2026-02-22 | self | `cargo test --lib config:: tui::` fails — unexpected argument syntax | Use `cargo test --lib -- config` and `cargo test --lib -- tui` separately |
| 2026-02-22 | self | Added unused imports (Stylize, NotificationLevel) in app.rs test module | Only import what's actually used in each scope; check after writing |
| 2026-02-22 | self | Wrote `make_app_state()` test helper requiring async service init — impossible in sync test | Services need async init; use focused unit tests on pure logic instead |
| 2026-02-22 | self | Used `crate::database::models::chat::ChatMessageRecord` — models is private | Use re-exports: `crate::database::ChatMessageRecord` |
| 2026-02-22 | self | Used `crate::core::llm::router::types::ChatMessage` — types is private | Use re-exports: `crate::core::llm::router::ChatMessage` |
| 2026-02-22 | self | `*c` in `Char(c)` pattern — c is already owned char, not a reference | In match on `(*modifiers, *code)`, sub-patterns like `Char(c)` bind by value — don't double-deref |
| 2026-02-22 | self | pulldown-cmark 0.10: `Tag::BlockQuote(_)` with parameter | Unit variant — use `Tag::BlockQuote` and `TagEnd::BlockQuote` (no param) |
| 2026-02-22 | self | Added tui-textarea 0.7 — depends on ratatui 0.29, conflicts with our 0.30 | Version mismatch causes dual-crate type incompatibility; implement InputBuffer instead |
| 2026-02-22 | self | `let Some(ref session_id) = self.session_id` then `self.finalize_response()` — borrow conflict | Clone upfront: `let session_id = match self.session_id { Some(ref s) => s.clone(), None => return }` |

## User Preferences
- Forked from TTTRPS (Tauri+Leptos) → TTTTRPS is TUI-only (ratatui + crossterm)
- SurrealDB replacing SQLite + Meilisearch — migration in progress
- Prefers consolidating stub views inline rather than file-per-view when they're just placeholders
- Detailed implementation plans with dependency graphs; execute tasks in dependency order

## Patterns That Work
- Explore agent for researching Clone-ability, constructors, and Arc requirements across 7+ types
- Consolidating placeholder views inline in app.rs (split to files when they get real content)
- `#[allow(dead_code)]` on fields that will be used in future phases (event_tx)
- Parallel task execution for independent items (0.1 + 0.3 + 0.4 simultaneously)
- Custom panic hook: install BEFORE entering raw mode, chain to original hook
- Lightweight InputBuffer over tui-textarea — avoids ratatui version conflicts, ~80 LOC, sufficient for chat input
- Two-phase input dispatch: view.handle_input() returns bool consumed; unconsumed falls through to global
- Three-phase input with modal priority: palette (Option state, consumes all) → help → view → global
- Replacing `show_foo: bool` with `foo: Option<FooState>` — presence = open, state carries data
- Fire-and-forget DB persistence via tokio::spawn — keeps UI responsive during streaming
- pulldown-cmark + syntect for markdown rendering, reusing OnceLock resources from core::logging

## Patterns That Don't Work
- Creating separate view files when they're just `Block::bordered().title("Name")` stubs
- tui-textarea with ratatui 0.30 — version mismatch causes dual-crate type hell (Style, Block, Widget from different versions)

## Domain Notes
- TUI TTRPG assistant — no web UI, no Tauri commands
- Core engine shared with TTTRPS but UI layer is terminal-native
- SurrealDB embedded (RocksDB backend) — no external process needed
- 5 meilisearch_integration_tests have pre-existing LMDB "Stale file handle" failures — not our fault
- ratatui 0.29→0.30 and crossterm 0.28→0.29 upgrade was seamless, zero API breakage
- Services clonability: LLMRouter(Clone), SurrealStorage(Clone), Database(Clone), SessionManager(NOT Clone→Arc), PersonalityApplicationManager(NOT Clone→Arc), SynthesisQueue(NOT Clone→Arc)
- QueueEventEmitter trait in `core/voice/queue/events.rs` — implement for TUI event forwarding
- Config at `~/.config/ttttrps/config.toml` (XDG), data at `~/.local/share/ttrpg-assistant/`

## Domain Notes — Phase 2
- database module re-exports models at crate::database level (ChatMessageRecord, MessageRole, etc.)
- LLM router re-exports types at crate::core::llm::router level (ChatRequest, ChatMessage, ChatChunk)
- pulldown-cmark 0.10 API: BlockQuote is unit variant (no kind parameter)
- ChatOps trait in database::chat — get_or_create_active_chat_session, add_chat_message, update_chat_message
- LLMRouter::stream_chat() returns mpsc::Receiver<Result<ChatChunk>>; cancel_stream() stops active stream

## Domain Notes — Phase 3
- nucleo 0.5 re-exports `pattern::Atom` (not `Pattern` for single atoms) — `Atom::new(needle, CaseMatching, Normalization, AtomKind, escape_whitespace)`, `.indices(haystack, &mut matcher, &mut indices) -> Option<u16>`
- `Utf32Str::new(label, &mut buf)` requires a `&mut Vec<u32>` scratch buffer — reuse across iterations
- Atom.indices() does NOT clear the indices vec — caller must clear before each call (but we create a new Vec each iteration so no issue)
- `*code` deref is needed in match on `&KeyEvent` fields, but `Char(c)` binds `c` as owned char — don't deref `c` again
- InputBuffer extraction: keep data+cursor in widgets, chat-specific render as free fn in chat.rs
- `cmd_clear` / `cmd_new_session` needed `pub` visibility for dispatch from app.rs handle_action

## Session Log
- 2026-02-22: First napkin session. Repo had active meilisearch migration work (14 files).
- 2026-02-22: Completed Phase 0 + Phase 1 (20 tasks). 4 commits: meilisearch migration, build.sh, Phase 0 scaffold, Phase 1 Elm architecture. TUI now has event loop, 6 navigable views, help modal, command palette stub, notification system, status bar.
- 2026-02-22: Completed Phase 2 — Chat Interface (tasks 2.0–2.10). Full chat with LLM streaming, markdown rendering (syntect), message persistence, vim-like Normal/Insert modes, slash commands (/clear, /new, /help). 4 new files, 6 modified. cargo check 0 errors, 13/13 TUI tests pass.
- 2026-02-22: Completed Phase 3 — Command Palette with Fuzzy Search (tasks 3.0–3.5). InputBuffer extracted to widgets, nucleo-powered fuzzy matching, 10 commands, category grouping, match highlighting, keybinding hints. 2 new files (command_palette.rs 645L, input_buffer.rs 157L), 5 modified. 0 errors, 14 new tests (27 total TUI tests pass).
