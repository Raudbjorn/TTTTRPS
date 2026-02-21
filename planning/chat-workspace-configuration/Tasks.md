# Tasks: Chat Workspace Configuration Migration

## Task Sequencing Strategy

This migration follows a **bottom-up implementation** approach:
1. **Helper Functions**: Build mapping utilities first
2. **Core Commands**: Implement the main commands
3. **Convenience Command**: Wire up the simple API
4. **Reindex Command**: Separate utility function
5. **Testing**: Verify end-to-end

---

## Phase 1: Helper Functions (~2 hours)

### Epic 1.1: Provider Mapping

- [ ] **1.1.1** Create `map_provider_to_chat_config()` function
  - Map all `ChatProviderConfig` variants to `meilisearch_lib::ChatConfig`
  - Handle native providers: OpenAI, Claude, Mistral, Azure
  - Handle proxy providers via VLlm with base_url
  - Handle Grok as OpenAI-compatible
  - _Requirements: FR-1.1, FR-1.2_

- [ ] **1.1.2** Create `parse_provider_string()` function
  - Parse string identifiers to `ChatProviderConfig`
  - Validate required fields per provider
  - Return descriptive errors for missing fields
  - _Requirements: FR-3.2_

### Epic 1.2: Prompt and Settings Mapping

- [ ] **1.2.1** Create `build_prompts()` function
  - Merge custom prompts with anti-filter defaults
  - Preserve system prompt if provided
  - Always include `search_q_param` and `search_index_uid_param` defaults
  - _Requirements: FR-1.3_

- [ ] **1.2.2** Create `build_ttrpg_index_configs()` function
  - Configure chunks index (semantic_ratio: 0.6)
  - Define Liquid template for document formatting
  - Set appropriate max_bytes and limits
  - _Requirements: FR-1.4_

- [ ] **1.2.3** Create `map_config_to_settings()` function
  - Map `meilisearch_lib::ChatConfig` to `ChatWorkspaceSettings`
  - Include `mask_api_key()` for security
  - Map `ChatSource` variants to `ChatLLMSource`
  - _Requirements: FR-2.1_

---

## Phase 2: Core Commands (~2 hours)

### Epic 2.1: Configure Workspace

- [ ] **2.1.1** Implement `configure_chat_workspace()` command
  - Remove stub implementation and error return
  - Get `embedded_search` from state
  - Call `map_provider_to_chat_config()`
  - Call `meili.set_chat_config(Some(config))`
  - Add info-level logging
  - _Requirements: FR-1.1_

### Epic 2.2: Get Settings

- [ ] **2.2.1** Implement `get_chat_workspace_settings()` command
  - Remove stub implementation
  - Call `meili.get_chat_config()`
  - Return `None` if no config
  - Map config to settings with masked API key
  - Add debug-level logging
  - _Requirements: FR-2.1, FR-2.2_

---

## Phase 3: Convenience Command (~1 hour)

### Epic 3.1: Simple Configuration API

- [ ] **3.1.1** Implement `configure_meilisearch_chat()` command
  - Remove stub implementation and error return
  - Call `parse_provider_string()` with inputs
  - Build custom prompts from `custom_system_prompt` if provided
  - Call `map_provider_to_chat_config()`
  - Call `meili.set_chat_config(Some(config))`
  - Add info-level logging
  - _Requirements: FR-3.1, FR-3.2_

---

## Phase 4: Reindex Command (~1 hour)

### Epic 4.1: Library Reindexing

- [ ] **4.1.1** Implement `reindex_library()` command
  - Remove stub implementation and error return
  - Handle single-index case: `meili.delete_all_documents(name)`
  - Handle all-indexes case: `meili.list_indexes()` then clear each
  - Wait for task completion with 60s timeout
  - Return user-friendly success message
  - Add info-level logging
  - _Requirements: FR-4.1, FR-4.2_

---

## Phase 5: Testing (~2 hours)

### Epic 5.1: Unit Tests

- [ ] **5.1.1** Test `parse_provider_string()`
  - Test all valid provider strings
  - Test missing API key errors
  - Test unknown provider errors
  - Test ollama with/without host
  - _Requirements: NFR-2.1_

- [ ] **5.1.2** Test `mask_api_key()`
  - Test empty key returns None
  - Test short key returns "****"
  - Test normal key shows first 4 and last 4
  - _Requirements: NFR-4.1_

- [ ] **5.1.3** Test `map_provider_to_chat_config()`
  - Test OpenAI mapping
  - Test Claude/Anthropic mapping
  - Test Ollama/VLlm mapping
  - Test proxy providers with base_url
  - _Requirements: FR-1.2_

### Epic 5.2: Integration Tests

- [ ] **5.2.1** Test configure and retrieve cycle
  - Initialize embedded search
  - Configure with test provider
  - Retrieve and verify settings
  - Verify API key is masked
  - _Requirements: FR-1.1, FR-2.1_

- [ ] **5.2.2** Test reindex_library
  - Create test index with documents
  - Call reindex_library for that index
  - Verify documents are deleted
  - _Requirements: FR-4.1_

---

## Phase 6: Cleanup (~30 min)

### Epic 6.1: Code Quality

- [ ] **6.1.1** Remove TODO comments from meilisearch.rs
  - Remove Phase 4 migration notes
  - Remove stub warnings
  - Update module documentation
  - _Requirements: N/A_

- [ ] **6.1.2** Update CLAUDE.md if needed
  - Document new command implementations
  - Update migration status table
  - _Requirements: N/A_

---

## Task Dependencies

```
Phase 1 (Helpers)
    │
    ├── 1.1.1 map_provider_to_chat_config
    │       │
    ├── 1.1.2 parse_provider_string ◄────────────┐
    │       │                                    │
    ├── 1.2.1 build_prompts                      │
    │       │                                    │
    ├── 1.2.2 build_ttrpg_index_configs          │
    │       │                                    │
    └── 1.2.3 map_config_to_settings             │
            │                                    │
Phase 2 (Core Commands)                          │
    │                                            │
    ├── 2.1.1 configure_chat_workspace ◄─────────┤
    │       │                                    │
    └── 2.2.1 get_chat_workspace_settings ◄──────┤
            │                                    │
Phase 3 (Convenience)                            │
    │                                            │
    └── 3.1.1 configure_meilisearch_chat ◄───────┘
            │
Phase 4 (Reindex)
    │
    └── 4.1.1 reindex_library (independent)
            │
Phase 5 (Testing)
    │
    ├── 5.1.* Unit tests (after Phase 1)
    │
    └── 5.2.* Integration tests (after Phase 4)
            │
Phase 6 (Cleanup)
    │
    └── 6.1.* Code quality (after Phase 5)
```

---

## Estimated Effort

| Phase | Epic | Tasks | Est. Hours | Complexity |
|-------|------|-------|------------|------------|
| 1 | 1.1 Provider Mapping | 2 | 1.0 | Medium |
| 1 | 1.2 Settings Mapping | 3 | 1.0 | Low |
| 2 | 2.1 Configure Workspace | 1 | 0.5 | Low |
| 2 | 2.2 Get Settings | 1 | 0.5 | Low |
| 3 | 3.1 Simple API | 1 | 0.5 | Low |
| 4 | 4.1 Reindex | 1 | 0.5 | Low |
| 5 | 5.1 Unit Tests | 3 | 1.0 | Low |
| 5 | 5.2 Integration Tests | 2 | 1.0 | Medium |
| 6 | 6.1 Cleanup | 2 | 0.5 | Low |

**Total Estimated**: ~6.5 hours

---

## Acceptance Criteria

| Requirement | Test Criteria |
|-------------|---------------|
| FR-1.1 | `configure_chat_workspace()` sets config without error |
| FR-1.2 | All 14 providers map correctly to ChatSource |
| FR-1.3 | Custom prompts merge with anti-filter defaults |
| FR-1.4 | Index configs include chunks with semantic_ratio 0.6 |
| FR-2.1 | `get_chat_workspace_settings()` returns masked settings |
| FR-2.2 | Returns `None` when no configuration exists |
| FR-3.1 | `configure_meilisearch_chat()` works with string provider |
| FR-3.2 | Returns descriptive error for unknown provider |
| FR-4.1 | `reindex_library(None)` clears all indexes |
| FR-4.2 | `reindex_library(Some(name))` clears specific index |
| NFR-1.1 | Configuration completes in < 100ms |
| NFR-2.1 | Error messages include operation, cause, and fix |
| NFR-3.1 | Command signatures unchanged |
| NFR-4.1 | API keys masked in returned settings |

---

## Risk Mitigation

### R-1: Proxy URL Availability

**Risk**: Proxy providers fail if no proxy URL configured.

**Mitigation**:
- Check `state.llm_proxy_url` before mapping proxy providers
- Return clear error if proxy URL missing
- Document proxy requirement in error message

### R-2: Type Mismatch Between Project and Library

**Risk**: `ChatProviderConfig` variants don't fully map to `ChatConfig` fields.

**Mitigation**:
- Create comprehensive mapping tests
- Handle edge cases (missing model, etc.) with defaults
- Log warnings for unmapped fields

### R-3: Index Configuration Changes

**Risk**: TTRPG index names may differ from defaults.

**Mitigation**:
- `build_ttrpg_index_configs()` returns minimal default config
- Per-document indexes are handled dynamically by meilisearch-lib
- Add flexibility for future index name changes

---

## Files to Modify

| File | Changes |
|------|---------|
| `src-tauri/src/commands/search/meilisearch.rs` | Implement all 4 stub commands, add helper functions |
| `src-tauri/src/commands/state.rs` | Add `llm_proxy_url: Option<String>` if not present |
| `src-tauri/src/tests/meilisearch_integration_tests.rs` | Add integration tests |

## Files to Reference (Read-Only)

| File | Purpose |
|------|---------|
| `meili-dev/crates/meilisearch-lib/src/chat/config.rs` | `ChatConfig`, `ChatSource` types |
| `meili-dev/crates/meilisearch-lib/src/client.rs` | `set_chat_config()`, `get_chat_config()` methods |
| `src-tauri/src/core/meilisearch_chat/config.rs` | `ChatProviderConfig`, `ChatWorkspaceSettings` types |
| `src-tauri/src/core/meilisearch_chat/prompts.rs` | Anti-filter prompt defaults |
