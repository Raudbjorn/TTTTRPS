# Design: Embedded Meilisearch with RAG Pipeline

## Overview

This document describes the technical design for integrating `meilisearch-lib` as an embedded search engine with full RAG capabilities, replacing the HTTP-based Meilisearch SDK and sidecar process management.

---

## 1. Architecture

### 1.1 Current Architecture (To Be Replaced)

```
┌─────────────────────────────────────────────────────────────┐
│                    TTRPG Assistant                          │
├─────────────────────────────────────────────────────────────┤
│  SearchClient (meilisearch-sdk)                             │
│       │                                                     │
│       │ HTTP                                                │
│       ▼                                                     │
│  ┌─────────────────┐                                        │
│  │ SidecarManager  │──spawns──▶ [meilisearch binary :7700]  │
│  └─────────────────┘                                        │
│                                                             │
│  LLM Integration (manual context building)                  │
│       │                                                     │
│       └── No integrated RAG pipeline                        │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 Target Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    TTRPG Assistant                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              MeilisearchLib (embedded)               │   │
│  │                                                      │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │   │
│  │  │   Indexes   │  │  Documents  │  │   Search    │  │   │
│  │  │   CRUD      │  │   CRUD      │  │  (Hybrid)   │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  │   │
│  │                                                      │   │
│  │  ┌───────────────────────────────────────────────┐  │   │
│  │  │              RAG Pipeline                      │  │   │
│  │  │                                                │  │   │
│  │  │  chat_completion() / chat_completion_stream() │  │   │
│  │  │       │                                        │  │   │
│  │  │       ├── 1. Extract query from messages       │  │   │
│  │  │       ├── 2. Hybrid search for context         │  │   │
│  │  │       ├── 3. Format with Liquid templates      │  │   │
│  │  │       ├── 4. Call LLM provider                 │  │   │
│  │  │       └── 5. Return response + sources         │  │   │
│  │  └───────────────────────────────────────────────┘  │   │
│  │                          │                          │   │
│  │                          ▼                          │   │
│  │  ┌───────────────────────────────────────────────┐  │   │
│  │  │  milli (search engine)                         │  │   │
│  │  │  ├── LMDB storage                             │  │   │
│  │  │  ├── BM25 ranking                             │  │   │
│  │  │  ├── Typo tolerance                           │  │   │
│  │  │  └── Vector indexing (arroy/HNSW)             │  │   │
│  │  └───────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Component Design

### 2.1 MeilisearchLib Wrapper

**Location**: `src-tauri/src/core/search/embedded.rs`

A thin wrapper around `MeilisearchLib` for TTRPS-specific functionality:

```rust
use meilisearch_lib::{
    MeilisearchLib, Config, SearchQuery, SearchResult,
    ChatConfig, ChatRequest, ChatResponse, ChatChunk,
    Message, ChatIndexConfig, ChatSearchParams, ChatSource,
};
use std::sync::Arc;
use std::path::PathBuf;

/// Embedded search engine with RAG capabilities
pub struct EmbeddedSearch {
    inner: Arc<MeilisearchLib>,
}

impl EmbeddedSearch {
    /// Initialize embedded Meilisearch
    pub fn new(db_path: PathBuf) -> Result<Self, SearchError> {
        let config = Config::builder()
            .db_path(&db_path)
            .max_index_size(10 * 1024 * 1024 * 1024) // 10 GiB
            .build()
            .map_err(SearchError::Config)?;

        let inner = MeilisearchLib::new(config)
            .map_err(SearchError::Init)?;

        Ok(Self { inner: Arc::new(inner) })
    }

    /// Get reference to inner MeilisearchLib
    pub fn inner(&self) -> &MeilisearchLib {
        &self.inner
    }

    /// Clone the Arc for sharing across threads
    pub fn clone_inner(&self) -> Arc<MeilisearchLib> {
        Arc::clone(&self.inner)
    }
}
```

### 2.2 RAG Configuration

**Location**: `src-tauri/src/core/search/rag_config.rs`

TTRPG-specific RAG configuration builder:

```rust
use meilisearch_lib::{ChatConfig, ChatIndexConfig, ChatSearchParams, ChatSource, ChatPrompts};
use std::collections::HashMap;

/// Build ChatConfig for TTRPG indexes
pub fn build_ttrpg_chat_config(
    provider: LlmProvider,
    api_key: &str,
    model: &str,
) -> ChatConfig {
    let source = match provider {
        LlmProvider::Anthropic => ChatSource::Anthropic,
        LlmProvider::OpenAi => ChatSource::OpenAi,
        LlmProvider::Mistral => ChatSource::Mistral,
        LlmProvider::VLlm { base_url } => ChatSource::VLlm,
        // ... other providers
    };

    let mut index_configs = HashMap::new();

    // Rules index configuration
    index_configs.insert("ttrpg_rules".to_string(), ChatIndexConfig {
        description: "TTRPG rules, mechanics, combat, spells, and character options".to_string(),
        template: Some(RULES_TEMPLATE.to_string()),
        max_bytes: Some(800),
        search_params: Some(ChatSearchParams {
            limit: Some(8),
            semantic_ratio: Some(0.7),
            embedder: Some("default".to_string()),
            ..Default::default()
        }),
    });

    // Fiction/lore index configuration
    index_configs.insert("ttrpg_fiction".to_string(), ChatIndexConfig {
        description: "TTRPG lore, world-building, NPCs, and narrative content".to_string(),
        template: Some(FICTION_TEMPLATE.to_string()),
        max_bytes: Some(1000),
        search_params: Some(ChatSearchParams {
            limit: Some(6),
            semantic_ratio: Some(0.8),  // More semantic for narrative
            embedder: Some("default".to_string()),
            ..Default::default()
        }),
    });

    // Chunks index for per-document content
    index_configs.insert("chunks".to_string(), ChatIndexConfig {
        description: "Semantic chunks from ingested TTRPG documents".to_string(),
        template: Some(CHUNK_TEMPLATE.to_string()),
        max_bytes: Some(600),
        search_params: Some(ChatSearchParams {
            limit: Some(10),
            semantic_ratio: Some(0.6),
            embedder: Some("default".to_string()),
            ..Default::default()
        }),
    });

    ChatConfig {
        source,
        api_key: api_key.to_string(),
        base_url: None,
        model: model.to_string(),
        org_id: None,
        project_id: None,
        api_version: None,
        deployment_id: None,
        prompts: ChatPrompts {
            system: Some(TTRPG_SYSTEM_PROMPT.to_string()),
            ..Default::default()
        },
        index_configs,
    }
}

const TTRPG_SYSTEM_PROMPT: &str = r#"
You are an expert TTRPG Game Master assistant with deep knowledge of tabletop roleplaying games.

When answering questions:
1. Use ONLY the provided context from the rulebooks
2. Always cite your sources with page numbers when available
3. If the context doesn't contain enough information, say so clearly
4. Format rules and mechanics clearly for quick reference
5. Distinguish between core rules and optional/variant rules

If asked about something not in the provided context, clearly state that you don't have that information in your indexed sources.
"#;

const RULES_TEMPLATE: &str = r#"
[{{ doc.source }}{% if doc.page_number %} (p.{{ doc.page_number }}){% endif %}]
{% if doc.section_path %}Section: {{ doc.section_path }}{% endif %}
{{ doc.content }}
"#;

const FICTION_TEMPLATE: &str = r#"
[{{ doc.source }}{% if doc.page_number %} (p.{{ doc.page_number }}){% endif %}]
{{ doc.content }}
"#;

const CHUNK_TEMPLATE: &str = r#"
[{{ doc.book_title }} - {{ doc.source_slug }}{% if doc.page_start %} (p.{{ doc.page_start }}){% endif %}]
{% if doc.section_path %}{{ doc.section_path }}{% endif %}
{{ doc.content }}
"#;
```

### 2.3 AppState Changes

**Location**: `src-tauri/src/commands/state.rs`

```rust
// REMOVE these fields:
// pub search_client: Arc<SearchClient>,
// pub sidecar_manager: Arc<SidecarManager>,

// ADD this field:
pub struct AppState {
    // ... existing fields ...

    /// Embedded Meilisearch with RAG capabilities
    pub embedded_search: Arc<EmbeddedSearch>,

    // ... rest of fields ...
}
```

### 2.4 Initialization

**Location**: `src-tauri/src/main.rs`

```rust
async fn initialize_search(data_dir: &Path) -> Result<Arc<EmbeddedSearch>, AppInitError> {
    let db_path = data_dir.join("meilisearch");

    // Initialize embedded search
    let embedded_search = EmbeddedSearch::new(db_path)
        .map_err(AppInitError::Search)?;

    let meili = embedded_search.inner();

    // Ensure core indexes exist
    let indexes = ["ttrpg_rules", "ttrpg_fiction", "ttrpg_chat",
                   "ttrpg_documents", "library_metadata"];

    for index_name in indexes {
        if !meili.index_exists(index_name)? {
            let task = meili.create_index(index_name, Some("id".into()))?;
            meili.wait_for_task(task.uid, None)?;
        }
    }

    // Configure embedder (if Ollama available)
    if let Ok(()) = check_ollama_available().await {
        let embedder_config = json!({
            "default": {
                "source": "rest",
                "url": "http://localhost:11434/api/embed",
                "request": { "model": "nomic-embed-text", "input": ["{{text}}"] },
                "response": { "embeddings": ["{{embedding}}"] },
                "dimensions": 768
            }
        });

        for index_name in indexes {
            if let Err(e) = meili.update_embedders(index_name, embedder_config.clone()) {
                tracing::warn!("Failed to configure embedder for {}: {}", index_name, e);
            }
        }
    }

    Ok(Arc::new(embedded_search))
}

// NO MORE SidecarManager initialization
// NO MORE meilisearch binary download
// NO MORE process spawning
```

---

## 3. API Mapping

### 3.1 Search Operations

| Current (meilisearch-sdk) | New (meilisearch-lib) |
|---------------------------|----------------------|
| `client.index(name)` | Direct method calls with `uid` parameter |
| `index.search().with_query(q).execute()` | `meili.search(uid, SearchQuery::new(q))` |
| `index.add_documents(&docs, pk)` | `meili.add_documents(uid, docs, pk)` |
| `index.delete_document(id)` | `meili.delete_document(uid, id)` |
| `index.get_document(id)` | `meili.get_document(uid, id)` |
| `index.set_settings(&settings)` | `meili.update_settings(uid, settings)` |
| `task.wait_for_completion()` | `meili.wait_for_task(task.uid, timeout)` |

### 3.2 Hybrid Search

| Current (raw HTTP) | New (native) |
|-------------------|--------------|
| `POST /indexes/{uid}/search` with hybrid body | `SearchQuery::new(q).with_hybrid(HybridQuery::new(0.7))` |

### 3.3 Embedder Configuration

| Current (raw HTTP) | New (native) |
|-------------------|--------------|
| `PATCH /indexes/{uid}/settings/embedders` | `meili.update_embedders(uid, config)` |
| `GET /indexes/{uid}/settings/embedders` | `meili.get_embedders(uid)` |

### 3.4 RAG Pipeline (NEW)

| Operation | Method |
|-----------|--------|
| Configure RAG | `meili.set_chat_config(Some(config))` |
| Non-streaming Q&A | `meili.chat_completion(request).await` |
| Streaming Q&A | `meili.chat_completion_stream(request).await` |

---

## 4. Tauri Commands

### 4.1 New RAG Commands

**Location**: `src-tauri/src/commands/search/rag.rs`

```rust
/// Configure RAG pipeline
#[tauri::command]
pub async fn configure_rag(
    state: State<'_, AppState>,
    provider: String,
    api_key: String,
    model: String,
) -> Result<(), String> {
    let provider = parse_provider(&provider)?;
    let config = build_ttrpg_chat_config(provider, &api_key, &model);

    state.embedded_search.inner().set_chat_config(Some(config));

    Ok(())
}

/// Ask a question about indexed rulebooks (non-streaming)
#[tauri::command]
pub async fn rag_query(
    state: State<'_, AppState>,
    question: String,
    index_uid: String,
    conversation: Vec<Message>,
) -> Result<RagResponse, String> {
    let meili = state.embedded_search.inner();

    let mut messages = conversation;
    messages.push(Message::user(question));

    let request = ChatRequest {
        messages,
        index_uid,
        stream: false,
    };

    let response = meili.chat_completion(request)
        .await
        .map_err(|e| e.to_string())?;

    Ok(RagResponse {
        content: response.content,
        sources: response.sources,
        usage: response.usage,
    })
}

/// Ask a question with streaming response
#[tauri::command]
pub async fn rag_query_stream(
    state: State<'_, AppState>,
    window: Window,
    question: String,
    index_uid: String,
    conversation: Vec<Message>,
    stream_id: String,
) -> Result<(), String> {
    let meili = state.embedded_search.clone_inner();

    let mut messages = conversation;
    messages.push(Message::user(question));

    let request = ChatRequest {
        messages,
        index_uid,
        stream: true,
    };

    // Spawn streaming task
    tokio::spawn(async move {
        match meili.chat_completion_stream(request).await {
            Ok(mut stream) => {
                use futures::StreamExt;
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            let _ = window.emit(&format!("rag-chunk-{}", stream_id), &chunk);
                            if chunk.done {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = window.emit(&format!("rag-error-{}", stream_id), e.to_string());
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                let _ = window.emit(&format!("rag-error-{}", stream_id), e.to_string());
            }
        }
    });

    Ok(())
}
```

### 4.2 Updated Search Commands

**Location**: `src-tauri/src/commands/search/basic.rs`

```rust
/// Perform search (updated to use embedded lib)
#[tauri::command]
pub async fn search(
    state: State<'_, AppState>,
    index_uid: String,
    query: String,
    filters: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SearchHit>, String> {
    let meili = state.embedded_search.inner();

    let mut search_query = SearchQuery::new(&query)
        .with_pagination(0, limit.unwrap_or(20));

    if let Some(f) = filters {
        search_query = search_query.with_filter(serde_json::json!(f));
    }

    let results = meili.search(&index_uid, search_query)
        .map_err(|e| e.to_string())?;

    Ok(results.hits.into_iter().map(SearchHit::from).collect())
}

/// Perform hybrid search
#[tauri::command]
pub async fn hybrid_search(
    state: State<'_, AppState>,
    index_uid: String,
    query: String,
    semantic_ratio: Option<f32>,
    filters: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SearchHit>, String> {
    let meili = state.embedded_search.inner();

    let ratio = semantic_ratio.unwrap_or(0.5);
    let mut search_query = SearchQuery::new(&query)
        .with_hybrid(HybridQuery::new(ratio))
        .with_pagination(0, limit.unwrap_or(20));

    if let Some(f) = filters {
        search_query = search_query.with_filter(serde_json::json!(f));
    }

    let results = meili.search(&index_uid, search_query)
        .map_err(|e| e.to_string())?;

    Ok(results.hits.into_iter().map(SearchHit::from).collect())
}
```

---

## 5. Data Flow

### 5.1 RAG Query Flow

```
User: "How does flanking work in 5e?"
                    │
                    ▼
┌─────────────────────────────────────────────────────────────┐
│  Tauri Command: rag_query()                                 │
└─────────────────────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────────────┐
│  MeilisearchLib::chat_completion()                          │
│                                                             │
│  1. Extract query: "How does flanking work in 5e?"          │
│                                                             │
│  2. Hybrid search on "ttrpg_rules" index                    │
│     ├── Keyword: "flanking" "5e" "work"                     │
│     └── Semantic: embed("How does flanking work in 5e?")    │
│                                                             │
│  3. Retrieve top 8 documents, format with Liquid template:  │
│     "[DMG (p.251)] Section: Combat > Optional Rules         │
│      Flanking provides advantage on melee attack rolls..."  │
│                                                             │
│  4. Build prompt:                                           │
│     - System: "You are a TTRPG rules expert..."            │
│     - Context: [formatted documents]                        │
│     - User: "How does flanking work in 5e?"                │
│                                                             │
│  5. Call Anthropic/OpenAI/Mistral API                       │
│                                                             │
│  6. Return response + source citations                      │
└─────────────────────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────────────┐
│  Response:                                                  │
│  content: "Flanking is an optional rule in the DMG..."     │
│  sources: ["dmg-combat-251", "phb-combat-chapter"]         │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 Document Ingestion Flow (Unchanged Pattern)

```
User drops PDF
       │
       ▼
MeilisearchPipeline (updated to use embedded lib)
       │
       ├── Phase 1: Extract pages → meili.add_documents("{slug}-raw", ...)
       │
       └── Phase 2: Chunk pages → meili.add_documents("{slug}", ...)
                                 → meili.add_documents("ttrpg_rules", ...)
```

---

## 6. Error Handling

### 6.1 Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Initialization failed: {0}")]
    Init(meilisearch_lib::Error),

    #[error("Search failed: {0}")]
    Search(meilisearch_lib::Error),

    #[error("RAG not configured - call configure_rag first")]
    RagNotConfigured,

    #[error("LLM provider error: {0}")]
    LlmProvider(String),

    #[error("Index not found: {0}")]
    IndexNotFound(String),
}

impl From<meilisearch_lib::Error> for SearchError {
    fn from(e: meilisearch_lib::Error) -> Self {
        match &e {
            meilisearch_lib::Error::IndexNotFound(uid) => {
                SearchError::IndexNotFound(uid.clone())
            }
            meilisearch_lib::Error::ChatNotConfigured => {
                SearchError::RagNotConfigured
            }
            meilisearch_lib::Error::ChatProvider(msg) => {
                SearchError::LlmProvider(msg.clone())
            }
            _ => SearchError::Search(e),
        }
    }
}
```

---

## 7. Configuration Storage

### 7.1 RAG Config Persistence

RAG configuration should be stored in the existing settings system:

```rust
// In settings table or dedicated config
pub struct RagSettings {
    pub provider: String,        // "anthropic", "openai", "mistral", "vllm"
    pub model: String,           // "claude-sonnet-4-20250514", "gpt-4o", etc.
    pub base_url: Option<String>, // For vLLM or Azure
    // API key stored in keyring, not here
}
```

On app startup, load RagSettings and call `set_chat_config()` if configured.

---

## 8. Migration Checklist

### 8.1 Files to Remove

- [ ] `src-tauri/src/core/sidecar_manager.rs` - Process management
- [ ] Meilisearch binary download logic
- [ ] Process health monitoring code

### 8.2 Files to Modify

- [ ] `src-tauri/Cargo.toml` - Replace meilisearch-sdk with meilisearch-lib
- [ ] `src-tauri/src/commands/state.rs` - Update AppState
- [ ] `src-tauri/src/main.rs` - Update initialization
- [ ] `src-tauri/src/core/search/client.rs` - Replace or remove
- [ ] `src-tauri/src/core/meilisearch_pipeline.rs` - Update to use embedded lib
- [ ] All files using `SearchClient` - Update to use `MeilisearchLib`

### 8.3 Files to Add

- [ ] `src-tauri/src/core/search/embedded.rs` - Wrapper module
- [ ] `src-tauri/src/core/search/rag_config.rs` - TTRPG RAG configuration
- [ ] `src-tauri/src/commands/search/rag.rs` - RAG Tauri commands

---

## 9. Testing Strategy

### 9.1 Unit Tests

```rust
#[tokio::test]
async fn test_rag_query() {
    let meili = setup_test_instance();

    // Add test documents
    meili.add_documents("ttrpg_rules", vec![
        json!({"id": "test-1", "content": "Flanking gives advantage on attack rolls", "source": "DMG", "page_number": 251}),
    ], Some("id".into())).unwrap();

    // Configure RAG
    let config = build_test_chat_config();
    meili.set_chat_config(Some(config));

    // Query
    let response = meili.chat_completion(ChatRequest {
        messages: vec![Message::user("How does flanking work?")],
        index_uid: "ttrpg_rules".to_string(),
        stream: false,
    }).await.unwrap();

    assert!(!response.content.is_empty());
    assert!(!response.sources.is_empty());
}
```

### 9.2 Integration Tests

- Test full document ingestion → RAG query flow
- Test streaming responses
- Test provider switching
- Test error handling for unavailable providers

---

## 10. Decisions Log

### Decision 1: Keep Full meilisearch-lib

**Context**: Considered stripping chat module and using SurrealDB for vectors

**Decision**: Use meilisearch-lib as-is with full RAG pipeline

**Rationale**:
- RAG pipeline is the primary value for TTRPG rulebook Q&A
- Would take weeks to reimplement what already exists
- Hybrid search already integrated and tested
- Multi-LLM provider support built-in

### Decision 2: No SurrealDB

**Context**: Originally planned to use SurrealDB for vector storage

**Decision**: Use meilisearch-lib's built-in vector support (via milli)

**Rationale**:
- RAG pipeline expects vectors in Meilisearch
- Adding SurrealDB would require custom RAG integration
- milli's arroy/HNSW is production-quality
- Simpler architecture with single database
