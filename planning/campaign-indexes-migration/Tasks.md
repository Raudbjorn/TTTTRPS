# Tasks: Campaign Indexes Migration to meilisearch-lib

## Task Sequencing Strategy

This migration follows a **foundation-first** approach:
1. **Index Config Updates**: Update settings to meilisearch-lib types
2. **Client Refactoring**: Update constructor and core structure
3. **Core Operations**: Implement CRUD, search, batch
4. **Typed Operations**: Update arc, plan, plot point methods
5. **Health and Retry**: Adapt for embedded engine
6. **Testing**: Verify end-to-end

---

## Phase 1: Index Configuration (~45 min)

### Epic 1.1: Settings Type Updates

- [ ] **1.1.1** Update `IndexConfig` trait in `meilisearch_indexes.rs`
  - Change `build_settings()` return type from `meilisearch_sdk::Settings` to `meilisearch_lib::Settings`
  - Update import statements
  - _Requirements: FR-1.2, FR-1.3, FR-1.4_

- [ ] **1.1.2** Update `CampaignArcsIndexConfig::build_settings()`
  - Convert to `meilisearch_lib::Settings` struct initialization
  - Use `BTreeSet` for filterable/sortable attributes
  - Verify searchable: name, description, premise
  - Verify filterable: id, campaign_id, arc_type, status, is_main_arc
  - Verify sortable: name, display_order, started_at, created_at
  - _Requirements: FR-1.2_

- [ ] **1.1.3** Update `SessionPlansIndexConfig::build_settings()`
  - Convert to `meilisearch_lib::Settings`
  - Verify searchable: title, summary, dramatic_questions
  - Verify filterable: id, campaign_id, session_id, arc_id, phase_id, status, is_template
  - Verify sortable: title, session_number, created_at
  - _Requirements: FR-1.3_

- [ ] **1.1.4** Update `PlotPointsIndexConfig::build_settings()`
  - Convert to `meilisearch_lib::Settings`
  - Verify searchable: title, description, dramatic_question, notes
  - Verify filterable: id, campaign_id, arc_id, plot_type, activation_state, status, urgency, tension_level, involved_npcs, involved_locations, tags
  - Verify sortable: title, tension_level, urgency, created_at, activated_at
  - _Requirements: FR-1.4_

- [ ] **1.1.5** Update `get_index_configs()` function
  - Ensure `IndexInitConfig.settings` uses new Settings type
  - _Requirements: FR-1.1_

---

## Phase 2: Client Structure (~1 hour)

### Epic 2.1: Client Refactoring

- [ ] **2.1.1** Update `MeilisearchCampaignClient` struct
  - Remove `client: Client` field
  - Remove `host: String` field
  - Remove `api_key: Option<String>` field
  - Add `meili: Arc<MeilisearchLib>` field
  - Keep `_write_lock: Mutex<()>` field
  - _Requirements: C-1_

- [ ] **2.1.2** Update constructor
  - Change signature from `new(host, api_key) -> Result<Self>` to `new(meili: Arc<MeilisearchLib>) -> Self`
  - Remove HTTP client creation
  - Remove host/api_key storage
  - Initialize meili field
  - _Requirements: C-1_

- [ ] **2.1.3** Remove `host()` method
  - No longer needed for embedded engine
  - _Requirements: N/A_

---

## Phase 3: Index Management (~1 hour)

### Epic 3.1: Index Initialization

- [ ] **3.1.1** Update `ensure_index()` internal method
  - Replace `client.get_index()` with `meili.index_exists()`
  - Replace `client.create_index()` with `meili.create_index()`
  - Replace `index.set_settings()` with `meili.update_settings()`
  - Wait for tasks with `meili.wait_for_task()`
  - Use `TASK_TIMEOUT_SHORT_SECS` for timeouts
  - _Requirements: FR-1.1, FR-1.5_

- [ ] **3.1.2** Update `ensure_indexes()` method
  - Should still iterate through `get_index_configs()`
  - Log index names after completion
  - _Requirements: FR-1.1_

- [ ] **3.1.3** Update `delete_index()` method
  - Replace `client.delete_index()` with `meili.delete_index()`
  - Handle IndexNotFound gracefully
  - Wait for task completion
  - _Requirements: N/A (utility method)_

---

## Phase 4: Generic CRUD Operations (~1.5 hours)

### Epic 4.1: Document Operations

- [ ] **4.1.1** Update `upsert_document()` method
  - Replace `index.add_documents()` with `meili.add_documents()`
  - Wait for task completion
  - Wrap in `with_retry()`
  - _Requirements: FR-2.1_

- [ ] **4.1.2** Update `upsert_documents()` batch method
  - Process in chunks of `MEILISEARCH_BATCH_SIZE`
  - Replace SDK calls with meilisearch-lib
  - Use `TASK_TIMEOUT_LONG_SECS` for batch timeouts
  - Log document count on success
  - _Requirements: FR-2.2_

- [ ] **4.1.3** Update `get_document()` method
  - Replace `index.get_document()` with `meili.get_document()`
  - Handle not-found by checking error message
  - Return `Option<T>`
  - _Requirements: FR-2.3_

- [ ] **4.1.4** Update `delete_document()` method
  - Replace `index.delete_document()` with `meili.delete_document()`
  - Wait for task completion
  - Wrap in `with_retry()`
  - _Requirements: FR-2.4_

- [ ] **4.1.5** Update `delete_documents()` batch method
  - Replace SDK calls with meilisearch-lib
  - Wait for task completion
  - _Requirements: FR-2.5_

- [ ] **4.1.6** Update `delete_by_filter()` method
  - Search for matching documents
  - Delete in batches
  - Return count of deleted
  - _Requirements: FR-2.6_

---

## Phase 5: Search Operations (~1 hour)

### Epic 5.1: Search Methods

- [ ] **5.1.1** Update `search()` method
  - Create `SearchParams` struct from parameters
  - Call `meili.search::<T>()`
  - Extract documents from results
  - _Requirements: FR-3.1_

- [ ] **5.1.2** Update `list()` method
  - Delegate to `search()` with empty query
  - _Requirements: FR-3.2_

- [ ] **5.1.3** Update `count()` method
  - Search with limit 0
  - Return `estimated_total_hits` from results
  - _Requirements: FR-3.3_

---

## Phase 6: Typed Arc Operations (~30 min)

### Epic 6.1: Arc Methods

- [ ] **6.1.1** Update `get_arc()` method
  - Delegate to `get_document(INDEX_CAMPAIGN_ARCS, id)`
  - _Requirements: FR-4.1_

- [ ] **6.1.2** Update `list_arcs()` method
  - Build filter: `campaign_id = "{value}"`
  - Call `list()` with sort `created_at:desc`
  - _Requirements: FR-4.2_

- [ ] **6.1.3** Update `save_arc()` method
  - Delegate to `upsert_document(INDEX_CAMPAIGN_ARCS, arc)`
  - _Requirements: FR-4.3_

- [ ] **6.1.4** Update `delete_arc()` method
  - Delegate to `delete_document(INDEX_CAMPAIGN_ARCS, id)`
  - _Requirements: FR-4.4_

---

## Phase 7: Typed Plan Operations (~30 min)

### Epic 7.1: Plan Methods

- [ ] **7.1.1** Update `get_plan()` method
  - Delegate to `get_document(INDEX_SESSION_PLANS, id)`
  - _Requirements: FR-5.1_

- [ ] **7.1.2** Update `get_plan_for_session()` method
  - Build filter: `session_id = "{value}"`
  - Call `list()` with limit 1
  - Return first result or None
  - _Requirements: FR-5.2_

- [ ] **7.1.3** Update `list_plans()` method
  - Build filter based on `include_templates` flag
  - Sort by `session_number:desc`
  - _Requirements: FR-5.3_

- [ ] **7.1.4** Update `list_plan_templates()` method
  - Filter: `campaign_id AND is_template = true`
  - Sort by `title:asc`
  - _Requirements: FR-5.4_

- [ ] **7.1.5** Update `save_plan()` and `delete_plan()` methods
  - Delegate to generic CRUD methods
  - _Requirements: FR-5.5, FR-5.6_

---

## Phase 8: Typed Plot Point Operations (~30 min)

### Epic 8.1: Plot Point Methods

- [ ] **8.1.1** Update `get_plot_point()` method
  - Delegate to `get_document(INDEX_PLOT_POINTS, id)`
  - _Requirements: FR-6.1_

- [ ] **8.1.2** Update `list_plot_points()` method
  - Filter: `campaign_id = "{value}"`
  - Sort by `created_at:desc`
  - _Requirements: FR-6.2_

- [ ] **8.1.3** Update `list_plot_points_by_state()` method
  - Filter: `campaign_id AND activation_state`
  - Sort by `tension_level:desc, urgency:desc`
  - _Requirements: FR-6.3_

- [ ] **8.1.4** Update `list_plot_points_by_arc()` method
  - Filter: `arc_id = "{value}"`
  - Sort by `created_at:asc`
  - _Requirements: FR-6.4_

- [ ] **8.1.5** Update `save_plot_point()` and `delete_plot_point()` methods
  - Delegate to generic CRUD methods
  - _Requirements: FR-6.5, FR-6.6_

---

## Phase 9: Health and Retry (~30 min)

### Epic 9.1: Health Check Adaptation

- [ ] **9.1.1** Update `health_check()` method
  - For embedded engine, always return true
  - Remove HTTP request code
  - _Requirements: FR-7.1_

- [ ] **9.1.2** Update `wait_for_health()` method
  - For embedded engine, return true immediately
  - Remove polling loop
  - _Requirements: FR-7.2_

### Epic 9.2: Retry Logic

- [ ] **9.2.1** Update `with_retry()` method
  - Change from async to sync (embedded engine is sync)
  - Use `std::thread::sleep()` instead of `tokio::time::sleep()`
  - Keep exponential backoff logic
  - _Requirements: FR-7.3_

- [ ] **9.2.2** Verify `is_transient_error()` method
  - Should still identify ConnectionError and TaskTimeout
  - No changes expected
  - _Requirements: NFR-2.1_

---

## Phase 10: Integration (~1 hour)

### Epic 10.1: Call Site Updates

- [ ] **10.1.1** Find all usages of `MeilisearchCampaignClient::new()`
  - Search codebase for constructor calls
  - Document all locations
  - _Requirements: N/A_

- [ ] **10.1.2** Update each call site
  - Change from `new(host, api_key)` to `new(meili.clone())`
  - Get `Arc<MeilisearchLib>` from `state.embedded_search.inner()`
  - _Requirements: C-1_

- [ ] **10.1.3** Update Tauri commands using campaign client
  - `initialize_campaign_indexes`, etc.
  - Use `spawn_blocking` pattern if needed
  - _Requirements: N/A_

---

## Phase 11: Testing (~2 hours)

### Epic 11.1: Unit Tests

- [ ] **11.1.1** Test index configurations
  - All three `IndexConfig` implementations return correct attributes
  - `get_index_configs()` returns all three
  - _Requirements: NFR-5.1_

- [ ] **11.1.2** Test error types
  - `is_transient_error()` returns correct boolean
  - Error Display includes context
  - _Requirements: NFR-3.1_

- [ ] **11.1.3** Test filter escaping
  - Special characters escaped correctly
  - _Requirements: N/A_

### Epic 11.2: Integration Tests

- [ ] **11.2.1** Test index initialization
  - `ensure_indexes()` creates all three
  - Re-calling is idempotent
  - Settings applied correctly
  - _Requirements: FR-1.1_

- [ ] **11.2.2** Test CRUD operations
  - Upsert creates/updates
  - Get returns document or None
  - Delete removes document
  - _Requirements: FR-2.1-2.4_

- [ ] **11.2.3** Test batch operations
  - Large batches split correctly
  - All documents indexed
  - _Requirements: FR-2.2_

- [ ] **11.2.4** Test search operations
  - Query returns matches
  - Filter restricts results
  - Sort orders correctly
  - Pagination works
  - _Requirements: FR-3.1-3.3_

- [ ] **11.2.5** Test typed operations
  - Arc CRUD works
  - Plan CRUD works
  - Plot point CRUD works
  - List operations filter correctly
  - _Requirements: FR-4.*, FR-5.*, FR-6.*_

---

## Phase 12: Cleanup (~30 min)

### Epic 12.1: Code Quality

- [ ] **12.1.1** Remove meilisearch-sdk imports
  - Remove `use meilisearch_sdk::*` from both files
  - Add `use meilisearch_lib::*` imports
  - Clean up unused imports
  - _Requirements: N/A_

- [ ] **12.1.2** Remove reqwest dependency (if only used for health check)
  - Check if reqwest is used elsewhere
  - Remove from Cargo.toml if not needed
  - _Requirements: N/A_

- [ ] **12.1.3** Update module documentation
  - Update doc comments for embedded usage
  - Remove HTTP/server references
  - Update examples
  - _Requirements: N/A_

- [ ] **12.1.4** Update CLAUDE.md migration status
  - Change Campaign indexes status to check mark
  - _Requirements: N/A_

---

## Task Dependencies

```
Phase 1 (Index Config)
    │
    ├── 1.1.1 IndexConfig trait
    │       │
    ├── 1.1.2-4 Config implementations ◄─┘
    │       │
    └── 1.1.5 get_index_configs
            │
Phase 2 (Client Structure)
    │       │
    ├── 2.1.1 Update struct ◄────────────┘
    │       │
    ├── 2.1.2 Update constructor
    │       │
    └── 2.1.3 Remove host()
            │
Phase 3 (Index Management)
    │       │
    ├── 3.1.1 ensure_index ◄─────────────┘
    │       │
    ├── 3.1.2 ensure_indexes
    │       │
    └── 3.1.3 delete_index
            │
Phase 4 (Generic CRUD)
    │       │
    ├── 4.1.1-6 CRUD methods ◄───────────┘
            │
Phase 5 (Search)
    │       │
    └── 5.1.1-3 Search methods ◄─────────┘
            │
Phase 6-8 (Typed Operations)
    │       │
    ├── 6.1.* Arc methods ◄──────────────┤
    │       │                            │
    ├── 7.1.* Plan methods ◄─────────────┤
    │       │                            │
    └── 8.1.* Plot point methods ◄───────┘
            │
Phase 9 (Health/Retry)
    │       │
    ├── 9.1.* Health check ◄─────────────┘
    │       │
    └── 9.2.* Retry logic
            │
Phase 10 (Integration)
    │       │
    └── 10.1.* Call sites ◄──────────────┘
            │
Phase 11 (Testing)
    │
    ├── 11.1.* Unit tests
    │
    └── 11.2.* Integration tests
            │
Phase 12 (Cleanup)
    │
    └── 12.1.* Code quality
```

---

## Estimated Effort

| Phase | Epic | Tasks | Est. Hours | Complexity |
|-------|------|-------|------------|------------|
| 1 | 1.1 Index Config | 5 | 0.75 | Low |
| 2 | 2.1 Client Structure | 3 | 1.0 | Medium |
| 3 | 3.1 Index Management | 3 | 1.0 | Medium |
| 4 | 4.1 Generic CRUD | 6 | 1.5 | Medium |
| 5 | 5.1 Search | 3 | 1.0 | Medium |
| 6 | 6.1 Arc Operations | 4 | 0.5 | Low |
| 7 | 7.1 Plan Operations | 5 | 0.5 | Low |
| 8 | 8.1 Plot Operations | 5 | 0.5 | Low |
| 9 | 9.1-9.2 Health/Retry | 4 | 0.5 | Low |
| 10 | 10.1 Integration | 3 | 1.0 | Medium |
| 11 | 11.1-11.2 Testing | 10 | 2.0 | Medium |
| 12 | 12.1 Cleanup | 4 | 0.5 | Low |

**Total Estimated**: ~10.75 hours

---

## Acceptance Criteria

| Requirement | Test Criteria |
|-------------|---------------|
| FR-1.1 | `ensure_indexes()` creates all 3 indexes |
| FR-1.2-1.4 | Index settings match specifications |
| FR-1.5 | Task timeouts work correctly |
| FR-2.1-2.6 | All CRUD operations work |
| FR-3.1-3.3 | Search, list, count work |
| FR-4.1-4.4 | Arc operations work |
| FR-5.1-5.6 | Plan operations work |
| FR-6.1-6.6 | Plot point operations work |
| FR-7.1-7.3 | Health check and retry work |
| NFR-2.1 | Transient errors are retried |
| NFR-2.2 | Non-transient errors not retried |
| NFR-3.1 | Error messages include context |
| NFR-4.1 | API signatures unchanged (except constructor) |
| NFR-5.1 | Index settings match SDK version |

---

## Files to Modify

| File | Changes |
|------|---------|
| `src-tauri/src/core/campaign/meilisearch_client.rs` | Replace SDK with lib, update all methods |
| `src-tauri/src/core/campaign/meilisearch_indexes.rs` | Update Settings type in IndexConfig trait |
| `src-tauri/src/core/campaign/mod.rs` | Update exports if needed |
| Various command files | Update constructor calls |

## Files to Reference (Read-Only)

| File | Purpose |
|------|---------|
| `meili-dev/crates/meilisearch-lib/src/client.rs` | MeilisearchLib API |
| `meili-dev/crates/meilisearch-lib/src/indexes/settings.rs` | Settings API |
| `src-tauri/src/core/search/embedded.rs` | EmbeddedSearch wrapper |
| `src-tauri/src/commands/search/query.rs` | Example of migrated code |

---

## Risk Mitigation

### R-1: Async to Sync Transition

**Risk**: Campaign client uses async/await, but meilisearch-lib is sync.

**Mitigation**:
- Methods can become sync if all callers are updated
- Or wrap sync calls in `spawn_blocking`
- Test both approaches for performance

### R-2: Health Check Removal

**Risk**: Code may depend on health check behavior.

**Mitigation**:
- Keep method signature but return true always
- Document that embedded engine is always "healthy"
- Log if health check is called (for debugging)

### R-3: Filter Expression Compatibility

**Risk**: Filter expressions may need adjustment for meilisearch-lib.

**Mitigation**:
- Keep `escape_filter_value()` function
- Test all filter patterns used
- Verify AND/OR logic works correctly
