# Architecture: TTRPG Chunking Pipeline v2

## Reference Material

Source code for novel chunking strategies:
- mbed-unified chunking strategies (pipeline/strategies module)
- rapydocs enhanced chunking with entity extraction
- rapydocs 9-step retrieval pipeline

## Current Architecture

```
File → Extract (kreuzberg) → Chunk (inline) → Index to Meilisearch
                                 ↓
                     ChunkedDocument {
                         id: "<slug>-c<N>",
                         content,
                         source_raw_ids: ["<slug>-p1", "<slug>-p2"],
                         page_start, page_end,
                         book_title, game_system,
                         semantic_keywords
                     }
```

The two-phase extraction→chunk architecture exists, but chunking is:
- **Content-agnostic** - No awareness of stat blocks, tables, boxed text
- **Boundary-naive** - Splits at character counts, not semantic boundaries
- **Context-poor** - Section hierarchy not tracked or injected

## Target Architecture

```
File → Extract → Classify → Store Raw → Chunk → Enrich → Index
         ↓           ↓          ↓         ↓        ↓        ↓
     kreuzberg  TTRPGClassifier RawDoc  TTRPG   Context  Meilisearch
                                        Chunker  Injection
```

### Phase 1: Extraction (Unchanged)
kreuzberg extracts pages with OCR fallback. No changes needed.

### Phase 2: Classification (New)
Classify each text block before chunking:

```rust
enum TTRPGElementType {
    StatBlock,       // Creature stats (AC, HP, abilities)
    RandomTable,     // Dice-based tables (d6, d20, d100)
    SpellDescription,// Spell blocks with components, range, etc.
    ItemDescription, // Magic item stat blocks
    ReadAloudText,   // GM read-aloud / boxed text
    Sidebar,         // Optional rules, tips, variants
    SectionHeader,   // h1-h6 equivalent
    CrossReference,  // "See page X" / "See Chapter Y"
    Narrative,       // Story/lore text
    Rules,           // Mechanical rules text
    GenericText,     // Fallback
}

struct ClassifiedElement {
    content: String,
    element_type: TTRPGElementType,
    confidence: f32,        // 0.0-1.0
    page_number: u32,
    line_range: (usize, usize),
    metadata: ElementMetadata,
}
```

### Phase 3: Raw Document Storage (Exists)
Store page-level content in `<slug>-raw` index. Already implemented.

### Phase 4: TTRPG-Aware Chunking (Enhanced)

```rust
struct TTRPGChunker {
    config: TTRPGChunkConfig,
    hierarchy: SectionHierarchy,
    classifier: TTRPGClassifier,
}

struct TTRPGChunkConfig {
    base: ChunkConfig,
    atomic_elements: Vec<TTRPGElementType>,  // Never split these
    atomic_max_multiplier: f32,              // 2.0x max size for atomics
    inject_hierarchy_context: bool,          // Prepend section path
    boundary_scoring: BoundaryScoring,       // How to find split points
}

struct SectionHierarchy {
    stack: Vec<(usize, String)>,  // (level, title) pairs
}

impl SectionHierarchy {
    fn update(&mut self, level: usize, title: &str);
    fn path(&self) -> String;        // "Chapter 3 > Combat > Grappling"
    fn depth(&self) -> usize;
    fn parent_at(&self, level: usize) -> Option<&str>;
}
```

#### Chunking Logic

```
for element in classified_elements:
    match element.element_type:
        SectionHeader =>
            hierarchy.update(detect_level(element), element.title)
            flush_buffer()

        StatBlock | RandomTable | SpellDescription | ItemDescription =>
            flush_buffer()
            emit_atomic_chunk(element, hierarchy.path())

        ReadAloudText | Sidebar =>
            flush_buffer()
            emit_chunk_with_type(element, hierarchy.path())

        Narrative | Rules | GenericText =>
            buffer.push(element)
            if buffer.size >= target_size:
                split_at_best_boundary()
                emit_chunk(buffer.take(), hierarchy.path())
```

### Phase 5: Context Injection (New)

Before embedding, prepend context to chunk content:

```rust
fn inject_context(chunk: &mut ChunkedDocument, hierarchy: &SectionHierarchy) {
    let mut header = String::new();

    // Section path
    if let Some(path) = hierarchy.path() {
        header.push_str(&format!("[Section: {}] ", path));
    }

    // Element type
    if let Some(etype) = &chunk.element_type {
        header.push_str(&format!("[Type: {}] ", etype));
    }

    // Game system
    if let Some(system) = &chunk.game_system {
        header.push_str(&format!("[System: {}] ", system));
    }

    chunk.content = format!("{}{}", header, chunk.content);
}
```

### Phase 6: Indexing (Enhanced)

Add TTRPG-specific filterable attributes to Meilisearch:

```rust
struct TTRPGSearchDocument {
    // Existing fields
    id: String,
    content: String,
    source_slug: String,
    page_start: u32,
    page_end: u32,

    // New filterable attributes
    element_type: String,           // stat_block, random_table, etc.
    section_path: String,           // "Chapter 3 > Combat > Grappling"
    section_depth: u32,             // 0=top-level, 1=subsection, etc.
    parent_sections: Vec<String>,   // ["Chapter 3", "Combat"]

    // TTRPG attributes (from existing vocabulary extraction)
    damage_types: Vec<String>,
    creature_types: Vec<String>,
    conditions: Vec<String>,
    challenge_rating: Option<f32>,
    spell_level: Option<u32>,

    // Cross-references
    cross_refs: Vec<CrossReference>,
}

struct CrossReference {
    ref_type: String,   // "page", "chapter", "section"
    ref_target: String, // "47", "Combat", "Grappling"
    ref_text: String,   // "See page 47"
}
```

## Boundary Scoring System

When text must be split, score potential break points:

| Boundary Type | Score | Description |
|---------------|-------|-------------|
| Section Header | 0.95 | Markdown headers, detected titles |
| Double Newline | 0.85 | Paragraph boundary |
| All-Caps Line | 0.80 | OSR-style section headers |
| Bullet/Number Start | 0.70 | List item boundary |
| Sentence End + Capital | 0.60 | Sentence boundary |
| Transition Word | 0.50 | "However", "Therefore", "Additionally" |
| Any Sentence End | 0.40 | Period + space |
| Clause Boundary | 0.20 | Comma, semicolon |
| Fallback | 0.10 | Character limit |

```rust
struct BoundaryScoring {
    weights: HashMap<BoundaryType, f32>,
    min_chunk_size: usize,  // Don't split below this
}

impl BoundaryScoring {
    fn find_best_split(&self, text: &str, target_pos: usize) -> usize {
        // Search window around target_pos
        let window = target_pos.saturating_sub(200)..target_pos.min(text.len());

        // Score each position
        let candidates: Vec<(usize, f32)> = self
            .find_boundaries_in_range(text, window)
            .map(|(pos, btype)| (pos, self.weights[&btype]))
            .collect();

        // Return highest-scored position
        candidates.into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(pos, _)| pos)
            .unwrap_or(target_pos)
    }
}
```

## LLM Fallback for Boundary Detection

When regex-based detection has low confidence, fall back to Ollama:

```rust
async fn detect_boundaries_with_llm(
    text: &str,
    ollama: &OllamaClient,
) -> Result<Vec<(usize, f32)>> {
    let prompt = format!(
        "Identify the major topic/section boundaries in this TTRPG text. \
         Return line numbers where new topics begin, with confidence 0-1.\n\n\
         Text:\n{}\n\n\
         Format: line_number,confidence (one per line)",
        &text[..text.len().min(4000)]
    );

    let response = ollama.generate("qwen2.5:7b", &prompt).await?;
    parse_boundary_response(&response)
}
```

## Data Flow Diagram

```
                                    ┌─────────────────┐
                                    │   Input File    │
                                    │  (PDF/EPUB/...)  │
                                    └────────┬────────┘
                                             │
                                             ▼
                                    ┌─────────────────┐
                                    │   Extraction    │
                                    │   (kreuzberg)   │
                                    └────────┬────────┘
                                             │
                            ┌────────────────┼────────────────┐
                            ▼                ▼                ▼
                    ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
                    │   Page 1    │  │   Page 2    │  │   Page N    │
                    └──────┬──────┘  └──────┬──────┘  └──────┬──────┘
                           │                │                │
                           ▼                ▼                ▼
                    ┌─────────────────────────────────────────────┐
                    │            TTRPGClassifier                  │
                    │  ┌─────────┐ ┌─────────┐ ┌─────────┐        │
                    │  │StatBlock│ │ Table   │ │Narrative│ ...    │
                    │  └─────────┘ └─────────┘ └─────────┘        │
                    └──────────────────┬──────────────────────────┘
                                       │
                           ┌───────────┼───────────┐
                           ▼           ▼           ▼
                    ┌───────────┐ ┌──────────┐ ┌──────────┐
                    │ RawDoc p1 │ │ RawDoc p2│ │ RawDoc pN│
                    │ <slug>-raw│ │          │ │          │
                    └───────────┘ └──────────┘ └──────────┘
                           │           │           │
                           └───────────┼───────────┘
                                       ▼
                    ┌─────────────────────────────────────────────┐
                    │             TTRPGChunker                    │
                    │  ┌──────────────────────────────────────┐   │
                    │  │ SectionHierarchy: "Ch3 > Combat"     │   │
                    │  │ BoundaryScoring: find_best_split()   │   │
                    │  │ AtomicPreservation: stat_block=2x    │   │
                    │  └──────────────────────────────────────┘   │
                    └──────────────────┬──────────────────────────┘
                                       │
                           ┌───────────┼───────────┐
                           ▼           ▼           ▼
                    ┌───────────┐ ┌──────────┐ ┌──────────┐
                    │ Chunk c1  │ │ Chunk c2 │ │ Chunk cN │
                    │ [Section:]│ │[StatBlock│ │[Table...]│
                    └───────────┘ └──────────┘ └──────────┘
                           │           │           │
                           └───────────┼───────────┘
                                       ▼
                    ┌─────────────────────────────────────────────┐
                    │              Meilisearch                    │
                    │  ┌─────────────┐  ┌─────────────┐           │
                    │  │ <slug>-raw  │  │   <slug>    │           │
                    │  │ (pages)     │  │  (chunks)   │           │
                    │  └─────────────┘  └─────────────┘           │
                    └─────────────────────────────────────────────┘
```

## File Changes Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `ingestion/ttrpg/classifier.rs` | NEW | TTRPG element classification |
| `ingestion/ttrpg/mod.rs` | MODIFY | Export classifier types |
| `ingestion/chunker.rs` | EXTEND | Add `SectionHierarchy`, `TTRPGChunker` |
| `core/meilisearch_pipeline.rs` | EXTEND | Add `TTRPGSearchDocument`, context injection |
| `core/search_client.rs` | EXTEND | Add TTRPG filterable attributes |
| `commands.rs` | EXTEND | Wire up new chunking options |

## Configuration

```toml
# Proposed config structure (CLAUDE.toml or in-app settings)
[chunking.ttrpg]
atomic_elements = ["stat_block", "random_table", "spell_description"]
atomic_max_multiplier = 2.0
inject_hierarchy_context = true
use_llm_boundary_fallback = false
llm_boundary_model = "qwen2.5:7b"

[chunking.ttrpg.boundary_weights]
section_header = 0.95
double_newline = 0.85
all_caps_line = 0.80
bullet_start = 0.70
sentence_boundary = 0.60
transition_word = 0.50
```
