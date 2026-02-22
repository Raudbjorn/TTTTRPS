# 04 — Search, RAG & Document Ingestion

**Gaps addressed:** #16 (RAG underspecified), #17 (query preprocessing), #18 (extraction pipeline), #21 (TTRPG search), #22 (Meilisearch chat)

## Hybrid Search Pipeline

```
User Query
    │
    ▼
┌─────────────────────┐
│  QueryPipeline       │
│  (core/preprocess/)  │
│  1. Normalize        │
│  2. TypoCorrector    │  ← SymSpell with corpus + English dictionaries
│  3. SynonymExpander  │  ← TTRPG-specific bidirectional groups
└────────┬────────────┘
         │
    ┌────┴────┐
    ▼         ▼
BM25 FTS   HNSW Vector
(milli)    (SurrealDB)
    │         │
    └────┬────┘
         ▼
   Score Fusion
   (MinMax/L2/Sigmoid)
         │
         ▼
   Ranked Results
```

### Query Preprocessing (`core/preprocess/`)

| Module | Purpose |
|--------|---------|
| `typo_corrector.rs` | SymSpell-based correction using corpus + English dictionaries |
| `synonyms.rs` | Bidirectional synonym groups with TTRPG defaults |
| `pipeline.rs` | Orchestrates full preprocessing flow |
| `dictionary.rs` | Generates corpus dictionaries from indexed content |
| `config.rs` | Toggleable features (typo, synonym, etc.) |

**Dictionary files:**
- Corpus: `~/.local/share/ttrpg-assistant/ttrpg_corpus.txt`
- Bigrams: `~/.local/share/ttrpg-assistant/ttrpg_bigrams.txt`
- English: bundled in app resources

**Async rebuild** triggered post-ingestion via `DictionaryGenerator`.

**Example flow:**
```
"firball damge" → "fireball damage" (typo corrected)
  → FTS: "(fireball OR fire bolt) AND (damage OR harm)" (synonym expanded)
  → Vector: embed("fireball damage") (corrected text)
```

### RAG System (`core/rag/`)

TTRPG-specific RAG with content-type-aware configuration:

| Content Type | Semantic Ratio | Context Limit | Use Case |
|-------------|----------------|---------------|----------|
| `rules` | 0.7 | High | Game mechanics, spell descriptions |
| `fiction` | 0.6 | Medium | Lore, world-building |
| `session_notes` | 0.5 | Medium | Campaign notes, summaries |
| `homebrew` | 0.7 | High | Custom content |

**Campaign Grounding** (`core/campaign/grounding/`):
- `CitationBuilder` — attaches source references to generated content
- `FlavourSearcher` — retrieves thematic elements from rulebooks/lore
- `RulebookLinker` — links generated content to page/section citations
- `UsageTracker` — tracks which sources were used in generation

### Document Extraction (`ingestion/`)

25+ files handling multi-format extraction:

| Module | Purpose |
|--------|---------|
| `kreuzberg_extractor.rs` | Core extraction (PDF via pdfium, EPUB, DOCX, images) |
| `chunker.rs` | Semantic TTRPG-aware text chunking |
| `layout/` | Column detection, region detection, table extraction |
| `ttrpg/` | Stat block extraction, dice patterns, cross-references, random tables |
| `classifier.rs` | Game system auto-detection (10+ systems) |
| `adaptive.rs` | Learning-based extraction quality improvement |

**Extraction flow:**
1. `DocumentExtractor::with_ocr()` — pdfium for native PDF, tesseract fallback for scanned
2. Layout analysis identifies columns, tables, stat blocks
3. TTRPG classifier detects game system
4. Semantic chunker splits by section boundaries, respecting stat blocks and tables
5. Chunks stored in SurrealDB with embeddings

### TTRPG Search (`core/ttrpg_search/`)

Domain-aware search layer on top of hybrid search:

- **Antonym mapper** — prevents confusion (fire resistance ≠ fire vulnerability)
- **Attribute filter** — structured filters for damage types, creature types, conditions, alignments, rarities, sizes, spell schools
- **Query expansion** — TTRPG grammar awareness
- **Result ranker** — domain-specific scoring beyond raw fusion scores
- **TTRPG constants** — game terminology database

### Meilisearch Chat (`core/meilisearch_chat/`)

LLM-powered RAG-search hybrid:
- Sends queries to Meilisearch's chat completions endpoint
- Combines search results with LLM context for conversational search
- Configurable provider/model selection
- System prompt templates for TTRPG context

### TUI Requirements

1. **Search bar** — query input with preprocessing preview (show corrections, expansions)
2. **Results view** — ranked results with content-type badges, source citations
3. **Filter panel** — TTRPG attribute filters (creature type, spell school, etc.)
4. **RAG context preview** — show what context will be sent to LLM before generation
5. **Ingestion status** — progress for document extraction (integrated with Library view)
6. **Dictionary rebuild trigger** — manual refresh of corpus dictionaries
