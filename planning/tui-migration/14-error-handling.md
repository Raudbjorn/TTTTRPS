# 14 — Error Handling

**Gap addressed:** #11 (MISSING — no segment)

## Overview

All error types use `thiserror` with errors-as-values philosophy. No panics in library code. Several key error types provide recovery guidance methods.

## Primary Error Types

### LLMError (`core/llm/router/error.rs`)

| Variant | Fields |
|---------|--------|
| HttpError | reqwest::Error |
| ApiError | status: u16, message: String |
| AuthError | String |
| RateLimited | retry_after_secs: u64 |
| InvalidResponse | String |
| NotConfigured | String |
| EmbeddingNotSupported | String |
| StreamingNotSupported | String |
| BudgetExceeded | String |
| NoProvidersAvailable | — |
| Timeout | — |
| SerializationError | serde_json::Error |
| StreamCanceled | — |
| EmbeddingError | String |

### StorageError (`core/storage/error.rs`)

Database, Config, Init, Query, Migration, NotFound, Embedding, LlmError, Serialization, Extraction, Chunking, Index, Transaction, Permission, ResourceLimit, Io

Constructor helpers: `StorageError::database()`, `::config()`, `::init()`, `::query()`, `::migration()`, `::not_found()`, `::embedding()`, `::llm()`

### OAuth Error (`oauth/error.rs`) — `#[non_exhaustive]`

| Variant | Fields |
|---------|--------|
| Auth | AuthError |
| Api | status: u16, message, retry_after: Option\<Duration\> |
| Network | reqwest::Error |
| Json | serde_json::Error |
| Config | String |
| Storage | String |
| Io | std::io::Error |
| Url | url::ParseError |

**Recovery methods:**
- `requires_reauth()` → true for Auth errors needing full re-auth, or API 401
- `is_recoverable()` → true for Network, API 5xx/429, transient IO
- `is_rate_limit()` → API 429
- `is_auth_error()` → Auth variant or API 401/403
- `retry_after()` → Duration from API variant

### AuthError (`oauth/error.rs`) — `#[non_exhaustive]`

NotAuthenticated, TokenExpired, InvalidGrant, StateMismatch, PkceVerificationFailed, Cancelled, ProjectDiscovery(String), RefreshFailed(String)

`requires_reauth()` → true for all except Cancelled and ProjectDiscovery

### NPC Errors (`core/npc_gen/errors.rs`)

VocabularyError, NameGenerationError, DialectError, NpcExtensionError — all have `is_recoverable() -> bool`

### Other Error Enums

| Error Type | Location |
|-----------|----------|
| SessionError | `core/session_manager.rs` |
| CampaignError | `core/campaign_manager.rs` |
| CredentialError | `core/credentials.rs` |
| PreprocessError | `core/preprocess/error.rs` |
| SearchError | `core/search/error.rs` |
| EmbeddingError | `core/search/embeddings.rs` |
| HybridSearchError | `core/search/hybrid.rs` |
| RelationshipError | `core/campaign/relationships.rs` |
| TemplateError | `core/personality/errors.rs` |
| BlendError | `core/personality/errors.rs` |
| ConversationError | `core/campaign/conversation/types.rs` |
| GenerationError | `core/campaign/generation/orchestrator.rs` |
| ArchetypeError | `core/archetype/error.rs` |

## TUI Error Display Strategy

No centralized error notification system exists. The TUI should implement:

1. **Error notification bar** — transient messages at bottom of screen (auto-dismiss after 5s)
2. **Recovery action hints** — based on `is_recoverable()` / `requires_reauth()` results
3. **Error log view** — scrollable list of recent errors with timestamps and context
4. **Rate limit indicator** — show retry countdown when `is_rate_limit()` is true
5. **Auth status indicator** — provider-specific auth state in status bar
