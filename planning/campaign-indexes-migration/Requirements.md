# Requirements: Campaign Indexes Migration to meilisearch-lib

## Overview

Migrate the campaign generation indexes from HTTP-based `meilisearch-sdk` to embedded `meilisearch-lib`. This enables campaign arcs, session plans, and plot points to be indexed and searched without requiring an external Meilisearch process.

---

## Context

### Current State

The campaign generation system uses three specialized Meilisearch indexes:

| Index Name | Purpose | Documents |
|------------|---------|-----------|
| `ttrpg_campaign_arcs` | Campaign narrative arcs | CampaignArc documents |
| `ttrpg_session_plans` | Session planning documents | SessionPlan documents |
| `ttrpg_plot_points` | Enhanced plot points with dependencies | PlotPoint documents |

**Current Implementation:**
- `core/campaign/meilisearch_client.rs` - `MeilisearchCampaignClient` with retry logic (~748 lines)
- `core/campaign/meilisearch_indexes.rs` - Index configurations with `IndexConfig` trait (~273 lines)

**Key Features:**
- Retry logic with exponential backoff for transient errors
- Batch document operations (up to 1000 documents)
- Health check with wait functionality
- Filter-based deletion for bulk operations
- Typed CRUD operations for each entity type

**Blocked Functionality:**
- Campaign arc search by status or type
- Session plan retrieval by campaign or session
- Plot point filtering by activation state, urgency, or tension level
- Campaign generation orchestration

### Target State

All campaign index operations use embedded `MeilisearchLib` via `state.embedded_search.inner()`, enabling:
- Offline campaign management
- No external process management
- Consistent API with other migrated modules

---

## User Stories

### US-1: Initialize Campaign Indexes
**As a** user starting the application for the first time,
**I want** campaign generation indexes to be automatically created,
**So that** campaign planning features work immediately.

### US-2: Create Campaign Arcs
**As a** Game Master planning a campaign,
**I want** to create and save narrative arcs,
**So that** I can track the overarching story structure.

### US-3: Search Session Plans
**As a** Game Master preparing for a session,
**I want** to search session plans by campaign and session number,
**So that** I can quickly find the relevant planning documents.

### US-4: Filter Plot Points by State
**As a** Game Master during gameplay,
**I want** to filter plot points by activation state and urgency,
**So that** I can identify the most relevant story hooks.

### US-5: Batch Update Documents
**As a** system processing campaign data,
**I want** to update multiple documents efficiently,
**So that** large campaigns can be processed quickly.

### US-6: Handle Transient Failures
**As a** system,
**I want** operations to retry on transient failures,
**So that** temporary issues don't cause data loss.

---

## Functional Requirements

### FR-1: Index Initialization

#### FR-1.1: Create Indexes
- WHEN `ensure_indexes()` is called THEN system SHALL create three indexes:
  - `ttrpg_campaign_arcs` with primary key `id`
  - `ttrpg_session_plans` with primary key `id`
  - `ttrpg_plot_points` with primary key `id`
- IF index already exists THEN system SHALL skip creation and update settings

#### FR-1.2: Configure Campaign Arcs Index
- WHEN arcs index is created THEN system SHALL set:
  - Searchable: `name`, `description`, `premise`
  - Filterable: `id`, `campaign_id`, `arc_type`, `status`, `is_main_arc`
  - Sortable: `name`, `display_order`, `started_at`, `created_at`

#### FR-1.3: Configure Session Plans Index
- WHEN plans index is created THEN system SHALL set:
  - Searchable: `title`, `summary`, `dramatic_questions`
  - Filterable: `id`, `campaign_id`, `session_id`, `arc_id`, `phase_id`, `status`, `is_template`
  - Sortable: `title`, `session_number`, `created_at`

#### FR-1.4: Configure Plot Points Index
- WHEN plot points index is created THEN system SHALL set:
  - Searchable: `title`, `description`, `dramatic_question`, `notes`
  - Filterable: `id`, `campaign_id`, `arc_id`, `plot_type`, `activation_state`, `status`, `urgency`, `tension_level`, `involved_npcs`, `involved_locations`, `tags`
  - Sortable: `title`, `tension_level`, `urgency`, `created_at`, `activated_at`

#### FR-1.5: Wait for Task Completion
- WHEN index operation is submitted THEN system SHALL wait for completion
- Short operations SHALL timeout after 30 seconds
- Long operations (batch, index creation) SHALL timeout after 300 seconds

### FR-2: Generic CRUD Operations

#### FR-2.1: Upsert Single Document
- WHEN `upsert_document(index, doc)` is called THEN system SHALL:
  - Add or update document in specified index
  - Wait for task completion
  - Retry on transient failures

#### FR-2.2: Upsert Multiple Documents (Batch)
- WHEN `upsert_documents(index, docs)` is called THEN system SHALL:
  - Process documents in batches of 1000
  - Wait for each batch to complete
  - Retry individual batches on transient failures
  - Log total documents upserted

#### FR-2.3: Get Document by ID
- WHEN `get_document(index, id)` is called THEN system SHALL:
  - Return `Some(T)` if document exists
  - Return `None` if document not found

#### FR-2.4: Delete Document by ID
- WHEN `delete_document(index, id)` is called THEN system SHALL:
  - Delete document from index
  - Wait for task completion
  - Retry on transient failures

#### FR-2.5: Delete Multiple Documents
- WHEN `delete_documents(index, ids)` is called THEN system SHALL:
  - Delete all documents with matching IDs
  - Wait for task completion

#### FR-2.6: Delete by Filter
- WHEN `delete_by_filter(index, filter)` is called THEN system SHALL:
  - Search for matching documents
  - Delete in batches
  - Return count of deleted documents

### FR-3: Search Operations

#### FR-3.1: Search with Filter and Sort
- WHEN `search(index, query, filter, sort, limit, offset)` is called THEN system SHALL:
  - Execute search query with optional filter
  - Apply optional sort order
  - Return up to `limit` results starting at `offset`

#### FR-3.2: List Documents
- WHEN `list(index, filter, sort, limit, offset)` is called THEN system SHALL:
  - Execute empty query search (list all)
  - Apply optional filter and sort
  - Return matching documents

#### FR-3.3: Count Documents
- WHEN `count(index, filter)` is called THEN system SHALL:
  - Execute search with limit 0
  - Return `estimated_total_hits` count

### FR-4: Typed Arc Operations

#### FR-4.1: Get Arc
- WHEN `get_arc(id)` is called THEN system SHALL return arc document or None

#### FR-4.2: List Arcs by Campaign
- WHEN `list_arcs(campaign_id)` is called THEN system SHALL:
  - Filter by `campaign_id`
  - Sort by `created_at:desc`
  - Return up to 1000 results

#### FR-4.3: Save Arc
- WHEN `save_arc(arc)` is called THEN system SHALL upsert to arcs index

#### FR-4.4: Delete Arc
- WHEN `delete_arc(id)` is called THEN system SHALL delete from arcs index

### FR-5: Typed Session Plan Operations

#### FR-5.1: Get Plan
- WHEN `get_plan(id)` is called THEN system SHALL return plan document or None

#### FR-5.2: Get Plan for Session
- WHEN `get_plan_for_session(session_id)` is called THEN system SHALL:
  - Filter by `session_id`
  - Return first matching plan or None

#### FR-5.3: List Plans by Campaign
- WHEN `list_plans(campaign_id, include_templates)` is called THEN system SHALL:
  - Filter by `campaign_id`
  - Optionally exclude templates
  - Sort by `session_number:desc`

#### FR-5.4: List Plan Templates
- WHEN `list_plan_templates(campaign_id)` is called THEN system SHALL:
  - Filter by `campaign_id` AND `is_template = true`
  - Sort by `title:asc`

#### FR-5.5: Save Plan
- WHEN `save_plan(plan)` is called THEN system SHALL upsert to plans index

#### FR-5.6: Delete Plan
- WHEN `delete_plan(id)` is called THEN system SHALL delete from plans index

### FR-6: Typed Plot Point Operations

#### FR-6.1: Get Plot Point
- WHEN `get_plot_point(id)` is called THEN system SHALL return plot point or None

#### FR-6.2: List Plot Points by Campaign
- WHEN `list_plot_points(campaign_id)` is called THEN system SHALL:
  - Filter by `campaign_id`
  - Sort by `created_at:desc`

#### FR-6.3: List by Activation State
- WHEN `list_plot_points_by_state(campaign_id, state)` is called THEN system SHALL:
  - Filter by `campaign_id` AND `activation_state`
  - Sort by `tension_level:desc`, `urgency:desc`

#### FR-6.4: List by Arc
- WHEN `list_plot_points_by_arc(arc_id)` is called THEN system SHALL:
  - Filter by `arc_id`
  - Sort by `created_at:asc`

#### FR-6.5: Save Plot Point
- WHEN `save_plot_point(point)` is called THEN system SHALL upsert to plot points index

#### FR-6.6: Delete Plot Point
- WHEN `delete_plot_point(id)` is called THEN system SHALL delete from plot points index

### FR-7: Health and Retry

#### FR-7.1: Health Check
- WHEN `health_check()` is called THEN system SHALL:
  - Check if Meilisearch is responsive
  - Return boolean indicating health status

#### FR-7.2: Wait for Health
- WHEN `wait_for_health(timeout)` is called THEN system SHALL:
  - Poll health every 500ms
  - Return true if healthy within timeout
  - Return false if timeout expires

#### FR-7.3: Retry Logic
- WHEN a transient error occurs THEN system SHALL:
  - Retry up to 3 times
  - Use exponential backoff (100ms base)
  - Only retry connection errors and timeouts
  - Not retry document-not-found or serialization errors

---

## Non-Functional Requirements

### NFR-1: Performance

#### NFR-1.1: Batch Operations
- Batch size SHALL be 1000 documents maximum
- Batch operations SHALL complete in < 5 seconds per batch

#### NFR-1.2: Single Document Operations
- CRUD operations SHALL complete in < 500ms

#### NFR-1.3: Search Operations
- Search operations SHALL complete in < 200ms

### NFR-2: Reliability

#### NFR-2.1: Transient Failure Handling
- Transient failures (connection, timeout) SHALL be retried
- Maximum 3 retry attempts
- Exponential backoff between attempts

#### NFR-2.2: Non-Transient Failures
- Document-not-found, serialization errors SHALL NOT be retried
- These SHALL be returned immediately to caller

### NFR-3: Error Handling

#### NFR-3.1: Clear Error Messages
- All errors SHALL include operation type
- Errors SHALL include index name where applicable
- Timeout errors SHALL include duration

### NFR-4: Backward Compatibility

#### NFR-4.1: API Preservation
- All public function signatures SHALL remain unchanged
- Return types SHALL remain unchanged
- Error types SHALL remain compatible

### NFR-5: Consistency

#### NFR-5.1: Index Settings Parity
- Migrated index settings SHALL match existing SDK-based settings
- `IndexConfig` trait implementations SHALL be preserved

---

## Constraints

### C-1: No External Process
- All operations SHALL use embedded `MeilisearchLib`
- Health check may need adaptation (no HTTP endpoint)

### C-2: Batch Size
- Maximum 1000 documents per batch operation
- This limit is maintained from current implementation

### C-3: Retry Logic
- Only connection and timeout errors are transient
- Document-level errors are not transient

---

## Assumptions

### A-1: EmbeddedSearch Availability
- `EmbeddedSearch` is initialized before campaign operations
- `EmbeddedSearch.inner()` provides `Arc<MeilisearchLib>`

### A-2: API Compatibility
- `MeilisearchLib` provides equivalent CRUD operations
- Search API supports query, filter, sort, limit, offset
- Stats API provides document counts

### A-3: Health Check Adaptation
- Embedded engine is always "healthy" if initialized
- May simplify or stub health check for embedded mode

---

## Glossary

| Term | Definition |
|------|------------|
| **CampaignArc** | Narrative arc spanning multiple sessions |
| **SessionPlan** | Planning document for a single session |
| **PlotPoint** | Story hook with tension, urgency, and dependencies |
| **ActivationState** | Plot point state: dormant, foreshadowed, active, resolved |
| **Transient Error** | Temporary failure that may succeed on retry |
| **Batch Size** | Maximum documents per batch operation (1000) |
