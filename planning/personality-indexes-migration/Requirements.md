# Requirements: Personality Indexes Migration to meilisearch-lib

## Overview

Migrate the personality template and blend rule indexes from HTTP-based `meilisearch-sdk` to embedded `meilisearch-lib`. This enables DM personality configuration and context-aware personality blending to work offline without requiring an external Meilisearch process.

---

## Context

### Current State

The personality system uses two specialized Meilisearch indexes:

| Index Name | Purpose | Documents |
|------------|---------|-----------|
| `ttrpg_personality_templates` | DM personality templates by setting/campaign | TemplateDocument |
| `ttrpg_blend_rules` | Context-aware personality blending rules | BlendRuleDocument |

**Current Implementation:**
- `core/personality/meilisearch.rs` - Uses `meilisearch_sdk::Client` via `PersonalityIndexManager`
- ~722 lines of code including CRUD operations, search, and utility functions

**Blocked Functionality:**
- DM personality template search by game system or campaign
- Context-aware personality blending (combat, social, exploration modes)
- Custom personality template creation and management

### Target State

All personality index operations use embedded `MeilisearchLib` via `state.embedded_search.inner()`, enabling:
- Offline personality management
- No external process management
- Consistent API with other migrated modules

---

## User Stories

### US-1: Initialize Personality Indexes
**As a** user starting the application for the first time,
**I want** personality indexes to be automatically created with proper configuration,
**So that** DM personality features work immediately.

### US-2: Create Custom Personality Templates
**As a** Game Master,
**I want** to create and save custom DM personality templates,
**So that** I can have consistent AI DM personalities across sessions.

### US-3: Search Templates by Setting
**As a** Game Master preparing a new campaign,
**I want** to search personality templates by game system and setting,
**So that** I can find appropriate DM personalities for my campaign.

### US-4: Configure Blend Rules
**As a** Game Master,
**I want** to configure context-aware personality blending rules,
**So that** the AI DM adjusts its tone for combat, social, and exploration scenes.

### US-5: View Personality Statistics
**As a** power user,
**I want** to view statistics about indexed personality templates and rules,
**So that** I can verify my configurations are properly stored.

### US-6: Clear and Rebuild Personality Data
**As a** user updating personality configurations,
**I want** to clear existing indexes and reload fresh data,
**So that** I can incorporate new templates or fix corrupted data.

---

## Functional Requirements

### FR-1: Index Initialization

#### FR-1.1: Create Indexes
- WHEN `initialize_indexes()` is called THEN system SHALL create two indexes:
  - `ttrpg_personality_templates` with primary key `id`
  - `ttrpg_blend_rules` with primary key `id`
- IF index already exists THEN system SHALL skip creation and update settings

#### FR-1.2: Configure Templates Index Settings
- WHEN templates index is created THEN system SHALL set:
  - Searchable: `name`, `description`, `vocabularyKeys`, `commonPhrases`
  - Filterable: `gameSystem`, `settingName`, `isBuiltin`, `tags`, `campaignId`
  - Sortable: `name`, `createdAt`, `updatedAt`

#### FR-1.3: Configure Blend Rules Index Settings
- WHEN blend rules index is created THEN system SHALL set:
  - Searchable: `name`, `description`
  - Filterable: `context`, `enabled`, `isBuiltin`, `tags`, `campaignId`
  - Sortable: `name`, `priority`, `createdAt`, `updatedAt`

#### FR-1.4: Wait for Task Completion
- WHEN index operation is submitted THEN system SHALL wait for task completion
- IF task takes longer than 30 seconds THEN system SHALL return timeout error

### FR-2: Template CRUD Operations

#### FR-2.1: Upsert Template
- WHEN `upsert_template()` is called THEN system SHALL:
  - Convert `SettingPersonalityTemplate` to `TemplateDocument`
  - Add document to templates index
  - Wait for task completion
  - Return `Ok(())` on success

#### FR-2.2: Get Template
- WHEN `get_template(id)` is called THEN system SHALL:
  - Retrieve document by ID from templates index
  - Return `Some(TemplateDocument)` if found
  - Return `None` if document not found

#### FR-2.3: Delete Template
- WHEN `delete_template(id)` is called THEN system SHALL:
  - Delete document by ID from templates index
  - Wait for task completion
  - Return `Ok(())` on success

#### FR-2.4: Search Templates
- WHEN `search_templates(query, filter, limit)` is called THEN system SHALL:
  - Execute search query on templates index
  - Apply optional filter expression
  - Return up to `limit` results
  - Return `Vec<TemplateDocument>`

### FR-3: Blend Rule CRUD Operations

#### FR-3.1: Upsert Blend Rule
- WHEN `upsert_blend_rule()` is called THEN system SHALL:
  - Convert `BlendRule` to `BlendRuleDocument`
  - Add document to blend rules index
  - Wait for task completion

#### FR-3.2: Get Blend Rule
- WHEN `get_blend_rule(id)` is called THEN system SHALL:
  - Retrieve document by ID from blend rules index
  - Return `Some(BlendRuleDocument)` if found
  - Return `None` if not found

#### FR-3.3: Delete Blend Rule
- WHEN `delete_blend_rule(id)` is called THEN system SHALL:
  - Delete document by ID from blend rules index
  - Wait for task completion

#### FR-3.4: Search Blend Rules
- WHEN `search_blend_rules(query, filter, limit)` is called THEN system SHALL:
  - Execute search query on blend rules index
  - Apply optional filter expression
  - Return up to `limit` results

#### FR-3.5: List Enabled Rules by Priority
- WHEN `list_enabled_rules(limit)` is called THEN system SHALL:
  - Filter by `enabled = true`
  - Sort by `priority:desc`
  - Return ordered list of enabled rules

### FR-4: Statistics and Cleanup

#### FR-4.1: Get Index Stats
- WHEN `get_stats()` is called THEN system SHALL return:
  - `template_count`: Number of documents in templates index
  - `rule_count`: Number of documents in blend rules index

#### FR-4.2: Handle Missing Indexes
- IF an index does not exist THEN system SHALL return 0 for that index's count
- System SHALL NOT fail if some indexes are missing

#### FR-4.3: Clear Templates
- WHEN `clear_templates()` is called THEN system SHALL:
  - Delete all documents from templates index
  - Preserve index structure and settings

#### FR-4.4: Clear Blend Rules
- WHEN `clear_blend_rules()` is called THEN system SHALL:
  - Delete all documents from blend rules index
  - Preserve index structure and settings

#### FR-4.5: Delete Indexes
- WHEN `delete_indexes()` is called THEN system SHALL:
  - Delete both personality indexes entirely
  - Continue on individual failures (best-effort cleanup)
  - Log warnings for failures but not crash

---

## Non-Functional Requirements

### NFR-1: Performance

#### NFR-1.1: Index Operations
- Index creation SHALL complete in < 5 seconds per index
- Settings application SHALL complete in < 2 seconds per index

#### NFR-1.2: CRUD Operations
- Single document operations SHALL complete in < 500ms
- Search operations SHALL return in < 100ms

### NFR-2: Error Handling

#### NFR-2.1: Clear Error Messages
- All errors SHALL include which index/operation failed
- Timeout errors SHALL include configured timeout duration

#### NFR-2.2: Graceful Degradation
- IF one index fails to initialize THEN other indexes SHALL still be created
- Delete operations SHALL be best-effort (log warnings, don't fail)

### NFR-3: Backward Compatibility

#### NFR-3.1: API Preservation
- All public function signatures in `PersonalityIndexManager` SHALL remain unchanged
- Return types (`PersonalityIndexStats`, `TemplateDocument`, etc.) SHALL remain unchanged

#### NFR-3.2: Document Format
- Document structures SHALL remain unchanged
- Existing serialized data SHALL deserialize correctly

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
- `PersonalityIndexManager` methods are async
- `MeilisearchLib` operations are sync but wrapped in blocking tasks when needed

### C-3: Thread Safety
- `MeilisearchLib` access via `Arc<MeilisearchLib>`
- Multiple operations may run concurrently

---

## Assumptions

### A-1: EmbeddedSearch Availability
- `EmbeddedSearch` is initialized before personality operations are called
- `EmbeddedSearch.inner()` provides thread-safe access to `MeilisearchLib`

### A-2: API Compatibility
- `MeilisearchLib` provides equivalent functionality to SDK:
  - `create_index()`, `index_exists()`
  - `update_settings()` with searchable/filterable/sortable
  - `add_documents()`, `get_document()`, `delete_document()`
  - `search()` with query, filter, sort, limit
  - `index_stats()` with document counts

### A-3: Task Model
- `MeilisearchLib` handles task management internally
- `wait_for_task()` available for synchronous completion

---

## Glossary

| Term | Definition |
|------|------------|
| **TemplateDocument** | Indexed personality template with vocabulary keys and common phrases |
| **BlendRuleDocument** | Indexed context-aware blending rule with priority |
| **PersonalityIndexStats** | Statistics about both personality indexes |
| **BlendContext** | Context type: combat, social, exploration, narrative |
| **Formality** | Personality tone register: formal, casual, etc. |
