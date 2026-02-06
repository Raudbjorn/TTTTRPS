# Design: SurrealDB Vector Storage Migration

## Overview

This document describes the technical design for replacing `meilisearch-lib` and SQLite with SurrealDB as the unified storage backend. The design prioritizes incremental migration, API compatibility with existing Tauri commands, and preservation of the hybrid search + RAG pipeline.

---

## 1. Architecture

### 1.1 Current Architecture (To Be Replaced)

```
┌─────────────────────────────────────────────────────────────────────┐
│                       TTRPG Assistant                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────────┐          ┌─────────────────────────────────┐  │
│  │     SQLite       │          │      meilisearch-lib            │  │
│  │                  │          │                                 │  │
│  │ • campaigns      │          │ • ttrpg_rules (vectors)        │  │
│  │ • npcs           │          │ • ttrpg_fiction (vectors)      │  │
│  │ • sessions       │          │ • session_notes (vectors)      │  │
│  │ • chat_messages  │          │ • library_metadata             │  │
│  │ • library_docs   │          │ • Document chunks              │  │
│  │                  │          │                                 │  │
│  │ (Relationships   │◄────────►│ (Search + Vectors,             │  │
│  │  via foreign     │ Sync     │  no relationships)             │  │
│  │  keys)           │ Required │                                 │  │
│  └──────────────────┘          └─────────────────────────────────┘  │
│                                                                      │
│  Problem: Two databases, manual sync, no graph capabilities          │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 Target Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                       TTRPG Assistant                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    SurrealDB (Embedded)                        │  │
│  │                                                                │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │  │
│  │  │  Documents  │  │   Vectors   │  │     Graph Relations      │ │  │
│  │  │             │  │             │  │                          │ │  │
│  │  │ • campaign  │  │ HNSW Index  │  │ campaign ──┐             │ │  │
│  │  │ • npc       │  │ on chunk    │  │            │ has_npc     │ │  │
│  │  │ • session   │  │ .embedding  │  │            ▼             │ │  │
│  │  │ • chunk     │  │             │  │          npc ────────┐   │ │  │
│  │  │ • chat_msg  │  │ Dimensions: │  │            │ allied  │   │ │  │
│  │  │ • library   │  │ 768-3072    │  │            ▼         ▼   │ │  │
│  │  └─────────────┘  └─────────────┘  │          npc ◄── faction │ │  │
│  │                                    └─────────────────────────┘ │  │
│  │                                                                │  │
│  │  ┌─────────────────────────────────────────────────────────┐   │  │
│  │  │              Hybrid Search Pipeline                      │   │  │
│  │  │                                                          │   │  │
│  │  │  User Query: "How does flanking work?"                   │   │  │
│  │  │       │                                                  │   │  │
│  │  │       ├──► Full-Text (BM25): "flanking" → chunks        │   │  │
│  │  │       │                                                  │   │  │
│  │  │       └──► Vector (KNN): embed(query) → similar chunks  │   │  │
│  │  │                 │                                        │   │  │
│  │  │                 ▼                                        │   │  │
│  │  │  search::linear([$ft, $vec], [0.4, 0.6], 10, 'minmax')  │   │  │
│  │  │                 │                                        │   │  │
│  │  │                 ▼                                        │   │  │
│  │  │  Fused Results (ranked by combined score)                │   │  │
│  │  └─────────────────────────────────────────────────────────┘   │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                      │
│  Benefits: Single database, native relations, unified queries        │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 2. Component Design

### 2.1 SurrealDB Wrapper Module

**Location**: `src-tauri/src/core/storage/surrealdb.rs`

```rust
use std::path::PathBuf;
use std::sync::Arc;
use surrealdb::{Surreal, engine::local::RocksDb, RecordId};
use tokio::sync::RwLock;

/// Embedded SurrealDB storage with unified document, vector, and graph capabilities.
#[derive(Clone)]
pub struct SurrealStorage {
    db: Arc<Surreal<RocksDb>>,
    config: Arc<RwLock<StorageConfig>>,
}

#[derive(Clone, Debug)]
pub struct StorageConfig {
    pub namespace: String,
    pub database: String,
    pub embedder: Option<EmbedderConfig>,
    pub default_vector_dimensions: u32,
}

impl SurrealStorage {
    /// Initialize SurrealDB with RocksDB persistence.
    pub async fn new(db_path: PathBuf) -> Result<Self, StorageError> {
        let db = Surreal::new::<RocksDb>(db_path).await?;

        db.use_ns("ttrpg").use_db("main").await?;

        let storage = Self {
            db: Arc::new(db),
            config: Arc::new(RwLock::new(StorageConfig::default())),
        };

        // Apply schema
        storage.apply_schema().await?;

        Ok(storage)
    }

    /// Apply database schema (tables, indexes, analyzers).
    async fn apply_schema(&self) -> Result<(), StorageError> {
        // Schema defined in separate constant for readability
        self.db.query(SCHEMA_V1).await?;
        Ok(())
    }

    /// Get direct database reference for advanced queries.
    pub fn db(&self) -> &Surreal<RocksDb> {
        &self.db
    }

    /// Clone Arc for sharing across async tasks.
    pub fn clone_db(&self) -> Arc<Surreal<RocksDb>> {
        Arc::clone(&self.db)
    }
}
```

### 2.2 Database Schema

**Location**: `src-tauri/src/core/storage/schema.rs`

```rust
pub const SCHEMA_V1: &str = r#"
-- ============================================================================
-- TTRPG Assistant SurrealDB Schema v1
-- ============================================================================

-- Namespace and database
USE NS ttrpg DB main;

-- ============================================================================
-- ANALYZERS (for full-text search)
-- ============================================================================

-- Standard TTRPG analyzer with English stemming
DEFINE ANALYZER ttrpg_analyzer
    TOKENIZERS class, blank, punct
    FILTERS lowercase, ascii, snowball(english);

-- Simple analyzer for exact matching (names, titles)
DEFINE ANALYZER exact_analyzer
    TOKENIZERS class
    FILTERS lowercase;

-- ============================================================================
-- CAMPAIGN TABLE
-- ============================================================================

DEFINE TABLE campaign SCHEMAFULL;
DEFINE FIELD name ON campaign TYPE string;
DEFINE FIELD description ON campaign TYPE option<string>;
DEFINE FIELD game_system ON campaign TYPE option<string>;
DEFINE FIELD game_system_id ON campaign TYPE option<string>;
DEFINE FIELD status ON campaign TYPE string DEFAULT "active";
DEFINE FIELD created_at ON campaign TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON campaign TYPE datetime DEFAULT time::now();
DEFINE FIELD metadata ON campaign TYPE option<object>;

DEFINE INDEX campaign_name ON campaign FIELDS name SEARCH ANALYZER exact_analyzer BM25;
DEFINE INDEX campaign_status ON campaign FIELDS status;

-- ============================================================================
-- NPC TABLE (with graph relations)
-- ============================================================================

DEFINE TABLE npc SCHEMAFULL;
DEFINE FIELD name ON npc TYPE string;
DEFINE FIELD description ON npc TYPE option<string>;
DEFINE FIELD personality ON npc TYPE option<string>;
DEFINE FIELD appearance ON npc TYPE option<string>;
DEFINE FIELD backstory ON npc TYPE option<string>;
DEFINE FIELD campaign ON npc TYPE option<record<campaign>>;
DEFINE FIELD faction ON npc TYPE option<array<record<faction>>>;
DEFINE FIELD location ON npc TYPE option<record<location>>;
DEFINE FIELD tags ON npc TYPE option<array<string>>;
DEFINE FIELD created_at ON npc TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON npc TYPE datetime DEFAULT time::now();
DEFINE FIELD metadata ON npc TYPE option<object>;

DEFINE INDEX npc_name ON npc FIELDS name SEARCH ANALYZER exact_analyzer BM25;
DEFINE INDEX npc_campaign ON npc FIELDS campaign;

-- NPC relationship edges (graph)
DEFINE TABLE npc_relation SCHEMAFULL;
DEFINE FIELD in ON npc_relation TYPE record<npc>;
DEFINE FIELD out ON npc_relation TYPE record<npc>;
DEFINE FIELD relation_type ON npc_relation TYPE string;  -- "allied", "hostile", "neutral", "knows"
DEFINE FIELD strength ON npc_relation TYPE option<float>;  -- 0.0 to 1.0
DEFINE FIELD notes ON npc_relation TYPE option<string>;

-- ============================================================================
-- SESSION TABLE
-- ============================================================================

DEFINE TABLE session SCHEMAFULL;
DEFINE FIELD campaign ON session TYPE record<campaign>;
DEFINE FIELD name ON session TYPE option<string>;
DEFINE FIELD session_number ON session TYPE option<int>;
DEFINE FIELD date ON session TYPE option<datetime>;
DEFINE FIELD summary ON session TYPE option<string>;
DEFINE FIELD notes ON session TYPE option<string>;
DEFINE FIELD status ON session TYPE string DEFAULT "planned";  -- planned, in_progress, completed
DEFINE FIELD created_at ON session TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON session TYPE datetime DEFAULT time::now();

DEFINE INDEX session_campaign ON session FIELDS campaign;
DEFINE INDEX session_status ON session FIELDS status;

-- ============================================================================
-- CHAT MESSAGE TABLE
-- ============================================================================

DEFINE TABLE chat_message SCHEMAFULL;
DEFINE FIELD session_id ON chat_message TYPE string;  -- Chat session UUID
DEFINE FIELD role ON chat_message TYPE string;  -- "user", "assistant", "system"
DEFINE FIELD content ON chat_message TYPE string;
DEFINE FIELD campaign ON chat_message TYPE option<record<campaign>>;
DEFINE FIELD npc ON chat_message TYPE option<record<npc>>;  -- For NPC conversations
DEFINE FIELD sources ON chat_message TYPE option<array<string>>;  -- RAG source IDs
DEFINE FIELD created_at ON chat_message TYPE datetime DEFAULT time::now();
DEFINE FIELD metadata ON chat_message TYPE option<object>;

DEFINE INDEX chat_session ON chat_message FIELDS session_id;
DEFINE INDEX chat_campaign ON chat_message FIELDS campaign;
DEFINE INDEX chat_created ON chat_message FIELDS created_at;

-- ============================================================================
-- LIBRARY ITEM TABLE (documents)
-- ============================================================================

DEFINE TABLE library_item SCHEMAFULL;
DEFINE FIELD slug ON library_item TYPE string;
DEFINE FIELD title ON library_item TYPE string;
DEFINE FIELD file_path ON library_item TYPE option<string>;
DEFINE FIELD file_type ON library_item TYPE option<string>;
DEFINE FIELD file_size ON library_item TYPE option<int>;
DEFINE FIELD page_count ON library_item TYPE option<int>;
DEFINE FIELD game_system ON library_item TYPE option<string>;
DEFINE FIELD game_system_id ON library_item TYPE option<string>;
DEFINE FIELD content_category ON library_item TYPE option<string>;  -- rulebook, adventure, sourcebook
DEFINE FIELD publisher ON library_item TYPE option<string>;
DEFINE FIELD status ON library_item TYPE string DEFAULT "pending";  -- pending, processing, ready, error
DEFINE FIELD error_message ON library_item TYPE option<string>;
DEFINE FIELD created_at ON library_item TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON library_item TYPE datetime DEFAULT time::now();
DEFINE FIELD metadata ON library_item TYPE option<object>;

DEFINE INDEX library_slug ON library_item FIELDS slug UNIQUE;
DEFINE INDEX library_status ON library_item FIELDS status;
DEFINE INDEX library_game ON library_item FIELDS game_system_id;

-- ============================================================================
-- CHUNK TABLE (document chunks with vectors)
-- ============================================================================

DEFINE TABLE chunk SCHEMAFULL;
DEFINE FIELD content ON chunk TYPE string;
DEFINE FIELD library_item ON chunk TYPE record<library_item>;
DEFINE FIELD content_type ON chunk TYPE string;  -- rules, fiction, session_notes, homebrew
DEFINE FIELD page_number ON chunk TYPE option<int>;
DEFINE FIELD page_start ON chunk TYPE option<int>;
DEFINE FIELD page_end ON chunk TYPE option<int>;
DEFINE FIELD chunk_index ON chunk TYPE option<int>;
DEFINE FIELD section_path ON chunk TYPE option<string>;
DEFINE FIELD chapter_title ON chunk TYPE option<string>;
DEFINE FIELD section_title ON chunk TYPE option<string>;
DEFINE FIELD chunk_type ON chunk TYPE option<string>;  -- text, stat_block, table, spell
DEFINE FIELD semantic_keywords ON chunk TYPE option<array<string>>;
DEFINE FIELD embedding ON chunk TYPE option<array<float>>;
DEFINE FIELD embedding_model ON chunk TYPE option<string>;
DEFINE FIELD created_at ON chunk TYPE datetime DEFAULT time::now();
DEFINE FIELD metadata ON chunk TYPE option<object>;

-- Full-text index on content
DEFINE INDEX chunk_content ON chunk FIELDS content SEARCH ANALYZER ttrpg_analyzer BM25 HIGHLIGHTS;

-- Vector index (HNSW) - dimension set dynamically based on embedder
-- Default: 768 for nomic-embed-text
DEFINE INDEX chunk_embedding ON chunk FIELDS embedding HNSW DIMENSION 768 DIST COSINE EFC 150 M 12;

-- Filtering indexes
DEFINE INDEX chunk_library ON chunk FIELDS library_item;
DEFINE INDEX chunk_type ON chunk FIELDS content_type;
DEFINE INDEX chunk_page ON chunk FIELDS page_number;

-- ============================================================================
-- CHUNK RELATIONS (for cross-references)
-- ============================================================================

DEFINE TABLE chunk_reference SCHEMAFULL;
DEFINE FIELD in ON chunk_reference TYPE record<chunk>;
DEFINE FIELD out ON chunk_reference TYPE record<chunk>;
DEFINE FIELD reference_type ON chunk_reference TYPE string;  -- "continues", "references", "see_also"

-- ============================================================================
-- FACTION TABLE (for graph relations)
-- ============================================================================

DEFINE TABLE faction SCHEMAFULL;
DEFINE FIELD name ON faction TYPE string;
DEFINE FIELD description ON faction TYPE option<string>;
DEFINE FIELD campaign ON faction TYPE record<campaign>;
DEFINE FIELD alignment ON faction TYPE option<string>;
DEFINE FIELD created_at ON faction TYPE datetime DEFAULT time::now();

DEFINE INDEX faction_campaign ON faction FIELDS campaign;

-- ============================================================================
-- LOCATION TABLE (for graph relations)
-- ============================================================================

DEFINE TABLE location SCHEMAFULL;
DEFINE FIELD name ON location TYPE string;
DEFINE FIELD description ON location TYPE option<string>;
DEFINE FIELD campaign ON location TYPE record<campaign>;
DEFINE FIELD parent_location ON location TYPE option<record<location>>;
DEFINE FIELD location_type ON location TYPE option<string>;  -- city, dungeon, wilderness, etc.
DEFINE FIELD created_at ON location TYPE datetime DEFAULT time::now();

DEFINE INDEX location_campaign ON location FIELDS campaign;
DEFINE INDEX location_parent ON location FIELDS parent_location;
"#;
```

### 2.3 Hybrid Search Engine

**Location**: `src-tauri/src/core/storage/search.rs`

```rust
use serde::{Deserialize, Serialize};
use surrealdb::{Surreal, engine::local::RocksDb, sql::Value};

/// Configuration for hybrid search operations.
#[derive(Clone, Debug)]
pub struct HybridSearchConfig {
    /// Weight for vector (semantic) search results (0.0 - 1.0)
    pub semantic_weight: f32,
    /// Weight for full-text (keyword) search results (0.0 - 1.0)
    pub keyword_weight: f32,
    /// Maximum results to return
    pub limit: usize,
    /// Minimum score threshold (0.0 - 1.0)
    pub min_score: f32,
    /// Score normalization method
    pub normalization: ScoreNormalization,
}

#[derive(Clone, Debug, Default)]
pub enum ScoreNormalization {
    #[default]
    MinMax,
    ZScore,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            semantic_weight: 0.6,
            keyword_weight: 0.4,
            limit: 10,
            min_score: 0.1,
            normalization: ScoreNormalization::MinMax,
        }
    }
}

impl HybridSearchConfig {
    /// Create config from semantic_ratio (0.0 = keyword only, 1.0 = semantic only)
    pub fn from_semantic_ratio(ratio: f32) -> Self {
        Self {
            semantic_weight: ratio.clamp(0.0, 1.0),
            keyword_weight: (1.0 - ratio).clamp(0.0, 1.0),
            ..Default::default()
        }
    }
}

/// Search result with fused score.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub source: String,
    pub page_number: Option<i32>,
    pub section_path: Option<String>,
    pub content_type: String,
    pub highlights: Option<String>,
}

/// Perform hybrid search combining vector and full-text.
pub async fn hybrid_search(
    db: &Surreal<RocksDb>,
    query: &str,
    query_embedding: Vec<f32>,
    config: &HybridSearchConfig,
    filters: Option<&str>,
) -> Result<Vec<SearchResult>, StorageError> {
    let norm = match config.normalization {
        ScoreNormalization::MinMax => "minmax",
        ScoreNormalization::ZScore => "zscore",
    };

    // Build filter clause
    let filter_clause = filters.map(|f| format!("AND {}", f)).unwrap_or_default();

    let query = format!(r#"
        -- Vector search: top K nearest neighbors
        LET $vec_results = SELECT
            id,
            content,
            library_item.slug as source,
            page_number,
            section_path,
            content_type,
            vector::distance::knn() as distance
        FROM chunk
        WHERE embedding <|{limit},COSINE|> $embedding
        {filter_clause};

        -- Full-text search: BM25 ranking
        LET $ft_results = SELECT
            id,
            content,
            library_item.slug as source,
            page_number,
            section_path,
            content_type,
            search::score(1) as score,
            search::highlight('<mark>', '</mark>', 1) as highlights
        FROM chunk
        WHERE content @1@ $query
        {filter_clause}
        ORDER BY score DESC
        LIMIT {limit};

        -- Fuse results with linear combination
        search::linear(
            [$vec_results, $ft_results],
            [{semantic_weight}, {keyword_weight}],
            {limit},
            '{norm}'
        );
    "#,
        limit = config.limit * 2,  // Fetch more for fusion
        filter_clause = filter_clause,
        semantic_weight = config.semantic_weight,
        keyword_weight = config.keyword_weight,
        norm = norm,
    );

    let mut response = db
        .query(&query)
        .bind(("embedding", query_embedding))
        .bind(("query", query))
        .await?;

    let results: Vec<SearchResult> = response.take(2)?;

    // Apply minimum score threshold and limit
    Ok(results
        .into_iter()
        .filter(|r| r.score >= config.min_score)
        .take(config.limit)
        .collect())
}

/// Perform vector-only search (KNN).
pub async fn vector_search(
    db: &Surreal<RocksDb>,
    embedding: Vec<f32>,
    limit: usize,
    filters: Option<&str>,
) -> Result<Vec<SearchResult>, StorageError> {
    let filter_clause = filters.map(|f| format!("AND {}", f)).unwrap_or_default();

    let query = format!(r#"
        SELECT
            id,
            content,
            library_item.slug as source,
            page_number,
            section_path,
            content_type,
            vector::distance::knn() as score
        FROM chunk
        WHERE embedding <|{limit},COSINE|> $embedding
        {filter_clause}
        ORDER BY score;
    "#);

    let mut response = db
        .query(&query)
        .bind(("embedding", embedding))
        .await?;

    response.take(0)
}

/// Perform full-text only search (BM25).
pub async fn fulltext_search(
    db: &Surreal<RocksDb>,
    query: &str,
    limit: usize,
    filters: Option<&str>,
) -> Result<Vec<SearchResult>, StorageError> {
    let filter_clause = filters.map(|f| format!("AND {}", f)).unwrap_or_default();

    let query_str = format!(r#"
        SELECT
            id,
            content,
            library_item.slug as source,
            page_number,
            section_path,
            content_type,
            search::score(1) as score,
            search::highlight('<mark>', '</mark>', 1) as highlights
        FROM chunk
        WHERE content @1@ $query
        {filter_clause}
        ORDER BY score DESC
        LIMIT {limit};
    "#);

    let mut response = db
        .query(&query_str)
        .bind(("query", query))
        .await?;

    response.take(0)
}
```

### 2.4 AppState Changes

**Location**: `src-tauri/src/commands/state.rs`

```rust
// REMOVE these fields (after migration):
// pub search_client: Arc<SearchClient>,
// pub embedded_search: Arc<EmbeddedSearch>,
// pub db_pool: sqlx::SqlitePool,

// REPLACE with:
pub struct AppState {
    /// Unified SurrealDB storage (replaces SQLite + Meilisearch)
    pub storage: Arc<SurrealStorage>,

    /// LLM router (unchanged)
    pub llm_router: Arc<RwLock<LlmRouter>>,

    /// Embedding cache (can be shared with SurrealDB or kept separate)
    pub embedding_cache: Arc<EmbeddingCache>,

    /// App configuration
    pub config: Arc<RwLock<AppConfig>>,
}

impl AppState {
    pub async fn new(data_dir: PathBuf) -> Result<Self, AppInitError> {
        let db_path = data_dir.join("surrealdb");

        let storage = SurrealStorage::new(db_path)
            .await
            .map_err(AppInitError::Storage)?;

        Ok(Self {
            storage: Arc::new(storage),
            llm_router: Arc::new(RwLock::new(LlmRouter::default())),
            embedding_cache: Arc::new(EmbeddingCache::new()),
            config: Arc::new(RwLock::new(AppConfig::default())),
        })
    }
}
```

### 2.5 Document Ingestion Pipeline

**Location**: `src-tauri/src/core/storage/ingestion.rs`

```rust
use super::{SurrealStorage, StorageError};
use crate::ingestion::chunker::ChunkedDocument;

/// Ingest document chunks into SurrealDB.
pub async fn ingest_chunks(
    storage: &SurrealStorage,
    library_item_id: &str,
    chunks: Vec<ChunkedDocument>,
    embeddings: Option<Vec<Vec<f32>>>,
) -> Result<usize, StorageError> {
    let db = storage.db();

    // Start transaction for atomic ingestion
    db.query("BEGIN TRANSACTION").await?;

    let mut inserted = 0;

    for (i, chunk) in chunks.into_iter().enumerate() {
        let embedding = embeddings.as_ref().and_then(|e| e.get(i).cloned());

        let chunk_id = format!("chunk:{}-{}", library_item_id, i);

        let query = r#"
            CREATE type::thing('chunk', $id) CONTENT {
                content: $content,
                library_item: type::thing('library_item', $library_id),
                content_type: $content_type,
                page_number: $page_number,
                page_start: $page_start,
                page_end: $page_end,
                chunk_index: $chunk_index,
                section_path: $section_path,
                chapter_title: $chapter_title,
                section_title: $section_title,
                chunk_type: $chunk_type,
                semantic_keywords: $keywords,
                embedding: $embedding,
                embedding_model: $embedding_model,
                metadata: $metadata
            };
        "#;

        db.query(query)
            .bind(("id", &chunk_id))
            .bind(("content", &chunk.content))
            .bind(("library_id", library_item_id))
            .bind(("content_type", &chunk.content_type.unwrap_or_default()))
            .bind(("page_number", chunk.page_number))
            .bind(("page_start", chunk.page_start))
            .bind(("page_end", chunk.page_end))
            .bind(("chunk_index", i as i32))
            .bind(("section_path", &chunk.section_path))
            .bind(("chapter_title", &chunk.chapter_title))
            .bind(("section_title", &chunk.section_title))
            .bind(("chunk_type", &chunk.chunk_type))
            .bind(("keywords", &chunk.semantic_keywords))
            .bind(("embedding", embedding))
            .bind(("embedding_model", &chunk.embedding_model))
            .bind(("metadata", &chunk.metadata))
            .await?;

        inserted += 1;
    }

    // Update library item status
    db.query(r#"
        UPDATE type::thing('library_item', $id) SET
            status = 'ready',
            updated_at = time::now();
    "#)
    .bind(("id", library_item_id))
    .await?;

    db.query("COMMIT TRANSACTION").await?;

    Ok(inserted)
}

/// Delete all chunks for a library item.
pub async fn delete_library_chunks(
    storage: &SurrealStorage,
    library_item_id: &str,
) -> Result<usize, StorageError> {
    let db = storage.db();

    let result = db.query(r#"
        DELETE chunk WHERE library_item = type::thing('library_item', $id);
    "#)
    .bind(("id", library_item_id))
    .await?;

    // Return count of deleted records
    Ok(result.num_statements())
}
```

### 2.6 RAG Pipeline

**Location**: `src-tauri/src/core/storage/rag.rs`

```rust
use super::{SurrealStorage, search::{hybrid_search, HybridSearchConfig, SearchResult}};
use crate::core::llm_router::LlmRouter;

/// RAG configuration for TTRPG queries.
#[derive(Clone, Debug)]
pub struct RagConfig {
    pub search_config: HybridSearchConfig,
    pub max_context_chunks: usize,
    pub max_context_bytes: usize,
    pub include_sources: bool,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            search_config: HybridSearchConfig::default(),
            max_context_chunks: 8,
            max_context_bytes: 4000,
            include_sources: true,
        }
    }
}

/// RAG response with sources.
#[derive(Clone, Debug, Serialize)]
pub struct RagResponse {
    pub content: String,
    pub sources: Vec<RagSource>,
    pub context_used: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct RagSource {
    pub id: String,
    pub title: String,
    pub page: Option<i32>,
    pub relevance: f32,
}

/// Execute RAG query.
pub async fn rag_query(
    storage: &SurrealStorage,
    llm_router: &LlmRouter,
    query: &str,
    embedding: Vec<f32>,
    config: &RagConfig,
    filters: Option<&str>,
) -> Result<RagResponse, StorageError> {
    // 1. Retrieve context via hybrid search
    let results = hybrid_search(
        storage.db(),
        query,
        embedding,
        &config.search_config,
        filters,
    ).await?;

    // 2. Format context with templates
    let (context, sources) = format_context(&results, config)?;

    // 3. Build prompt
    let system_prompt = build_system_prompt(&context);

    // 4. Call LLM (using existing router)
    let response = llm_router
        .chat_completion(&system_prompt, query)
        .await
        .map_err(|e| StorageError::LlmError(e.to_string()))?;

    Ok(RagResponse {
        content: response,
        sources,
        context_used: context.len(),
    })
}

/// Format search results into context for LLM.
fn format_context(
    results: &[SearchResult],
    config: &RagConfig,
) -> Result<(String, Vec<RagSource>), StorageError> {
    let mut context = String::new();
    let mut sources = Vec::new();
    let mut total_bytes = 0;

    for (i, result) in results.iter().take(config.max_context_chunks).enumerate() {
        let formatted = format!(
            "[{}] {} (p.{})\n{}\n\n",
            i + 1,
            result.source,
            result.page_number.map(|p| p.to_string()).unwrap_or_default(),
            result.content
        );

        if total_bytes + formatted.len() > config.max_context_bytes {
            break;
        }

        context.push_str(&formatted);
        total_bytes += formatted.len();

        sources.push(RagSource {
            id: result.id.clone(),
            title: result.source.clone(),
            page: result.page_number,
            relevance: result.score,
        });
    }

    Ok((context, sources))
}

/// Build system prompt with context.
fn build_system_prompt(context: &str) -> String {
    format!(r#"
You are an expert TTRPG Game Master assistant with deep knowledge of tabletop roleplaying games.

## Context from Indexed Rulebooks

{context}

## Instructions

1. Use ONLY the provided context to answer questions
2. Always cite your sources with [N] references
3. If the context doesn't contain enough information, say so clearly
4. Format rules and mechanics clearly for quick reference
5. Distinguish between core rules and optional/variant rules
"#, context = context)
}
```

---

## 3. Data Migration

### 3.1 Migration Strategy

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Migration Pipeline                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Phase 1: Pre-Migration                                              │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │ 1. Detect existing data (SQLite + Meilisearch)               │   │
│  │ 2. Create backup of both databases                           │   │
│  │ 3. Calculate expected record counts                          │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                              │                                       │
│                              ▼                                       │
│  Phase 2: SQLite Migration                                          │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │ 1. campaigns → campaign table                                │   │
│  │ 2. npcs → npc table (with campaign record links)             │   │
│  │ 3. sessions → session table                                  │   │
│  │ 4. chat_messages → chat_message table                        │   │
│  │ 5. library_documents → library_item table                    │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                              │                                       │
│                              ▼                                       │
│  Phase 3: Meilisearch Migration                                     │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │ 1. Extract all documents from each index                     │   │
│  │ 2. Map to chunk table with content_type field                │   │
│  │ 3. Preserve embeddings if dimensions match                   │   │
│  │ 4. Link chunks to library_item via record links              │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                              │                                       │
│                              ▼                                       │
│  Phase 4: Validation                                                │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │ 1. Verify record counts match                                │   │
│  │ 2. Test sample queries                                       │   │
│  │ 3. Verify graph relations                                    │   │
│  │ 4. Mark migration complete                                   │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.2 Migration Module

**Location**: `src-tauri/src/core/storage/migration.rs`

```rust
use super::{SurrealStorage, StorageError};
use sqlx::SqlitePool;
use meilisearch_lib::MeilisearchLib;

/// Migration status tracking.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MigrationStatus {
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub phase: MigrationPhase,
    pub records_migrated: MigrationCounts,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum MigrationPhase {
    #[default]
    NotStarted,
    BackingUp,
    MigratingSqlite,
    MigratingMeilisearch,
    Validating,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MigrationCounts {
    pub campaigns: usize,
    pub npcs: usize,
    pub sessions: usize,
    pub chat_messages: usize,
    pub library_items: usize,
    pub chunks: usize,
}

/// Run full migration from SQLite + Meilisearch to SurrealDB.
pub async fn run_migration(
    storage: &SurrealStorage,
    sqlite_pool: &SqlitePool,
    meilisearch: &MeilisearchLib,
    on_progress: impl Fn(MigrationStatus),
) -> Result<MigrationStatus, StorageError> {
    let mut status = MigrationStatus::default();
    status.started_at = Some(Utc::now());

    // Phase 1: Backup
    status.phase = MigrationPhase::BackingUp;
    on_progress(status.clone());
    create_backups(sqlite_pool, meilisearch).await?;

    // Phase 2: SQLite migration
    status.phase = MigrationPhase::MigratingSqlite;
    on_progress(status.clone());

    status.records_migrated.campaigns = migrate_campaigns(storage, sqlite_pool).await?;
    status.records_migrated.npcs = migrate_npcs(storage, sqlite_pool).await?;
    status.records_migrated.sessions = migrate_sessions(storage, sqlite_pool).await?;
    status.records_migrated.chat_messages = migrate_chat_messages(storage, sqlite_pool).await?;
    status.records_migrated.library_items = migrate_library_items(storage, sqlite_pool).await?;

    // Phase 3: Meilisearch migration
    status.phase = MigrationPhase::MigratingMeilisearch;
    on_progress(status.clone());

    status.records_migrated.chunks = migrate_meilisearch_indexes(storage, meilisearch).await?;

    // Phase 4: Validation
    status.phase = MigrationPhase::Validating;
    on_progress(status.clone());

    validate_migration(storage, &status.records_migrated).await?;

    status.phase = MigrationPhase::Completed;
    status.completed_at = Some(Utc::now());
    on_progress(status.clone());

    Ok(status)
}

async fn migrate_campaigns(
    storage: &SurrealStorage,
    sqlite: &SqlitePool,
) -> Result<usize, StorageError> {
    let campaigns = sqlx::query_as!(Campaign, "SELECT * FROM campaigns")
        .fetch_all(sqlite)
        .await?;

    let db = storage.db();
    let mut count = 0;

    for campaign in campaigns {
        db.query(r#"
            CREATE campaign CONTENT {
                id: type::string($id),
                name: $name,
                description: $description,
                game_system: $game_system,
                game_system_id: $game_system_id,
                status: $status,
                created_at: type::datetime($created_at),
                updated_at: type::datetime($updated_at),
                metadata: $metadata
            };
        "#)
        .bind(("id", &campaign.id))
        .bind(("name", &campaign.name))
        .bind(("description", &campaign.description))
        .bind(("game_system", &campaign.game_system))
        .bind(("game_system_id", &campaign.game_system_id))
        .bind(("status", &campaign.status))
        .bind(("created_at", &campaign.created_at))
        .bind(("updated_at", &campaign.updated_at))
        .bind(("metadata", &campaign.metadata))
        .await?;

        count += 1;
    }

    Ok(count)
}

// Similar implementations for other tables...
```

---

## 4. Tauri Command Updates

### 4.1 Search Commands

**Location**: `src-tauri/src/commands/search/query.rs`

```rust
use crate::core::storage::{SurrealStorage, search::{hybrid_search, HybridSearchConfig}};

/// Perform hybrid search (updated to use SurrealDB).
#[tauri::command]
pub async fn search(
    state: State<'_, AppState>,
    query: String,
    semantic_ratio: Option<f32>,
    filters: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SearchHit>, String> {
    let storage = &state.storage;

    // Generate embedding for query
    let embedding = state.embedding_cache
        .get_or_generate(&query)
        .await
        .map_err(|e| e.to_string())?;

    // Configure search
    let config = HybridSearchConfig {
        semantic_weight: semantic_ratio.unwrap_or(0.6),
        keyword_weight: 1.0 - semantic_ratio.unwrap_or(0.6),
        limit: limit.unwrap_or(10),
        ..Default::default()
    };

    // Execute search
    let results = hybrid_search(
        storage.db(),
        &query,
        embedding,
        &config,
        filters.as_deref(),
    ).await.map_err(|e| e.to_string())?;

    // Convert to frontend format
    Ok(results.into_iter().map(SearchHit::from).collect())
}
```

### 4.2 RAG Commands

**Location**: `src-tauri/src/commands/search/rag.rs`

```rust
use crate::core::storage::rag::{rag_query, RagConfig, RagResponse};

/// Execute RAG query (non-streaming).
#[tauri::command]
pub async fn rag_query_cmd(
    state: State<'_, AppState>,
    question: String,
    content_types: Option<Vec<String>>,
    conversation: Option<Vec<ChatMessage>>,
) -> Result<RagResponse, String> {
    let storage = &state.storage;
    let llm_router = state.llm_router.read().await;

    // Generate embedding
    let embedding = state.embedding_cache
        .get_or_generate(&question)
        .await
        .map_err(|e| e.to_string())?;

    // Build filter from content types
    let filter = content_types.map(|types| {
        let type_list = types.iter()
            .map(|t| format!("'{}'", t))
            .collect::<Vec<_>>()
            .join(", ");
        format!("content_type IN [{}]", type_list)
    });

    // Execute RAG
    let response = rag_query(
        storage,
        &llm_router,
        &question,
        embedding,
        &RagConfig::default(),
        filter.as_deref(),
    ).await.map_err(|e| e.to_string())?;

    Ok(response)
}

/// Execute RAG query with streaming.
#[tauri::command]
pub async fn rag_query_stream(
    state: State<'_, AppState>,
    window: Window,
    question: String,
    stream_id: String,
    content_types: Option<Vec<String>>,
) -> Result<(), String> {
    let storage = state.storage.clone();
    let llm_router = state.llm_router.clone();
    let embedding_cache = state.embedding_cache.clone();

    tokio::spawn(async move {
        // Generate embedding
        let embedding = match embedding_cache.get_or_generate(&question).await {
            Ok(e) => e,
            Err(e) => {
                let _ = window.emit(&format!("rag-error-{}", stream_id), e.to_string());
                return;
            }
        };

        // Build filter
        let filter = content_types.map(|types| {
            let type_list = types.iter()
                .map(|t| format!("'{}'", t))
                .collect::<Vec<_>>()
                .join(", ");
            format!("content_type IN [{}]", type_list)
        });

        // Execute hybrid search for context
        let results = match hybrid_search(
            storage.db(),
            &question,
            embedding,
            &HybridSearchConfig::default(),
            filter.as_deref(),
        ).await {
            Ok(r) => r,
            Err(e) => {
                let _ = window.emit(&format!("rag-error-{}", stream_id), e.to_string());
                return;
            }
        };

        // Format context and stream LLM response
        let router = llm_router.read().await;
        match router.stream_with_context(&question, &results, |chunk| {
            let _ = window.emit(&format!("rag-chunk-{}", stream_id), &chunk);
        }).await {
            Ok(sources) => {
                let _ = window.emit(&format!("rag-complete-{}", stream_id), sources);
            }
            Err(e) => {
                let _ = window.emit(&format!("rag-error-{}", stream_id), e.to_string());
            }
        }
    });

    Ok(())
}
```

---

## 5. Error Handling

### 5.1 Error Types

**Location**: `src-tauri/src/core/storage/error.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] surrealdb::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Initialization failed: {0}")]
    Init(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Schema migration failed: {0}")]
    Migration(String),

    #[error("Record not found: {0}")]
    NotFound(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl From<StorageError> for String {
    fn from(e: StorageError) -> String {
        e.to_string()
    }
}
```

---

## 6. Testing Strategy

### 6.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_test_db() -> (TempDir, SurrealStorage) {
        let temp_dir = TempDir::new().unwrap();
        let storage = SurrealStorage::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();
        (temp_dir, storage)
    }

    #[tokio::test]
    async fn test_hybrid_search() {
        let (_dir, storage) = setup_test_db().await;

        // Insert test data with embeddings
        let embedding = vec![0.1f32; 768];
        storage.db().query(r#"
            CREATE chunk CONTENT {
                content: "Flanking gives advantage on attack rolls",
                library_item: library_item:test,
                content_type: "rules",
                page_number: 251,
                embedding: $embedding
            };
        "#)
        .bind(("embedding", &embedding))
        .await
        .unwrap();

        // Search
        let results = hybrid_search(
            storage.db(),
            "flanking advantage",
            embedding.clone(),
            &HybridSearchConfig::default(),
            None,
        ).await.unwrap();

        assert!(!results.is_empty());
        assert!(results[0].content.contains("Flanking"));
    }

    #[tokio::test]
    async fn test_graph_relations() {
        let (_dir, storage) = setup_test_db().await;

        // Create campaign and NPC with relation
        storage.db().query(r#"
            CREATE campaign:test CONTENT { name: "Test Campaign" };
            CREATE npc:alice CONTENT {
                name: "Alice",
                campaign: campaign:test
            };
            CREATE npc:bob CONTENT {
                name: "Bob",
                campaign: campaign:test
            };
            RELATE npc:alice->npc_relation->npc:bob CONTENT {
                relation_type: "allied",
                strength: 0.8
            };
        "#).await.unwrap();

        // Query allies
        let allies: Vec<String> = storage.db()
            .query("SELECT out.name FROM npc_relation WHERE in = npc:alice AND relation_type = 'allied'")
            .await
            .unwrap()
            .take(0)
            .unwrap();

        assert_eq!(allies, vec!["Bob"]);
    }
}
```

### 6.2 Integration Tests

- Test full document ingestion → search flow
- Test migration from SQLite + Meilisearch
- Test RAG pipeline end-to-end
- Test streaming responses
- Test error handling and recovery

---

## 7. File Structure

```
src-tauri/src/core/storage/
├── mod.rs                 # Module exports
├── surrealdb.rs           # SurrealStorage wrapper
├── schema.rs              # Database schema definitions
├── search.rs              # Hybrid, vector, and fulltext search
├── rag.rs                 # RAG pipeline
├── ingestion.rs           # Document chunk ingestion
├── migration.rs           # SQLite/Meilisearch migration
├── error.rs               # Error types
└── models.rs              # Data models (Campaign, NPC, Chunk, etc.)

src-tauri/src/commands/
├── search/
│   ├── query.rs           # Updated search commands
│   ├── rag.rs             # Updated RAG commands
│   └── embeddings.rs      # Embedder configuration
└── ...
```

---

## 8. Decisions Log

### Decision 1: Full SurrealDB Replacement (Not Hybrid)

**Context**: Considered keeping SQLite for relational data

**Decision**: Replace both SQLite and Meilisearch with SurrealDB

**Rationale**:
- SurrealDB handles relational, document, graph, and vector models
- Single database simplifies architecture
- Record links provide type-safe foreign keys
- No synchronization between databases

### Decision 2: RocksDB Storage Engine

**Context**: SurrealDB supports multiple storage backends

**Decision**: Use RocksDB for persistence

**Rationale**:
- Proven production stability
- WAL ensures durability
- Works embedded without external process
- Good performance for our scale

### Decision 3: Custom RAG Pipeline (Not SurrealDB ML)

**Context**: SurrealDB has ML capabilities via `.surml` models

**Decision**: Keep existing LLM router for RAG generation

**Rationale**:
- Existing multi-provider support (Claude, GPT-4, Ollama)
- Streaming support already implemented
- More flexibility for prompt engineering
- SurrealDB ML focuses on in-database inference, not chat completion

### Decision 4: search::linear() for Fusion

**Context**: Multiple fusion algorithms available (RRF, weighted sum, etc.)

**Decision**: Use SurrealDB's native `search::linear()` function

**Rationale**:
- Native to SurrealQL, optimized for performance
- Supports configurable weights
- Supports normalization (minmax, zscore)
- Preserves semantic_ratio compatibility
