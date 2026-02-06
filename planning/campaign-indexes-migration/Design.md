# Design: Campaign Indexes Migration to meilisearch-lib

## Architecture Overview

### Current Architecture (SDK-based with HTTP)

```
┌─────────────────────────────────────────────────────────────────────┐
│                    MeilisearchCampaignClient                         │
│                                                                      │
│  ┌──────────────┐    ┌────────────────────────────────────────────┐ │
│  │    client    │───▶│         meilisearch_sdk::Client            │ │
│  │   (Client)   │    │        (HTTP to external server)           │ │
│  └──────────────┘    └────────────────────────────────────────────┘ │
│                                        │                             │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                      Retry Layer                                 ││
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  ││
│  │  │ with_retry  │  │ is_transient│  │ exponential_backoff     │  ││
│  │  │             │  │ _error()    │  │ (100ms * 2^attempt)     │  ││
│  │  └─────────────┘  └─────────────┘  └─────────────────────────┘  ││
│  └─────────────────────────────────────────────────────────────────┘│
│                                        │                             │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │              Typed Operations (Arc, Plan, PlotPoint)            ││
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  ││
│  │  │   Arcs      │  │   Plans     │  │   Plot Points           │  ││
│  │  │ get/save/   │  │ get/save/   │  │ get/save/delete         │  ││
│  │  │ delete/list │  │ delete/list │  │ list_by_state/arc       │  ││
│  │  └─────────────┘  └─────────────┘  └─────────────────────────┘  ││
│  └─────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼ HTTP + reqwest
                    ┌───────────────────────┐
                    │  External Meilisearch │
                    │  Process              │
                    └───────────────────────┘
```

### Target Architecture (Embedded)

```
┌─────────────────────────────────────────────────────────────────────┐
│                    MeilisearchCampaignClient                         │
│                                                                      │
│  ┌──────────────┐    ┌────────────────────────────────────────────┐ │
│  │    meili     │───▶│         Arc<MeilisearchLib>                │ │
│  │   (Arc)      │    │        (In-process embedded)               │ │
│  └──────────────┘    └────────────────────────────────────────────┘ │
│                                        │                             │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                      Retry Layer (Simplified)                    ││
│  │  ┌─────────────┐                                                ││
│  │  │ with_retry  │  Retry logic preserved but fewer transient     ││
│  │  │             │  errors expected with embedded engine          ││
│  │  └─────────────┘                                                ││
│  └─────────────────────────────────────────────────────────────────┘│
│                                        │                             │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │              Typed Operations (Arc, Plan, PlotPoint)            ││
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  ││
│  │  │   Arcs      │  │   Plans     │  │   Plot Points           │  ││
│  │  │ get/save/   │  │ get/save/   │  │ get/save/delete         │  ││
│  │  │ delete/list │  │ delete/list │  │ list_by_state/arc       │  ││
│  │  └─────────────┘  └─────────────┘  └─────────────────────────┘  ││
│  └─────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼ Direct function calls
                    ┌───────────────────────┐
                    │  Embedded milli       │
                    │  (In-process)         │
                    └───────────────────────┘
```

---

## Component Design

### Error Types (Preserved)

The existing `MeilisearchCampaignError` enum is well-designed and should be preserved:

```rust
#[derive(Error, Debug)]
pub enum MeilisearchCampaignError {
    #[error("Meilisearch connection error: {0}")]
    ConnectionError(String),

    #[error("Meilisearch operation error: {0}")]
    OperationError(String),

    #[error("Document not found: {index}/{id}")]
    DocumentNotFound { index: String, id: String },

    #[error("Index not found: {0}")]
    IndexNotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Task timeout: operation did not complete within {0} seconds")]
    TaskTimeout(u64),

    #[error("Health check failed: Meilisearch is not available")]
    HealthCheckFailed,

    #[error("Batch operation failed: {0}")]
    BatchOperationFailed(String),
}
```

### Index Constants (From meilisearch_indexes.rs)

```rust
pub const INDEX_CAMPAIGN_ARCS: &str = "ttrpg_campaign_arcs";
pub const INDEX_SESSION_PLANS: &str = "ttrpg_session_plans";
pub const INDEX_PLOT_POINTS: &str = "ttrpg_plot_points";

pub const MEILISEARCH_BATCH_SIZE: usize = 1000;
pub const TASK_TIMEOUT_SHORT_SECS: u64 = 30;
pub const TASK_TIMEOUT_LONG_SECS: u64 = 300;
```

---

## Settings via IndexConfig Trait

The existing `IndexConfig` trait approach is preserved, with settings converted to meilisearch-lib types:

### CampaignArcsIndexConfig

```rust
impl IndexConfig for CampaignArcsIndexConfig {
    fn index_name() -> &'static str { INDEX_CAMPAIGN_ARCS }
    fn primary_key() -> &'static str { "id" }

    fn searchable_attributes() -> Vec<&'static str> {
        vec!["name", "description", "premise"]
    }

    fn filterable_attributes() -> Vec<&'static str> {
        vec!["id", "campaign_id", "arc_type", "status", "is_main_arc"]
    }

    fn sortable_attributes() -> Vec<&'static str> {
        vec!["name", "display_order", "started_at", "created_at"]
    }

    fn build_settings() -> meilisearch_lib::Settings {
        // Convert to meilisearch_lib::Settings
        meilisearch_lib::Settings {
            searchable_attributes: Some(Self::searchable_attributes().into_iter()
                .map(String::from).collect()),
            filterable_attributes: Some(Self::filterable_attributes().into_iter()
                .map(String::from).collect()),
            sortable_attributes: Some(Self::sortable_attributes().into_iter()
                .map(String::from).collect()),
            ..Default::default()
        }
    }
}
```

---

## MeilisearchCampaignClient Refactoring

### Constructor Change

```rust
// Before (SDK)
impl MeilisearchCampaignClient {
    pub fn new(host: &str, api_key: Option<&str>) -> Result<Self> {
        let client = Client::new(host, api_key)
            .map_err(|e| MeilisearchCampaignError::ConnectionError(e.to_string()))?;

        Ok(Self {
            client,
            host: host.to_string(),
            api_key: api_key.map(|s| s.to_string()),
            _write_lock: Mutex::new(()),
        })
    }
}

// After (meilisearch-lib)
impl MeilisearchCampaignClient {
    pub fn new(meili: Arc<MeilisearchLib>) -> Self {
        Self {
            meili,
            _write_lock: Mutex::new(()),
        }
    }
}
```

### Health Check Adaptation

```rust
// Before (HTTP-based)
pub async fn health_check(&self) -> bool {
    let url = format!("{}/health", self.host);
    let client = reqwest::Client::new();
    match client.get(&url).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

// After (embedded - always healthy if initialized)
pub fn health_check(&self) -> bool {
    // Embedded engine is always healthy if we have a valid reference
    true
}

pub fn wait_for_health(&self, _timeout_secs: u64) -> bool {
    // No waiting needed for embedded engine
    true
}
```

### Ensure Indexes Implementation

```rust
pub fn ensure_indexes(&self) -> Result<()> {
    for config in get_index_configs() {
        self.ensure_index(config.name, config.primary_key, config.settings)?;
    }

    log::info!(
        "Ensured campaign generation indexes: {}, {}, {}",
        INDEX_CAMPAIGN_ARCS,
        INDEX_SESSION_PLANS,
        INDEX_PLOT_POINTS
    );

    Ok(())
}

fn ensure_index(
    &self,
    name: &str,
    primary_key: &str,
    settings: meilisearch_lib::Settings,
) -> Result<()> {
    if self.meili.index_exists(name).map_err(|e| {
        MeilisearchCampaignError::OperationError(format!("Check index '{}': {}", name, e))
    })? {
        // Update settings
        let uid = self.meili.update_settings(name, settings).map_err(|e| {
            MeilisearchCampaignError::OperationError(format!("Update settings '{}': {}", name, e))
        })?;
        self.meili.wait_for_task(uid, Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)))
            .map_err(|_| MeilisearchCampaignError::TaskTimeout(TASK_TIMEOUT_SHORT_SECS))?;
        log::debug!("Updated settings for index '{}'", name);
    } else {
        // Create index
        let uid = self.meili.create_index(name, Some(primary_key)).map_err(|e| {
            MeilisearchCampaignError::OperationError(format!("Create index '{}': {}", name, e))
        })?;
        self.meili.wait_for_task(uid, Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)))
            .map_err(|_| MeilisearchCampaignError::TaskTimeout(TASK_TIMEOUT_SHORT_SECS))?;

        // Apply settings
        let uid = self.meili.update_settings(name, settings).map_err(|e| {
            MeilisearchCampaignError::OperationError(format!("Apply settings '{}': {}", name, e))
        })?;
        self.meili.wait_for_task(uid, Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)))
            .map_err(|_| MeilisearchCampaignError::TaskTimeout(TASK_TIMEOUT_SHORT_SECS))?;

        log::info!("Created index '{}' with settings", name);
    }

    Ok(())
}
```

### Generic CRUD with Retry

```rust
pub fn upsert_document<T: Serialize + Send + Sync>(
    &self,
    index_name: &str,
    document: &T,
) -> Result<()> {
    self.with_retry(|| {
        let docs = vec![document];
        let uid = self.meili.add_documents(index_name, &docs)
            .map_err(|e| MeilisearchCampaignError::OperationError(e.to_string()))?;
        self.meili.wait_for_task(uid, Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)))
            .map_err(|_| MeilisearchCampaignError::TaskTimeout(TASK_TIMEOUT_SHORT_SECS))?;
        Ok(())
    })
}

pub fn get_document<T: DeserializeOwned + Send + Sync + 'static>(
    &self,
    index_name: &str,
    id: &str,
) -> Result<Option<T>> {
    self.meili.get_document::<T>(index_name, id)
        .map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("not found") {
                return MeilisearchCampaignError::DocumentNotFound {
                    index: index_name.to_string(),
                    id: id.to_string(),
                };
            }
            MeilisearchCampaignError::OperationError(err_str)
        })
}

pub fn delete_document(&self, index_name: &str, id: &str) -> Result<()> {
    self.with_retry(|| {
        let uid = self.meili.delete_document(index_name, id)
            .map_err(|e| MeilisearchCampaignError::OperationError(e.to_string()))?;
        self.meili.wait_for_task(uid, Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)))
            .map_err(|_| MeilisearchCampaignError::TaskTimeout(TASK_TIMEOUT_SHORT_SECS))?;
        Ok(())
    })
}
```

### Batch Operations

```rust
pub fn upsert_documents<T: Serialize + Clone + Send + Sync>(
    &self,
    index_name: &str,
    documents: &[T],
) -> Result<()> {
    if documents.is_empty() {
        return Ok(());
    }

    for chunk in documents.chunks(MEILISEARCH_BATCH_SIZE) {
        self.with_retry(|| {
            let uid = self.meili.add_documents(index_name, chunk)
                .map_err(|e| MeilisearchCampaignError::OperationError(e.to_string()))?;
            self.meili.wait_for_task(uid, Some(Duration::from_secs(TASK_TIMEOUT_LONG_SECS)))
                .map_err(|_| MeilisearchCampaignError::TaskTimeout(TASK_TIMEOUT_LONG_SECS))?;
            Ok(())
        })?;
    }

    log::info!(
        "Upserted {} documents to index '{}'",
        documents.len(),
        index_name
    );
    Ok(())
}
```

### Search Implementation

```rust
pub fn search<T: DeserializeOwned + Send + Sync + 'static>(
    &self,
    index_name: &str,
    query: &str,
    filter: Option<&str>,
    sort: Option<&[&str]>,
    limit: usize,
    offset: usize,
) -> Result<Vec<T>> {
    use meilisearch_lib::SearchParams;

    let params = SearchParams {
        query: query.to_string(),
        filter: filter.map(|s| s.to_string()),
        sort: sort.map(|s| s.iter().map(|x| x.to_string()).collect()),
        limit: Some(limit),
        offset: Some(offset),
        ..Default::default()
    };

    let results = self.meili.search::<T>(index_name, params)
        .map_err(|e| MeilisearchCampaignError::OperationError(e.to_string()))?;

    Ok(results.hits.into_iter().map(|h| h.document).collect())
}
```

### Retry Logic (Preserved)

```rust
fn with_retry<F, T>(&self, operation: F) -> Result<T>
where
    F: Fn() -> Result<T>,
{
    let mut attempt = 0;
    let mut last_error = None;

    while attempt < MAX_RETRY_ATTEMPTS {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                if Self::is_transient_error(&e) {
                    attempt += 1;
                    if attempt < MAX_RETRY_ATTEMPTS {
                        let delay = RETRY_BASE_DELAY_MS * (2_u64.pow(attempt));
                        log::warn!(
                            "Operation failed (attempt {}/{}), retrying in {}ms: {}",
                            attempt, MAX_RETRY_ATTEMPTS, delay, e
                        );
                        std::thread::sleep(Duration::from_millis(delay));
                    }
                    last_error = Some(e);
                } else {
                    return Err(e);
                }
            }
        }
    }

    Err(last_error.unwrap_or(MeilisearchCampaignError::OperationError(
        "Unknown error after retries".to_string(),
    )))
}

fn is_transient_error(error: &MeilisearchCampaignError) -> bool {
    matches!(
        error,
        MeilisearchCampaignError::ConnectionError(_)
            | MeilisearchCampaignError::TaskTimeout(_)
    )
}
```

---

## Filter Value Escaping (Preserved)

```rust
fn escape_filter_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
```

---

## Testing Strategy

### Unit Tests

1. **Index Configuration**
   - All three `IndexConfig` implementations return correct attributes
   - `get_index_configs()` returns all three configurations

2. **Error Types**
   - `is_transient_error()` correctly identifies retry-eligible errors
   - Error Display implementations include relevant context

3. **Filter Escaping**
   - Special characters are properly escaped

### Integration Tests

1. **Index Lifecycle**
   - `ensure_indexes()` creates all three indexes
   - Re-calling is idempotent
   - Settings are applied correctly

2. **CRUD Operations**
   - Upsert creates new document
   - Upsert updates existing document
   - Get returns None for missing
   - Delete removes document

3. **Batch Operations**
   - Large batches are split correctly
   - All documents are indexed

4. **Search Operations**
   - Query returns matching documents
   - Filter restricts results
   - Sort orders correctly
   - Offset/limit work for pagination

5. **Typed Operations**
   - Arc CRUD works correctly
   - Plan CRUD works correctly
   - Plot point CRUD works correctly

---

## Migration Considerations

### Breaking Changes

1. **Constructor**: `new(host, api_key)` becomes `new(meili: Arc<MeilisearchLib>)`
2. **Health Check**: May become no-op for embedded engine

### Preserved Behavior

1. **Retry Logic**: Same transient error handling
2. **Batch Size**: Same 1000 document limit
3. **Timeouts**: Same 30s/300s timeouts
4. **Filter Escaping**: Same security measures

### API Compatibility

All typed operations (get_arc, save_plan, etc.) maintain their signatures.
