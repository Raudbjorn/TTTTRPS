# Requirements: NPC Indexes Migration to meilisearch-lib

## Overview

Migrate the NPC indexes subsystem from HTTP-based `meilisearch-sdk` to embedded `meilisearch-lib`. This enables NPC vocabulary, name components, and exclamation templates to be indexed and searched without requiring an external Meilisearch process.

---

## Context

### Current State

The NPC system uses three specialized Meilisearch indexes for NPC generation data:

| Index Name | Purpose | Documents |
|------------|---------|-----------|
| `ttrpg_vocabulary_banks` | Speech phrases by culture/role/race | VocabularyPhraseDocument |
| `ttrpg_name_components` | Name prefixes/roots/suffixes | NameComponentDocument |
| `ttrpg_exclamation_templates` | Emotional interjections | ExclamationTemplateDocument |

**Current Implementation:**
- `core/npc_gen/indexes.rs` - Uses `meilisearch_sdk::Client` for index management
- `commands/npc/indexes.rs` - Three Tauri commands currently disabled with TODO

**Blocked Functionality:**
- NPC vocabulary search for speech generation
- Cultural name generation with phonetic patterns
- Exclamation template lookup by emotion/intensity

### Target State

All NPC index operations use embedded `MeilisearchLib` via `state.embedded_search.inner()`, enabling:
- Offline NPC generation
- No external process management
- Consistent API with other migrated modules

---

## User Stories

### US-1: Initialize NPC Indexes
**As a** user starting the application for the first time,
**I want** NPC indexes to be automatically created with proper configuration,
**So that** NPC generation features work immediately.

### US-2: View Index Statistics
**As a** power user or developer,
**I want** to view statistics about indexed NPC data,
**So that** I can verify data was loaded correctly and troubleshoot issues.

### US-3: Clear and Rebuild Indexes
**As a** user updating NPC vocabulary data,
**I want** to clear existing indexes and reload fresh data,
**So that** I can incorporate new vocabulary banks or naming rules.

### US-4: Search Vocabulary Phrases
**As a** Game Master using NPC generation,
**I want** to search for speech phrases by culture, role, and formality,
**So that** generated NPCs speak authentically for their background.

### US-5: Generate Cultural Names
**As a** Game Master creating NPCs,
**I want** names generated from culturally-appropriate components,
**So that** NPC names feel consistent with their origins.

### US-6: Filter Exclamations by Emotion
**As a** Game Master during gameplay,
**I want** exclamations filtered by intensity and emotion,
**So that** NPC reactions match the dramatic context.

---

## Functional Requirements

### FR-1: Index Initialization

#### FR-1.1: Create Indexes
- WHEN `initialize_npc_indexes()` is called THEN system SHALL create three indexes:
  - `ttrpg_vocabulary_banks` with primary key `id`
  - `ttrpg_name_components` with primary key `id`
  - `ttrpg_exclamation_templates` with primary key `id`
- IF index already exists THEN system SHALL skip creation and update settings

#### FR-1.2: Configure Vocabulary Index Settings
- WHEN vocabulary index is created THEN system SHALL set:
  - Searchable: `phrase`, `category`, `bank_id`, `tags`
  - Filterable: `culture`, `role`, `race`, `category`, `formality`, `bank_id`, `tags`
  - Sortable: `frequency`

#### FR-1.3: Configure Name Components Index Settings
- WHEN name components index is created THEN system SHALL set:
  - Searchable: `component`, `meaning`, `phonetic_tags`
  - Filterable: `culture`, `component_type`, `gender`, `phonetic_tags`
  - Sortable: `frequency`

#### FR-1.4: Configure Exclamation Templates Index Settings
- WHEN exclamation templates index is created THEN system SHALL set:
  - Searchable: `template`, `emotion`
  - Filterable: `culture`, `intensity`, `emotion`, `religious`
  - Sortable: `frequency`

#### FR-1.5: Wait for Task Completion
- WHEN index operation is submitted THEN system SHALL wait for task completion
- IF task takes longer than 30 seconds THEN system SHALL return timeout error

### FR-2: Index Statistics

#### FR-2.1: Get Index Stats
- WHEN `get_npc_indexes_stats()` is called THEN system SHALL return:
  - `vocabulary_phrase_count`: Number of documents in vocabulary index
  - `name_component_count`: Number of documents in name components index
  - `exclamation_template_count`: Number of documents in exclamation templates index

#### FR-2.2: Handle Missing Indexes
- IF an index does not exist THEN system SHALL return 0 for that index's count
- System SHALL NOT fail if some indexes are missing

### FR-3: Clear Indexes

#### FR-3.1: Clear All Documents
- WHEN `clear_npc_indexes()` is called THEN system SHALL:
  - Delete all documents from `ttrpg_vocabulary_banks`
  - Delete all documents from `ttrpg_name_components`
  - Delete all documents from `ttrpg_exclamation_templates`
- System SHALL NOT delete the index structures themselves

#### FR-3.2: Wait for Completion
- WHEN clearing documents THEN system SHALL wait for all three operations to complete
- System SHALL return success only when all indexes are cleared

### FR-4: Document Operations

#### FR-4.1: Add Vocabulary Documents
- WHEN vocabulary phrases are loaded THEN system SHALL index `VocabularyPhraseDocument`:
  ```rust
  {
    id: String,           // "{bank_id}_{category}_{index}"
    phrase: String,
    bank_id: String,
    category: String,     // greeting, farewell, exclamation, negotiation, combat
    formality: String,    // formal, casual, hostile
    culture: Option<String>,
    role: Option<String>,
    race: Option<String>,
    frequency: f32,       // 0.0-1.0
    tags: Vec<String>,
  }
  ```

#### FR-4.2: Add Name Component Documents
- WHEN name components are loaded THEN system SHALL index `NameComponentDocument`:
  ```rust
  {
    id: String,           // "{culture}_{type}_{index}"
    component: String,    // e.g., "Aer", "iel"
    culture: String,
    component_type: String, // prefix, root, suffix, title, epithet
    gender: String,       // male, female, neutral, any
    frequency: f32,
    meaning: Option<String>,
    phonetic_tags: Vec<String>,
  }
  ```

#### FR-4.3: Add Exclamation Template Documents
- WHEN exclamation templates are loaded THEN system SHALL index `ExclamationTemplateDocument`:
  ```rust
  {
    id: String,
    template: String,     // May contain {placeholders}
    culture: String,
    intensity: String,    // mild, moderate, strong
    emotion: String,      // surprise, anger, joy, fear
    religious: bool,
    frequency: f32,
  }
  ```

### FR-5: Search Operations

#### FR-5.1: Search Vocabulary Phrases
- WHEN searching vocabulary THEN system SHALL support:
  - Text query on `phrase`, `category`, `bank_id`, `tags`
  - Filter by `culture`, `role`, `race`, `category`, `formality`, `bank_id`
  - Sort by `frequency` (descending for common phrases first)

#### FR-5.2: Search Name Components
- WHEN searching name components THEN system SHALL support:
  - Text query on `component`, `meaning`
  - Filter by `culture`, `component_type`, `gender`
  - Sort by `frequency`

#### FR-5.3: Search Exclamation Templates
- WHEN searching exclamations THEN system SHALL support:
  - Text query on `template`, `emotion`
  - Filter by `culture`, `intensity`, `emotion`, `religious`
  - Sort by `frequency`

---

## Non-Functional Requirements

### NFR-1: Performance

#### NFR-1.1: Index Initialization
- Index creation SHALL complete in < 5 seconds per index
- Settings application SHALL complete in < 2 seconds per index

#### NFR-1.2: Search Latency
- Searches on NPC indexes SHALL return in < 50ms
- Searches with filters SHALL return in < 100ms

#### NFR-1.3: Document Indexing
- Batch indexing of 1000 documents SHALL complete in < 10 seconds

### NFR-2: Error Handling

#### NFR-2.1: Clear Error Messages
- All errors SHALL include which index/operation failed
- Timeout errors SHALL include configured timeout duration

#### NFR-2.2: Graceful Degradation
- IF one index fails to initialize THEN other indexes SHALL still be created
- System SHALL log failures but not crash

### NFR-3: Backward Compatibility

#### NFR-3.1: API Preservation
- Tauri command signatures SHALL remain unchanged
- Return types (`NpcIndexStats`) SHALL remain unchanged

#### NFR-3.2: Document Format
- Document structures SHALL remain unchanged
- Existing YAML vocabulary files SHALL work without modification

### NFR-4: Consistency

#### NFR-4.1: Index Settings Parity
- Migrated index settings SHALL match existing SDK-based settings exactly
- Searchable/filterable/sortable attributes SHALL be identical

---

## Constraints

### C-1: No External Process
- All operations SHALL use embedded `MeilisearchLib`
- No HTTP communication to external Meilisearch instances

### C-2: Async Context
- Tauri commands are async
- `MeilisearchLib` operations are sync but wrapped in `spawn_blocking` when needed

### C-3: Shared State
- Access `MeilisearchLib` via `state.embedded_search.inner()`
- Multiple commands may run concurrently

---

## Assumptions

### A-1: EmbeddedSearch Availability
- `AppState.embedded_search` is initialized before NPC commands are called
- `EmbeddedSearch.inner()` provides thread-safe access to `MeilisearchLib`

### A-2: API Compatibility
- `MeilisearchLib` provides equivalent functionality to SDK:
  - `create_index()`, `index_exists()`
  - `update_settings()` with searchable/filterable/sortable
  - `add_documents()`, `delete_all_documents()`
  - `index_stats()` with document counts

### A-3: Task Model
- `MeilisearchLib` handles task management internally
- `wait_for_task()` available for synchronous completion

---

## Glossary

| Term | Definition |
|------|------------|
| **VocabularyPhraseDocument** | Indexed speech phrase with cultural/role metadata |
| **NameComponentDocument** | Indexed name part (prefix/root/suffix) with phonetic data |
| **ExclamationTemplateDocument** | Indexed emotional exclamation template |
| **NpcIndexStats** | Statistics about all three NPC indexes |
| **Formality** | Speech register: formal, casual, hostile |
| **ComponentType** | Name part type: prefix, root, suffix, title, epithet |
| **Intensity** | Exclamation strength: mild, moderate, strong |
