# Query Preprocessing Implementation Tasks

## Phase 1: Core Preprocessing Module (Foundation)

### Task 1.1: Module Structure and Dependencies
**Requirement:** REQ-QP-001, REQ-QP-002
**Estimate:** 1 hour

1. Add dependencies to `Cargo.toml`:
   ```toml
   symspell = "0.4"
   strsim = "0.11"
   ```
2. Create module structure:
   - `src/core/preprocess/mod.rs`
   - `src/core/preprocess/error.rs`
   - `src/core/preprocess/config.rs`
3. Export from `src/core/mod.rs`

### Task 1.2: TypoConfig and Configuration Loading
**Requirement:** REQ-QP-001.1-001.3
**Estimate:** 1 hour

1. Implement `TypoConfig` struct with Meilisearch-compatible defaults
2. Implement `PreprocessConfig` for overall configuration
3. Add TOML deserialization support
4. Create default config file at `config/preprocessing.toml`

### Task 1.3: TypoCorrector Implementation
**Requirement:** REQ-QP-001.1-001.7
**Estimate:** 3 hours

1. Implement `TypoCorrector::new()` with SymSpell initialization
2. Implement dictionary loading (English base, corpus overlay, bigrams)
3. Implement `correct_query()` with:
   - Word-length-based edit distance rules
   - Protected words bypass
   - Compound word correction
4. Implement `reload_dictionaries()` for runtime updates
5. Write unit tests for:
   - Known typos (firball → fireball, damge → damage)
   - Short words not corrected
   - Protected words not corrected
   - Compound correction (magicmissle → magic missile)

### Task 1.4: SynonymMap Implementation
**Requirement:** REQ-QP-002.1-002.3
**Estimate:** 2 hours

1. Implement `SynonymMap` struct with multi-way and one-way storage
2. Implement `add_multi_way()` and `add_one_way()` methods
3. Implement `expand_term()` with max_expansions limit
4. Implement `expand_query()` returning `ExpandedQuery`
5. Write unit tests for:
   - Multi-way expansion (hp → [hp, hit points, health])
   - One-way expansion (dragon → [dragon, wyrm, drake])
   - Expansion limit enforcement
   - Case-insensitive matching

### Task 1.5: ExpandedQuery Formatters
**Requirement:** REQ-QP-005
**Estimate:** 2 hours

1. Implement `ExpandedQuery::to_surrealdb_fts()`:
   - OR-join within synonym groups
   - AND-join between term groups
   - Handle multi-word synonyms
2. Implement `ExpandedQuery::to_fts5_match()` (for future sqlite-vec)
3. Write unit tests for query generation:
   - Single term with synonyms
   - Multiple terms with synonyms
   - Multi-word synonym handling

---

## Phase 2: Configuration and Default Data

### Task 2.1: Default TTRPG Synonyms
**Requirement:** REQ-QP-002.4
**Estimate:** 2 hours

1. Create `config/synonyms.toml` with comprehensive TTRPG synonyms:
   - Stat abbreviations (str, dex, con, int, wis, cha)
   - Game mechanics (hp, ac, dc, cr, xp, aoo, crit)
   - Conditions (prone, grappled, stunned, frightened, etc.)
   - Book abbreviations (phb, dmg, mm, xge, tce)
   - Creature types and damage types
2. Implement `SynonymMap::from_toml()` loader
3. Implement `build_default_ttrpg_synonyms()` fallback function

### Task 2.2: Ship English Frequency Dictionary
**Requirement:** REQ-QP-001.5
**Estimate:** 1 hour

1. Add `data/frequency_dictionary_en_82_765.txt` to resources
2. Configure cargo to include in release builds
3. Implement `get_dictionary_path()` to find resource files

### Task 2.3: Dictionary Generator
**Requirement:** REQ-QP-004.1-004.5
**Estimate:** 3 hours

1. Implement `DictionaryGenerator` struct
2. Implement `build_corpus_dictionary()`:
   - Query all chunks from SurrealDB
   - Tokenize searchable content
   - Count word frequencies
   - Apply domain boost (10x)
   - Write to output file
3. Implement `build_bigram_dictionary()`:
   - Extract word pairs from content
   - Count bigram frequencies
   - Write to output file
4. Implement `rebuild_all()` combining both
5. Write integration test with sample documents

---

## Phase 3: Pipeline Assembly

### Task 3.1: QueryPipeline Implementation
**Requirement:** REQ-QP-003.1-003.6
**Estimate:** 2 hours

1. Implement `QueryPipeline` struct
2. Implement `process()` method:
   - Normalize input
   - Apply typo correction
   - Apply synonym expansion
   - Generate `ProcessedQuery`
3. Write integration tests for full pipeline:
   - Raw query → processed output
   - Corrections captured correctly
   - Embedding text uses corrected form

### Task 3.2: AppState Integration
**Requirement:** REQ-QP-003
**Estimate:** 1 hour

1. Add `query_pipeline: Option<Arc<QueryPipeline>>` to `AppState`
2. Initialize pipeline during app startup
3. Handle missing dictionaries gracefully (use defaults)

---

## Phase 4: Search Integration

### Task 4.1: Update Hybrid Search
**Requirement:** REQ-QP-003.4, REQ-QP-005.3
**Estimate:** 2 hours

1. Implement `hybrid_search_with_preprocessing()` in `search.rs`
2. Update search to use FTS query from `ExpandedQuery`
3. Return corrections alongside results
4. Write integration test:
   - Search "firball" finds "fireball" documents
   - Search "hp" finds "hit points" documents

### Task 4.2: Tauri Commands
**Requirement:** REQ-QP-003
**Estimate:** 2 hours

1. Implement `search_with_preprocessing` Tauri command
2. Add `SearchResponse` struct with corrections field
3. Implement `rebuild_dictionaries` admin command
4. Update existing search commands to use preprocessing (opt-in)

### Task 4.3: Frontend Integration (Optional)
**Estimate:** 2 hours

1. Add "Did you mean?" UI component
2. Display corrections when search returns them
3. Allow user to search with original vs corrected query

---

## Phase 5: Post-Ingestion Hooks

### Task 5.1: Dictionary Rebuild Trigger
**Requirement:** REQ-QP-004.4
**Estimate:** 2 hours

1. Add dictionary rebuild to ingestion completion hook
2. Implement debounced rebuild (wait for batch completion)
3. Reload TypoCorrector dictionaries after rebuild
4. Log rebuild statistics

### Task 5.2: User-Defined Synonyms (Optional)
**Requirement:** REQ-QP-002.5, REQ-QP-002.6
**Estimate:** 3 hours

1. Add `synonym` table to SurrealDB schema:
   ```sql
   DEFINE TABLE synonym SCHEMAFULL;
   DEFINE FIELD group_type ON synonym TYPE string;
   DEFINE FIELD source ON synonym TYPE option<string>;
   DEFINE FIELD terms ON synonym TYPE array<string>;
   DEFINE FIELD campaign_id ON synonym TYPE option<record<campaign>>;
   ```
2. Implement CRUD operations for user synonyms
3. Merge user synonyms with default map at runtime
4. Add Tauri commands for synonym management

---

## Phase 6: Testing and Documentation

### Task 6.1: Unit Test Suite
**Estimate:** 2 hours

1. Complete unit tests for all preprocessing modules
2. Add property tests for edge cases (proptest)
3. Test error handling paths

### Task 6.2: Integration Tests
**Estimate:** 2 hours

1. Full pipeline test with SurrealDB
2. Search accuracy tests with intentional typos
3. Synonym expansion verification
4. Dictionary rebuild test

### Task 6.3: Performance Benchmarks
**Estimate:** 1 hour

1. Benchmark typo correction (target: < 5ms)
2. Benchmark synonym expansion (target: < 1ms)
3. Benchmark full pipeline (target: < 10ms)
4. Document results

### Task 6.4: Update CLAUDE.md
**Estimate:** 30 min

1. Document preprocessing module in architecture section
2. Add configuration file locations
3. Update search command documentation

---

## Task Dependencies

```
Phase 1 (Foundation)
├── 1.1 Module Structure
├── 1.2 Config (depends on 1.1)
├── 1.3 TypoCorrector (depends on 1.2)
├── 1.4 SynonymMap (depends on 1.1)
└── 1.5 Formatters (depends on 1.4)

Phase 2 (Configuration)
├── 2.1 Default Synonyms (depends on 1.4)
├── 2.2 English Dictionary (depends on 1.3)
└── 2.3 Dictionary Generator (depends on 1.3)

Phase 3 (Pipeline)
├── 3.1 QueryPipeline (depends on 1.3, 1.4, 1.5)
└── 3.2 AppState (depends on 3.1)

Phase 4 (Integration)
├── 4.1 Hybrid Search (depends on 3.1)
├── 4.2 Tauri Commands (depends on 3.2, 4.1)
└── 4.3 Frontend (depends on 4.2)

Phase 5 (Hooks)
├── 5.1 Rebuild Trigger (depends on 2.3, 3.2)
└── 5.2 User Synonyms (depends on 3.2)

Phase 6 (Testing)
├── 6.1 Unit Tests (parallel with implementation)
├── 6.2 Integration Tests (depends on Phase 4)
├── 6.3 Benchmarks (depends on Phase 4)
└── 6.4 Documentation (depends on Phase 5)
```

---

## Estimated Total Effort

| Phase | Hours |
|-------|-------|
| Phase 1: Core Module | 9 |
| Phase 2: Configuration | 6 |
| Phase 3: Pipeline | 3 |
| Phase 4: Search Integration | 6 |
| Phase 5: Post-Ingestion Hooks | 5 |
| Phase 6: Testing & Docs | 5.5 |
| **Total** | **34.5** |

---

## Acceptance Criteria Checklist

- [ ] Typo "firball" corrected to "fireball" in search
- [ ] Search for "hp" returns "hit points" documents
- [ ] Corrections displayed in search response
- [ ] Dictionary regenerates after ingestion
- [ ] < 10ms total preprocessing time
- [ ] Default TTRPG synonyms comprehensive
- [ ] Tests pass for all modules
- [ ] CLAUDE.md updated with new architecture
