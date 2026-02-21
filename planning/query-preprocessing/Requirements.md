# Query Preprocessing Architecture Requirements

## REQ-QP-001: Typo Tolerance via SymSpell

**Category:** Functional
**Priority:** High
**Source:** Reducing Meilisearch reliance while maintaining search quality

### User Story
As a GM using the TTRPG assistant, I want my search queries to find relevant results even when I make typos, so that I can quickly find rules during live sessions without perfect spelling.

### Acceptance Criteria

1. **REQ-QP-001.1**: The system SHALL correct single-character typos in words ≥5 characters (e.g., "firball" → "fireball")
2. **REQ-QP-001.2**: The system SHALL correct up to 2-character typos in words ≥9 characters (e.g., "resistence" → "resistance")
3. **REQ-QP-001.3**: The system SHALL NOT modify words shorter than 5 characters
4. **REQ-QP-001.4**: The system SHALL support a protected words list that bypasses correction (proper nouns, game-specific terms)
5. **REQ-QP-001.5**: The system SHALL use a domain-specific TTRPG corpus dictionary layered on English dictionary
6. **REQ-QP-001.6**: The system SHALL handle compound word correction (e.g., "magicmissle" → "magic missile")
7. **REQ-QP-001.7**: Typo correction SHALL complete in < 5ms for typical queries (≤10 words)

### Dependencies
- SymSpell crate (symspell or fast_symspell)
- TTRPG corpus frequency dictionary (generated from indexed content)
- Bigram dictionary for compound words

---

## REQ-QP-002: Synonym Expansion

**Category:** Functional
**Priority:** High
**Source:** Domain-specific search enhancement for TTRPG terminology

### User Story
As a GM searching for rules, I want my search for "hp" to also find documents containing "hit points" or "health", so that I can find relevant content regardless of which terminology the source uses.

### Acceptance Criteria

1. **REQ-QP-002.1**: The system SHALL support multi-way synonyms where all terms are interchangeable (e.g., "hp" ↔ "hit points" ↔ "health")
2. **REQ-QP-002.2**: The system SHALL support one-way synonyms where source expands to targets but not reverse (e.g., "dragon" → ["wyrm", "drake"])
3. **REQ-QP-002.3**: The system SHALL limit synonym expansions per term to prevent query explosion (default: 5)
4. **REQ-QP-002.4**: The system SHALL ship with a default TTRPG synonym dictionary covering:
   - Stat abbreviations (str/strength, dex/dexterity, etc.)
   - Game mechanics (aoo/attack of opportunity, crit/critical hit, etc.)
   - Condition terms (prone/knocked down, grappled/grabbed, etc.)
   - Book abbreviations (phb/player's handbook, dmg/dungeon master's guide, etc.)
5. **REQ-QP-002.5**: Users SHALL be able to define custom synonyms via configuration file or database
6. **REQ-QP-002.6**: Campaign-specific synonyms SHALL be supported (isolated per campaign)

### Dependencies
- Synonym map data structure
- TOML/JSON configuration loading
- Optional: Database storage for user-defined synonyms

---

## REQ-QP-003: Query Pipeline Integration

**Category:** Functional
**Priority:** High
**Source:** Unified preprocessing before search execution

### User Story
As a system component, I need to process raw user queries through typo correction and synonym expansion before executing search, so that both BM25 and vector search receive optimized queries.

### Acceptance Criteria

1. **REQ-QP-003.1**: Query preprocessing SHALL follow this order:
   - Normalize input (trim, lowercase)
   - Apply typo correction
   - Apply synonym expansion (on corrected text)
   - Generate search queries for both BM25 and vector paths
2. **REQ-QP-003.2**: The system SHALL provide corrections metadata (original → corrected) for "Did you mean?" UI
3. **REQ-QP-003.3**: Vector search input SHALL use corrected text (not synonym-expanded) to avoid embedding noise
4. **REQ-QP-003.4**: BM25 search input SHALL use synonym-expanded text with OR-joined groups
5. **REQ-QP-003.5**: The pipeline SHALL be backend-agnostic (work with SurrealDB, sqlite-vec, or any future backend)
6. **REQ-QP-003.6**: Total preprocessing time SHALL be < 10ms for typical queries

### Dependencies
- TypoCorrector module
- SynonymMap module
- Backend-specific query formatters

---

## REQ-QP-004: Corpus Dictionary Generation

**Category:** Functional
**Priority:** Medium
**Source:** Keeping typo correction in sync with indexed content

### User Story
As the system, I need to rebuild the corpus frequency dictionary when content is indexed, so that domain-specific terms are correctly recognized and prioritized in typo correction.

### Acceptance Criteria

1. **REQ-QP-004.1**: The system SHALL generate a frequency dictionary from all indexed chunks
2. **REQ-QP-004.2**: Corpus terms SHALL be boosted relative to general English dictionary (10x frequency multiplier)
3. **REQ-QP-004.3**: The system SHALL generate a bigram dictionary for compound word detection
4. **REQ-QP-004.4**: Dictionary regeneration SHALL run after bulk ingestion operations
5. **REQ-QP-004.5**: Dictionary files SHALL be stored in the application data directory

### Dependencies
- Document ingestion pipeline hooks
- File I/O for dictionary generation

---

## REQ-QP-005: SurrealDB Query Formatting

**Category:** Functional
**Priority:** High
**Source:** Integration with SurrealDB backend

### User Story
As the search system, I need to format preprocessed queries into valid SurrealQL for hybrid search execution.

### Acceptance Criteria

1. **REQ-QP-005.1**: The system SHALL generate SurrealQL FTS queries with OR-expanded synonyms:
   ```sql
   (content @1@ 'fireball' OR content @1@ 'fire bolt') AND (content @1@ 'damage' OR content @1@ 'harm')
   ```
2. **REQ-QP-005.2**: The system SHALL support multi-word synonyms in FTS queries
3. **REQ-QP-005.3**: The system SHALL integrate with existing hybrid search (BM25 + HNSW + RRF)

### Dependencies
- SurrealDB storage module
- Existing hybrid search implementation

---

## Non-Functional Requirements

### NFR-QP-001: Performance
- Typo correction: < 5ms per query
- Synonym expansion: < 1ms per query
- Total preprocessing: < 10ms per query
- Memory for SymSpell dictionary: < 50MB

### NFR-QP-002: Maintainability
- Preprocessing modules SHALL be independently testable (pure functions)
- Configuration SHALL be externalized (not hardcoded)
- Logging SHALL record corrections for debugging

### NFR-QP-003: Extensibility
- Architecture SHALL support additional preprocessing steps (e.g., stemming, lemmatization)
- Synonym sources SHALL be pluggable (file, database, remote API)

---

## Constraints

1. **CONS-001**: Must work in embedded mode (no external services)
2. **CONS-002**: Must compile to WASM for future web deployment (excludes some crates)
3. **CONS-003**: BSL-licensed dependencies are acceptable (already using SurrealDB)
4. **CONS-004**: SymSpell dictionary format must be compatible with standard frequency dictionaries

---

## Glossary

| Term | Definition |
|------|------------|
| **BM25** | Best Matching 25 ranking function for full-text search |
| **HNSW** | Hierarchical Navigable Small World graph for vector search |
| **RRF** | Reciprocal Rank Fusion for combining search results |
| **SymSpell** | Symmetric delete spelling correction algorithm |
| **FST** | Finite State Transducer for efficient string matching |
| **Corpus Dictionary** | Word frequency list built from indexed content |
| **Protected Words** | Terms that should never be typo-corrected |
