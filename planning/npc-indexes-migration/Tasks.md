# Tasks: NPC Indexes Migration to meilisearch-lib

## Task Sequencing Strategy

This migration follows a **foundation-first** approach:
1. **Error Types**: Define error handling first
2. **Settings Functions**: Build index configuration helpers
3. **Core Functions**: Implement index management
4. **Tauri Commands**: Wire up to frontend
5. **Testing**: Verify end-to-end

---

## Phase 1: Error Handling (~30 min)

### Epic 1.1: Error Types

- [ ] **1.1.1** Add `NpcIndexError` enum to `core/npc_gen/indexes.rs`
  - Define variants: `Check`, `Create`, `Settings`, `Timeout`, `Stats`, `Clear`, `AddDocuments`
  - Use `thiserror::Error` for derive
  - Implement `From<NpcIndexError> for String`
  - _Requirements: NFR-2.1_

---

## Phase 2: Settings Functions (~1 hour)

### Epic 2.1: Index Configuration

- [ ] **2.1.1** Add index name constants
  - `VOCABULARY_INDEX = "ttrpg_vocabulary_banks"`
  - `NAME_COMPONENTS_INDEX = "ttrpg_name_components"`
  - `EXCLAMATION_INDEX = "ttrpg_exclamation_templates"`
  - `INDEX_TIMEOUT = Duration::from_secs(30)`
  - _Requirements: FR-1.1_

- [ ] **2.1.2** Create `vocabulary_settings()` function
  - Searchable: phrase, category, bank_id, tags
  - Filterable: culture, role, race, category, formality, bank_id, tags
  - Sortable: frequency
  - _Requirements: FR-1.2_

- [ ] **2.1.3** Create `name_components_settings()` function
  - Searchable: component, meaning, phonetic_tags
  - Filterable: culture, component_type, gender, phonetic_tags
  - Sortable: frequency
  - _Requirements: FR-1.3_

- [ ] **2.1.4** Create `exclamation_settings()` function
  - Searchable: template, emotion
  - Filterable: culture, intensity, emotion, religious
  - Sortable: frequency
  - _Requirements: FR-1.4_

---

## Phase 3: Core Functions (~2 hours)

### Epic 3.1: Index Initialization

- [ ] **3.1.1** Create `ensure_single_index()` helper function
  - Check if index exists with `meili.index_exists()`
  - Create index if missing with `meili.create_index()`
  - Apply settings with `meili.update_settings()`
  - Wait for tasks with `meili.wait_for_task()`
  - Return `Result<(), NpcIndexError>`
  - _Requirements: FR-1.1, FR-1.5_

- [ ] **3.1.2** Update `ensure_npc_indexes()` function
  - Replace meilisearch-sdk calls with meilisearch-lib
  - Accept `&MeilisearchLib` parameter
  - Call `ensure_single_index()` for all three indexes
  - Log success at info level
  - _Requirements: FR-1.1_

### Epic 3.2: Statistics

- [ ] **3.2.1** Create `get_document_count()` helper function
  - Check if index exists (return 0 if not)
  - Get stats with `meili.index_stats()`
  - Return document count
  - _Requirements: FR-2.2_

- [ ] **3.2.2** Update `get_npc_index_stats()` function
  - Replace meilisearch-sdk calls with meilisearch-lib
  - Accept `&MeilisearchLib` parameter
  - Call `get_document_count()` for each index
  - Return `NpcIndexStats` struct
  - _Requirements: FR-2.1_

### Epic 3.3: Clear Indexes

- [ ] **3.3.1** Create `clear_single_index()` helper function
  - Check if index exists (skip if not)
  - Delete all documents with `meili.delete_all_documents()`
  - Wait for task completion
  - Return `Result<(), NpcIndexError>`
  - _Requirements: FR-3.1_

- [ ] **3.3.2** Update `clear_npc_indexes()` function
  - Replace meilisearch-sdk calls with meilisearch-lib
  - Accept `&MeilisearchLib` parameter
  - Call `clear_single_index()` for all three indexes
  - Log success at info level
  - _Requirements: FR-3.1, FR-3.2_

---

## Phase 4: Tauri Commands (~1 hour)

### Epic 4.1: Command Implementations

- [ ] **4.1.1** Implement `initialize_npc_indexes` command
  - Remove TODO comments
  - Get `embedded_search` from state
  - Clone Arc for spawn_blocking
  - Call `ensure_npc_indexes()` in blocking task
  - Map errors to String
  - _Requirements: FR-1.1_

- [ ] **4.1.2** Implement `get_npc_indexes_stats` command
  - Remove TODO comments
  - Get `embedded_search` from state
  - Clone Arc for spawn_blocking
  - Call `get_npc_index_stats()` in blocking task
  - Return `NpcIndexStats`
  - _Requirements: FR-2.1_

- [ ] **4.1.3** Implement `clear_npc_indexes` command
  - Remove TODO comments
  - Get `embedded_search` from state
  - Clone Arc for spawn_blocking
  - Call `clear_npc_indexes()` in blocking task
  - Map errors to String
  - _Requirements: FR-3.1_

---

## Phase 5: Testing (~2 hours)

### Epic 5.1: Unit Tests

- [ ] **5.1.1** Test settings functions
  - `vocabulary_settings()` returns correct fields
  - `name_components_settings()` returns correct fields
  - `exclamation_settings()` returns correct fields
  - _Requirements: NFR-4.1_

- [ ] **5.1.2** Test error types
  - All variants format correctly
  - Include index name in message
  - Include source error in message
  - _Requirements: NFR-2.1_

### Epic 5.2: Integration Tests

- [ ] **5.2.1** Test index initialization
  - Initialize creates all three indexes
  - Re-initialization is idempotent
  - Settings are applied correctly
  - _Requirements: FR-1.1_

- [ ] **5.2.2** Test statistics
  - Empty indexes return 0
  - Counts reflect actual documents
  - Missing indexes return 0 (not error)
  - _Requirements: FR-2.1, FR-2.2_

- [ ] **5.2.3** Test clear operations
  - Clear removes all documents
  - Clear on missing index succeeds
  - Index structure preserved after clear
  - _Requirements: FR-3.1, FR-3.2_

---

## Phase 6: Cleanup (~30 min)

### Epic 6.1: Code Quality

- [ ] **6.1.1** Remove meilisearch-sdk imports from indexes.rs
  - Remove `use meilisearch_sdk::*`
  - Add `use meilisearch_lib::MeilisearchLib`
  - Clean up unused imports
  - _Requirements: N/A_

- [ ] **6.1.2** Update module documentation
  - Update doc comments to reflect embedded lib usage
  - Add examples if helpful
  - Remove outdated notes
  - _Requirements: N/A_

- [ ] **6.1.3** Update CLAUDE.md migration status
  - Change NPC indexes status to ✅
  - Update any related documentation
  - _Requirements: N/A_

---

## Task Dependencies

```
Phase 1 (Errors)
    │
    └── 1.1.1 NpcIndexError
            │
Phase 2 (Settings)
    │       │
    ├── 2.1.1 Constants ◄─┘
    │       │
    ├── 2.1.2 vocabulary_settings
    │       │
    ├── 2.1.3 name_components_settings
    │       │
    └── 2.1.4 exclamation_settings
            │
Phase 3 (Core Functions)
    │       │
    ├── 3.1.1 ensure_single_index ◄──┘
    │       │
    ├── 3.1.2 ensure_npc_indexes ◄───┤
    │       │                        │
    ├── 3.2.1 get_document_count     │
    │       │                        │
    ├── 3.2.2 get_npc_index_stats ◄──┤
    │       │                        │
    ├── 3.3.1 clear_single_index     │
    │       │                        │
    └── 3.3.2 clear_npc_indexes ◄────┘
            │
Phase 4 (Tauri Commands)
    │       │
    ├── 4.1.1 initialize_npc_indexes ◄─┤
    │       │                          │
    ├── 4.1.2 get_npc_indexes_stats ◄──┤
    │       │                          │
    └── 4.1.3 clear_npc_indexes ◄──────┘
            │
Phase 5 (Testing)
    │       │
    ├── 5.1.* Unit tests ◄─────────────┘
    │       │
    └── 5.2.* Integration tests
            │
Phase 6 (Cleanup)
    │
    └── 6.1.* Code quality
```

---

## Estimated Effort

| Phase | Epic | Tasks | Est. Hours | Complexity |
|-------|------|-------|------------|------------|
| 1 | 1.1 Error Types | 1 | 0.5 | Low |
| 2 | 2.1 Index Configuration | 4 | 1.0 | Low |
| 3 | 3.1 Index Initialization | 2 | 0.75 | Medium |
| 3 | 3.2 Statistics | 2 | 0.5 | Low |
| 3 | 3.3 Clear Indexes | 2 | 0.5 | Low |
| 4 | 4.1 Tauri Commands | 3 | 1.0 | Low |
| 5 | 5.1 Unit Tests | 2 | 0.5 | Low |
| 5 | 5.2 Integration Tests | 3 | 1.5 | Medium |
| 6 | 6.1 Cleanup | 3 | 0.5 | Low |

**Total Estimated**: ~6.75 hours

---

## Acceptance Criteria

| Requirement | Test Criteria |
|-------------|---------------|
| FR-1.1 | `initialize_npc_indexes()` creates all 3 indexes |
| FR-1.2 | Vocabulary index has correct searchable/filterable/sortable |
| FR-1.3 | Name components index has correct attributes |
| FR-1.4 | Exclamation index has correct attributes |
| FR-1.5 | Task completion waits up to 30 seconds |
| FR-2.1 | `get_npc_indexes_stats()` returns document counts |
| FR-2.2 | Missing indexes return 0 count, not error |
| FR-3.1 | `clear_npc_indexes()` removes all documents |
| FR-3.2 | All three indexes cleared successfully |
| NFR-1.1 | Index creation < 5s per index |
| NFR-2.1 | Errors include index name and cause |
| NFR-2.2 | One index failure doesn't crash others |
| NFR-3.1 | Command signatures unchanged |
| NFR-4.1 | Settings match existing SDK configuration |

---

## Files to Modify

| File | Changes |
|------|---------|
| `src-tauri/src/core/npc_gen/indexes.rs` | Replace SDK with lib, add error types, update all functions |
| `src-tauri/src/commands/npc/indexes.rs` | Implement 3 commands with spawn_blocking |
| `src-tauri/src/core/npc_gen/mod.rs` | Update exports if needed |

## Files to Reference (Read-Only)

| File | Purpose |
|------|---------|
| `meili-dev/crates/meilisearch-lib/src/client.rs` | MeilisearchLib API |
| `meili-dev/crates/meilisearch-lib/src/indexes/settings.rs` | Settings API |
| `src-tauri/src/core/search/embedded.rs` | EmbeddedSearch wrapper |
| `src-tauri/src/commands/search/query.rs` | Example of migrated search commands |

---

## Risk Mitigation

### R-1: Settings API Differences

**Risk**: meilisearch-lib settings API may differ from SDK.

**Mitigation**:
- Reference `milli::update::Settings` documentation
- Check `core/meilisearch_pipeline.rs` for working examples
- Settings use `BTreeSet` instead of `Vec` for some fields

### R-2: Task Completion Model

**Risk**: `wait_for_task()` may behave differently.

**Mitigation**:
- Use existing `wait_for_task(uid, Some(timeout))` pattern
- Add explicit timeout to avoid infinite waits
- Handle timeout errors gracefully

### R-3: Index Existence Check

**Risk**: `index_exists()` may not exist in meilisearch-lib.

**Mitigation**:
- Check if `index_exists()` is available
- Fallback: try `get_index()` and handle not-found error
- Document chosen approach

---

## Dependencies on Other Migrations

This migration is **independent** of:
- Chat workspace configuration (separate feature)
- Campaign indexes (separate data domain)
- Personality indexes (separate data domain)

Can be completed in parallel with other migrations.

---

## Future Enhancements (Out of Scope)

1. **Culture Aggregation**: `indexed_cultures` field in `NpcIndexStats` is TODO
2. **Hybrid Search for NPCs**: Could add semantic search for vocabulary
3. **Batch Document Loading**: Currently done outside indexes.rs
4. **Index Versioning**: Handle schema migrations
