# Tasks: Archetype Indexes Migration to meilisearch-lib

## Task Sequencing Strategy

This migration follows a **foundation-first** approach:
1. **Error Types**: Adapt existing error handling
2. **Settings Functions**: Update to meilisearch-lib types
3. **Manager Refactoring**: Remove lifetime, use Arc
4. **Core Functions**: Implement index operations
5. **Testing**: Verify end-to-end

---

## Phase 1: Error Handling (~20 min)

### Epic 1.1: Error Type Updates

- [ ] **1.1.1** Review `ArchetypeError` in `core/archetype/error.rs`
  - Verify `Meilisearch(String)` variant exists
  - Ensure it can represent all operation failures
  - Consider adding more specific variants if needed
  - _Requirements: NFR-2.1_

---

## Phase 2: Settings Functions (~30 min)

### Epic 2.1: Index Configuration

- [ ] **2.1.1** Verify index name constants
  - `INDEX_ARCHETYPES = "ttrpg_archetypes"` unchanged
  - `INDEX_VOCABULARY_BANKS = "ttrpg_npc_vocabulary_banks"` unchanged
  - Add `TASK_TIMEOUT = Duration::from_secs(30)` if not present
  - _Requirements: FR-1.1_

- [ ] **2.1.2** Update `build_archetype_settings()` function
  - Change return type from `meilisearch_sdk::Settings` to `meilisearch_lib::Settings`
  - Use `BTreeSet` for filterable/sortable attributes
  - Keep same attribute lists
  - _Requirements: FR-1.2_

- [ ] **2.1.3** Update `build_vocabulary_bank_settings()` function
  - Change return type from `meilisearch_sdk::Settings` to `meilisearch_lib::Settings`
  - Use `BTreeSet` for filterable/sortable attributes
  - Keep same attribute lists
  - _Requirements: FR-1.3_

- [ ] **2.1.4** Update convenience functions
  - `get_archetype_settings()` returns new Settings type
  - `get_vocabulary_bank_settings()` returns new Settings type
  - _Requirements: NFR-3.1_

---

## Phase 3: Manager Refactoring (~1 hour)

### Epic 3.1: Structure Changes

- [ ] **3.1.1** Update `ArchetypeIndexManager` struct
  - Remove `<'a>` lifetime parameter
  - Remove `client: &'a Client` field
  - Add `meili: Arc<MeilisearchLib>` field
  - _Requirements: C-2, C-3_

- [ ] **3.1.2** Update constructor
  - Change from `new(client: &'a Client)` to `new(meili: Arc<MeilisearchLib>)`
  - Update all call sites in codebase
  - _Requirements: C-2_

---

## Phase 4: Core Functions (~1.5 hours)

### Epic 4.1: Index Initialization

- [ ] **4.1.1** Create internal `ensure_index()` helper
  - Check if index exists with `meili.index_exists()`
  - Create index if missing with `meili.create_index()`
  - Apply settings with `meili.update_settings()`
  - Wait for tasks with `meili.wait_for_task()`
  - Map errors to `ArchetypeError::Meilisearch`
  - _Requirements: FR-1.1, FR-1.4_

- [ ] **4.1.2** Update `ensure_indexes()` function
  - Replace SDK calls with meilisearch-lib
  - Call `ensure_index()` for both indexes
  - Log success at info level
  - _Requirements: FR-1.1_

### Epic 4.2: Index Existence Checks

- [ ] **4.2.1** Update internal `index_exists()` function
  - Replace `client.get_index()` with `meili.index_exists()`
  - Map errors to `ArchetypeError::Meilisearch`
  - _Requirements: FR-2.1, FR-2.2_

- [ ] **4.2.2** Verify `archetypes_index_exists()` works
  - Should delegate to `index_exists(INDEX_ARCHETYPES)`
  - _Requirements: FR-2.1_

- [ ] **4.2.3** Verify `vocabulary_banks_index_exists()` works
  - Should delegate to `index_exists(INDEX_VOCABULARY_BANKS)`
  - _Requirements: FR-2.2_

### Epic 4.3: Document Counts

- [ ] **4.3.1** Update internal `document_count()` function
  - Check existence first (return 0 if not exists)
  - Replace `index.get_stats()` with `meili.index_stats()`
  - Return `stats.number_of_documents`
  - _Requirements: FR-3.1, FR-3.2_

- [ ] **4.3.2** Verify `archetype_count()` works
  - Should delegate to `document_count(INDEX_ARCHETYPES)`
  - _Requirements: FR-3.1_

- [ ] **4.3.3** Verify `vocabulary_bank_count()` works
  - Should delegate to `document_count(INDEX_VOCABULARY_BANKS)`
  - _Requirements: FR-3.2_

### Epic 4.4: Index Deletion

- [ ] **4.4.1** Update `delete_indexes()` function
  - Replace `client.delete_index()` with `meili.delete_index()`
  - Handle IndexNotFound gracefully (check error message)
  - Wait for task completion
  - Log warnings for failures, continue with other indexes
  - _Requirements: FR-4.1_

---

## Phase 5: Integration Updates (~45 min)

### Epic 5.1: Update Call Sites

- [ ] **5.1.1** Find all usages of `ArchetypeIndexManager`
  - Search for `ArchetypeIndexManager::new`
  - Identify all files that create managers
  - _Requirements: N/A_

- [ ] **5.1.2** Update each call site
  - Change from `ArchetypeIndexManager::new(&client)` to `ArchetypeIndexManager::new(meili.clone())`
  - Ensure `Arc<MeilisearchLib>` is available at each site
  - _Requirements: C-2_

- [ ] **5.1.3** Update any Tauri commands using archetype indexes
  - Get `embedded_search` from state
  - Pass `Arc<MeilisearchLib>` to manager
  - _Requirements: N/A_

---

## Phase 6: Testing (~1.5 hours)

### Epic 6.1: Unit Tests

- [ ] **6.1.1** Test settings functions
  - `build_archetype_settings()` returns correct fields
  - `build_vocabulary_bank_settings()` returns correct fields
  - Uses correct collection types
  - _Requirements: NFR-4.1_

- [ ] **6.1.2** Test convenience functions
  - `archetype_index_name()` returns correct constant
  - `vocabulary_banks_index_name()` returns correct constant
  - `get_archetype_settings()` returns valid Settings
  - `get_vocabulary_bank_settings()` returns valid Settings
  - _Requirements: NFR-3.1_

### Epic 6.2: Integration Tests

- [ ] **6.2.1** Test index initialization
  - `ensure_indexes()` creates both indexes
  - Re-calling is idempotent (no error)
  - Settings are applied correctly
  - _Requirements: FR-1.1_

- [ ] **6.2.2** Test index existence
  - Returns true for existing index
  - Returns false for non-existing index
  - _Requirements: FR-2.1, FR-2.2_

- [ ] **6.2.3** Test document counts
  - Returns 0 for empty index
  - Returns 0 for non-existing index
  - Returns correct count after documents added
  - _Requirements: FR-3.1, FR-3.2_

- [ ] **6.2.4** Test index deletion
  - Deletes both indexes successfully
  - Handles non-existing indexes gracefully
  - _Requirements: FR-4.1_

---

## Phase 7: Cleanup (~30 min)

### Epic 7.1: Code Quality

- [ ] **7.1.1** Remove meilisearch-sdk imports
  - Remove `use meilisearch_sdk::*`
  - Add `use meilisearch_lib::*` imports
  - Clean up unused imports
  - _Requirements: N/A_

- [ ] **7.1.2** Update module documentation
  - Update doc comments for new API
  - Update examples in doc comments
  - Remove references to HTTP client
  - _Requirements: N/A_

- [ ] **7.1.3** Update CLAUDE.md migration status
  - Change Archetype indexes status to check mark
  - _Requirements: N/A_

---

## Task Dependencies

```
Phase 1 (Errors)
    │
    └── 1.1.1 Review ArchetypeError
            │
Phase 2 (Settings)
    │       │
    ├── 2.1.1 Constants ◄─┘
    │       │
    ├── 2.1.2 build_archetype_settings
    │       │
    ├── 2.1.3 build_vocabulary_bank_settings
    │       │
    └── 2.1.4 Convenience functions
            │
Phase 3 (Manager Refactoring)
    │       │
    ├── 3.1.1 Update struct ◄──┘
    │       │
    └── 3.1.2 Update constructor
            │
Phase 4 (Core Functions)
    │       │
    ├── 4.1.1 ensure_index helper ◄──┘
    │       │
    ├── 4.1.2 ensure_indexes ◄───────┤
    │       │                        │
    ├── 4.2.* Existence checks ◄─────┤
    │       │                        │
    ├── 4.3.* Document counts ◄──────┤
    │       │                        │
    └── 4.4.1 delete_indexes ◄───────┘
            │
Phase 5 (Integration)
    │       │
    ├── 5.1.1 Find usages ◄──────────┘
    │       │
    ├── 5.1.2 Update call sites
    │       │
    └── 5.1.3 Update Tauri commands
            │
Phase 6 (Testing)
    │       │
    ├── 6.1.* Unit tests ◄───────────┘
    │       │
    └── 6.2.* Integration tests
            │
Phase 7 (Cleanup)
    │
    └── 7.1.* Code quality
```

---

## Estimated Effort

| Phase | Epic | Tasks | Est. Hours | Complexity |
|-------|------|-------|------------|------------|
| 1 | 1.1 Error Types | 1 | 0.33 | Low |
| 2 | 2.1 Settings | 4 | 0.5 | Low |
| 3 | 3.1 Manager Refactoring | 2 | 1.0 | Medium |
| 4 | 4.1-4.4 Core Functions | 10 | 1.5 | Medium |
| 5 | 5.1 Integration | 3 | 0.75 | Low |
| 6 | 6.1-6.2 Testing | 8 | 1.5 | Medium |
| 7 | 7.1 Cleanup | 3 | 0.5 | Low |

**Total Estimated**: ~6.08 hours

---

## Acceptance Criteria

| Requirement | Test Criteria |
|-------------|---------------|
| FR-1.1 | `ensure_indexes()` creates both indexes |
| FR-1.2 | Archetypes index has correct attributes |
| FR-1.3 | Vocabulary banks index has correct attributes |
| FR-1.4 | Operations wait up to 30 seconds |
| FR-2.1 | `archetypes_index_exists()` returns correct boolean |
| FR-2.2 | `vocabulary_banks_index_exists()` returns correct boolean |
| FR-3.1 | `archetype_count()` returns document count or 0 |
| FR-3.2 | `vocabulary_bank_count()` returns document count or 0 |
| FR-4.1 | `delete_indexes()` deletes both, handles missing gracefully |
| NFR-2.1 | Errors include index name |
| NFR-2.2 | One index failure doesn't stop other |
| NFR-3.1 | Public API unchanged except constructor |
| NFR-4.1 | Settings match existing SDK configuration |

---

## Files to Modify

| File | Changes |
|------|---------|
| `src-tauri/src/core/archetype/meilisearch.rs` | Replace SDK with lib, remove lifetime, update all functions |
| `src-tauri/src/core/archetype/error.rs` | Verify error types sufficient |
| `src-tauri/src/core/archetype/mod.rs` | Update exports if needed |
| Various files using `ArchetypeIndexManager` | Update constructor calls |

## Files to Reference (Read-Only)

| File | Purpose |
|------|---------|
| `meili-dev/crates/meilisearch-lib/src/client.rs` | MeilisearchLib API |
| `meili-dev/crates/meilisearch-lib/src/indexes/settings.rs` | Settings API |
| `src-tauri/src/core/search/embedded.rs` | EmbeddedSearch wrapper |
| `src-tauri/src/commands/search/query.rs` | Example of migrated code |

---

## Risk Mitigation

### R-1: Lifetime Removal Breaking Change

**Risk**: Removing `<'a>` lifetime is a breaking change.

**Mitigation**:
- Update all call sites systematically
- Search for `ArchetypeIndexManager` before starting
- Test compilation after changes

### R-2: Settings Type Incompatibility

**Risk**: `meilisearch_sdk::Settings` methods differ from `meilisearch_lib::Settings`.

**Mitigation**:
- Use struct initialization instead of builder pattern
- Reference other migrated settings functions
- Test settings are applied correctly

### R-3: Error Type Mismatch

**Risk**: Need to detect IndexNotFound by error message.

**Mitigation**:
- Check if `meili.index_exists()` method exists
- If not, catch error and check message contains "not found"
- Document approach chosen
