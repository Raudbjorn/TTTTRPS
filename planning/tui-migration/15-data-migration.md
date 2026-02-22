# 15 — Data Migration

**Gap addressed:** #12 (MISSING — no segment)

## Overview

`core/storage/migration.rs` provides one-way migration from SQLite + Meilisearch to embedded SurrealDB (RocksDB-backed). Resumable, phase-gated, with progress callbacks.

## Migration Phases

```
NotStarted → BackingUp → MigratingSqlite → MigratingMeilisearch → Validating → Completed
                                                                                  ↓
                                                                                Failed
```

Phases are ordered and comparable — resumption skips completed phases via `<=` comparison.

## SQLite → SurrealDB Table Mapping

| SQLite Table | SurrealDB Table | Key Transformations |
|-------------|-----------------|---------------------|
| `campaigns` | `campaign` | `system` → `game_system`; `archived_at` → `status: "archived"`; setting/house_rules/world_state → `metadata` JSON |
| `npcs` | `npc` | `personality_json` parsed for description/personality/appearance/backstory; `role` → `tags[0]`; location_id/voice_profile_id → `metadata` |
| `sessions` | `session` | `title` or `"Session N"` → `name`; status mapped (active/in_progress→active, completed/ended→completed, planned→planned); campaign record link |
| `global_chat_sessions` + `chat_messages` | `chat_message` | session_id kept; campaign_id looked up; npc_id/sources/tokens/streaming → metadata |
| `documents` | `library_item` | `name` → `title` + slug; source_type/file_path ext → `file_type`; status mapped |

## Meilisearch → SurrealDB Chunk Mapping

| Meilisearch Index | SurrealDB `chunk.content_type` |
|------------------|-------------------------------|
| `ttrpg_rules` | `"rules"` |
| `ttrpg_fiction` | `"fiction"` |
| `session_notes` | `"session_notes"` |
| `homebrew` | `"homebrew"` |

Embeddings preserved if `len() == 768`, nulled otherwise (triggers re-embedding). Metadata fields (`section_path`, `chapter_title`, `section_title`) extracted.

## Key Types

**MigrationStatus:**
- `started_at`, `completed_at` (optional DateTime)
- `phase: MigrationPhase`
- `records_migrated: MigrationCounts`
- `errors: Vec<String>`
- `backup_path: Option<String>`

**MigrationCounts:** campaigns, npcs, sessions, chat_messages, library_items, chunks (all usize)

## Migration Strategy

- **Resumable**: progress stored in SurrealDB as `migration_status:current`
- **Non-fatal errors**: per-table errors collected, not aborting
- **Backup**: copies SQLite DB before migration (continues if DB missing)
- **Validation**: count comparison (expected vs actual per table)
- **Progress callback**: `on_progress: impl Fn(&MigrationStatus)`
- **Meilisearch phase**: placeholder — `meilisearch-lib` integration not yet wired in `run_migration`

## Shared Data Directory

Both old and new storage share `~/.local/share/ttrpg-assistant/`:
- Legacy SQLite: `ttrpg_assistant.db`
- New SurrealDB: `surrealdb/` (RocksDB)
- Preprocessing dictionaries: `ttrpg_corpus.txt`, `ttrpg_bigrams.txt`

## TUI Requirements

1. **Migration wizard** — phase indicator with progress bar
2. **Record counts** — before/after comparison per table
3. **Error log** — scrollable list of migration errors
4. **Validation report** — expected vs actual counts
5. **Backup status** — path and size of SQLite backup
