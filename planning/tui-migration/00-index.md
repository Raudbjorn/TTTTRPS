# TTTTRPS Ratatui TUI Migration — Corrected Planning Document

**Status:** Addresses all 23 gaps from `~/wily.md` gap analysis.
**Date:** 2026-02-22
**Base commit:** 717fb4a (Phase 6 complete)

## Document Structure

| Section | Topic | Gap(s) Addressed |
|---------|-------|------------------|
| 01 | Storage Architecture | #1 (WRONG), #20 (engine) |
| 02 | Implemented TUI Baseline | #3 (STALE) |
| 03 | Binary Architecture | #2 (WRONG workspace) |
| 04 | Search, RAG & Ingestion | #16, #17, #18, #21, #22 |
| 05 | OAuth & Credentials | #15 |
| 06 | Voice System | #14 |
| 07 | NPC System | #4 (MISSING) |
| 08 | Personality System | #5 (MISSING) |
| 09 | Archetype System | #6 (MISSING) |
| 10 | Character Generation | #7 (MISSING) |
| 11 | Location System | #8 (MISSING) |
| 12 | Campaign Generation | #10 (MISSING) |
| 13 | Relationship Graph | #9 (MISSING) |
| 14 | Error Handling Strategy | #11 (MISSING) |
| 15 | Data Migration Path | #12 (MISSING) |
| 16 | Audit, Analytics & Usage | #13 (MISSING) |
| 17 | Configuration & Settings | #19 |
| 18 | Dual MessageRole Types | #23 |

## Architecture Summary

```
TTTTRPS Binary (src/tui_main.rs)
  |
  +-- ratatui + crossterm (terminal UI)
  +-- tokio (async runtime)
  |
  +-- core/           (shared with TTTRPS Tauri app)
  |   +-- llm/        (multi-provider LLM routing)
  |   +-- storage/    (SurrealDB + RocksDB — migration target)
  |   +-- npc_gen/    (NPC generation + vocabulary + dialects)
  |   +-- personality/ (4-phase personality system)
  |   +-- archetype/  (registry, resolution, setting packs)
  |   +-- character_gen/ (10+ game systems)
  |   +-- location_gen/  (35+ location types)
  |   +-- campaign/   (generation, grounding, relationships)
  |   +-- voice/      (11 providers, queue, profiles)
  |   +-- search/     (embedded Meilisearch/milli)
  |   +-- preprocess/ (SymSpell + synonyms)
  |   +-- rag/        (TTRPG-specific retrieval)
  |   +-- ttrpg_search/ (domain-aware search)
  |   +-- meilisearch_chat/ (LLM-powered search)
  |   +-- usage/      (token tracking, cost)
  |   +-- security/   (audit)
  |
  +-- database/       (SQLite via SQLx — legacy, shared)
  +-- ingestion/      (kreuzberg extraction + TTRPG chunking)
  +-- oauth/          (Claude PKCE, Gemini OAuth, Copilot device flow)
```

Both TUI and Tauri binaries share all `core/`, `database/`, `ingestion/`, and `oauth/` modules.
The TUI lives at `src/tui/` (4721 LOC across 13 files as of Phase 6).
