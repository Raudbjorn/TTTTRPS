# SurrealDB Vector Storage Migration

## Executive Summary

Replace `meilisearch-lib` and SQLite with **SurrealDB** as the unified storage backend for the TTRPG Assistant. This migration consolidates document storage, vector embeddings, and graph relationships into a single embedded database.

### Motivation

| Current (Dual Storage) | Target (Unified) |
|------------------------|------------------|
| SQLite: campaigns, NPCs, sessions, chat | SurrealDB: all entities |
| Meilisearch: document chunks, vectors, search | SurrealDB: chunks, vectors, search |
| Manual sync between databases | Single source of truth |
| No graph relations | Native graph traversal |
| Limited vector filtering | Rich metadata filtering during KNN |

### Key Benefits

1. **Unified Storage**: One database for documents, vectors, relations, metadata
2. **Graph Capabilities**: Express NPC alliances, document references, faction hierarchies
3. **Hybrid Search**: Native BM25 + HNSW vector fusion with `search::linear()`
4. **Embedded**: No external process (RocksDB backend, like meilisearch-lib)
5. **Simpler Architecture**: Remove synchronization complexity

---

## Specification Documents

| Document | Purpose | Status |
|----------|---------|--------|
| [Requirements.md](./Requirements.md) | User stories, functional requirements, constraints | ✅ Complete |
| [Design.md](./Design.md) | Technical architecture, component design, data models | ✅ Complete |
| [Tasks.md](./Tasks.md) | Implementation tasks with estimates (~45-55 hours) | ✅ Complete |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                    SurrealDB (Embedded with RocksDB)                 │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌─────────────┐  ┌─────────────┐  ┌───────────────────────────────┐│
│  │  Documents  │  │   Vectors   │  │       Graph Relations          ││
│  │             │  │             │  │                                ││
│  │ • campaign  │  │ HNSW Index  │  │ campaign ──┐                   ││
│  │ • npc       │  │ on chunk    │  │            │ has_npc           ││
│  │ • session   │  │ .embedding  │  │            ▼                   ││
│  │ • chunk     │  │             │  │          npc ────────┐         ││
│  │ • chat_msg  │  │ 768-3072    │  │            │ allied  │         ││
│  │ • library   │  │ dimensions  │  │            ▼         ▼         ││
│  └─────────────┘  └─────────────┘  │          npc ◄── faction       ││
│                                    └───────────────────────────────┘│
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                    Hybrid Search Pipeline                        ││
│  │                                                                  ││
│  │  Query → [Vector KNN] + [BM25 Full-Text] → search::linear()     ││
│  │                                              │                   ││
│  │                                              ▼                   ││
│  │                                    Fused Ranked Results          ││
│  └─────────────────────────────────────────────────────────────────┘│
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Phases

### Phase 1: Foundation (~8 hours)
- Add `surrealdb` crate with RocksDB feature
- Create storage module structure
- Implement `SurrealStorage` wrapper
- Define database schema (tables, indexes, analyzers)

### Phase 2: Search Implementation (~12 hours)
- Vector search (KNN with metadata filtering)
- Full-text search (BM25 with highlighting)
- Hybrid search fusion (`search::linear()`)

### Phase 3: Document Ingestion (~8 hours)
- Chunk ingestion pipeline
- Library item management
- Integration with existing extraction pipeline

### Phase 4: RAG Pipeline (~8 hours)
- Context retrieval with hybrid search
- LLM integration (existing router)
- Streaming responses

### Phase 5: Data Migration (~10 hours)
- SQLite → SurrealDB migration
- Meilisearch → SurrealDB migration
- Validation and rollback procedures

### Phase 6: Command Updates (~6 hours)
- Update Tauri commands for new backend
- Preserve API compatibility

### Phase 7: Cleanup (~5 hours)
- Remove meilisearch-lib, SQLite dependencies
- Testing and documentation

**Total: ~45-55 hours (~6-7 working days)**

---

## SurrealDB Features Used

| Feature | Use Case |
|---------|----------|
| **HNSW Vector Index** | Semantic similarity search on embeddings |
| **BM25 Full-Text Index** | Keyword search with ranking |
| **search::linear()** | Fuse vector + text results with weights |
| **Record Links** | Graph relations (NPC→Campaign, Chunk→LibraryItem) |
| **RELATE statements** | NPC alliances, document cross-references |
| **Embedded RocksDB** | Persistent storage without external process |
| **Transactions** | Atomic document ingestion |

---

## Migration Strategy

1. **Parallel Operation**: Build SurrealDB backend alongside existing system
2. **Feature Parity**: Ensure all existing features work before switching
3. **Data Migration**: One-time migration on version upgrade
4. **Rollback Ready**: Keep backup and migration reversal procedure

### Migration Triggers
- First app launch after upgrade detects old data
- User prompted to migrate (with backup)
- Progress indicator during migration
- Validation before marking complete

---

## Dependencies

### New Dependencies
```toml
surrealdb = { version = "2.x", features = ["kv-rocksdb"] }
```

### Dependencies to Remove (After Migration)
- `meilisearch-lib` (path dependency)
- `meilisearch-sdk`
- `sqlx` (SQLite features)

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| SurrealDB SDK bugs | Medium | High | Extensive testing, fallback branch |
| Migration data loss | Low | Critical | Pre-backup, validation, dry-run |
| Performance regression | Medium | Medium | Benchmarking, HNSW tuning |
| Graph query complexity | Low | Medium | Start simple, iterate |

---

## Success Criteria

- [ ] Single database dependency (no SQLite, no Meilisearch)
- [ ] Hybrid search quality comparable to Meilisearch
- [ ] Graph queries enable new features (NPC relations)
- [ ] Migration completes without data loss
- [ ] Startup time < 3 seconds
- [ ] Search latency < 300ms
- [ ] Frontend unchanged (API compatible)
- [ ] All platforms supported (Linux, Windows, macOS)

---

## Related Documentation

- [Current Meilisearch Integration](../meilisearch-lib-integration/)
- [Embeddings Design](../done/embeddings/)
- [SurrealDB Documentation](https://surrealdb.com/docs)
- [SurrealDB Rust SDK](https://surrealdb.com/docs/sdk/rust)
- [SurrealDB Vector Search](https://surrealdb.com/docs/surrealdb/models/vector)
