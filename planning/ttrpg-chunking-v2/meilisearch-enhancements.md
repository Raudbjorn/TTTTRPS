# Meilisearch Enhancements for TTRPG Search

Based on review of `/home/svnbjrn/dev/knowledgebase/embeddings/meilisearch_docs/meilisearch_docs_optimized.md`

## Faceted Search

Meilisearch has native faceted search that can provide filter counts to users. This is highly applicable for TTRPG content browsing.

### Facet Distribution

When searching, request facet distribution to show counts:

```rust
// Request
{
    "q": "fire spell",
    "facets": ["damage_types", "spell_level", "element_type", "game_system"]
}

// Response includes facetDistribution
{
    "hits": [...],
    "facetDistribution": {
        "damage_types": {
            "fire": 42,
            "cold": 12,
            "lightning": 8
        },
        "spell_level": {
            "1": 15,
            "2": 18,
            "3": 24,
            "cantrip": 5
        },
        "element_type": {
            "spell": 45,
            "stat_block": 12,
            "rules": 5
        }
    }
}
```

### Faceting Settings

Configure faceting behavior per-index:

```rust
// settings/faceting
{
    "maxValuesPerFacet": 100,
    "sortFacetValuesBy": {
        "damage_types": "count",   // Sort by frequency (most common first)
        "spell_level": "alpha",    // Sort alphabetically (0, 1, 2, 3...)
        "*": "alpha"               // Default for all other facets
    }
}
```

### TTRPG Facet Categories

| Facet | Sort By | Description |
|-------|---------|-------------|
| `damage_types` | count | Fire, cold, lightning, etc. - show popular first |
| `creature_types` | count | Humanoid, undead, dragon - show common types first |
| `element_type` | alpha | stat_block, spell, table - consistent ordering |
| `game_system` | alpha | D&D 5e, Pathfinder 2e, etc. |
| `spell_level` | alpha | Cantrip, 1, 2, 3... - numeric order |
| `challenge_rating` | alpha | Numeric order for CR |
| `section_path` | alpha | Hierarchical paths |

## Filterable Attributes with Features

The new `filterableAttributes` format allows fine-grained control:

```rust
// Advanced filterable attributes configuration
{
    "filterableAttributes": [
        // Simple string (equality only, no facet search)
        "source_slug",

        // Object with features
        {
            "attributePatterns": ["damage_types", "creature_types", "conditions"],
            "features": {
                "facetSearch": true,      // Enable searching within facet values
                "filter": {
                    "equality": true,     // =, !=, IN, EXISTS
                    "comparison": false   // No >, <, >= (not needed for strings)
                }
            }
        },
        {
            "attributePatterns": ["challenge_rating", "spell_level", "page_start"],
            "features": {
                "facetSearch": false,
                "filter": {
                    "equality": true,
                    "comparison": true    // Enable >, <, >= for numeric ranges
                }
            }
        }
    ]
}
```

### Facet Search

With `facetSearch: true`, users can search within facet values:

```rust
// API call to /indexes/books/facet-search
{
    "facetName": "creature_types",
    "facetQuery": "drag"  // Returns "dragon", "dracolich", etc.
}

// Response
{
    "facetHits": [
        { "value": "dragon", "count": 127 },
        { "value": "dracolich", "count": 3 }
    ]
}
```

This is useful for large TTRPG facet vocabularies (hundreds of creature types, spell names, etc.).

## Custom Ranking Rules

Add TTRPG-specific ranking:

```rust
{
    "rankingRules": [
        "words",
        "typo",
        "proximity",
        "attribute",
        "sort",
        "exactness",
        // Custom: boost higher CR creatures for "powerful monster" queries
        "challenge_rating:desc"
    ]
}
```

### Dynamic Ranking at Query Time

Use `sort` parameter for user-controlled ranking:

```rust
{
    "q": "undead",
    "sort": ["challenge_rating:asc"]  // Low CR first (for new players)
}
```

## Embedders Configuration

Meilisearch has native Ollama support:

```rust
{
    "embedders": {
        "default": {
            "source": "ollama",
            "url": "http://localhost:11434/api/embeddings",
            "model": "nomic-embed-text",
            "documentTemplate": "{{doc.content}}",
            "dimensions": 768
        }
    }
}
```

### Hybrid Search with semanticRatio

Control keyword vs semantic balance:

```rust
{
    "q": "fire breathing monster",
    "hybrid": {
        "embedder": "default",
        "semanticRatio": 0.5  // 0 = pure keyword, 1 = pure semantic
    }
}
```

For TTRPG queries:
- **Rules queries** (`semanticRatio: 0.3`): "grappling rules" - prefer exact keyword match
- **Narrative queries** (`semanticRatio: 0.7`): "describe a haunted castle" - prefer semantic
- **Stat block queries** (`semanticRatio: 0.2`): "AC 18 undead" - very keyword-heavy

## Implementation Tasks

### Task M.1: Configure TTRPG Faceting

**File**: `src/core/search_client.rs` [EXTEND]

```rust
pub async fn configure_ttrpg_faceting(&self, index_name: &str) -> Result<(), SearchError> {
    let settings = json!({
        "faceting": {
            "maxValuesPerFacet": 100,
            "sortFacetValuesBy": {
                "damage_types": "count",
                "creature_types": "count",
                "spell_level": "alpha",
                "challenge_rating": "alpha",
                "*": "alpha"
            }
        }
    });

    self.client
        .index(index_name)
        .set_settings(&settings)
        .await?;

    Ok(())
}
```

### Task M.2: Add Facet Distribution to Search Results

**File**: `src/core/search_client.rs` [EXTEND]

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTRPGSearchResponse {
    pub hits: Vec<ChunkedDocument>,
    pub facet_distribution: Option<HashMap<String, HashMap<String, u64>>>,
    pub total_hits: u64,
    pub processing_time_ms: u64,
}

pub async fn search_with_facets(
    &self,
    index: &str,
    query: &str,
    filters: Option<&str>,
    facets: &[&str],
) -> Result<TTRPGSearchResponse, SearchError> {
    let mut search = self.client.index(index).search();
    search.with_query(query);

    if let Some(f) = filters {
        search.with_filter(f);
    }

    search.with_facets(facets);

    let results = search.execute::<ChunkedDocument>().await?;

    Ok(TTRPGSearchResponse {
        hits: results.hits.into_iter().map(|r| r.result).collect(),
        facet_distribution: results.facet_distribution,
        total_hits: results.estimated_total_hits.unwrap_or(0),
        processing_time_ms: results.processing_time_ms as u64,
    })
}
```

### Task M.3: Add Facet Search Endpoint

**File**: `src/commands.rs` [EXTEND]

```rust
#[tauri::command]
pub async fn search_facet_values(
    state: tauri::State<'_, AppState>,
    index: String,
    facet_name: String,
    facet_query: String,
) -> Result<Vec<FacetHit>, String> {
    let search_client = state.search_client.lock().await;

    search_client
        .facet_search(&index, &facet_name, &facet_query)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetHit {
    pub value: String,
    pub count: u64,
}
```

### Task M.4: Configure Ollama Embedder

**File**: `src/core/search_client.rs` [EXTEND]

```rust
pub async fn configure_embedder(
    &self,
    index_name: &str,
    ollama_url: &str,
    model: &str,
    dimensions: u32,
) -> Result<(), SearchError> {
    let settings = json!({
        "embedders": {
            "default": {
                "source": "ollama",
                "url": format!("{}/api/embeddings", ollama_url),
                "model": model,
                "documentTemplate": "{{doc.content}}",
                "dimensions": dimensions
            }
        }
    });

    self.client
        .index(index_name)
        .set_settings(&settings)
        .await?;

    Ok(())
}
```

### Task M.5: Add Hybrid Search with Semantic Ratio

**File**: `src/core/search_client.rs` [EXTEND]

```rust
#[derive(Debug, Clone)]
pub struct HybridSearchParams {
    pub query: String,
    pub semantic_ratio: f32,  // 0.0 to 1.0
    pub filters: Option<String>,
    pub facets: Vec<String>,
    pub limit: usize,
}

pub async fn hybrid_search(
    &self,
    index: &str,
    params: HybridSearchParams,
) -> Result<TTRPGSearchResponse, SearchError> {
    let mut search = self.client.index(index).search();

    search
        .with_query(&params.query)
        .with_limit(params.limit)
        .with_hybrid("default", params.semantic_ratio);

    if let Some(f) = &params.filters {
        search.with_filter(f);
    }

    if !params.facets.is_empty() {
        search.with_facets(&params.facets);
    }

    let results = search.execute::<ChunkedDocument>().await?;

    Ok(TTRPGSearchResponse {
        hits: results.hits.into_iter().map(|r| r.result).collect(),
        facet_distribution: results.facet_distribution,
        total_hits: results.estimated_total_hits.unwrap_or(0),
        processing_time_ms: results.processing_time_ms as u64,
    })
}
```

## Frontend Integration

### Faceted Filter UI

The facet distribution enables a filter sidebar:

```
Damage Types
  [x] Fire (42)
  [ ] Cold (12)
  [ ] Lightning (8)

Spell Level
  [ ] Cantrip (5)
  [ ] 1st (15)
  [x] 2nd (18)
  [ ] 3rd (24)

Content Type
  [x] Spells (45)
  [ ] Stat Blocks (12)
  [ ] Rules (5)
```

### Semantic Ratio Slider

For advanced users:

```
Search Mode: [Keyword -------|------- Semantic]
             0.0            0.5            1.0

Tips:
- Rules/mechanics: slide left (keyword-focused)
- Narrative/descriptions: slide right (semantic-focused)
- Monster stats: far left (exact attribute match)
```

## Performance Considerations

1. **Facet limit**: Keep `maxValuesPerFacet` reasonable (100-200) to avoid slow queries
2. **Facet search**: Only enable for facets with many values (creature_types, spell names)
3. **Comparison filters**: Only enable for numeric fields needing range queries
4. **Embedder caching**: Ollama embeddings are cached, but initial indexing is slow

## Summary of Enhancements

| Feature | Benefit for TTRPG |
|---------|-------------------|
| Facet distribution | Show damage type/creature counts in UI |
| sortFacetValuesBy: count | Most common values first |
| Facet search | Search through hundreds of creature types |
| Comparison filters | CR >= 5 AND CR <= 10 queries |
| Native Ollama embedder | No external embedding service needed |
| Hybrid search | Balance keyword vs semantic per query type |
| Custom ranking rules | Boost higher CR for "powerful" queries |
