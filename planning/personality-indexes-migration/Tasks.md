# Tasks: Personality Indexes Migration to meilisearch-lib

## Task Sequencing Strategy

This migration follows a **foundation-first** approach:
1. **Error Types**: Define error handling first
2. **Settings Functions**: Build index configuration helpers
3. **Core Functions**: Implement index management and CRUD
4. **Search Functions**: Implement search operations
5. **Tauri Integration**: Wire up to application state
6. **Testing**: Verify end-to-end

---

## Phase 1: Error Handling (~30 min)

### Epic 1.1: Error Types

- [ ] **1.1.1** Add `PersonalityIndexError` enum to `core/personality/meilisearch.rs`
  - Define variants: `Check`, `Create`, `Settings`, `Timeout`, `Stats`, `AddDocuments`, `GetDocument`, `DeleteDocument`, `Search`, `Clear`
  - Use `thiserror::Error` for derive
  - Implement `From<PersonalityIndexError> for String`
  - _Requirements: NFR-2.1_

---

## Phase 2: Settings Functions (~45 min)

### Epic 2.1: Index Configuration

- [ ] **2.1.1** Update index name constants
  - Verify `INDEX_PERSONALITY_TEMPLATES = "ttrpg_personality_templates"`
  - Verify `INDEX_BLEND_RULES = "ttrpg_blend_rules"`
  - Add `INDEX_TIMEOUT = Duration::from_secs(30)`
  - _Requirements: FR-1.1_

- [ ] **2.1.2** Update `personality_templates_settings()` function
  - Convert from `meilisearch_sdk::Settings` to `meilisearch_lib::Settings`
  - Use `BTreeSet` for filterable/sortable attributes
  - Searchable: name, description, vocabularyKeys, commonPhrases
  - Filterable: gameSystem, settingName, isBuiltin, tags, campaignId
  - Sortable: name, createdAt, updatedAt
  - _Requirements: FR-1.2_

- [ ] **2.1.3** Update `blend_rules_settings()` function
  - Convert from `meilisearch_sdk::Settings` to `meilisearch_lib::Settings`
  - Searchable: name, description
  - Filterable: context, enabled, isBuiltin, tags, campaignId
  - Sortable: name, priority, createdAt, updatedAt
  - _Requirements: FR-1.3_

---

## Phase 3: Core Manager Refactoring (~2 hours)

### Epic 3.1: Manager Structure

- [ ] **3.1.1** Update `PersonalityIndexManager` struct
  - Remove `client: Client`, `host: String`, `api_key: Option<String>`
  - Add `meili: Arc<MeilisearchLib>` field
  - Update constructor to accept `Arc<MeilisearchLib>`
  - Remove `host()` and `client()` methods
  - _Requirements: C-1, C-3_

### Epic 3.2: Index Initialization

- [ ] **3.2.1** Create `ensure_single_index()` helper function
  - Check if index exists with `meili.index_exists()`
  - Create index if missing with `meili.create_index()`
  - Apply settings with `meili.update_settings()`
  - Wait for tasks with `meili.wait_for_task()`
  - Return `Result<(), PersonalityIndexError>`
  - _Requirements: FR-1.1, FR-1.4_

- [ ] **3.2.2** Update `initialize_indexes()` function
  - Replace SDK calls with meilisearch-lib
  - Call `ensure_single_index()` for both indexes
  - Log success at info level
  - _Requirements: FR-1.1_

### Epic 3.3: Template CRUD Operations

- [ ] **3.3.1** Update `upsert_template()` function
  - Replace `index.add_documents()` with `meili.add_documents()`
  - Wait for task completion
  - Map errors to `PersonalityIndexError::AddDocuments`
  - _Requirements: FR-2.1_

- [ ] **3.3.2** Update `get_template()` function
  - Replace `index.get_document()` with `meili.get_document()`
  - Handle not-found case returning `None`
  - Map errors to `PersonalityIndexError::GetDocument`
  - _Requirements: FR-2.2_

- [ ] **3.3.3** Update `delete_template()` function
  - Replace `index.delete_document()` with `meili.delete_document()`
  - Wait for task completion
  - Map errors to `PersonalityIndexError::DeleteDocument`
  - _Requirements: FR-2.3_

### Epic 3.4: Blend Rule CRUD Operations

- [ ] **3.4.1** Update `upsert_blend_rule()` function
  - Replace SDK calls with meilisearch-lib
  - Map errors to appropriate variants
  - _Requirements: FR-3.1_

- [ ] **3.4.2** Update `get_blend_rule()` function
  - Replace SDK calls with meilisearch-lib
  - Handle not-found case
  - _Requirements: FR-3.2_

- [ ] **3.4.3** Update `delete_blend_rule()` function
  - Replace SDK calls with meilisearch-lib
  - Wait for task completion
  - _Requirements: FR-3.3_

---

## Phase 4: Search Operations (~1.5 hours)

### Epic 4.1: Template Search

- [ ] **4.1.1** Update `search_templates()` function
  - Create `SearchParams` from query, filter, limit
  - Call `meili.search::<TemplateDocument>()`
  - Extract documents from search results
  - Map errors to `PersonalityIndexError::Search`
  - _Requirements: FR-2.4_

- [ ] **4.1.2** Update `list_templates()` function
  - Delegate to `search_templates()` with empty query
  - _Requirements: FR-2.4_

- [ ] **4.1.3** Update `list_templates_by_game_system()` function
  - Build filter expression: `gameSystem = "{value}"`
  - Call `list_templates()` with filter
  - _Requirements: FR-2.4_

- [ ] **4.1.4** Update `list_templates_by_campaign()` function
  - Build filter expression: `campaignId = "{value}"`
  - _Requirements: FR-2.4_

- [ ] **4.1.5** Update `list_builtin_templates()` function
  - Use filter: `isBuiltin = true`
  - _Requirements: FR-2.4_

### Epic 4.2: Blend Rule Search

- [ ] **4.2.1** Update `search_blend_rules()` function
  - Create `SearchParams` from query, filter, limit
  - Call `meili.search::<BlendRuleDocument>()`
  - Map errors appropriately
  - _Requirements: FR-3.4_

- [ ] **4.2.2** Update `list_blend_rules()` function
  - Delegate to `search_blend_rules()` with empty query
  - _Requirements: FR-3.4_

- [ ] **4.2.3** Update `list_rules_by_context()` function
  - Build filter expression: `context = "{value}"`
  - _Requirements: FR-3.4_

- [ ] **4.2.4** Update `list_enabled_rules()` function
  - Use filter: `enabled = true`
  - Add sort: `priority:desc`
  - _Requirements: FR-3.5_

- [ ] **4.2.5** Update `list_rules_by_campaign()` function
  - Build filter expression: `campaignId = "{value}"`
  - _Requirements: FR-3.4_

---

## Phase 5: Statistics and Cleanup (~1 hour)

### Epic 5.1: Statistics

- [ ] **5.1.1** Create `get_document_count()` helper function
  - Check if index exists (return 0 if not)
  - Get stats with `meili.index_stats()`
  - Return document count
  - _Requirements: FR-4.2_

- [ ] **5.1.2** Update `get_stats()` function
  - Call `get_document_count()` for each index
  - Return `PersonalityIndexStats`
  - _Requirements: FR-4.1_

### Epic 5.2: Clear Operations

- [ ] **5.2.1** Update `clear_templates()` function
  - Replace `index.delete_all_documents()` with `meili.delete_all_documents()`
  - Wait for task completion
  - _Requirements: FR-4.3_

- [ ] **5.2.2** Update `clear_blend_rules()` function
  - Replace SDK calls with meilisearch-lib
  - Wait for task completion
  - _Requirements: FR-4.4_

- [ ] **5.2.3** Update `delete_indexes()` function
  - Replace `client.delete_index()` with `meili.delete_index()`
  - Continue on individual failures (best-effort)
  - Log warnings for failures
  - _Requirements: FR-4.5_

---

## Phase 6: Tauri Integration (~1 hour)

### Epic 6.1: Command Implementations

- [ ] **6.1.1** Create/Update personality Tauri commands
  - `initialize_personality_indexes` command
  - Get `embedded_search` from state
  - Clone Arc for spawn_blocking
  - Call `PersonalityIndexManager::initialize_indexes()`
  - _Requirements: FR-1.1_

- [ ] **6.1.2** Implement `get_personality_stats` command
  - Get `embedded_search` from state
  - Call `PersonalityIndexManager::get_stats()`
  - Return `PersonalityIndexStats`
  - _Requirements: FR-4.1_

- [ ] **6.1.3** Implement CRUD commands as needed
  - `create_personality_template`, `get_personality_template`, etc.
  - Follow existing command patterns
  - _Requirements: FR-2.*, FR-3.*_

---

## Phase 7: Testing (~2 hours)

### Epic 7.1: Unit Tests

- [ ] **7.1.1** Test settings functions
  - `personality_templates_settings()` returns correct fields
  - `blend_rules_settings()` returns correct fields
  - _Requirements: NFR-4.1_

- [ ] **7.1.2** Test error types
  - All variants format correctly
  - Include index name in message
  - Include document ID where applicable
  - _Requirements: NFR-2.1_

### Epic 7.2: Integration Tests

- [ ] **7.2.1** Test index initialization
  - Initialize creates both indexes
  - Re-initialization is idempotent
  - Settings are applied correctly
  - _Requirements: FR-1.1_

- [ ] **7.2.2** Test template CRUD
  - Upsert creates document
  - Get retrieves document
  - Get returns None for missing
  - Delete removes document
  - _Requirements: FR-2.1, FR-2.2, FR-2.3_

- [ ] **7.2.3** Test blend rule CRUD
  - Same as template CRUD
  - _Requirements: FR-3.1, FR-3.2, FR-3.3_

- [ ] **7.2.4** Test search operations
  - Empty query returns all
  - Filter restricts results
  - Sort orders correctly
  - _Requirements: FR-2.4, FR-3.4_

- [ ] **7.2.5** Test statistics
  - Empty indexes return 0
  - Counts reflect actual documents
  - Missing indexes return 0
  - _Requirements: FR-4.1, FR-4.2_

---

## Phase 8: Cleanup (~30 min)

### Epic 8.1: Code Quality

- [ ] **8.1.1** Remove meilisearch-sdk imports from meilisearch.rs
  - Remove `use meilisearch_sdk::*`
  - Add `use meilisearch_lib::MeilisearchLib`
  - Clean up unused imports
  - _Requirements: N/A_

- [ ] **8.1.2** Update module documentation
  - Update doc comments to reflect embedded lib usage
  - Add examples if helpful
  - Remove outdated notes
  - _Requirements: N/A_

- [ ] **8.1.3** Update CLAUDE.md migration status
  - Change Personality indexes status to check mark
  - Update any related documentation
  - _Requirements: N/A_

---

## Task Dependencies

```
Phase 1 (Errors)
    │
    └── 1.1.1 PersonalityIndexError
            │
Phase 2 (Settings)
    │       │
    ├── 2.1.1 Constants ◄─┘
    │       │
    ├── 2.1.2 personality_templates_settings
    │       │
    └── 2.1.3 blend_rules_settings
            │
Phase 3 (Core Manager)
    │       │
    ├── 3.1.1 Manager struct ◄──┘
    │       │
    ├── 3.2.1 ensure_single_index
    │       │
    ├── 3.2.2 initialize_indexes ◄──┤
    │       │                       │
    ├── 3.3.* Template CRUD ◄───────┤
    │       │                       │
    └── 3.4.* Blend Rule CRUD ◄─────┘
            │
Phase 4 (Search)
    │       │
    ├── 4.1.* Template search ◄─────┤
    │       │                       │
    └── 4.2.* Blend rule search ◄───┘
            │
Phase 5 (Stats/Cleanup)
    │       │
    ├── 5.1.* Statistics ◄──────────┤
    │       │                       │
    └── 5.2.* Clear operations ◄────┘
            │
Phase 6 (Tauri)
    │       │
    └── 6.1.* Commands ◄────────────┘
            │
Phase 7 (Testing)
    │
    ├── 7.1.* Unit tests
    │
    └── 7.2.* Integration tests
            │
Phase 8 (Cleanup)
    │
    └── 8.1.* Code quality
```

---

## Estimated Effort

| Phase | Epic | Tasks | Est. Hours | Complexity |
|-------|------|-------|------------|------------|
| 1 | 1.1 Error Types | 1 | 0.5 | Low |
| 2 | 2.1 Settings | 3 | 0.75 | Low |
| 3 | 3.1-3.4 Manager/CRUD | 9 | 2.0 | Medium |
| 4 | 4.1-4.2 Search | 10 | 1.5 | Medium |
| 5 | 5.1-5.2 Stats/Clear | 5 | 1.0 | Low |
| 6 | 6.1 Tauri Commands | 3 | 1.0 | Low |
| 7 | 7.1-7.2 Testing | 5 | 2.0 | Medium |
| 8 | 8.1 Cleanup | 3 | 0.5 | Low |

**Total Estimated**: ~9.25 hours

---

## Acceptance Criteria

| Requirement | Test Criteria |
|-------------|---------------|
| FR-1.1 | `initialize_indexes()` creates both indexes |
| FR-1.2 | Templates index has correct searchable/filterable/sortable |
| FR-1.3 | Blend rules index has correct attributes |
| FR-1.4 | Task completion waits up to 30 seconds |
| FR-2.1 | `upsert_template()` adds/updates document |
| FR-2.2 | `get_template()` returns document or None |
| FR-2.3 | `delete_template()` removes document |
| FR-2.4 | `search_templates()` returns matching results |
| FR-3.1-3.5 | Blend rule CRUD works correctly |
| FR-4.1 | `get_stats()` returns document counts |
| FR-4.2 | Missing indexes return 0 count |
| FR-4.3 | `clear_templates()` removes all documents |
| FR-4.4 | `clear_blend_rules()` removes all documents |
| FR-4.5 | `delete_indexes()` is best-effort |
| NFR-2.1 | Errors include index name and cause |
| NFR-3.1 | API signatures unchanged |
| NFR-4.1 | Settings match existing SDK configuration |

---

## Files to Modify

| File | Changes |
|------|---------|
| `src-tauri/src/core/personality/meilisearch.rs` | Replace SDK with lib, add error types, update all functions |
| `src-tauri/src/core/personality/mod.rs` | Update exports if needed |
| `src-tauri/src/commands/personality/` | Add or update Tauri commands |

## Files to Reference (Read-Only)

| File | Purpose |
|------|---------|
| `meili-dev/crates/meilisearch-lib/src/client.rs` | MeilisearchLib API |
| `meili-dev/crates/meilisearch-lib/src/indexes/settings.rs` | Settings API |
| `src-tauri/src/core/search/embedded.rs` | EmbeddedSearch wrapper |
| `src-tauri/src/commands/search/query.rs` | Example of migrated search commands |

---

## Risk Mitigation

### R-1: Search API Differences

**Risk**: meilisearch-lib search API may differ significantly from SDK.

**Mitigation**:
- Create adapter types if needed
- Reference `SearchParams` in existing migrated code
- Test with identical queries to SDK version

### R-2: Complex Filter Expressions

**Risk**: Filter expressions may need adjustment.

**Mitigation**:
- Keep `escape_filter_value()` function
- Test filter edge cases
- Verify boolean, string, array filters work

### R-3: Sorted Search

**Risk**: Sort parameter format may differ.

**Mitigation**:
- Verify `priority:desc` format works
- Check sort on multiple fields
- Test with real data
