# TTRPG Chunking Pipeline v2

## Executive Summary

This feature enhances the existing document ingestion pipeline with TTRPG-specific chunking strategies and semantic processing. Building on the already-implemented two-phase architecture (RawDocument → ChunkedDocument), this adds:

1. **TTRPG Element Classification** - Detect and preserve stat blocks, random tables, boxed text
2. **Hierarchical Context Injection** - Track section hierarchy and prepend context to chunks
3. **Semantic Boundary Detection** - LLM-assisted topic shift detection for intelligent splitting
4. **Cross-Reference Tracking** - Parse "See page X" references for future linking

## Current State Analysis

### Already Implemented (meilisearch_pipeline.rs)

| Feature | Status | Location |
|---------|--------|----------|
| Slug generation | ✅ Complete | `generate_source_slug()`, `slugify()` |
| RawDocument (page-level) | ✅ Complete | `RawDocument { id: "<slug>-p<N>", raw_content, page_metadata }` |
| ChunkedDocument with provenance | ✅ Complete | `ChunkedDocument { source_raw_ids: Vec<String>, page_start, page_end }` |
| Per-source index naming | ✅ Complete | `raw_index_name()`, `chunks_index_name()` |
| TTRPG metadata extraction | ✅ Basic | `TTRPGMetadata::extract()` - game system, category, publisher |
| Page metadata | ✅ Basic | `PageMetadata { has_tables, has_images, page_header }` |

### Already Implemented (chunker.rs)

| Feature | Status | Location |
|---------|--------|----------|
| Semantic chunking | ✅ Complete | `SemanticChunker::chunk_with_pages()` |
| Configurable sizes | ✅ Complete | `ChunkConfig { target_size, min_size, max_size, overlap_size }` |
| Chunk type tagging | ✅ Partial | `chunk_type` field (values: text, table, header, stat_block, spell, monster, rule, narrative) |
| Hierarchical metadata | ✅ Partial | `chapter_title`, `subsection_title`, `section` fields exist |

### Gaps to Address

| Gap | Priority | Complexity | Notes |
|-----|----------|------------|-------|
| TTRPG Element Classifier | P0 | Medium | Regex patterns for stat blocks, tables, boxed text |
| SectionHierarchy tracker | P0 | Low | Stack-based header level tracking |
| Atomic element preservation | P0 | Medium | Never split stat blocks, tables mid-content |
| Context header injection | P1 | Low | Prepend `[Section: X]` to chunk content |
| Semantic boundary scoring | P1 | Medium | Assign split-priority scores to text positions |
| Two-page spread handling | P2 | Medium | Merge even-odd page pairs before chunking |
| Cross-reference parsing | P2 | Low | Regex for "see page X", "p. 47" patterns |
| LLM fallback for boundaries | P3 | High | Ollama call for topic shift detection |

## Related Documents

- [architecture.md](./architecture.md) - Detailed system design and data flow
- [tasks.md](./tasks.md) - Actionable implementation tasks by phase
- [meilisearch-enhancements.md](./meilisearch-enhancements.md) - Faceted search and hybrid search features

## Novel Approaches from Research

### 1. LLM Semantic Boundary Detection
**Source**: mbed-unified semantic chunker reference implementation

Uses Ollama to detect topic shifts with confidence scores at text positions. Not standard in RAG pipelines. Fallback when regex-based boundary detection fails.

```python
# Concept: Ask LLM to identify major topic shifts
prompt = f"Identify semantic boundaries in this text: {text[:2000]}"
boundaries = ollama.generate(prompt)  # Returns line numbers
```

### 2. Hierarchical Chunk Relationships
**Source**: mbed-unified hierarchical chunker reference implementation

Maintains parent→child bidirectional references. Chunks can traverse hierarchy for context. Useful for TTRPG where a "Fireball" spell needs "Evocation School" context.

```rust
struct HierarchicalChunk {
    parent_id: Option<String>,
    child_ids: Vec<String>,
    section_path: String,  // "Chapter 3 > Spells > Evocation"
}
```

### 3. Entity Header Augmentation
**Source**: rapydocs enhanced chunking reference implementation

Prepends extracted entities (locations, types, qualifiers) to chunks before embedding. Improves retrieval precision.

```rust
// Before: "Deals 8d6 fire damage in a 20-foot radius."
// After:  "[Spell: Fireball] [School: Evocation] [Level: 3] Deals 8d6 fire damage..."
```

### 4. Atomic "Functional Unit" Preservation
**Source**: `TTRPGChunker` design (Tasks.md)

Stat blocks, random tables, and spell descriptions are "functional units" that must never be split. Apply 2x `max_size` multiplier for atomic elements.

```rust
// If max_size = 1000 tokens:
// - Normal text: split at 1000
// - Stat block: allow up to 2000 before splitting
```

## What's Missing for TTRPG Materials

Based on The Design Language document analysis:

1. **Stat Block Detection** - No parser for AC, HD, hp, #AT patterns
2. **Random Table Preservation** - Tables chunked without probability-awareness
3. **Boxed Text/Read-Aloud** - No GM vs player-facing information detection
4. **Two-Page Spread Units** - PDF layout doesn't preserve spread boundaries
5. **Typography-Based Hierarchy** - Relies on markdown headers, not font extraction
6. **Cross-Reference Linking** - "See page 47" references not tracked

## Success Criteria

1. Stat blocks are never split mid-content (100% atomic preservation)
2. Section hierarchy path included in 100% of chunks
3. Random tables preserved with dice notation metadata
4. Cross-references parsed and stored (no linking required for v2)
5. Chunk retrieval precision improves by 15%+ on TTRPG queries

## Dependencies

- Existing `kreuzberg` extraction (no changes needed)
- Existing `RawDocument` / `ChunkedDocument` types (extend, don't replace)
- Meilisearch hybrid search (add filterable attributes)
- Optional: Ollama for LLM boundary detection fallback
