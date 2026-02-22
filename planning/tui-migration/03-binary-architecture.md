# 03 — Binary Architecture

**Gap addressed:** #2 (WRONG workspace proposal)

## Actual Structure

The project is a **single crate with two binary targets**, NOT a workspace:

```toml
# Cargo.toml (root)
[[bin]]
name = "ttrpg-assistant"        # Tauri desktop app
path = "src/main.rs"

[[bin]]
name = "ttttrps"                # TUI terminal app
path = "src/tui_main.rs"

[lib]
name = "ttrpg_assistant"
path = "src/lib.rs"
```

Both binaries share all backend code via `ttrpg_assistant::core`, `::database`, `::ingestion`, `::oauth`.

### Why NOT a Workspace

1. **Zero code duplication** — both UIs call the same `core::llm::LLMRouter`, `core::storage::SurrealStorage`, `database::Database`, etc.
2. **No refactor needed** — the [[bin]] approach already works
3. **Shared data** — both apps read/write `~/.local/share/ttrpg-assistant/`
4. **Build efficiency** — single compilation unit, shared dependencies

### Binary Differences

| Aspect | `ttrpg-assistant` (Tauri) | `ttttrps` (TUI) |
|--------|---------------------------|-----------------|
| UI framework | Leptos (WASM) | ratatui + crossterm |
| IPC | `#[tauri::command]` | Direct function calls |
| Window | Webview | Terminal |
| Entry | `src/main.rs` | `src/tui_main.rs` |
| State | Tauri State<> | `AppState` struct |

### Build Commands

```bash
cargo run --bin ttttrps        # Run TUI
cargo run                       # Run Tauri app (default)
cargo build --release --bin ttttrps  # Release TUI only
```
