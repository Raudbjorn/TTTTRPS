# Design: NPC Indexes Migration to meilisearch-lib

## Overview

This document describes the technical design for migrating NPC index operations from `meilisearch-sdk` HTTP client to embedded `meilisearch-lib` API calls.

---

## 1. Architecture

### 1.1 Current Architecture (meilisearch-sdk)

```
┌─────────────────────────────────────────────────────────────────┐
│                      Tauri Commands                              │
│  commands/npc/indexes.rs (TODO - disabled)                      │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                  core/npc_gen/indexes.rs                        │
│  Uses: meilisearch_sdk::Client                                  │
│        meilisearch_sdk::Settings                                │
│        meilisearch_sdk::Index                                   │
└──────────────────────────┬──────────────────────────────────────┘
                           │ HTTP
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│               External Meilisearch Process                      │
│               (via SidecarManager - REMOVED)                    │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Target Architecture (meilisearch-lib)

```
┌─────────────────────────────────────────────────────────────────┐
│                      Tauri Commands                              │
│  commands/npc/indexes.rs                                        │
│  - initialize_npc_indexes()                                      │
│  - get_npc_indexes_stats()                                       │
│  - clear_npc_indexes()                                           │
└──────────────────────────┬──────────────────────────────────────┘
                           │ state.embedded_search.inner()
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                  core/npc_gen/indexes.rs                        │
│  Uses: meilisearch_lib::MeilisearchLib                          │
│        meilisearch_lib::Settings                                │
│        meilisearch_lib::SearchQuery                             │
└──────────────────────────┬──────────────────────────────────────┘
                           │ Direct calls
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│               Embedded MeilisearchLib                            │
│               (In-process, LMDB storage)                        │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Component Design

### 2.1 Index Configuration Constants

**Location**: `src-tauri/src/core/npc_gen/indexes.rs`

```rust
/// NPC Index Names
pub const VOCABULARY_INDEX: &str = "ttrpg_vocabulary_banks";
pub const NAME_COMPONENTS_INDEX: &str = "ttrpg_name_components";
pub const EXCLAMATION_INDEX: &str = "ttrpg_exclamation_templates";

/// Timeout for index operations
const INDEX_TIMEOUT: Duration = Duration::from_secs(30);
```

### 2.2 Index Settings Functions

**Location**: `src-tauri/src/core/npc_gen/indexes.rs`

```rust
use meilisearch_lib::MeilisearchLib;
use milli::update::Settings;
use std::collections::BTreeSet;

/// Build settings for vocabulary phrases index
fn vocabulary_settings() -> Settings<milli::update::Unchecked> {
    let mut settings = Settings::default();

    // Searchable attributes (order matters for ranking)
    settings.set_searchable_fields(vec![
        "phrase".into(),
        "category".into(),
        "bank_id".into(),
        "tags".into(),
    ]);

    // Filterable attributes for faceted search
    settings.set_filterable_fields(BTreeSet::from([
        "culture".into(),
        "role".into(),
        "race".into(),
        "category".into(),
        "formality".into(),
        "bank_id".into(),
        "tags".into(),
    ]));

    // Sortable for frequency-based selection
    settings.set_sortable_fields(BTreeSet::from([
        "frequency".into(),
    ]));

    settings
}

/// Build settings for name components index
fn name_components_settings() -> Settings<milli::update::Unchecked> {
    let mut settings = Settings::default();

    settings.set_searchable_fields(vec![
        "component".into(),
        "meaning".into(),
        "phonetic_tags".into(),
    ]);

    settings.set_filterable_fields(BTreeSet::from([
        "culture".into(),
        "component_type".into(),
        "gender".into(),
        "phonetic_tags".into(),
    ]));

    settings.set_sortable_fields(BTreeSet::from([
        "frequency".into(),
    ]));

    settings
}

/// Build settings for exclamation templates index
fn exclamation_settings() -> Settings<milli::update::Unchecked> {
    let mut settings = Settings::default();

    settings.set_searchable_fields(vec![
        "template".into(),
        "emotion".into(),
    ]);

    settings.set_filterable_fields(BTreeSet::from([
        "culture".into(),
        "intensity".into(),
        "emotion".into(),
        "religious".into(),
    ]));

    settings.set_sortable_fields(BTreeSet::from([
        "frequency".into(),
    ]));

    settings
}
```

### 2.3 Index Initialization Function

**Location**: `src-tauri/src/core/npc_gen/indexes.rs`

```rust
/// Ensure all NPC indexes exist with proper settings
///
/// This function is idempotent - safe to call multiple times.
pub fn ensure_npc_indexes(meili: &MeilisearchLib) -> Result<(), NpcIndexError> {
    // Create vocabulary index
    ensure_single_index(
        meili,
        VOCABULARY_INDEX,
        "id",
        vocabulary_settings(),
    )?;

    // Create name components index
    ensure_single_index(
        meili,
        NAME_COMPONENTS_INDEX,
        "id",
        name_components_settings(),
    )?;

    // Create exclamation templates index
    ensure_single_index(
        meili,
        EXCLAMATION_INDEX,
        "id",
        exclamation_settings(),
    )?;

    log::info!("NPC indexes initialized successfully");
    Ok(())
}

/// Create or update a single index with settings
fn ensure_single_index(
    meili: &MeilisearchLib,
    index_uid: &str,
    primary_key: &str,
    settings: Settings<milli::update::Unchecked>,
) -> Result<(), NpcIndexError> {
    // Check if index exists
    if !meili.index_exists(index_uid)
        .map_err(|e| NpcIndexError::Check { index: index_uid.into(), source: e.to_string() })?
    {
        // Create index
        let task = meili.create_index(index_uid.to_string(), Some(primary_key.into()))
            .map_err(|e| NpcIndexError::Create { index: index_uid.into(), source: e.to_string() })?;

        meili.wait_for_task(task.uid, Some(INDEX_TIMEOUT))
            .map_err(|e| NpcIndexError::Timeout { index: index_uid.into(), source: e.to_string() })?;

        log::info!("Created NPC index: {}", index_uid);
    }

    // Apply settings
    let task = meili.update_settings(index_uid, settings)
        .map_err(|e| NpcIndexError::Settings { index: index_uid.into(), source: e.to_string() })?;

    meili.wait_for_task(task.uid, Some(INDEX_TIMEOUT))
        .map_err(|e| NpcIndexError::Timeout { index: index_uid.into(), source: e.to_string() })?;

    log::debug!("Updated settings for index: {}", index_uid);
    Ok(())
}
```

### 2.4 Statistics Function

**Location**: `src-tauri/src/core/npc_gen/indexes.rs`

```rust
/// Get statistics for all NPC indexes
pub fn get_npc_index_stats(meili: &MeilisearchLib) -> Result<NpcIndexStats, NpcIndexError> {
    let vocabulary_count = get_document_count(meili, VOCABULARY_INDEX)?;
    let name_count = get_document_count(meili, NAME_COMPONENTS_INDEX)?;
    let exclamation_count = get_document_count(meili, EXCLAMATION_INDEX)?;

    Ok(NpcIndexStats {
        vocabulary_phrase_count: vocabulary_count,
        name_component_count: name_count,
        exclamation_template_count: exclamation_count,
        indexed_cultures: Vec::new(), // TODO: Implement culture aggregation
    })
}

/// Get document count for a single index
fn get_document_count(meili: &MeilisearchLib, index_uid: &str) -> Result<u64, NpcIndexError> {
    if !meili.index_exists(index_uid)
        .map_err(|e| NpcIndexError::Check { index: index_uid.into(), source: e.to_string() })?
    {
        return Ok(0);
    }

    let stats = meili.index_stats(index_uid)
        .map_err(|e| NpcIndexError::Stats { index: index_uid.into(), source: e.to_string() })?;

    Ok(stats.number_of_documents)
}
```

### 2.5 Clear Indexes Function

**Location**: `src-tauri/src/core/npc_gen/indexes.rs`

```rust
/// Clear all documents from NPC indexes
///
/// Indexes are preserved, only documents are deleted.
pub fn clear_npc_indexes(meili: &MeilisearchLib) -> Result<(), NpcIndexError> {
    clear_single_index(meili, VOCABULARY_INDEX)?;
    clear_single_index(meili, NAME_COMPONENTS_INDEX)?;
    clear_single_index(meili, EXCLAMATION_INDEX)?;

    log::info!("Cleared all NPC indexes");
    Ok(())
}

/// Clear documents from a single index
fn clear_single_index(meili: &MeilisearchLib, index_uid: &str) -> Result<(), NpcIndexError> {
    if !meili.index_exists(index_uid)
        .map_err(|e| NpcIndexError::Check { index: index_uid.into(), source: e.to_string() })?
    {
        log::debug!("Index {} does not exist, skipping clear", index_uid);
        return Ok(());
    }

    let task = meili.delete_all_documents(index_uid)
        .map_err(|e| NpcIndexError::Clear { index: index_uid.into(), source: e.to_string() })?;

    meili.wait_for_task(task.uid, Some(INDEX_TIMEOUT))
        .map_err(|e| NpcIndexError::Timeout { index: index_uid.into(), source: e.to_string() })?;

    log::debug!("Cleared index: {}", index_uid);
    Ok(())
}
```

### 2.6 Error Types

**Location**: `src-tauri/src/core/npc_gen/indexes.rs`

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NpcIndexError {
    #[error("Failed to check index existence for '{index}': {source}")]
    Check { index: String, source: String },

    #[error("Failed to create index '{index}': {source}")]
    Create { index: String, source: String },

    #[error("Failed to apply settings to index '{index}': {source}")]
    Settings { index: String, source: String },

    #[error("Operation on index '{index}' timed out: {source}")]
    Timeout { index: String, source: String },

    #[error("Failed to get stats for index '{index}': {source}")]
    Stats { index: String, source: String },

    #[error("Failed to clear index '{index}': {source}")]
    Clear { index: String, source: String },

    #[error("Failed to add documents to index '{index}': {source}")]
    AddDocuments { index: String, source: String },
}

impl From<NpcIndexError> for String {
    fn from(e: NpcIndexError) -> String {
        e.to_string()
    }
}
```

---

## 3. Tauri Commands

### 3.1 Initialize NPC Indexes

**Location**: `src-tauri/src/commands/npc/indexes.rs`

```rust
use tauri::State;
use crate::commands::AppState;
use crate::core::npc_gen::{ensure_npc_indexes, NpcIndexError};

/// Initialize NPC indexes with proper settings
///
/// Creates indexes if they don't exist, updates settings if they do.
/// Safe to call multiple times (idempotent).
#[tauri::command]
pub async fn initialize_npc_indexes(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let meili = state.embedded_search.clone_inner();

    // Run in blocking task since MeilisearchLib is sync
    tokio::task::spawn_blocking(move || {
        ensure_npc_indexes(&meili)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e: NpcIndexError| e.to_string())?;

    Ok(())
}
```

### 3.2 Get NPC Index Stats

**Location**: `src-tauri/src/commands/npc/indexes.rs`

```rust
use crate::core::npc_gen::{get_npc_index_stats, NpcIndexStats};

/// Get statistics for all NPC indexes
///
/// Returns document counts for each index.
#[tauri::command]
pub async fn get_npc_indexes_stats(
    state: State<'_, AppState>,
) -> Result<NpcIndexStats, String> {
    let meili = state.embedded_search.clone_inner();

    tokio::task::spawn_blocking(move || {
        get_npc_index_stats(&meili)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e: NpcIndexError| e.to_string())
}
```

### 3.3 Clear NPC Indexes

**Location**: `src-tauri/src/commands/npc/indexes.rs`

```rust
use crate::core::npc_gen::clear_npc_indexes as clear_indexes;

/// Clear all documents from NPC indexes
///
/// Preserves index structure, only removes documents.
#[tauri::command]
pub async fn clear_npc_indexes(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let meili = state.embedded_search.clone_inner();

    tokio::task::spawn_blocking(move || {
        clear_indexes(&meili)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e: NpcIndexError| e.to_string())?;

    Ok(())
}
```

---

## 4. API Mapping

### 4.1 meilisearch-sdk → meilisearch-lib

| meilisearch-sdk | meilisearch-lib | Notes |
|-----------------|-----------------|-------|
| `Client::new(url, key)` | N/A | No client needed, use `MeilisearchLib` directly |
| `Client::get_index(name)` | N/A | Use methods with `uid` parameter |
| `Client::create_index(name, pk)` | `meili.create_index(name, pk)` | Returns `TaskView` |
| `Index::set_settings(settings)` | `meili.update_settings(uid, settings)` | Use `milli::Settings` |
| `Index::get_stats()` | `meili.index_stats(uid)` | Returns `IndexStats` |
| `Index::delete_all_documents()` | `meili.delete_all_documents(uid)` | Returns `TaskView` |
| `Task::wait_for_completion()` | `meili.wait_for_task(uid, timeout)` | Blocks until complete |

### 4.2 Settings Type Mapping

| meilisearch-sdk | meilisearch-lib |
|-----------------|-----------------|
| `Settings::new()` | `Settings::default()` |
| `.with_searchable_attributes(vec)` | `.set_searchable_fields(vec)` |
| `.with_filterable_attributes(vec)` | `.set_filterable_fields(BTreeSet)` |
| `.with_sortable_attributes(vec)` | `.set_sortable_fields(BTreeSet)` |

---

## 5. Data Flow

### 5.1 Index Initialization Flow

```
Frontend: Settings Panel "Initialize NPC Data"
                    │
                    ▼
Tauri: initialize_npc_indexes()
                    │
                    ▼
spawn_blocking(ensure_npc_indexes(&meili))
                    │
    ┌───────────────┼───────────────┐
    │               │               │
    ▼               ▼               ▼
Vocabulary      Names        Exclamations
    │               │               │
    └───────┬───────┴───────┬───────┘
            │               │
            ▼               ▼
    index_exists?     create_index
            │               │
            ▼               ▼
    update_settings   wait_for_task
            │               │
            └───────┬───────┘
                    │
                    ▼
            Return Ok(())
```

### 5.2 Statistics Flow

```
Frontend: NPC Panel "View Stats"
                    │
                    ▼
Tauri: get_npc_indexes_stats()
                    │
                    ▼
spawn_blocking(get_npc_index_stats(&meili))
                    │
    ┌───────────────┼───────────────┐
    │               │               │
    ▼               ▼               ▼
Vocabulary      Names        Exclamations
    │               │               │
    ▼               ▼               ▼
index_stats()  index_stats()  index_stats()
    │               │               │
    └───────┬───────┴───────┬───────┘
            │               │
            ▼               ▼
    NpcIndexStats { vocab: N, names: M, excl: K }
```

---

## 6. Thread Safety

### 6.1 Async Context Handling

`MeilisearchLib` is synchronous, but Tauri commands are async. Solution:

```rust
// Clone Arc for move into spawn_blocking
let meili = state.embedded_search.clone_inner();

// Run sync code in blocking task pool
tokio::task::spawn_blocking(move || {
    // Sync operations here
    ensure_npc_indexes(&meili)
})
.await
```

### 6.2 Concurrent Access

- `MeilisearchLib` is wrapped in `Arc<MeilisearchLib>`
- Internal LMDB handles concurrent reads
- Write operations are serialized by task queue
- No additional locking needed

---

## 7. Error Handling

### 7.1 Error Categories

| Category | Example | User Message |
|----------|---------|--------------|
| Check | `index_exists()` fails | "Failed to check index existence for 'X': ..." |
| Create | `create_index()` fails | "Failed to create index 'X': ..." |
| Settings | `update_settings()` fails | "Failed to apply settings to index 'X': ..." |
| Timeout | `wait_for_task()` exceeds 30s | "Operation on index 'X' timed out: ..." |
| Stats | `index_stats()` fails | "Failed to get stats for index 'X': ..." |
| Clear | `delete_all_documents()` fails | "Failed to clear index 'X': ..." |

### 7.2 Error Propagation

```rust
// In core/npc_gen/indexes.rs
fn ensure_single_index(...) -> Result<(), NpcIndexError> {
    meili.create_index(...)
        .map_err(|e| NpcIndexError::Create {
            index: index_uid.into(),
            source: e.to_string()
        })?;
    ...
}

// In commands/npc/indexes.rs
#[tauri::command]
pub async fn initialize_npc_indexes(...) -> Result<(), String> {
    ...
    .map_err(|e: NpcIndexError| e.to_string())
}
```

---

## 8. Testing Strategy

### 8.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vocabulary_settings() {
        let settings = vocabulary_settings();
        // Verify searchable fields
        // Verify filterable fields
        // Verify sortable fields
    }

    #[test]
    fn test_npc_index_error_display() {
        let err = NpcIndexError::Create {
            index: "test".into(),
            source: "connection failed".into(),
        };
        assert!(err.to_string().contains("test"));
        assert!(err.to_string().contains("connection failed"));
    }
}
```

### 8.2 Integration Tests

```rust
#[tokio::test]
async fn test_npc_index_lifecycle() {
    let meili = create_test_meilisearch();

    // Initialize
    ensure_npc_indexes(&meili).expect("init failed");

    // Get stats (should be 0)
    let stats = get_npc_index_stats(&meili).expect("stats failed");
    assert_eq!(stats.vocabulary_phrase_count, 0);

    // Add document
    let doc = serde_json::json!({
        "id": "test_1",
        "phrase": "Hello, traveler!",
        "bank_id": "tavern",
        "category": "greeting",
        "formality": "casual",
        "frequency": 0.8
    });
    meili.add_documents(VOCABULARY_INDEX, vec![doc], Some("id".into()))
        .expect("add failed");

    // Get stats (should be 1)
    let stats = get_npc_index_stats(&meili).expect("stats failed");
    assert_eq!(stats.vocabulary_phrase_count, 1);

    // Clear
    clear_npc_indexes(&meili).expect("clear failed");

    // Get stats (should be 0)
    let stats = get_npc_index_stats(&meili).expect("stats failed");
    assert_eq!(stats.vocabulary_phrase_count, 0);
}
```

---

## 9. Decisions Log

### Decision 1: Keep Functions in core/npc_gen/indexes.rs

**Context**: Could move all logic into Tauri commands directly.

**Decision**: Keep domain logic in `core/npc_gen/indexes.rs`, use thin command wrappers.

**Rationale**:
- Separation of concerns
- Testable without Tauri runtime
- Reusable from other modules (e.g., vocabulary loading)

### Decision 2: Use spawn_blocking for Sync Operations

**Context**: `MeilisearchLib` is synchronous, Tauri commands are async.

**Decision**: Use `tokio::task::spawn_blocking()` to wrap sync calls.

**Rationale**:
- Avoids blocking the Tokio runtime
- Standard pattern in the codebase
- Alternative (async wrapper in meilisearch-lib) would require upstream changes

### Decision 3: 30-Second Timeout for All Operations

**Context**: Need consistent timeout behavior.

**Decision**: Use 30-second timeout for all task waiting operations.

**Rationale**:
- Matches existing SDK implementation
- Long enough for index creation with many documents
- Short enough to surface stuck operations

### Decision 4: Preserve Index Structure on Clear

**Context**: `clear_npc_indexes` could delete indexes entirely or just documents.

**Decision**: Only delete documents, preserve index structure.

**Rationale**:
- Faster rebuild (no need to recreate indexes/settings)
- Consistent with user expectation of "clear" vs "delete"
- Matches existing SDK implementation behavior
