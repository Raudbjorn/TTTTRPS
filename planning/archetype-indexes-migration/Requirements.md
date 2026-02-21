# Requirements: Archetype Indexes Migration to meilisearch-lib

## Overview

Migrate the archetype and vocabulary bank indexes from HTTP-based `meilisearch-sdk` to embedded `meilisearch-lib`. This enables character archetype definitions and NPC vocabulary banks to be indexed and searched without requiring an external Meilisearch process.

---

## Context

### Current State

The archetype system uses two specialized Meilisearch indexes:

| Index Name | Purpose | Documents |
|------------|---------|-----------|
| `ttrpg_archetypes` | Character archetype definitions | Archetype documents |
| `ttrpg_npc_vocabulary_banks` | NPC phrase collections | VocabularyBank documents |

**Current Implementation:**
- `core/archetype/meilisearch.rs` - Uses `meilisearch_sdk::Client` via `ArchetypeIndexManager`
- ~482 lines of code including index management, existence checks, and statistics

**Blocked Functionality:**
- Archetype search by category, game system, or setting
- Vocabulary bank search by culture, role, or race
- NPC dialogue generation using indexed phrase collections

### Target State

All archetype index operations use embedded `MeilisearchLib` via `state.embedded_search.inner()`, enabling:
- Offline archetype and vocabulary management
- No external process management
- Consistent API with other migrated modules

---

## User Stories

### US-1: Initialize Archetype Indexes
**As a** user starting the application for the first time,
**I want** archetype indexes to be automatically created with proper configuration,
**So that** NPC generation features work immediately.

### US-2: Search Archetypes by Category
**As a** Game Master building NPCs,
**I want** to search archetypes by category (role, race, class),
**So that** I can find appropriate character templates quickly.

### US-3: Filter by Game System
**As a** Game Master using a specific RPG system,
**I want** to filter archetypes by game system,
**So that** I only see archetypes compatible with my campaign.

### US-4: Search Vocabulary Banks
**As a** Game Master generating NPC dialogue,
**I want** to search vocabulary banks by culture and role,
**So that** NPCs speak authentically for their background.

### US-5: View Index Statistics
**As a** power user,
**I want** to view statistics about indexed archetypes and vocabulary banks,
**So that** I can verify data was loaded correctly.

### US-6: Clear and Rebuild Indexes
**As a** user updating archetype or vocabulary data,
**I want** to clear existing indexes and reload fresh data,
**So that** I can incorporate new content packs.

---

## Functional Requirements

### FR-1: Index Initialization

#### FR-1.1: Create Indexes
- WHEN `ensure_indexes()` is called THEN system SHALL create two indexes:
  - `ttrpg_archetypes` with primary key `id`
  - `ttrpg_npc_vocabulary_banks` with primary key `id`
- IF index already exists THEN system SHALL skip creation and update settings

#### FR-1.2: Configure Archetypes Index Settings
- WHEN archetypes index is created THEN system SHALL set:
  - Searchable: `display_name`, `description`, `tags`
  - Filterable: `id`, `category`, `parent_id`, `setting_pack_id`, `game_system`, `tags`
  - Sortable: `display_name`, `category`, `created_at`

#### FR-1.3: Configure Vocabulary Banks Index Settings
- WHEN vocabulary banks index is created THEN system SHALL set:
  - Searchable: `display_name`, `description`, `phrase_texts`
  - Filterable: `id`, `culture`, `role`, `race`, `categories`, `formality_range`
  - Sortable: `display_name`, `created_at`

#### FR-1.4: Wait for Task Completion
- WHEN index operation is submitted THEN system SHALL wait for task completion
- IF task takes longer than 30 seconds THEN system SHALL return timeout error

### FR-2: Index Existence Checks

#### FR-2.1: Check Archetypes Index
- WHEN `archetypes_index_exists()` is called THEN system SHALL:
  - Return `Ok(true)` if the index exists
  - Return `Ok(false)` if the index does not exist
  - Return `Err(...)` if there was an error checking

#### FR-2.2: Check Vocabulary Banks Index
- WHEN `vocabulary_banks_index_exists()` is called THEN system SHALL:
  - Return `Ok(true)` if the index exists
  - Return `Ok(false)` if the index does not exist
  - Return `Err(...)` if there was an error checking

### FR-3: Document Counts

#### FR-3.1: Get Archetype Count
- WHEN `archetype_count()` is called THEN system SHALL:
  - Return number of documents in archetypes index
  - Return 0 if index doesn't exist

#### FR-3.2: Get Vocabulary Bank Count
- WHEN `vocabulary_bank_count()` is called THEN system SHALL:
  - Return number of documents in vocabulary banks index
  - Return 0 if index doesn't exist

### FR-4: Index Deletion

#### FR-4.1: Delete Both Indexes
- WHEN `delete_indexes()` is called THEN system SHALL:
  - Delete `ttrpg_archetypes` index
  - Delete `ttrpg_npc_vocabulary_banks` index
  - Continue if individual deletions fail (index not found is OK)
  - Log appropriate messages

---

## Non-Functional Requirements

### NFR-1: Performance

#### NFR-1.1: Index Operations
- Index creation SHALL complete in < 5 seconds per index
- Settings application SHALL complete in < 2 seconds per index

#### NFR-1.2: Existence Checks
- Index existence checks SHALL complete in < 100ms

#### NFR-1.3: Document Counts
- Document count retrieval SHALL complete in < 500ms

### NFR-2: Error Handling

#### NFR-2.1: Clear Error Messages
- All errors SHALL include which index/operation failed
- Error messages SHALL be user-friendly and actionable

#### NFR-2.2: Graceful Degradation
- IF one index fails to initialize THEN other index SHALL still be created
- Index-not-found errors during deletion SHALL be handled gracefully

### NFR-3: Backward Compatibility

#### NFR-3.1: API Preservation
- All public function signatures in `ArchetypeIndexManager` SHALL remain unchanged
- Convenience functions (`archetype_index_name()`, etc.) SHALL remain unchanged

#### NFR-3.2: Index Names
- Index name constants SHALL remain unchanged
- Public accessor functions SHALL return same values

### NFR-4: Consistency

#### NFR-4.1: Index Settings Parity
- Migrated index settings SHALL match existing SDK-based settings exactly
- Searchable/filterable/sortable attributes SHALL be identical

---

## Constraints

### C-1: No External Process
- All operations SHALL use embedded `MeilisearchLib`
- No HTTP communication to external Meilisearch instances

### C-2: Reference Pattern
- `ArchetypeIndexManager` takes a reference `&'a Client` in SDK version
- Must adapt to take `Arc<MeilisearchLib>` or reference

### C-3: Thread Safety
- `MeilisearchLib` access via `Arc<MeilisearchLib>`
- Manager may be used from multiple threads

---

## Assumptions

### A-1: EmbeddedSearch Availability
- `EmbeddedSearch` is initialized before archetype operations are called
- `EmbeddedSearch.inner()` provides thread-safe access to `MeilisearchLib`

### A-2: API Compatibility
- `MeilisearchLib` provides equivalent functionality to SDK:
  - `create_index()`, `get_index()`, `delete_index()`
  - `update_settings()` with searchable/filterable/sortable
  - `get_stats()` for document counts
  - `wait_for_task()` for synchronous completion

### A-3: Index Existence
- `MeilisearchLib.index_exists(name)` or equivalent method available
- Can detect IndexNotFound error from `get_index()`

---

## Glossary

| Term | Definition |
|------|------------|
| **Archetype** | Character template with personality affinities and role mappings |
| **VocabularyBank** | Collection of phrases organized by culture, role, and race |
| **SettingPack** | Collection of archetypes for a specific game setting |
| **Category** | Archetype type: role, race, class, setting, custom |
| **Formality** | Speech register level in vocabulary banks |
