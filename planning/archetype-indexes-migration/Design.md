# Design: Archetype Indexes Migration to meilisearch-lib

## Architecture Overview

### Current Architecture (SDK-based)

```
┌─────────────────────────────────────────────────────────────────┐
│                    ArchetypeIndexManager<'a>                     │
│                                                                  │
│  ┌──────────────┐    ┌──────────────────────────────────────┐   │
│  │   client     │───▶│      &'a meilisearch_sdk::Client     │   │
│  │   (&'a)      │    │   (Reference to HTTP client)         │   │
│  └──────────────┘    └──────────────────────────────────────┘   │
│                                      │                           │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   Index Operations                        │   │
│  │  ┌─────────────────┐   ┌─────────────────────────────┐   │   │
│  │  │  Archetypes     │   │  Vocabulary Banks           │   │   │
│  │  │  - ensure       │   │  - ensure                   │   │   │
│  │  │  - exists       │   │  - exists                   │   │   │
│  │  │  - count        │   │  - count                    │   │   │
│  │  │  - delete       │   │  - delete                   │   │   │
│  │  └─────────────────┘   └─────────────────────────────┘   │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ HTTP
                    ┌─────────────────────┐
                    │  External Meilisearch│
                    └─────────────────────┘
```

### Target Architecture (Embedded)

```
┌─────────────────────────────────────────────────────────────────┐
│                    ArchetypeIndexManager                         │
│                                                                  │
│  ┌──────────────┐    ┌──────────────────────────────────────┐   │
│  │    meili     │───▶│       Arc<MeilisearchLib>            │   │
│  │   (Arc)      │    │   (In-process embedded engine)       │   │
│  └──────────────┘    └──────────────────────────────────────┘   │
│                                      │                           │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   Index Operations                        │   │
│  │  ┌─────────────────┐   ┌─────────────────────────────┐   │   │
│  │  │  Archetypes     │   │  Vocabulary Banks           │   │   │
│  │  │  - ensure       │   │  - ensure                   │   │   │
│  │  │  - exists       │   │  - exists                   │   │   │
│  │  │  - count        │   │  - count                    │   │   │
│  │  │  - delete       │   │  - delete                   │   │   │
│  │  └─────────────────┘   └─────────────────────────────┘   │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ Direct function calls
                    ┌─────────────────────┐
                    │  Embedded milli     │
                    │  (In-process)       │
                    └─────────────────────┘
```

---

## Component Design

### ArchetypeIndexError Enum

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArchetypeIndexError {
    #[error("Failed to check index '{index}': {message}")]
    Check { index: String, message: String },

    #[error("Failed to create index '{index}': {message}")]
    Create { index: String, message: String },

    #[error("Failed to update settings for '{index}': {message}")]
    Settings { index: String, message: String },

    #[error("Operation on '{index}' timed out after {timeout_secs}s")]
    Timeout { index: String, timeout_secs: u64 },

    #[error("Failed to get stats for '{index}': {message}")]
    Stats { index: String, message: String },

    #[error("Failed to delete index '{index}': {message}")]
    Delete { index: String, message: String },
}

impl From<ArchetypeIndexError> for String {
    fn from(e: ArchetypeIndexError) -> Self {
        e.to_string()
    }
}
```

### Index Constants

```rust
/// Index name for character archetypes.
pub const INDEX_ARCHETYPES: &str = "ttrpg_archetypes";

/// Index name for NPC vocabulary banks.
pub const INDEX_VOCABULARY_BANKS: &str = "ttrpg_npc_vocabulary_banks";

/// Default timeout for index operations (30 seconds).
const TASK_TIMEOUT: Duration = Duration::from_secs(30);
```

---

## Settings Functions

### build_archetype_settings()

```rust
use meilisearch_lib::Settings;
use std::collections::BTreeSet;

pub fn build_archetype_settings() -> Settings {
    Settings {
        searchable_attributes: Some(vec![
            "display_name".to_string(),
            "description".to_string(),
            "tags".to_string(),
        ]),
        filterable_attributes: Some(BTreeSet::from([
            "id".to_string(),
            "category".to_string(),
            "parent_id".to_string(),
            "setting_pack_id".to_string(),
            "game_system".to_string(),
            "tags".to_string(),
        ])),
        sortable_attributes: Some(BTreeSet::from([
            "display_name".to_string(),
            "category".to_string(),
            "created_at".to_string(),
        ])),
        ..Default::default()
    }
}
```

### build_vocabulary_bank_settings()

```rust
pub fn build_vocabulary_bank_settings() -> Settings {
    Settings {
        searchable_attributes: Some(vec![
            "display_name".to_string(),
            "description".to_string(),
            "phrase_texts".to_string(),
        ]),
        filterable_attributes: Some(BTreeSet::from([
            "id".to_string(),
            "culture".to_string(),
            "role".to_string(),
            "race".to_string(),
            "categories".to_string(),
            "formality_range".to_string(),
        ])),
        sortable_attributes: Some(BTreeSet::from([
            "display_name".to_string(),
            "created_at".to_string(),
        ])),
        ..Default::default()
    }
}
```

---

## API Mapping

### SDK to meilisearch-lib Mapping

| SDK Method | meilisearch-lib Method | Notes |
|------------|------------------------|-------|
| `client.get_index(name)` | `meili.get_index(name)` | Returns Result |
| `client.create_index(name, pk)` | `meili.create_index(name, pk)` | Returns task uid |
| `client.delete_index(name)` | `meili.delete_index(name)` | Returns task uid |
| `index.set_settings(&settings)` | `meili.update_settings(name, settings)` | Returns task uid |
| `index.get_stats()` | `meili.index_stats(name)` | Returns stats |
| `task.wait_for_completion()` | `meili.wait_for_task(uid, timeout)` | Blocks until done |
| ErrorCode::IndexNotFound check | `meili.index_exists(name)` | Or catch error |

---

## ArchetypeIndexManager Refactoring

### Structure Change

```rust
// Before (SDK - with lifetime)
pub struct ArchetypeIndexManager<'a> {
    client: &'a Client,
}

impl<'a> ArchetypeIndexManager<'a> {
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }
}

// After (meilisearch-lib - owned Arc)
pub struct ArchetypeIndexManager {
    meili: Arc<MeilisearchLib>,
}

impl ArchetypeIndexManager {
    pub fn new(meili: Arc<MeilisearchLib>) -> Self {
        Self { meili }
    }
}
```

### ensure_indexes() Implementation

```rust
pub async fn ensure_indexes(&self) -> Result<()> {
    log::info!("Ensuring archetype indexes exist with proper configuration");

    // Create or update archetypes index
    self.ensure_index(
        INDEX_ARCHETYPES,
        build_archetype_settings(),
    ).await?;

    // Create or update vocabulary banks index
    self.ensure_index(
        INDEX_VOCABULARY_BANKS,
        build_vocabulary_bank_settings(),
    ).await?;

    log::info!(
        "Archetype indexes configured: {}, {}",
        INDEX_ARCHETYPES,
        INDEX_VOCABULARY_BANKS
    );

    Ok(())
}
```

### ensure_index() Helper

```rust
async fn ensure_index(
    &self,
    index_name: &str,
    settings: Settings,
) -> Result<()> {
    // Check if index exists
    let exists = self.meili.index_exists(index_name).map_err(|e| {
        ArchetypeError::Meilisearch(format!(
            "Failed to check index '{}': {}",
            index_name, e
        ))
    })?;

    if exists {
        // Index exists, update settings
        log::debug!("Index '{}' exists, updating settings", index_name);
        let uid = self.meili.update_settings(index_name, settings).map_err(|e| {
            ArchetypeError::Meilisearch(format!(
                "Failed to update settings for '{}': {}",
                index_name, e
            ))
        })?;

        self.meili.wait_for_task(uid, Some(TASK_TIMEOUT)).map_err(|e| {
            ArchetypeError::Meilisearch(format!(
                "Timeout waiting for settings update on '{}': {}",
                index_name, e
            ))
        })?;
    } else {
        // Create new index
        log::info!("Creating index '{}' with primary key 'id'", index_name);
        let uid = self.meili.create_index(index_name, Some("id")).map_err(|e| {
            ArchetypeError::Meilisearch(format!(
                "Failed to create index '{}': {}",
                index_name, e
            ))
        })?;

        self.meili.wait_for_task(uid, Some(TASK_TIMEOUT)).map_err(|e| {
            ArchetypeError::Meilisearch(format!(
                "Timeout waiting for index creation '{}': {}",
                index_name, e
            ))
        })?;

        // Apply settings to new index
        let uid = self.meili.update_settings(index_name, settings).map_err(|e| {
            ArchetypeError::Meilisearch(format!(
                "Failed to apply settings to '{}': {}",
                index_name, e
            ))
        })?;

        self.meili.wait_for_task(uid, Some(TASK_TIMEOUT)).map_err(|e| {
            ArchetypeError::Meilisearch(format!(
                "Timeout waiting for settings on '{}': {}",
                index_name, e
            ))
        })?;
    }

    Ok(())
}
```

### Index Existence Checks

```rust
pub async fn archetypes_index_exists(&self) -> Result<bool> {
    self.index_exists(INDEX_ARCHETYPES).await
}

pub async fn vocabulary_banks_index_exists(&self) -> Result<bool> {
    self.index_exists(INDEX_VOCABULARY_BANKS).await
}

async fn index_exists(&self, index_name: &str) -> Result<bool> {
    self.meili.index_exists(index_name).map_err(|e| {
        ArchetypeError::Meilisearch(format!(
            "Failed to check index '{}': {}",
            index_name, e
        ))
    })
}
```

### Document Counts

```rust
pub async fn archetype_count(&self) -> Result<u64> {
    self.document_count(INDEX_ARCHETYPES).await
}

pub async fn vocabulary_bank_count(&self) -> Result<u64> {
    self.document_count(INDEX_VOCABULARY_BANKS).await
}

async fn document_count(&self, index_name: &str) -> Result<u64> {
    // Check existence first
    if !self.meili.index_exists(index_name).unwrap_or(false) {
        return Ok(0);
    }

    let stats = self.meili.index_stats(index_name).map_err(|e| {
        ArchetypeError::Meilisearch(format!(
            "Failed to get stats for index '{}': {}",
            index_name, e
        ))
    })?;

    Ok(stats.number_of_documents)
}
```

### Delete Indexes

```rust
pub async fn delete_indexes(&self) -> Result<()> {
    log::warn!("Deleting archetype indexes");

    for index_name in [INDEX_ARCHETYPES, INDEX_VOCABULARY_BANKS] {
        match self.meili.delete_index(index_name) {
            Ok(uid) => {
                if let Err(e) = self.meili.wait_for_task(uid, Some(TASK_TIMEOUT)) {
                    log::warn!("Timeout waiting for deletion of '{}': {}", index_name, e);
                } else {
                    log::info!("Deleted index '{}'", index_name);
                }
            }
            Err(e) => {
                // Check if it's an IndexNotFound error (which is OK)
                let err_str = e.to_string();
                if err_str.contains("not found") || err_str.contains("IndexNotFound") {
                    log::debug!("Index '{}' already doesn't exist", index_name);
                } else {
                    return Err(ArchetypeError::Meilisearch(format!(
                        "Failed to delete index '{}': {}",
                        index_name, e
                    )));
                }
            }
        }
    }

    Ok(())
}
```

---

## Convenience Functions

These remain unchanged in signature but may need internal updates:

```rust
/// Get the archetype index name.
#[inline]
pub fn archetype_index_name() -> &'static str {
    INDEX_ARCHETYPES
}

/// Get the vocabulary banks index name.
#[inline]
pub fn vocabulary_banks_index_name() -> &'static str {
    INDEX_VOCABULARY_BANKS
}

/// Get the archetype index settings.
pub fn get_archetype_settings() -> Settings {
    build_archetype_settings()
}

/// Get the vocabulary banks index settings.
pub fn get_vocabulary_bank_settings() -> Settings {
    build_vocabulary_bank_settings()
}
```

---

## Integration Points

### From Tauri Commands

```rust
// Example usage in commands
#[tauri::command]
pub async fn ensure_archetype_indexes(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let meili = state.embedded_search.inner();
    let manager = ArchetypeIndexManager::new(meili);

    tokio::task::spawn_blocking(move || {
        tokio::runtime::Handle::current().block_on(manager.ensure_indexes())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}
```

---

## Testing Strategy

### Unit Tests

1. **Settings Functions**
   - `build_archetype_settings()` returns correct attributes
   - `build_vocabulary_bank_settings()` returns correct attributes

2. **Convenience Functions**
   - `archetype_index_name()` returns correct constant
   - `vocabulary_banks_index_name()` returns correct constant

### Integration Tests

1. **Index Lifecycle**
   - `ensure_indexes()` creates both indexes
   - Re-calling is idempotent
   - Settings are applied correctly

2. **Existence Checks**
   - Returns true for existing index
   - Returns false for non-existing index
   - Handles errors appropriately

3. **Document Counts**
   - Returns 0 for empty or non-existing index
   - Returns correct count after adding documents

4. **Deletion**
   - Deletes both indexes
   - Handles non-existing indexes gracefully

---

## Risk Mitigation

### R-1: Lifetime Removal

**Risk**: Removing `<'a>` lifetime changes the API for callers.

**Mitigation**:
- Change to owned `Arc<MeilisearchLib>`
- Update all call sites
- This is a breaking change but necessary for embedded usage

### R-2: Error Handling Differences

**Risk**: SDK errors differ from meilisearch-lib errors.

**Mitigation**:
- Map all errors to `ArchetypeError::Meilisearch`
- Include original error message
- Check for specific error types by message content if needed

### R-3: Settings Type Differences

**Risk**: `meilisearch_sdk::Settings` differs from `meilisearch_lib::Settings`.

**Mitigation**:
- Use `meilisearch_lib::Settings` directly
- Convert attributes to correct collection types (`BTreeSet`)
- Test settings are applied correctly
