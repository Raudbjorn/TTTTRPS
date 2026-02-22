# 01 — Storage Architecture

**Gaps addressed:** #1 (storage wrong), #20 (engine module)

## Actual State

The storage stack is:

1. **Embedded Meilisearch (milli)** — inlined as `core/engine/` (or `core/search/`), 35+ files. No HTTP sidecar. Provides BM25 full-text search, faceted filtering, and index management directly in-process.

2. **SurrealDB with RocksDB backend** (`core/storage/`, 9 files) — the migration target. Provides:
   - HNSW vector indexes (768-dim, cosine distance)
   - BM25 full-text indexes via `ttrpg_analyzer`
   - Hybrid search (weighted fusion of vector + keyword)
   - Graph relations (`npc_relation` table)
   - Record links between campaigns, sessions, messages
   - Persistence at `~/.local/share/ttrpg-assistant/surrealdb/`

3. **SQLite via SQLx** (`database/`, legacy) — chat sessions, campaigns, game sessions, NPCs, settings, audit. Being migrated to SurrealDB incrementally. Both TUI and Tauri read/write the same SQLite database at `~/.local/share/ttrpg-assistant/ttrpg_assistant.db`.

**NOT in the codebase:** LanceDB, fastembed. These were proposed in the original document but never adopted. SurrealDB already provides vector storage + search.

## SurrealDB Schema

Key tables in `core/storage/schema.rs`:

| Table | Indexes | Purpose |
|-------|---------|---------|
| `chunk` | BM25 full-text + HNSW vector (768d) | Document chunks for RAG |
| `library_item` | — | Ingested documents metadata |
| `npc_relation` | — | Graph edges for NPC relationships |
| `campaign` | — | Campaign records with record links |
| `session` | — | Game session records |
| `chat_message` | — | Chat message persistence |

## Search Architecture

```
Query → [TypoCorrector] → [SynonymExpander] → ┬─ BM25 full-text (milli)
                                                ├─ HNSW vector (SurrealDB)
                                                └─ Fusion → Ranked results
```

Content-type-specific semantic ratios:
- `rules`: 0.7 (high semantic weight)
- `fiction`: 0.6
- `session_notes`: 0.5
- `homebrew`: 0.7

## TUI Integration

The TUI accesses storage through `Services`:
- `services.storage: SurrealStorage` (Clone) — `storage.db()` returns `&Surreal<Db>`
- `services.database: Database` (Clone) — SQLite pool

All storage operations are async. Views use internal `mpsc::UnboundedChannel` to bridge async loading with synchronous rendering.

## TTRPG Search Module (`core/ttrpg_search/`)

8 files providing domain-aware search:
- **Antonym mapping** — prevents search confusion (e.g., "fire resistance" vs "fire vulnerability")
- **Attribute filtering** — damage types, creature types, conditions, alignments, rarities, spell schools
- **Query expansion** — TTRPG grammar awareness
- **Query parser** — Structured query parsing
- **Result ranker** — Domain-specific scoring
- **TTRPG constants** — Game terminology database

## Meilisearch Chat (`core/meilisearch_chat/`)

LLM-powered search using Meilisearch's chat completions API:
- `client.rs` — HTTP client for chat completions endpoint
- `config.rs` — Provider/model configuration
- `prompts.rs` — System prompt templates
- `types.rs` — Request/response types

This is a RAG-search-chat hybrid — a key differentiator of the application. The TUI should expose this as an alternative search mode alongside standard hybrid search.
