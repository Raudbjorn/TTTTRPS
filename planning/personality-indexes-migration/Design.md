# Design: Personality Indexes Migration to meilisearch-lib

## Architecture Overview

### Current Architecture (SDK-based)

```
┌─────────────────────────────────────────────────────────────────┐
│                    PersonalityIndexManager                       │
│                                                                  │
│  ┌──────────────┐    ┌──────────────────────────────────────┐   │
│  │    client    │───▶│      meilisearch_sdk::Client         │   │
│  │   (Client)   │    │   (HTTP connection to external)      │   │
│  └──────────────┘    └──────────────────────────────────────┘   │
│                                      │                           │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   Index Operations                        │   │
│  │  ┌─────────────────┐   ┌─────────────────────────────┐   │   │
│  │  │  Templates      │   │  Blend Rules                │   │   │
│  │  │  - upsert       │   │  - upsert                   │   │   │
│  │  │  - get          │   │  - get                      │   │   │
│  │  │  - delete       │   │  - delete                   │   │   │
│  │  │  - search       │   │  - search                   │   │   │
│  │  │  - list_by_*    │   │  - list_by_*                │   │   │
│  │  └─────────────────┘   └─────────────────────────────┘   │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ HTTP
                    ┌─────────────────────┐
                    │  External Meilisearch│
                    │  Process             │
                    └─────────────────────┘
```

### Target Architecture (Embedded)

```
┌─────────────────────────────────────────────────────────────────┐
│                    PersonalityIndexManager                       │
│                                                                  │
│  ┌──────────────┐    ┌──────────────────────────────────────┐   │
│  │    meili     │───▶│       Arc<MeilisearchLib>            │   │
│  │   (&Meili)   │    │   (In-process embedded engine)       │   │
│  └──────────────┘    └──────────────────────────────────────┘   │
│                                      │                           │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   Index Operations                        │   │
│  │  ┌─────────────────┐   ┌─────────────────────────────┐   │   │
│  │  │  Templates      │   │  Blend Rules                │   │   │
│  │  │  - upsert       │   │  - upsert                   │   │   │
│  │  │  - get          │   │  - get                      │   │   │
│  │  │  - delete       │   │  - delete                   │   │   │
│  │  │  - search       │   │  - search                   │   │   │
│  │  │  - list_by_*    │   │  - list_by_*                │   │   │
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

### PersonalityIndexError Enum

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PersonalityIndexError {
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

    #[error("Failed to add documents to '{index}': {message}")]
    AddDocuments { index: String, message: String },

    #[error("Failed to get document '{doc_id}' from '{index}': {message}")]
    GetDocument { index: String, doc_id: String, message: String },

    #[error("Failed to delete document '{doc_id}' from '{index}': {message}")]
    DeleteDocument { index: String, doc_id: String, message: String },

    #[error("Search failed on '{index}': {message}")]
    Search { index: String, message: String },

    #[error("Failed to clear '{index}': {message}")]
    Clear { index: String, message: String },
}

impl From<PersonalityIndexError> for String {
    fn from(e: PersonalityIndexError) -> Self {
        e.to_string()
    }
}
```

### Index Constants

```rust
/// Index name for personality templates.
pub const INDEX_PERSONALITY_TEMPLATES: &str = "ttrpg_personality_templates";

/// Index name for blend rules.
pub const INDEX_BLEND_RULES: &str = "ttrpg_blend_rules";

/// Default timeout for index operations (30 seconds).
pub const INDEX_TIMEOUT: Duration = Duration::from_secs(30);
```

---

## Settings Functions

### personality_templates_settings()

```rust
use meilisearch_lib::Settings;
use std::collections::BTreeSet;

pub fn personality_templates_settings() -> Settings {
    Settings {
        searchable_attributes: Some(vec![
            "name".to_string(),
            "description".to_string(),
            "vocabularyKeys".to_string(),
            "commonPhrases".to_string(),
        ]),
        filterable_attributes: Some(BTreeSet::from([
            "gameSystem".to_string(),
            "settingName".to_string(),
            "isBuiltin".to_string(),
            "tags".to_string(),
            "campaignId".to_string(),
        ])),
        sortable_attributes: Some(BTreeSet::from([
            "name".to_string(),
            "createdAt".to_string(),
            "updatedAt".to_string(),
        ])),
        ..Default::default()
    }
}
```

### blend_rules_settings()

```rust
pub fn blend_rules_settings() -> Settings {
    Settings {
        searchable_attributes: Some(vec![
            "name".to_string(),
            "description".to_string(),
        ]),
        filterable_attributes: Some(BTreeSet::from([
            "context".to_string(),
            "enabled".to_string(),
            "isBuiltin".to_string(),
            "tags".to_string(),
            "campaignId".to_string(),
        ])),
        sortable_attributes: Some(BTreeSet::from([
            "name".to_string(),
            "priority".to_string(),
            "createdAt".to_string(),
            "updatedAt".to_string(),
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
| `index.set_settings(&settings)` | `meili.update_settings(name, settings)` | Returns task uid |
| `index.add_documents(&docs, pk)` | `meili.add_documents(name, docs)` | Returns task uid |
| `index.get_document::<T>(id)` | `meili.get_document(name, id)` | Returns Option |
| `index.delete_document(id)` | `meili.delete_document(name, id)` | Returns task uid |
| `index.search().execute()` | `meili.search(name, query, params)` | Different params |
| `index.get_stats()` | `meili.index_stats(name)` | Returns stats struct |
| `index.delete_all_documents()` | `meili.delete_all_documents(name)` | Returns task uid |
| `task.wait_for_completion()` | `meili.wait_for_task(uid, timeout)` | Blocks until done |

---

## PersonalityIndexManager Refactoring

### Constructor Change

```rust
// Before (SDK)
impl PersonalityIndexManager {
    pub fn new(host: &str, api_key: Option<&str>) -> Self {
        Self {
            client: Client::new(host, api_key).expect("Failed to create client"),
            host: host.to_string(),
            api_key: api_key.map(|s| s.to_string()),
        }
    }
}

// After (meilisearch-lib)
impl PersonalityIndexManager {
    pub fn new(meili: Arc<MeilisearchLib>) -> Self {
        Self { meili }
    }
}
```

### Initialize Indexes Implementation

```rust
pub async fn initialize_indexes(&self) -> Result<(), PersonalityIndexError> {
    // Ensure templates index
    self.ensure_single_index(
        INDEX_PERSONALITY_TEMPLATES,
        personality_templates_settings(),
    ).await?;

    // Ensure blend rules index
    self.ensure_single_index(
        INDEX_BLEND_RULES,
        blend_rules_settings(),
    ).await?;

    log::info!(
        "Initialized personality indexes: {}, {}",
        INDEX_PERSONALITY_TEMPLATES,
        INDEX_BLEND_RULES
    );

    Ok(())
}

async fn ensure_single_index(
    &self,
    name: &str,
    settings: Settings,
) -> Result<(), PersonalityIndexError> {
    // Check if index exists
    if !self.meili.index_exists(name).map_err(|e| PersonalityIndexError::Check {
        index: name.to_string(),
        message: e.to_string(),
    })? {
        // Create index
        let uid = self.meili.create_index(name, Some("id")).map_err(|e| {
            PersonalityIndexError::Create {
                index: name.to_string(),
                message: e.to_string(),
            }
        })?;

        self.meili.wait_for_task(uid, Some(INDEX_TIMEOUT)).map_err(|e| {
            PersonalityIndexError::Timeout {
                index: name.to_string(),
                timeout_secs: INDEX_TIMEOUT.as_secs(),
            }
        })?;
    }

    // Update settings
    let uid = self.meili.update_settings(name, settings).map_err(|e| {
        PersonalityIndexError::Settings {
            index: name.to_string(),
            message: e.to_string(),
        }
    })?;

    self.meili.wait_for_task(uid, Some(INDEX_TIMEOUT)).map_err(|e| {
        PersonalityIndexError::Timeout {
            index: name.to_string(),
            timeout_secs: INDEX_TIMEOUT.as_secs(),
        }
    })?;

    Ok(())
}
```

### Template CRUD Operations

```rust
pub async fn upsert_template(
    &self,
    template: &SettingPersonalityTemplate,
) -> Result<(), PersonalityIndexError> {
    let doc: TemplateDocument = template.clone().into();
    let docs = vec![doc];

    let uid = self.meili.add_documents(INDEX_PERSONALITY_TEMPLATES, &docs)
        .map_err(|e| PersonalityIndexError::AddDocuments {
            index: INDEX_PERSONALITY_TEMPLATES.to_string(),
            message: e.to_string(),
        })?;

    self.meili.wait_for_task(uid, Some(INDEX_TIMEOUT)).map_err(|e| {
        PersonalityIndexError::Timeout {
            index: INDEX_PERSONALITY_TEMPLATES.to_string(),
            timeout_secs: INDEX_TIMEOUT.as_secs(),
        }
    })?;

    log::debug!("Upserted template: {} ({})", template.name, template.id);
    Ok(())
}

pub async fn get_template(
    &self,
    id: &TemplateId,
) -> Result<Option<TemplateDocument>, PersonalityIndexError> {
    self.meili.get_document::<TemplateDocument>(INDEX_PERSONALITY_TEMPLATES, id.as_str())
        .map_err(|e| PersonalityIndexError::GetDocument {
            index: INDEX_PERSONALITY_TEMPLATES.to_string(),
            doc_id: id.to_string(),
            message: e.to_string(),
        })
}

pub async fn delete_template(&self, id: &TemplateId) -> Result<(), PersonalityIndexError> {
    let uid = self.meili.delete_document(INDEX_PERSONALITY_TEMPLATES, id.as_str())
        .map_err(|e| PersonalityIndexError::DeleteDocument {
            index: INDEX_PERSONALITY_TEMPLATES.to_string(),
            doc_id: id.to_string(),
            message: e.to_string(),
        })?;

    self.meili.wait_for_task(uid, Some(INDEX_TIMEOUT)).map_err(|e| {
        PersonalityIndexError::Timeout {
            index: INDEX_PERSONALITY_TEMPLATES.to_string(),
            timeout_secs: INDEX_TIMEOUT.as_secs(),
        }
    })?;

    log::debug!("Deleted template: {}", id);
    Ok(())
}
```

### Search Implementation

```rust
pub async fn search_templates(
    &self,
    query: &str,
    filter: Option<&str>,
    limit: usize,
) -> Result<Vec<TemplateDocument>, PersonalityIndexError> {
    use meilisearch_lib::SearchParams;

    let params = SearchParams {
        query: query.to_string(),
        filter: filter.map(|s| s.to_string()),
        limit: Some(limit),
        ..Default::default()
    };

    let results = self.meili.search::<TemplateDocument>(INDEX_PERSONALITY_TEMPLATES, params)
        .map_err(|e| PersonalityIndexError::Search {
            index: INDEX_PERSONALITY_TEMPLATES.to_string(),
            message: e.to_string(),
        })?;

    Ok(results.hits.into_iter().map(|h| h.document).collect())
}
```

### Statistics Implementation

```rust
pub async fn get_stats(&self) -> Result<PersonalityIndexStats, PersonalityIndexError> {
    let template_count = self.get_document_count(INDEX_PERSONALITY_TEMPLATES).await?;
    let rule_count = self.get_document_count(INDEX_BLEND_RULES).await?;

    Ok(PersonalityIndexStats {
        template_count,
        rule_count,
    })
}

async fn get_document_count(&self, index: &str) -> Result<u64, PersonalityIndexError> {
    if !self.meili.index_exists(index).unwrap_or(false) {
        return Ok(0);
    }

    let stats = self.meili.index_stats(index).map_err(|e| {
        PersonalityIndexError::Stats {
            index: index.to_string(),
            message: e.to_string(),
        }
    })?;

    Ok(stats.number_of_documents)
}
```

---

## Integration Points

### From Tauri Commands

```rust
// In commands/personality.rs or similar

#[tauri::command]
pub async fn initialize_personality_indexes(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let meili = state.embedded_search.inner();
    let manager = PersonalityIndexManager::new(meili);

    tokio::task::spawn_blocking(move || {
        tokio::runtime::Handle::current().block_on(manager.initialize_indexes())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_personality_stats(
    state: State<'_, AppState>,
) -> Result<PersonalityIndexStats, String> {
    let meili = state.embedded_search.inner();
    let manager = PersonalityIndexManager::new(meili);

    tokio::task::spawn_blocking(move || {
        tokio::runtime::Handle::current().block_on(manager.get_stats())
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
   - Verify `personality_templates_settings()` returns correct attributes
   - Verify `blend_rules_settings()` returns correct attributes

2. **Error Types**
   - All `PersonalityIndexError` variants format correctly
   - Include index name and context in messages

### Integration Tests

1. **Index Lifecycle**
   - Initialize creates both indexes
   - Re-initialization is idempotent
   - Settings are applied correctly

2. **CRUD Operations**
   - Upsert creates new document
   - Upsert updates existing document
   - Get returns None for missing document
   - Delete removes document

3. **Search Operations**
   - Empty query returns all documents
   - Filter restricts results correctly
   - Sort orders results correctly

---

## Risk Mitigation

### R-1: Search Parameters Difference

**Risk**: meilisearch-lib search API differs from SDK.

**Mitigation**:
- Create `SearchParams` adapter if needed
- Reference existing migrated search commands
- Test with same queries used by SDK

### R-2: Task Completion Behavior

**Risk**: `wait_for_task()` may behave differently.

**Mitigation**:
- Use explicit timeout parameter
- Handle timeout errors gracefully
- Log task progress for debugging

### R-3: Document Serialization

**Risk**: `TemplateDocument` serialization may differ.

**Mitigation**:
- Test round-trip serialization
- Verify camelCase field names preserved
- Check nested object handling
