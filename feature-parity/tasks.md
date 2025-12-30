# Feature Parity Implementation Tasks

This document contains actionable tasks for achieving feature parity, organized by priority and phase. Each task references the requirement(s) it fulfills.

## Document Information

| Field | Value |
|-------|-------|
| Version | 1.1.0 |
| Last Updated | 2025-12-29 |
| Status | Draft |

---

## Task Status Legend

| Status | Description |
|--------|-------------|
| `[ ]` | Not Started |
| `[~]` | In Progress |
| `[x]` | Completed |
| `[!]` | Blocked |

---

## Phase 1: Core Infrastructure (P0)

### TASK-001: Setup SQLite Database Layer
**Requirement:** REQ-CAMP-001, REQ-CAMP-002, REQ-SESS-001

**Status:** `[ ]`

**Description:**
Implement SQLite database layer for persistent structured data storage alongside Meilisearch.

**Subtasks:**
- [ ] Add rusqlite dependency to Cargo.toml
- [ ] Create `src-tauri/src/core/database/mod.rs` module
- [ ] Implement database connection pool
- [ ] Create migration system for schema versioning
- [ ] Implement initial schema (campaigns, sessions, characters, npcs, locations)
- [ ] Add database initialization to AppState
- [ ] Write unit tests for database operations

**Files to Create/Modify:**
- `src-tauri/Cargo.toml`
- `src-tauri/src/core/database/mod.rs`
- `src-tauri/src/core/database/migrations.rs`
- `src-tauri/src/core/database/schema.rs`
- `src-tauri/src/commands.rs` (add state)
- `src-tauri/src/main.rs` (initialize)

**Acceptance Criteria:**
- Database file created in app data directory
- Migrations run automatically on startup
- All tables from design.md schema exist
- CRUD operations work for all entity types

---

### TASK-002: Implement LLM Provider Router
**Requirement:** REQ-LLM-001, REQ-LLM-004, REQ-LLM-005

**Status:** `[ ]`

**Description:**
Create a unified router for LLM providers with health monitoring and cost tracking.

**Subtasks:**
- [ ] Create `src-tauri/src/core/llm/router.rs`
- [ ] Define `LLMProvider` trait with unified interface
- [ ] Implement provider health tracking
- [ ] Add cost calculation per provider
- [ ] Implement request routing logic
- [ ] Add fallback on provider failure
- [ ] Create usage statistics tracking
- [ ] Add Tauri commands for router operations

**Files to Create/Modify:**
- `src-tauri/src/core/llm/router.rs`
- `src-tauri/src/core/llm/health.rs`
- `src-tauri/src/core/llm/cost.rs`
- `src-tauri/src/core/llm/mod.rs`
- `src-tauri/src/commands.rs`

**Acceptance Criteria:**
- All 10 providers accessible through router
- Health status tracked per provider
- Cost estimates available before request
- Automatic failover when provider fails

---

### TASK-003: Implement Streaming Chat Responses
**Requirement:** REQ-LLM-003

**Status:** `[ ]`

**Description:**
Add streaming response support to chat UI for real-time token delivery.

**Subtasks:**
- [ ] Add streaming endpoint to LLMClient
- [ ] Implement SSE parsing for each provider
- [ ] Create Tauri event for streaming chunks
- [ ] Update chat component to handle streaming
- [ ] Add typing indicator during stream
- [ ] Handle stream termination gracefully
- [ ] Add cancel stream functionality

**Files to Create/Modify:**
- `src-tauri/src/core/llm.rs` (add streaming methods)
- `src-tauri/src/commands.rs` (streaming command)
- `frontend/src/components/chat.rs` (streaming UI)
- `frontend/src/bindings.rs` (event listener)

**Acceptance Criteria:**
- Tokens appear in UI as they arrive
- User can cancel ongoing stream
- Error handling for stream interruption
- Works with all streaming-capable providers

---

### TASK-004: Create Voice Profile System
**Requirement:** REQ-VOICE-002

**Status:** `[ ]`

**Description:**
Implement voice profile management with NPC linking.

**Subtasks:**
- [ ] Create `VoiceProfile` struct and database table
- [ ] Add profile CRUD operations
- [ ] Create 13+ preset DM personas
- [ ] Implement NPC-to-profile linking
- [ ] Add profile metadata (age, gender, personality)
- [ ] Create profile selector component
- [ ] Add Tauri commands for profile management

**Files to Create/Modify:**
- `src-tauri/src/core/voice/profiles.rs`
- `src-tauri/src/core/voice/presets.rs`
- `src-tauri/src/commands.rs`
- `frontend/src/components/voice/profile_manager.rs`
- `frontend/src/bindings.rs`

**Acceptance Criteria:**
- Create, edit, delete voice profiles
- Link profiles to NPCs
- 13+ preset personas available
- Profile selection in synthesis

---

### TASK-005: Implement Audio Cache System
**Requirement:** REQ-VOICE-003

**Status:** `[ ]`

**Description:**
Create disk-based audio cache with LRU eviction.

**Subtasks:**
- [ ] Create `AudioCache` struct
- [ ] Implement cache key generation (text + voice + settings hash)
- [ ] Add disk storage with size tracking
- [ ] Implement LRU eviction policy
- [ ] Add cache hit/miss tracking
- [ ] Create cache management commands
- [ ] Add cache statistics UI

**Files to Create/Modify:**
- `src-tauri/src/core/voice/cache.rs`
- `src-tauri/src/core/voice/manager.rs`
- `src-tauri/src/commands.rs`
- `frontend/src/components/settings.rs`

**Acceptance Criteria:**
- Audio cached to disk
- Size limits enforced
- LRU eviction works
- Cache stats available

---

## Phase 2: Campaign Enhancement (P1)

### TASK-006: Implement Campaign Versioning
**Requirement:** REQ-CAMP-002

**Status:** `[ ]`

**Description:**
Add version history and rollback capability for campaigns.

**Subtasks:**
- [ ] Create campaign_versions table
- [ ] Implement snapshot creation (manual and auto)
- [ ] Add diff calculation between versions
- [ ] Create rollback functionality
- [ ] Add version listing command
- [ ] Create version comparison command
- [ ] Add version history UI component

**Files to Create/Modify:**
- `src-tauri/src/core/campaign/versioning.rs`
- `src-tauri/src/core/campaign_manager.rs`
- `src-tauri/src/commands.rs`
- `frontend/src/components/campaign/version_history.rs`
- `frontend/src/bindings.rs`

**Acceptance Criteria:**
- Manual snapshots with descriptions
- Auto-snapshots on significant changes
- Version comparison shows diff
- Rollback restores campaign state

---

### TASK-007: Add World State Tracking
**Requirement:** REQ-CAMP-003

**Status:** `[ ]`

**Description:**
Implement world state management for campaigns.

**Subtasks:**
- [ ] Add world_state JSON column to campaigns
- [ ] Create in-game date tracking
- [ ] Implement world events timeline
- [ ] Add location state changes
- [ ] Create NPC relationship tracking
- [ ] Add custom state fields support
- [ ] Create world state editor UI

**Files to Create/Modify:**
- `src-tauri/src/core/models.rs` (Campaign struct)
- `src-tauri/src/core/campaign/world_state.rs`
- `src-tauri/src/commands.rs`
- `frontend/src/components/campaign/world_state_editor.rs`

**Acceptance Criteria:**
- In-game date tracked and editable
- World events recorded
- Location states updated
- Custom fields supported

---

### TASK-008: Create Campaign Dashboard UI
**Requirement:** REQ-UI-002

**Status:** `[ ]`

**Description:**
Build comprehensive campaign management dashboard.

**Subtasks:**
- [ ] Create campaign list view with cards
- [ ] Add campaign creation wizard/modal
- [ ] Build campaign details view
- [ ] Add quick actions (start session, add NPC, etc.)
- [ ] Create entity browser (characters, NPCs, locations)
- [ ] Add campaign switcher in header
- [ ] Implement campaign archive/restore

**Files to Create/Modify:**
- `frontend/src/components/campaign/mod.rs`
- `frontend/src/components/campaign/campaign_dashboard.rs`
- `frontend/src/components/campaign/campaign_card.rs`
- `frontend/src/components/campaign/entity_browser.rs`
- `frontend/src/routes.rs`

**Acceptance Criteria:**
- View all campaigns
- Create new campaigns with wizard
- Quick access to campaign entities
- Start sessions from dashboard

---

### TASK-009: Implement Entity Relationships
**Requirement:** REQ-CAMP-003

**Status:** `[ ]`

**Description:**
Add relationship tracking between campaign entities using a dedicated relationships table (see design.md §3.1 for schema). This replaces the previous JSON-based approach for better query performance and relational integrity.

**Subtasks:**
- [ ] Implement `entity_relationships` table migration (schema in design.md)
- [ ] Create `EntityRelationship` struct with proper types
- [ ] Implement CRUD operations for relationships
- [ ] Add bidirectional relationship queries
- [ ] Support relationship types: ally, enemy, family, employee, located_at, etc.
- [ ] Implement relationship strength (0.0-1.0 scale)
- [ ] Add Tauri commands for relationship management
- [ ] Create relationship graph visualization component
- [ ] Build relationship editor UI

**Files to Create/Modify:**
- `src-tauri/src/core/database/schema.rs` (add migration)
- `src-tauri/src/core/campaign/relationships.rs` (new module)
- `src-tauri/src/core/campaign/mod.rs` (export module)
- `src-tauri/src/commands.rs` (add commands)
- `frontend/src/components/campaign/relationship_graph.rs`
- `frontend/src/components/campaign/relationship_editor.rs`

**Acceptance Criteria:**
- Entity relationships stored in dedicated table with proper indexes
- Support NPC↔NPC, NPC↔Location, Character↔NPC, Quest↔Entity relationships
- Query relationships by source, target, or type
- Bidirectional relationships handled correctly
- Relationship visualization as interactive graph
- Edit/delete relationships from UI

---

## Phase 3: Search & RAG (P1)

### TASK-010: Add Embedding Provider Integration
**Requirement:** REQ-SEARCH-001

**Status:** `[ ]`

**Description:**
Integrate embedding providers for vector search.

**Subtasks:**
- [ ] Define EmbeddingProvider trait
- [ ] Implement Ollama embeddings provider
- [ ] Add OpenAI embeddings support
- [ ] Create embedding cache
- [ ] Add batch embedding support
- [ ] Implement embedding on document ingestion
- [ ] Store embeddings in Meilisearch

**Files to Create/Modify:**
- `src-tauri/src/core/search/embeddings.rs`
- `src-tauri/src/core/search/providers/ollama.rs`
- `src-tauri/src/core/search/providers/openai.rs`
- `src-tauri/src/ingestion/pipeline.rs`

**Acceptance Criteria:**
- Generate embeddings for text
- Store embeddings with documents
- Support multiple embedding providers
- Batch processing for large documents

---

### TASK-011: Implement Hybrid Search Engine
**Requirement:** REQ-SEARCH-001, REQ-SEARCH-003

**Status:** `[ ]`

**Description:**
Create hybrid search combining keyword and vector search with RRF.

**Subtasks:**
- [ ] Create HybridSearchEngine struct
- [ ] Implement keyword search (Meilisearch)
- [ ] Add vector similarity search
- [ ] Implement Reciprocal Rank Fusion
- [ ] Add configurable weights
- [ ] Create search options struct
- [ ] Add Tauri hybrid_search command

**Files to Create/Modify:**
- `src-tauri/src/core/search/hybrid.rs`
- `src-tauri/src/core/search/fusion.rs`
- `src-tauri/src/commands.rs`
- `frontend/src/bindings.rs`

**Acceptance Criteria:**
- Combine keyword and semantic results
- RRF produces better rankings
- Weight configuration works
- Performance within requirements

---

### TASK-012: Add Query Enhancement
**Requirement:** REQ-SEARCH-003

**Status:** `[ ]`

**Description:**
Implement query expansion and correction for TTRPG searches.

**Subtasks:**
- [ ] Create TTRPG synonym dictionary
- [ ] Implement query expansion
- [ ] Add spell correction
- [ ] Create query completion suggestions
- [ ] Add clarification prompt system
- [ ] Implement search hints UI

**Files to Create/Modify:**
- `src-tauri/src/core/search/synonyms.rs`
- `src-tauri/src/core/search/query.rs`
- `src-tauri/src/core/search/spelling.rs`
- `frontend/src/components/library/search_panel.rs`

**Acceptance Criteria:**
- HP expands to "hit points"
- Typos corrected
- Suggestions shown
- Better search results

---

### TASK-013: Create Library Browser UI
**Requirement:** REQ-UI-004

**Status:** `[ ]`

**Description:**
Build document library browser with search.

**Subtasks:**
- [ ] Create library route and layout
- [ ] Add document list view
- [ ] Implement source type filtering
- [ ] Add search panel with filters
- [ ] Create document detail view
- [ ] Add ingestion from browser
- [ ] Implement source management

**Files to Create/Modify:**
- `frontend/src/components/library/mod.rs`
- `frontend/src/components/library/document_browser.rs`
- `frontend/src/components/library/search_panel.rs`
- `frontend/src/components/library/source_manager.rs`
- `frontend/src/routes.rs`

**Acceptance Criteria:**
- Browse all documents
- Filter by source type
- Search within library
- Ingest new documents

---

## Phase 4: Session Features (P2)

### TASK-014: Implement Session Timeline
**Requirement:** REQ-SESS-005

**Status:** `[ ]`

**Description:**
Add chronological event tracking within sessions.

**Subtasks:**
- [ ] Create session_events table
- [ ] Define event types enum
- [ ] Implement event recording
- [ ] Add entity involvement tracking
- [ ] Create timeline query functions
- [ ] Add session summary generation
- [ ] Build timeline view component

**Files to Create/Modify:**
- `src-tauri/src/core/session/timeline.rs`
- `src-tauri/src/core/session_manager.rs`
- `src-tauri/src/commands.rs`
- `frontend/src/components/session/timeline_view.rs`

**Acceptance Criteria:**
- Events recorded with timestamps
- Entity links preserved
- Timeline queryable
- Visual timeline display

---

### TASK-015: Advanced Condition System
**Requirement:** REQ-SESS-003

**Status:** `[ ]`

**Description:**
Enhance condition system with duration and custom conditions.

**Subtasks:**
- [ ] Add duration field to conditions
- [ ] Implement turn-based duration tracking
- [ ] Create custom condition builder
- [ ] Add condition effect descriptions
- [ ] Implement auto-removal on expiry
- [ ] Add condition stacking rules
- [ ] Create condition manager UI

**Files to Create/Modify:**
- `src-tauri/src/core/session_manager.rs`
- `src-tauri/src/core/session/conditions.rs`
- `frontend/src/components/session/condition_manager.rs`

**Acceptance Criteria:**
- Conditions have durations
- Auto-expire after duration
- Custom conditions supported
- Stack rules enforced

---

### TASK-016: Build Combat Tracker UI
**Requirement:** REQ-UI-005

**Status:** `[ ]`

**Description:**
Create visual combat tracking interface.

**Subtasks:**
- [ ] Create combat tracker layout
- [ ] Add initiative order display
- [ ] Implement current combatant highlight
- [ ] Add HP bars with damage/heal
- [ ] Create condition icons and tooltips
- [ ] Add quick action buttons
- [ ] Display round counter
- [ ] Add combatant management

**Files to Create/Modify:**
- `frontend/src/components/session/combat_tracker.rs`
- `frontend/src/components/session/combatant_card.rs`
- `frontend/src/components/session/initiative_list.rs`
- `frontend/src/styles/combat.css`

**Acceptance Criteria:**
- Clear initiative order
- Current turn obvious
- HP visible at glance
- Conditions displayed
- Quick damage/heal

---

### TASK-017: Implement Session Notes with AI
**Requirement:** REQ-SESS-004

**Status:** `[ ]`

**Description:**
Add AI-assisted session note organization.

**Subtasks:**
- [ ] Create session_notes table
- [ ] Implement note CRUD operations
- [ ] Add tag-based organization
- [ ] Create entity linking (NPC, location)
- [ ] Implement search within notes
- [ ] Add AI categorization of notes
- [ ] Create notes panel UI

**Files to Create/Modify:**
- `src-tauri/src/core/session/notes.rs`
- `src-tauri/src/commands.rs`
- `frontend/src/components/session/notes_panel.rs`

**Acceptance Criteria:**
- Create and edit notes
- Tag notes for organization
- Link entities in notes
- AI suggests categories

---

## Phase 5: Generation & Personality (P2)

### TASK-018: Multi-System Character Generation
**Requirement:** REQ-CHAR-001

**Status:** `[ ]`

**Description:**
Implement character generation for multiple TTRPG systems.

**Subtasks:**
- [ ] Create SystemGenerator trait
- [ ] Implement D&D 5e generator
- [ ] Add Pathfinder 2e generator
- [ ] Create generators for 6+ more systems
- [ ] Define stat templates per system
- [ ] Add class/race selection
- [ ] Implement equipment generation
- [ ] Create character generation UI

**Files to Create/Modify:**
- `src-tauri/src/core/character_gen/mod.rs`
- `src-tauri/src/core/character_gen/systems/dnd5e.rs`
- `src-tauri/src/core/character_gen/systems/pf2e.rs`
- `src-tauri/src/core/character_gen/systems/*.rs`
- `frontend/src/components/generation/character_generator.rs`

**Acceptance Criteria:**
- 8+ systems supported
- Valid stat arrays per system
- Class/race combinations
- Equipment appropriate

---

### TASK-019: AI-Powered Backstory Generation
**Requirement:** REQ-CHAR-003

**Status:** `[ ]`

**Description:**
Add LLM-based character backstory generation.

**Subtasks:**
- [ ] Create backstory generation prompt template
- [ ] Implement backstory request to LLM
- [ ] Add style matching to campaign setting
- [ ] Integrate with character traits
- [ ] Create regenerate/edit functionality
- [ ] Add backstory length options
- [ ] Create backstory preview UI

**Files to Create/Modify:**
- `src-tauri/src/core/character_gen/backstory.rs`
- `src-tauri/src/core/character_gen/prompts.rs`
- `frontend/src/components/generation/backstory_editor.rs`

**Acceptance Criteria:**
- Generate coherent backstories
- Match campaign setting tone
- Editable after generation
- Multiple length options

---

### TASK-020: Location Generation
**Requirement:** REQ-CHAR-005

**Status:** `[ ]`

**Description:**
Implement location generation for campaigns.

**Subtasks:**
- [ ] Create location type definitions
- [ ] Implement feature generation
- [ ] Add inhabitant generation
- [ ] Create connected locations logic
- [ ] Generate secrets and encounters
- [ ] Add map reference support
- [ ] Build location generator UI

**Files to Create/Modify:**
- `src-tauri/src/core/character_gen/location.rs`
- `src-tauri/src/commands.rs`
- `frontend/src/components/generation/location_generator.rs`

**Acceptance Criteria:**
- Generate various location types
- Notable features included
- NPCs and encounters
- Connection suggestions

---

### TASK-021: Personality Application Layer
**Requirement:** REQ-PERS-002, REQ-PERS-003

**Status:** `[ ]`

**Description:**
Implement personality application to generated content.

**Subtasks:**
- [ ] Create personality injection for chat
- [ ] Add NPC dialogue styling
- [ ] Implement narration tone matching
- [ ] Create active personality management
- [ ] Add personality per campaign/session
- [ ] Build personality selector in chat
- [ ] Add personality preview

**Files to Create/Modify:**
- `src-tauri/src/core/personality/application.rs`
- `src-tauri/src/core/personality/mod.rs`
- `src-tauri/src/commands.rs`
- `frontend/src/components/chat.rs`

**Acceptance Criteria:**
- Personality affects chat responses
- NPC dialogue styled
- Easy personality switching
- Preview before selection

---

## Phase 6: Polish & Analytics (P3)

### TASK-022: Implement Usage Tracking
**Requirement:** REQ-LLM-005

**Status:** `[ ]`

**Description:**
Add comprehensive usage analytics and cost tracking.

**Subtasks:**
- [ ] Create usage tracking module
- [ ] Track tokens per request
- [ ] Calculate costs per provider
- [ ] Store historical usage data
- [ ] Create usage statistics commands
- [ ] Build usage dashboard UI
- [ ] Add budget warning system

**Files to Create/Modify:**
- `src-tauri/src/core/usage/tracking.rs`
- `src-tauri/src/core/usage/costs.rs`
- `src-tauri/src/commands.rs`
- `frontend/src/components/analytics/usage_dashboard.rs`

**Acceptance Criteria:**
- Track all token usage
- Accurate cost calculation
- Historical data available
- Budget warnings work

---

### TASK-023: Add Search Analytics
**Requirement:** REQ-SEARCH-005

**Status:** `[ ]`

**Description:**
Implement search usage tracking and reporting.

**Subtasks:**
- [ ] Track query frequency
- [ ] Record result selections
- [ ] Calculate cache statistics
- [ ] Identify popular searches
- [ ] Create analytics commands
- [ ] Add analytics UI section

**Files to Create/Modify:**
- `src-tauri/src/core/search/analytics.rs`
- `src-tauri/src/commands.rs`
- `frontend/src/components/analytics/search_analytics.rs`

**Acceptance Criteria:**
- Query tracking works
- Popular terms identified
- Cache stats available
- UI shows analytics

---

### TASK-024: Security Audit Logging
**Requirement:** REQ-SEC-003

**Status:** `[ ]`

**Description:**
Implement security event logging and audit trail.

**Subtasks:**
- [ ] Create audit log module
- [ ] Log API key usage
- [ ] Track file operations
- [ ] Record configuration changes
- [ ] Add log rotation
- [ ] Create log viewer UI
- [ ] Add log export functionality

**Files to Create/Modify:**
- `src-tauri/src/core/security/audit.rs`
- `src-tauri/src/core/security/mod.rs`
- `src-tauri/src/commands.rs`
- `frontend/src/components/settings.rs`

**Acceptance Criteria:**
- Security events logged
- Logs rotated by size/age
- Viewer in settings
- Export capability

---

### TASK-025: Voice Pre-Generation Queue
**Requirement:** REQ-VOICE-004

**Status:** `[ ]`

**Description:**
Add background voice synthesis for upcoming content.

**Subtasks:**
- [ ] Create synthesis job queue
- [ ] Implement priority handling
- [ ] Add progress tracking
- [ ] Create session batch pre-gen
- [ ] Implement queue management commands
- [ ] Add queue status UI
- [ ] Handle cancellation

**Files to Create/Modify:**
- `src-tauri/src/core/voice/queue.rs`
- `src-tauri/src/core/voice/manager.rs`
- `src-tauri/src/commands.rs`
- `frontend/src/components/voice/synthesis_queue.rs`

**Acceptance Criteria:**
- Queue processes in background
- Priority respected
- Progress visible
- Session pre-gen works

---

## Task Dependencies

```
TASK-001 (Database)
    ├─► TASK-006 (Versioning)
    ├─► TASK-007 (World State)
    ├─► TASK-009 (Relationships)
    └─► TASK-014 (Timeline)

TASK-002 (Router)
    ├─► TASK-003 (Streaming)
    ├─► TASK-019 (Backstory Gen)
    └─► TASK-022 (Usage Tracking)

TASK-004 (Voice Profiles)
    └─► TASK-005 (Audio Cache)
        └─► TASK-025 (Pre-Gen Queue)

TASK-010 (Embeddings)
    └─► TASK-011 (Hybrid Search)
        └─► TASK-012 (Query Enhancement)

TASK-018 (Multi-System Gen)
    └─► TASK-019 (Backstory Gen)
```

---

## Effort Estimates

| Task | Complexity | Estimated Story Points |
|------|------------|------------------------|
| TASK-001 | High | 8 |
| TASK-002 | High | 8 |
| TASK-003 | Medium | 5 |
| TASK-004 | Medium | 5 |
| TASK-005 | Medium | 5 |
| TASK-006 | High | 8 |
| TASK-007 | Medium | 5 |
| TASK-008 | High | 8 |
| TASK-009 | Medium | 5 |
| TASK-010 | High | 8 |
| TASK-011 | High | 8 |
| TASK-012 | Medium | 5 |
| TASK-013 | Medium | 5 |
| TASK-014 | Medium | 5 |
| TASK-015 | Low | 3 |
| TASK-016 | High | 8 |
| TASK-017 | Medium | 5 |
| TASK-018 | High | 13 |
| TASK-019 | Medium | 5 |
| TASK-020 | Medium | 5 |
| TASK-021 | Medium | 5 |
| TASK-022 | Medium | 5 |
| TASK-023 | Low | 3 |
| TASK-024 | Low | 3 |
| TASK-025 | Medium | 5 |

**Total Estimated Points:** 144

---

## Sprint Suggestions

### Sprint 1: Foundation
- TASK-001: SQLite Database Layer
- TASK-002: LLM Provider Router
- TASK-004: Voice Profile System

### Sprint 2: Campaign Core
- TASK-005: Audio Cache System
- TASK-006: Campaign Versioning
- TASK-008: Campaign Dashboard UI

### Sprint 3: Search
- TASK-010: Embedding Provider
- TASK-011: Hybrid Search
- TASK-013: Library Browser UI

### Sprint 4: Session
- TASK-014: Session Timeline
- TASK-015: Advanced Conditions
- TASK-016: Combat Tracker UI

### Sprint 5: Generation
- TASK-018: Multi-System Generation
- TASK-019: AI Backstory
- TASK-021: Personality Application

### Sprint 6: Polish
- TASK-003: Streaming Responses
- TASK-022: Usage Tracking
- TASK-023: Search Analytics

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.1.0 | 2025-12-29 | Update TASK-009 to use dedicated entity_relationships table per design.md |
| 1.0.0 | 2025-12-29 | Initial task breakdown |
