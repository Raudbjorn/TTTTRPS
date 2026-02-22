# 18 — Dual MessageRole Definition

**Gap addressed:** #23 (type duality between core::llm and database::models)

## The Problem

Two `MessageRole` enums exist with overlapping but incompatible definitions.

### Definition 1: LLM Router (`core/llm/router/types.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}
```

Used by: LLM router, all provider implementations, request/response types.
Lives in `ChatMessage` struct (also has `images`, `name`, `tool_calls`, `tool_call_id`).

### Definition 2: Database (`database/models/chat.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    Error,
    System,
}
```

Used by: SQLite database layer, `ChatMessageRecord`.
Extra variant: **`Error`** — used for displaying error messages in chat UI.
Has `Display`, `as_str()`, and `TryFrom<&str>` impls.

## Divergence Points

| Aspect | LLM Router | Database |
|--------|-----------|----------|
| Variants | System, User, Assistant | User, Assistant, Error, System |
| Extra | — | `Error` (chat UI errors) |
| Derives | Clone | Clone, Copy |
| Storage | In-memory only | SQLite `role` column (raw String) |

## Migration Impact

`migrate_chat_messages` in `core/storage/migration.rs` copies the raw `role` string directly to SurrealDB without mapping through either enum. This means `"error"` role values propagate to SurrealDB if present.

## TUI Implication

The TUI chat view must handle both standard roles (system/user/assistant) and the `Error` variant for rendering error messages inline. When sending messages to the LLM router, `Error` role messages must be filtered out.

## Unification Status

No unification attempts found. The two types are used independently in their respective layers. A future task could create a shared enum with `Error` as a display-only extension, but this is not blocking for TUI implementation.
