# Query Preprocessing Architecture Design

## Overview

This document specifies the technical design for a backend-agnostic query preprocessing layer that provides typo tolerance and synonym expansion. This eliminates the need for Meilisearch's built-in typo/synonym features while giving us more control over domain-specific behavior.

```
User Query: "firball damge resistence"
       │
       ▼
┌──────────────────────────────┐
│  1. Normalize                │  → "firball damge resistence"
│     (trim, lowercase)        │
└──────────────┬───────────────┘
               ▼
┌──────────────────────────────┐
│  2. Typo Correction          │  "firball" → "fireball"
│     (SymSpell + corpus)      │  "damge" → "damage"
│                              │  "resistence" → "resistance"
└──────────────┬───────────────┘
               ▼
┌──────────────────────────────┐
│  3. Synonym Expansion        │  "fireball" → ["fireball", "fire bolt"]
│     (domain dictionary)      │  "damage" → ["damage", "harm"]
│                              │  "resistance" → ["resistance", "immunity"]
└──────────────┬───────────────┘
               ▼
┌──────────────────────────────────────────────────┐
│  4. Query Generation                              │
│  ┌─────────────────┐  ┌────────────────────────┐ │
│  │ BM25 FTS Query  │  │ Embedding Text         │ │
│  │ (OR-expanded)   │  │ (corrected only)       │ │
│  └────────┬────────┘  └───────────┬────────────┘ │
│           └──────────┬────────────┘              │
└──────────────────────┼───────────────────────────┘
                       ▼
                 Search Backends
```

---

## Module Structure

```
src-tauri/src/core/
├── preprocess/
│   ├── mod.rs              # Module exports
│   ├── typo.rs             # TypoCorrector with SymSpell
│   ├── synonyms.rs         # SynonymMap and expansion
│   ├── pipeline.rs         # QueryPipeline orchestrator
│   ├── dictionary.rs       # Corpus dictionary generation
│   └── config.rs           # TypoConfig, SynonymConfig
└── storage/
    └── search.rs           # Updated to use preprocessed queries
```

---

## Component Design

### 1. TypoCorrector (typo.rs)

**Purpose:** Correct spelling errors using SymSpell with domain-specific dictionaries.

```rust
use symspell::{SymSpell, UnicodeStringStrategy, Verbosity};
use std::collections::HashSet;

/// Configuration matching Meilisearch's typo tolerance behavior.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypoConfig {
    /// Minimum word length to allow 1 typo (default: 5)
    pub min_word_size_one_typo: usize,
    /// Minimum word length to allow 2 typos (default: 9)
    pub min_word_size_two_typos: usize,
    /// Words where typo tolerance is disabled entirely
    pub disabled_on_words: Vec<String>,
    /// Path to English frequency dictionary
    pub english_dict_path: PathBuf,
    /// Path to TTRPG corpus dictionary (generated)
    pub corpus_dict_path: PathBuf,
    /// Path to bigram dictionary for compounds
    pub bigram_dict_path: PathBuf,
}

/// Spelling correction engine with domain-specific vocabulary.
pub struct TypoCorrector {
    engine: SymSpell<UnicodeStringStrategy>,
    protected_words: HashSet<String>,
    config: TypoConfig,
}

impl TypoCorrector {
    /// Initialize with layered dictionaries.
    pub fn new(config: TypoConfig) -> Result<Self, TypoError>;

    /// Reload dictionaries (after corpus update).
    pub fn reload_dictionaries(&mut self) -> Result<(), TypoError>;

    /// Correct a full search query.
    /// Returns (corrected_query, Vec<(original, correction)>).
    pub fn correct_query(&self, query: &str) -> (String, Vec<Correction>);
}

#[derive(Clone, Debug)]
pub struct Correction {
    pub original: String,
    pub corrected: String,
    pub edit_distance: usize,
}
```

**Dictionary Layering Strategy:**
1. **Layer 1 - English Base**: Standard 82K word frequency dictionary (ships with symspell crate)
2. **Layer 2 - TTRPG Corpus**: Generated from indexed content, 10x frequency boost so domain terms win
3. **Layer 3 - Bigrams**: Common TTRPG compounds ("magic missile", "hit points")

**Edit Distance Rules (Meilisearch-compatible):**
- Words < 5 chars: No correction
- Words 5-8 chars: Max 1 edit distance
- Words ≥ 9 chars: Max 2 edit distance

### 2. SynonymMap (synonyms.rs)

**Purpose:** Expand search terms with TTRPG-specific synonyms.

```rust
use std::collections::{HashMap, HashSet};

/// Bidirectional synonym map supporting multi-way and one-way synonyms.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SynonymMap {
    /// Multi-way synonym groups: all terms interchangeable
    multi_way: Vec<HashSet<String>>,
    /// One-way synonyms: source → targets only
    one_way: HashMap<String, Vec<String>>,
    /// Maximum expansions per term (prevents query explosion)
    max_expansions: usize,
}

impl SynonymMap {
    /// Create empty map with expansion limit.
    pub fn new(max_expansions: usize) -> Self;

    /// Load from TOML configuration file.
    pub fn from_toml(path: &Path) -> Result<Self, SynonymError>;

    /// Add multi-way synonym group.
    pub fn add_multi_way(&mut self, terms: &[&str]);

    /// Add one-way synonym mapping.
    pub fn add_one_way(&mut self, source: &str, targets: &[&str]);

    /// Expand a single term to its synonyms.
    pub fn expand_term(&self, term: &str) -> Vec<String>;

    /// Expand all terms in a query.
    pub fn expand_query(&self, query: &str) -> ExpandedQuery;
}

/// Result of synonym expansion.
#[derive(Debug)]
pub struct ExpandedQuery {
    /// Original query string
    pub original: String,
    /// Term groups: [["hp", "hit points", "health"], ["restore", "heal"]]
    pub term_groups: Vec<Vec<String>>,
}

impl ExpandedQuery {
    /// Generate SurrealDB FTS query with OR-expanded synonyms.
    pub fn to_surrealdb_fts(&self, field: &str, analyzer_ref: u32) -> String;

    /// Generate SQLite FTS5 MATCH expression (for future sqlite-vec support).
    pub fn to_fts5_match(&self) -> String;
}
```

**Default TTRPG Synonyms:**

```toml
# config/synonyms.toml

[multi_way]
# Stat abbreviations
hp = ["hit points", "health", "life"]
ac = ["armor class", "armour class"]
str = ["strength"]
dex = ["dexterity"]
con = ["constitution"]
int = ["intelligence"]
wis = ["wisdom"]
cha = ["charisma"]

# Game mechanics
aoo = ["attack of opportunity", "opportunity attack"]
crit = ["critical hit", "nat 20", "natural 20"]
dm = ["dungeon master", "game master", "gm"]
pc = ["player character"]
npc = ["non-player character"]

# Book abbreviations
phb = ["player's handbook", "players handbook"]
dmg = ["dungeon master's guide"]
mm = ["monster manual"]

[one_way]
# Creature hierarchies (dragon doesn't mean wyrm, but wyrm means dragon)
dragon = ["wyrm", "drake"]
undead = ["zombie", "skeleton", "vampire", "lich"]

# Damage types
"fire damage" = ["flame damage", "burning"]
"cold damage" = ["frost damage", "ice damage"]
```

### 3. QueryPipeline (pipeline.rs)

**Purpose:** Orchestrate the full preprocessing flow.

```rust
/// Complete query preprocessing pipeline.
pub struct QueryPipeline {
    typo_corrector: TypoCorrector,
    synonym_map: SynonymMap,
}

/// Result of preprocessing a raw query.
#[derive(Debug)]
pub struct ProcessedQuery {
    /// Original user input
    pub original: String,
    /// Typo-corrected version
    pub corrected: String,
    /// Individual corrections made (for UI feedback)
    pub corrections: Vec<Correction>,
    /// Synonym-expanded query for BM25
    pub expanded: ExpandedQuery,
    /// Text to embed for vector search (corrected, not expanded)
    pub text_for_embedding: String,
}

impl QueryPipeline {
    /// Create pipeline with components.
    pub fn new(typo_corrector: TypoCorrector, synonym_map: SynonymMap) -> Self;

    /// Process a raw user query through the full pipeline.
    pub fn process(&self, raw_query: &str) -> ProcessedQuery;
}
```

### 4. Dictionary Generator (dictionary.rs)

**Purpose:** Generate corpus dictionaries from indexed content.

```rust
/// Generates frequency dictionaries from indexed TTRPG content.
pub struct DictionaryGenerator {
    /// Frequency boost for corpus terms over English
    domain_boost: u64,
    /// Minimum word length to include
    min_word_length: usize,
}

impl DictionaryGenerator {
    /// Generate word frequency dictionary from document chunks.
    pub async fn build_corpus_dictionary(
        &self,
        db: &Surreal<Db>,
        output_path: &Path,
    ) -> Result<usize, DictionaryError>;

    /// Generate bigram frequency dictionary.
    pub async fn build_bigram_dictionary(
        &self,
        db: &Surreal<Db>,
        output_path: &Path,
    ) -> Result<usize, DictionaryError>;

    /// Rebuild all dictionaries (call after bulk ingestion).
    pub async fn rebuild_all(&self, db: &Surreal<Db>, data_dir: &Path) -> Result<(), DictionaryError>;
}
```

---

## Integration Points

### Search Module Integration

Update `src/core/storage/search.rs` to use preprocessed queries:

```rust
use crate::core::preprocess::{QueryPipeline, ProcessedQuery};

/// Hybrid search with query preprocessing.
pub async fn hybrid_search_with_preprocessing(
    db: &Surreal<Db>,
    pipeline: &QueryPipeline,
    raw_query: &str,
    query_embedding: Vec<f32>,
    config: &HybridSearchConfig,
    filter: Option<&SearchFilter>,
) -> Result<(Vec<SearchResult>, Vec<Correction>), StorageError> {
    // Preprocess the query
    let processed = pipeline.process(raw_query);

    // Build FTS query with expanded synonyms
    let fts_query = processed.expanded.to_surrealdb_fts("content", 1);

    // Execute hybrid search with preprocessed query
    let results = hybrid_search_internal(db, &fts_query, query_embedding, config, filter).await?;

    Ok((results, processed.corrections))
}
```

### AppState Integration

Add pipeline to application state:

```rust
pub struct AppState {
    // ... existing fields ...
    pub query_pipeline: Option<Arc<QueryPipeline>>,
}
```

### Tauri Commands

```rust
/// Search with typo correction and synonyms.
#[tauri::command]
pub async fn search_with_preprocessing(
    state: State<'_, AppState>,
    query: String,
    embedding: Vec<f32>,
    options: SearchOptions,
) -> Result<SearchResponse, String> {
    let storage = state.surreal_storage.as_ref()
        .ok_or("SurrealDB not initialized")?;
    let pipeline = state.query_pipeline.as_ref()
        .ok_or("Query pipeline not initialized")?;

    let (results, corrections) = hybrid_search_with_preprocessing(
        storage.db(),
        pipeline,
        &query,
        embedding,
        &options.into(),
        None,
    ).await.map_err(|e| e.to_string())?;

    Ok(SearchResponse {
        hits: results.into_iter().map(Into::into).collect(),
        corrections: corrections.into_iter().map(Into::into).collect(),
        query: query,
    })
}
```

---

## Data Flow

### Startup Initialization

```
1. Load TypoConfig from config file
2. Load English frequency dictionary
3. Load TTRPG corpus dictionary (if exists)
4. Load bigram dictionary (if exists)
5. Initialize TypoCorrector
6. Load SynonymMap from TOML config
7. Create QueryPipeline
8. Store in AppState
```

### Post-Ingestion Dictionary Rebuild

```
1. User imports new documents
2. Ingestion completes successfully
3. DictionaryGenerator.rebuild_all() called
4. New corpus/bigram dictionaries written to disk
5. TypoCorrector.reload_dictionaries() called
```

### Search Request Flow

```
1. Frontend sends raw query + embedding
2. QueryPipeline.process(raw_query)
   - Normalize → Typo correct → Synonym expand
3. Build SurrealQL FTS query from expanded terms
4. Execute hybrid search (BM25 + HNSW + RRF)
5. Return results + corrections for "Did you mean?" UI
```

---

## Configuration

### Default Configuration File

```toml
# config/preprocessing.toml

[typo]
min_word_size_one_typo = 5
min_word_size_two_typos = 9
disabled_on_words = ["dnd", "5e", "phb", "dmg"]

[synonyms]
max_expansions = 5
```

### Data Directories

```
~/.local/share/ttrpg-assistant/
├── dictionaries/
│   ├── english_82k.txt          # Ships with app
│   ├── ttrpg_corpus.txt         # Generated
│   └── ttrpg_bigrams.txt        # Generated
└── config/
    ├── preprocessing.toml
    └── synonyms.toml
```

---

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum PreprocessError {
    #[error("Dictionary load failed: {0}")]
    DictionaryLoad(String),

    #[error("Synonym config parse failed: {0}")]
    SynonymParse(String),

    #[error("Dictionary generation failed: {0}")]
    DictionaryGeneration(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

---

## Performance Considerations

1. **SymSpell Initialization**: ~50ms to load 82K English + 20K corpus dictionaries
2. **Query Correction**: O(1) per word lookup, < 1ms for typical queries
3. **Synonym Expansion**: O(n) where n = words, < 1ms for typical queries
4. **Dictionary Rebuild**: ~2-5s for 10K documents (background task)
5. **Memory**: ~40MB for SymSpell dictionaries

---

## Testing Strategy

### Unit Tests
- `TypoCorrector::correct_query` with known typos
- `SynonymMap::expand_term` for all synonym types
- `QueryPipeline::process` end-to-end
- `ExpandedQuery::to_surrealdb_fts` query formatting

### Integration Tests
- Full search with preprocessing → verify results include synonym matches
- Dictionary rebuild → verify new terms are corrected
- Protected words → verify no correction applied

### Property Tests (proptest)
- Arbitrary strings don't cause panics
- Corrections preserve word count
- Synonym expansion is bounded by max_expansions
