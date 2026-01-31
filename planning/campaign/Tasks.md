# Tasks: Campaign Generation & Management Overhaul

## Implementation Overview

This task plan follows a **foundation-first** strategy: database schema → core pipeline → managers → Tauri commands → frontend components. Each major phase produces working, testable code before moving to the next.

### Architecture: Campaign Intelligence Pipeline (CIP)

All generation flows through a single spine:

```
Input → Context Assembly → Generation Engine → Normalization → Acceptance → Artifacts
```

Modules map to pipeline stages:
- **Input**: WizardManager, ConversationManager
- **Context Assembly**: ContextAssembler, RulebookLinker
- **Generation Engine**: GenerationOrchestrator, TemplateRegistry
- **Normalization**: TrustAssigner, CitationBuilder
- **Acceptance**: AcceptanceManager (Draft → Approved → Canonical)
- **Artifacts**: CampaignManager (single source of truth)

### Implementation Phases

The implementation is divided into 9 detailed phases that map to 5 conceptual phases:

| Conceptual | Detailed Phases | Goal |
|------------|-----------------|------|
| **A - Core Loop** | 1, 2, 5, 6 (partial) | Prove the campaign creation loop |
| **B - Core Artifacts** | 3, 4 | Artifact generation pipeline |
| **C - Grounding** | 3, 4 (partial) | Trust and citations |
| **D - GM Tools** | 8, 9 | Advanced productivity |
| **E - Refinement** | 7 | Scale and polish |

**Detailed Phases:**
1. Database foundation and pipeline types (CampaignIntent, TrustLevel, CanonStatus, Patch/Proposal)
2. Wizard state machine and persistence (implements `DraftStore`)
3. Content grounding layer (implements `Grounder`, `KnowledgeIndex`)
4. Generation orchestration + pipeline components (implements `Generator`, `ArtifactGenerator`)
5. Conversation management (implements `ConversationStore`)
6. Frontend wizard components (Creation Workspace: Guidance + Dialogue + Commit surfaces)
7. Integration and polish
8. Random tables and session recaps
9. Quick reference cards and cheat sheets

**Trait Implementation Order:**
```
Phase 1: Domain types (Patch, Proposal, Decision, ArtifactBundle)
Phase 2: DraftStore + DraftValidator
Phase 3: KnowledgeIndex + Grounder + ReferenceResolver
Phase 4: LlmClient + PromptRenderer + Generator + ArtifactGenerator
Phase 5: ConversationStore
Phase 6: CreationFlow (composes all traits)
Phase 7: CampaignWriter + SessionWriter (canonical persistence)
```

**Estimated Scope:** ~80 sub-tasks across 9 phases

---

## Implementation Plan

### Phase 1: Database Foundation

- [ ] **1. Set up database schema**

- [ ] 1.1 Create migration for wizard_states table
  - Add `wizard_states` table with columns: id, current_step, completed_steps (JSON), campaign_draft (JSON), conversation_thread_id, ai_assisted, created_at, updated_at, auto_saved_at
  - Add foreign key constraint to conversation_threads
  - Add index on created_at for listing incomplete wizards
  - Write migration test to verify schema
  - _Requirements: 1.3, 9.1, 9.2_

- [ ] 1.2 Create migration for conversation tables
  - Add `conversation_threads` table with columns: id, campaign_id, wizard_id, purpose, title, active_personality (JSON), message_count, branched_from, created_at, updated_at, archived_at
  - Add `conversation_messages` table with columns: id, thread_id, role, content, suggestions (JSON), citations (JSON), created_at
  - Add foreign key constraints and indexes
  - Write migration tests
  - _Requirements: 2.6, 9.3, 11.1_

- [ ] 1.3 Create migration for citation and generation tracking
  - Add `source_citations` table with columns: id, campaign_id, source_type, source_id, source_name, location (JSON), excerpt, confidence, used_in, created_at
  - Add `party_compositions` table with columns: id, campaign_id, name, composition (JSON), analysis (JSON), created_at
  - Add indexes for common queries
  - Write migration tests
  - _Requirements: 7.3, 9.4, 9.6_

- [ ] 1.4 Create migration for pipeline core tables (NEW)
  - Add `campaign_intents` table: id, campaign_id, fantasy, player_experiences (JSON), constraints (JSON), themes (JSON), tone_keywords (JSON), avoid (JSON), created_at, updated_at, migrated_from
  - Add `generation_drafts` table: id, campaign_id, wizard_id, entity_type, data (JSON), status, trust_level, trust_confidence, citations (JSON), created_at, updated_at, applied_entity_id
  - Add `canon_status_log` table: id, draft_id, previous_status, new_status, reason, triggered_by, timestamp
  - Add `acceptance_events` table: id, draft_id, entity_type, decision, modifications (JSON), reason, timestamp
  - Add indexes for all FK columns and common queries
  - Write migration tests
  - _Requirements: 19.1, 20.1, 21.1_

- [ ] 1.5 Create database model structs in Rust
  - Create `WizardStateRecord` struct with SQLx FromRow derive
  - Create `ConversationThreadRecord` and `ConversationMessageRecord` structs
  - Create `SourceCitationRecord`, `PartyCompositionRecord` structs
  - Create `CampaignIntentRecord`, `GenerationDraftRecord`, `CanonStatusLogRecord`, `AcceptanceEventRecord` structs
  - Add serialization/deserialization helpers for JSON fields
  - Write unit tests for model conversions
  - _Requirements: All persistence requirements_

- [ ] 1.6 Define core pipeline types (NEW)
  - Create `TrustLevel` enum: Canonical, Derived, Creative, Unverified
  - Create `CanonStatus` enum: Draft, Approved, Canonical, Deprecated
  - Create `CampaignIntent` struct with all fields
  - Create `EntityDraft<T>` generic wrapper struct
  - Implement `is_reliable()`, `is_editable()`, `is_locked()` helpers
  - Write unit tests for type behavior
  - _Requirements: 19.1, 20.1, 21.1_

---

### Phase 2: Wizard State Machine

- [ ] **2. Implement wizard manager**

- [ ] 2.1 Define wizard step enum and state types
  - Create `WizardStep` enum: Basics, Intent, Scope, Players, PartyComposition, ArcStructure, InitialContent, Review
  - Create `WizardState` struct with all fields from design
  - Create `PartialCampaign` struct with `Option<CampaignIntent>` field and `EntityDraft<T>` wrappers for entities
  - Create `StepData` enum for step-specific input (including IntentData)
  - Define allowed state transitions
  - _Requirements: 1.1, 1.2, 19.1_

- [ ] 2.2 Implement WizardManager core lifecycle
  - Create `WizardManager` struct with SqlitePool
  - Implement `start_wizard()` - create new wizard state, persist to DB
  - Implement `get_wizard()` - retrieve by ID
  - Implement `list_incomplete_wizards()` - query non-completed wizards
  - Implement `delete_wizard()` - hard delete
  - Write unit tests for each method
  - _Requirements: 1.1, 9.2_

- [ ] 2.3 Implement wizard step management
  - Implement `advance_step()` - validate transition, persist step data, move to next step
  - Implement `go_back()` - navigate to previous step preserving data
  - Implement `skip_step()` - mark step skipped, move forward (if allowed)
  - Implement step validation logic per step type
  - Write unit tests for all step transitions
  - _Requirements: 1.2, 1.4_

- [ ] 2.4 Implement wizard completion and cancellation
  - Implement `complete_wizard()` - validate all required data, create Campaign, clean up wizard state
  - Implement `cancel_wizard()` - optionally save draft, delete wizard state
  - Implement `auto_save()` - debounced partial data persistence
  - Write integration tests for full wizard lifecycle
  - _Requirements: 1.5, 1.6, 9.1_

- [ ] 2.5 Create wizard Tauri commands
  - Create `commands/campaign/wizard.rs` module
  - Implement `start_campaign_wizard` command
  - Implement `get_wizard_state` command
  - Implement `advance_wizard_step` command
  - Implement `wizard_go_back` command
  - Implement `complete_wizard` command
  - Implement `cancel_wizard` command
  - Implement `list_incomplete_wizards` command
  - Register commands in Tauri builder
  - Write integration tests for commands
  - _Requirements: 1.1-1.6_

---

### Phase 3: Content Grounding Layer

- [ ] **3. Implement content grounding**

- [ ] 3.1 Create Citation model and builder
  - Create `Citation` struct with all fields from design
  - Create `SourceType` enum: Rulebook, FlavourSource, Adventure, Homebrew
  - Create `SourceLocation` struct for page/section/chapter
  - Implement `CitationBuilder` with fluent API
  - Write unit tests for citation building
  - _Requirements: 7.3, 8.1_

- [ ] 3.2 Implement RulebookLinker reference detection
  - Create `RulebookLinker` struct with SearchClient dependency
  - Implement regex patterns for common citation formats:
    - "PHB p.123", "DMG Chapter 5", "(Player's Handbook)"
    - Spell/feat/monster name detection
    - Game mechanics notation (DC, AC, CR)
  - Implement `find_references()` - extract potential references from text
  - Write unit tests with sample text containing various reference formats
  - _Requirements: 8.1, 8.4_

- [ ] 3.3 Implement RulebookLinker search and linking
  - Implement `link_to_rulebook()` - search Meilisearch for matching content
  - Implement confidence scoring based on match quality
  - Implement `build_citation()` - create Citation from RulebookReference
  - Implement `validate_references()` - check referenced content still exists
  - Write integration tests with Meilisearch
  - _Requirements: 7.2, 8.1, 8.2_

- [ ] 3.4 Implement content usage tracking
  - Implement `mark_content_used()` - record that citation was used in campaign
  - Implement `get_used_content()` - retrieve used citations for campaign
  - Add deduplication to prevent same content from appearing too often
  - Write unit tests for usage tracking
  - _Requirements: 7.3_

- [ ] 3.5 Implement FlavourSearcher for lore retrieval
  - Create `FlavourSearcher` struct wrapping SearchClient
  - Implement `search_setting_lore()` - query flavour sources for setting info
  - Implement `search_names()` - find setting-appropriate names
  - Implement `search_locations()` - find canonical setting locations
  - Add setting/campaign filter support
  - Write integration tests
  - _Requirements: 7.1, 7.2, 7.6_

---

### Phase 4: Generation Orchestration

- [ ] **4. Implement generation system**

- [ ] 4.1 Create generation template system
  - Define template YAML schema for prompts
  - Create `TemplateRegistry` struct to load and cache templates
  - Create default templates for:
    - Character background generation
    - NPC generation (minor/supporting/major)
    - Session plan generation
    - Party composition suggestions
    - Arc outline generation
  - Store templates in `resources/templates/generation/`
  - Write unit tests for template loading and validation
  - _Requirements: 4.1, 5.1, 6.1_

- [ ] 4.2 Implement GenerationOrchestrator core
  - Create `GenerationOrchestrator` struct with LLMRouter, SearchClient, TemplateRegistry
  - Implement `load_personality_profile()` - load system-specific tone/style
  - Implement `build_context()` - gather campaign context, search relevant sources using **Hybrid Search**
  - Implement `build_prompt()` - render template with context and personality
  - Implement streaming response handling
  - Write unit tests for context and prompt building
  - _Requirements: 2.3, 7.2, 11.1_

- [ ] 4.3 Implement character background generation
  - Create `CharacterBackgroundRequest` struct
  - Create `GeneratedCharacterBackground` struct with all fields from design
  - Implement `generate_character_background()` - build context, generate, parse result
  - Extract NPC connections and offer to create NPCs
  - Extract locations and offer to add to campaign
  - Add citation attachment for lore references
  - Write unit tests for generation and parsing
  - _Requirements: 4.1-4.7_

- [ ] 4.4 Implement NPC generation
  - Create `NpcGenerationRequest` struct with role, importance, associations
  - Create `GeneratedNpc` struct with all fields from design
  - Implement `generate_npc()` with importance-based detail levels
  - Implement **Recursive Stat Block** logic:
    - Detect spells/traits references
    - Search and fetch definitions
    - Inline definitions into stat block text
  - Implement faction alignment logic
  - Add relationship suggestions to existing entities
  - Write unit tests for each importance level
  - _Requirements: 5.1-5.7, 12.2_

- [ ] 4.5 Implement session plan generation
  - Create `SessionPlanRequest` struct with goals, duration, pacing preference
  - Create `GeneratedSessionPlan` struct with scenes, NPCs, encounters, contingencies
  - Implement `generate_session_plan()` - build context from campaign arc state
  - Consult party composition for encounter difficulty
  - Link to relevant plot points and milestones
  - Write unit tests for plan generation
  - _Requirements: 6.1-6.5_

- [ ] 4.6 Implement party composition suggestions
  - Create `PartyRequest` struct with player_count, system, preferences
  - Create `PartySuggestion` struct with roles, strengths, weaknesses
  - Create `PartyBalancer` struct with SearchClient
  - Implement `suggest_compositions()` - generate multiple options
  - Implement small party handling (1-2 players) with sidekick suggestions
  - Implement large party warnings (6+)
  - Implement gap analysis for existing parties
  - Write unit tests for various player counts and systems
  - _Requirements: 3.1-3.7_

- [ ] 4.7 Implement arc outline generation
  - Create `ArcRequest` struct with type, phase count, tone
  - Create `GeneratedArcOutline` struct with phases, milestones, plot suggestions
  - Implement `generate_arc_outline()` - create dramatic structure
  - Support arc types: linear, branching, sandbox, mystery, heist
  - Generate tension curve visualization data
  - Write unit tests for each arc type
  - _Requirements: 10.1-10.4_

- [ ] 4.8 Implement ContextAssembler (Pipeline: Context Assembly stage)
  - Create `ContextAssembler` struct with SearchClient, CampaignManager
  - Implement `assemble_context()` - build full context from:
    - Campaign snapshot (entities, relationships)
    - CampaignIntent (for tone consistency)
    - Grounded rules (hybrid search for mechanics)
    - Grounded lore (hybrid search for setting content)
    - Conversation window (recent messages)
    - Personality profile (system tone)
  - Implement `TokenBudget` management to prevent context overflow
  - Implement relevance scoring for retrieved content
  - Write unit tests for context assembly
  - _Requirements: 7.2, 19.2, 2.6_

- [ ] 4.9 Implement TrustAssigner (Pipeline: Normalization stage)
  - Create `TrustAssigner` struct with SearchClient
  - Create `TrustThresholds` config (canonical ≥95%, derived ≥75%)
  - Implement `assign_trust()` - analyze citations and assign TrustLevel
  - Implement `analyze_claims()` - break content into claims, score each
  - Implement `verify_citation()` - check citation accuracy against index
  - Implement trust score calculation for quality metrics
  - Write unit tests for each trust level assignment
  - _Requirements: 20.1-20.6_

- [ ] 4.10 Implement AcceptanceManager (Pipeline: Acceptance Layer)
  - Create `AcceptanceManager` struct with SqlitePool, CampaignManager
  - Implement `create_draft()` - store EntityDraft with trust and status
  - Implement `approve_draft()` - transition Draft → Approved
  - Implement `reject_draft()` - remove or mark rejected
  - Implement `modify_draft()` - update data while preserving history
  - Implement `apply_to_campaign()` - create canonical entity in CampaignManager
  - Implement `approve_all()` - bulk approval for wizard completion
  - Implement acceptance event logging for audit trail
  - Write unit tests for all status transitions
  - _Requirements: 21.1-21.9_

- [ ] 4.11 Create generation Tauri commands
  - Create `commands/campaign/generation.rs` module
  - Implement streaming generation commands using Tauri events
  - Implement `generate_character_background` command
  - Implement `generate_npc` command
  - Implement `generate_session_plan` command
  - Implement `suggest_party_composition` command
  - Create `commands/campaign/pipeline.rs` module for pipeline commands
  - Implement `approve_draft`, `reject_draft`, `modify_draft` commands
  - Implement `apply_draft_to_campaign` command
  - Implement `get_pending_drafts` command
  - Register all commands
  - Write integration tests
  - _Requirements: All generation requirements, 21.1-21.9_

---

### Phase 5: Conversation Management

- [ ] **5. Implement conversation system**

- [ ] 5.1 Implement ConversationManager core
  - Create `ConversationManager` struct with SqlitePool
  - Create `ConversationThread` struct with all fields from design
  - Create `ConversationMessage` struct with suggestions and citations
  - Implement `create_thread()` - initialize conversation with purpose
  - Implement `get_thread()` and `list_threads()`
  - Implement `archive_thread()`
  - Write unit tests for thread lifecycle
  - _Requirements: 2.1, 2.6, 9.3_

- [ ] 5.2 Implement message management
  - Implement `add_message()` - persist user or assistant message
  - Implement `get_messages()` with pagination (limit, before cursor)
  - Implement message role enum: User, Assistant, System
  - Create `Suggestion` struct with status tracking
  - Write unit tests for message operations
  - _Requirements: 2.3, 2.4, 2.5_

- [ ] 5.3 Implement suggestion tracking
  - Implement `mark_suggestion_accepted()` - update suggestion status, apply to campaign
  - Implement `mark_suggestion_rejected()` - update status, record rejection
  - Track suggestion decisions for future prompt context
  - Write unit tests for suggestion state transitions
  - _Requirements: 2.4, 2.5_

- [ ] 5.4 Implement conversation branching
  - Implement `branch_from()` - create new thread from specific message
  - Copy messages up to branch point
  - Track branch relationship in branched_from field
  - Write unit tests for branching scenarios
  - _Requirements: 2.6 (conversation context)_

- [ ] 5.5 Implement AI conversation integration
  - Create `ConversationAI` struct wrapping LLMRouter
  - Implement `generate_response()` - build context from history, generate streaming response
  - Implement clarifying question logic based on campaign phase
  - Parse AI responses for embedded suggestions
  - Attach citations when source material is referenced
  - Write integration tests with mock LLM
  - _Requirements: 2.1, 2.2, 2.3_

- [ ] 5.6 Create conversation Tauri commands
  - Create `commands/campaign/conversation.rs` module
  - Implement `create_conversation_thread` command
  - Implement `send_conversation_message` command (streaming response)
  - Implement `get_conversation_messages` command
  - Implement `accept_suggestion` command
  - Implement `reject_suggestion` command
  - Register all commands
  - Write integration tests
  - _Requirements: All conversation requirements_

---

### Phase 6: Frontend Components

- [ ] **6. Implement frontend wizard**

- [ ] 6.1 Create wizard state management
  - Create `wizard_state.rs` in frontend services
  - Define Leptos signals for wizard state
  - Implement IPC bindings for wizard commands
  - Create wizard context provider
  - Write tests for state management
  - _Requirements: 1.1, 1.3_

- [ ] 6.2 Create wizard shell component
  - Create `components/campaign_wizard/mod.rs`
  - Create `WizardShell` component with:
    - Progress indicator showing all steps
    - Navigation buttons (back/next/skip)
    - Cancel/save draft button
    - Auto-save indicator
  - Style with existing design system
  - Write component tests
  - _Requirements: 1.1, 1.4, 1.5_

- [ ] 6.3 Create wizard step components - basics
  - Create `BasicsStep` component
    - Campaign name input with validation
    - System selection dropdown (populated from indexed rulebooks)
    - Description textarea
  - Create `ScopeStep` component
    - Session scope radio buttons (one-shot, short arc, full campaign, ongoing)
    - Session count input (conditional on scope)
  - Write component tests
  - _Requirements: 1.2_

- [ ] 6.4 Create wizard step components - players and tone
  - Create `PlayersStep` component
    - Player count selector (1-12)
    - Party archetype preferences (optional)
  - Create `ToneStep` component
    - Theme tag selector (multi-select)
    - Tone slider or selection
    - Starting level input
  - Write component tests
  - _Requirements: 1.2_

- [ ] 6.5 Create wizard step components - party and arc
  - Create `PartyCompositionStep` component
    - Display AI-generated party suggestions
    - Allow custom party definition
    - Show gap analysis
  - Create `ArcStructureStep` component
    - Arc type selection with descriptions
    - Phase/milestone editor (if not ongoing)
  - Write component tests
  - _Requirements: 1.2, 3.1, 10.1_

- [ ] 6.6 Create wizard step components - content and review
  - Create `InitialContentStep` component
    - Optional NPC creation (expandable list)
    - Optional location creation
    - Optional starting plot hooks
  - Create `ReviewStep` component
    - Summary of all entered data
    - Validation warnings
    - Create campaign button
  - Write component tests
  - _Requirements: 1.2, 1.6_

- [ ] 6.7 Create AI conversation panel
  - Create `components/campaign_wizard/conversation_panel.rs`
  - Implement chat message display with markdown rendering
  - Implement suggestion chips (accept/reject/edit)
  - Implement citation links with "show source" expansion
  - Implement streaming message display
  - Implement thread selection for multiple conversations
  - Write component tests
  - _Requirements: 2.1-2.7_

- [ ] 6.8 Create generation preview components
  - Create `components/generation/generation_preview.rs`
  - Create `CharacterBackgroundPreview` component with editable fields
  - Create `NpcPreview` component with importance-based detail display
  - Create `SessionPlanPreview` component with scene timeline
  - Implement accept/reject/modify controls
  - Write component tests
  - _Requirements: 4.2, 5.2, 6.2_

- [ ] 6.9 Implement Session Control Panel
  - Create `components/session/control_panel.rs`
  - Implement **Two-Column Dashboard**:
    - Left: Narrative stream, read-aloud box, story beats
    - Right: Active mechanics, initiative, quick rules
  - Implement pinned tables widget
  - Apply typographic hierarchy (Visual Information Hierarchy)
  - _Requirements: 6.2, 12.1_

- [ ] 6.10 Integrate wizard into campaign section
  - Add "New Campaign" button to campaign sidebar
  - Wire up wizard modal/page navigation
  - Handle incomplete wizard detection on app start
  - Implement wizard recovery prompt
  - Write E2E tests for wizard integration
  - _Requirements: 9.2_

---

### Phase 7: Integration and Polish

- [ ] **7. Integration and refinement**

- [ ] 7.1 Implement draft recovery system
  - Detect incomplete wizards on app start
  - Show recovery prompt with wizard summary
  - Implement "resume" flow to open wizard at last step
  - Implement "discard" flow to clean up old drafts
  - Write E2E tests for crash recovery
  - _Requirements: 9.2, 9.8_

- [ ] 7.2 Implement auto-save system
  - Add debounced auto-save in frontend (30-second interval)
  - Show save indicator in wizard UI
  - Handle save failures gracefully
  - Write integration tests for auto-save
  - _Requirements: 9.1_

- [ ] 7.3 Implement offline fallback
  - Detect LLM provider availability
  - Show "AI unavailable" indicator in conversation panel
  - Allow wizard completion without AI assistance
  - Queue AI requests for later if offline
  - Write tests for offline scenarios
  - _Requirements: Reliability NFR_

- [ ] 7.4 Add tooltips and help text
  - Add tooltip explanations for TTRPG terminology
  - Add help icons with expandable explanations
  - Add contextual hints in wizard steps
  - _Requirements: Usability NFR_

- [ ] 7.5 Implement error handling and user feedback
  - Create consistent error display component
  - Add retry buttons for recoverable errors
  - Add loading states for all async operations
  - Implement graceful degradation messaging
  - _Requirements: Error handling requirements_

- [ ] 7.6 Performance optimization
  - Profile wizard step transitions
  - Optimize conversation history loading (virtual scroll if needed)
  - Cache frequently accessed templates
  - Lazy-load generation preview components
  - _Requirements: Performance NFR_

- [ ] 7.7 Write comprehensive integration tests
  - Test complete wizard flow (manual mode)
  - Test complete wizard flow (AI-assisted mode)
  - Test generation → accept → entity creation flow
  - Test conversation → suggestion → campaign update flow
  - Test crash recovery scenarios
  - Test multiple incomplete wizards handling
  - _Requirements: All_

- [ ] 7.8 Documentation and cleanup
  - Document new Tauri commands in API docs
  - Add JSDoc/rustdoc to public interfaces
  - Remove any debug logging
  - Final code review pass
  - Update CLAUDE.md with new patterns
  - _Requirements: N/A (quality)_

---

### Phase 8: Random Tables & Session Recaps

- [ ] **8. Implement TTRPG-native tools**

- [ ] 8.1 Create random table database migration
  - Add `random_tables`, `random_table_entries`, `roll_history` tables
  - Add indexes and foreign key constraints
  - Write migration tests
  - _Requirements: 13.1, 13.2_

- [ ] 8.2 Implement dice notation parser
  - Create `DiceNotation` parser module
  - Support standard dice: d4, d6, d8, d10, d12, d20, d100
  - Support compound dice: 2d6, 3d6, modifiers (d20+5)
  - Support d66 tables (read as tens/ones)
  - Write comprehensive unit tests for parsing
  - _Requirements: 13.1_

- [ ] 8.3 Implement RandomTableEngine core
  - Create `RandomTableEngine` struct with SqlitePool
  - Implement table CRUD operations
  - Implement `roll_on_table()` with proper probability
  - Implement nested/cascading roll resolution
  - Implement roll history tracking per session
  - Write unit tests
  - _Requirements: 13.2, 13.3, 13.4, 13.6_

- [ ] 8.4 Implement AI random table generation
  - Create prompt template for table generation
  - Generate setting-appropriate content
  - Ensure balanced probability distributions
  - No "nothing happens" entries (validate all results usable)
  - Write integration tests with mock LLM
  - _Requirements: 13.5_

- [ ] 8.5 Create random table Tauri commands
  - Implement `create_random_table`, `get_random_table`, `list_random_tables`
  - Implement `roll_on_table`, `roll_dice`, `get_roll_history`
  - Implement `generate_random_table` (streaming)
  - Register all commands
  - Write integration tests
  - _Requirements: 13.1-13.6_

- [ ] 8.6 Create session recap database migration
  - Add `session_recaps` table
  - Add indexes on session_id
  - Write migration tests
  - _Requirements: 14.1_

- [ ] 8.7 Implement RecapGenerator core
  - Create `RecapGenerator` struct with LLMRouter
  - Implement `generate_session_recap()` from timeline + notes
  - Generate read-aloud prose and bullet summary formats
  - Extract cliffhanger from session end state
  - Implement per-PC knowledge filtering
  - Write unit tests
  - _Requirements: 14.2, 14.3, 14.5_

- [ ] 8.8 Implement arc and campaign recaps
  - Implement `generate_arc_recap()` aggregating sessions
  - Implement `generate_campaign_summary()` for full timeline
  - Add entity linking for NPCs and locations
  - Write unit tests
  - _Requirements: 14.4, 14.6_

- [ ] 8.9 Create recap Tauri commands
  - Implement `generate_session_recap`, `get_session_recap`, `update_session_recap`
  - Implement `generate_arc_recap`, `filter_recap_by_pc`
  - Register all commands
  - Write integration tests
  - _Requirements: 14.1-14.6_

- [ ] 8.10 Create random table frontend component
  - Create `components/campaign/random_table.rs`
  - Display table with probability visualization
  - Implement roll button with animated dice result
  - Show roll history sidebar
  - Support table editing
  - Write component tests
  - _Requirements: 13.2, 13.3, 13.6_

- [ ] 8.11 Create recap viewer frontend component
  - Create `components/session/recap_viewer.rs`
  - Display read-aloud version with copy button
  - Display bullet summary with checkboxes for key events
  - Show cliffhanger prominently
  - Support editing generated recap
  - Write component tests
  - _Requirements: 14.2, 14.3_

---

### Phase 9: Quick Reference Cards & Cheat Sheets

- [ ] **9. Implement GM productivity tools**

- [ ] 9.1 Create pinned cards database migration
  - Add `pinned_cards` table with max 6 constraint
  - Add `cheat_sheet_preferences` table
  - Write migration tests
  - _Requirements: 16.4_

- [ ] 9.2 Implement QuickReferenceCardManager
  - Create card rendering for NPC, Location, Item, PlotPoint types
  - Implement card tray management (pin/unpin/reorder)
  - Enforce 6 card maximum
  - Create hover preview generation
  - Write unit tests
  - _Requirements: 16.1, 16.2, 16.3, 16.4_

- [ ] 9.3 Implement progressive disclosure levels
  - Create `DisclosureLevel` enum (Minimal, Summary, Complete)
  - Add level-aware rendering for NPCs, Locations, Scenes
  - Implement smooth animation between levels
  - Store user preference for default level
  - Write unit tests
  - _Requirements: 17.1, 17.2, 17.3, 17.4, 17.5, 17.6_

- [ ] 9.4 Implement CheatSheetBuilder
  - Create `CheatSheetBuilder` struct
  - Implement content aggregation from session plan
  - Implement priority-based truncation with warnings
  - Implement print-friendly HTML rendering
  - Support user preferences (always/never include)
  - Write unit tests
  - _Requirements: 18.1, 18.2, 18.3, 18.4, 18.5_

- [ ] 9.5 Create card and cheat sheet Tauri commands
  - Implement `get_entity_card`, `pin_card`, `unpin_card`
  - Implement `get_pinned_cards`, `reorder_pinned_cards`
  - Implement `build_cheat_sheet`, `build_custom_cheat_sheet`
  - Implement `export_cheat_sheet_html`
  - Implement `save_cheat_sheet_preferences`, `get_cheat_sheet_preferences`
  - Register all commands
  - Write integration tests
  - _Requirements: 16.5, 16.6, 18.5, 18.6_

- [ ] 9.6 Create quick reference card component
  - Create `components/common/entity_card.rs`
  - Implement NPC, Location, Item, Plot card variants
  - Support click-to-expand to full detail
  - Support hover preview
  - Style cards for compact, scannable display
  - Write component tests
  - _Requirements: 16.1, 16.2, 16.3_

- [ ] 9.7 Create card tray component
  - Create `components/session/card_tray.rs`
  - Display pinned cards in horizontal row/grid
  - Support drag-and-drop reordering
  - Enforce max 6 visual limit
  - Integrate with session control panel
  - Write component tests
  - _Requirements: 16.4, 16.5_

- [ ] 9.8 Create cheat sheet viewer component
  - Create `components/session/cheat_sheet.rs`
  - Display sections with collapsible headers
  - Show truncation warnings prominently
  - Support print-friendly mode toggle
  - Support floating panel mode for during-session use
  - Write component tests
  - _Requirements: 18.2, 18.3, 18.6_

- [ ] 9.9 Implement interview mode for wizard
  - Create conversational interview flow as primary path
  - Implement one-question-at-a-time UI
  - Add suggestion chips for 2-4 answers per question
  - Implement "I'm stuck" helper with random inspiration
  - Add summarize-and-edit flow at end
  - Write E2E tests
  - _Requirements: 15.1, 15.2, 15.3, 15.4, 15.5, 15.6_

---

## Dependency Graph

```
Phase 1 (Database + Pipeline Types)
        │
        ├───────────────────────────────────────┐
        │                                       │
        v                                       v
Phase 2 (Wizard) ────────> Phase 3 (Grounding) ─> Phase 4 (Generation + Pipeline)
        │                         │                        │
        │                         │                        │
        │         ┌───────────────┘                        │
        │         │                                        │
        v         v                                        │
        Phase 5 (Conversation) <───────────────────────────┘
                  │
                  v
        Phase 6 (Frontend: Wizard + Acceptance UI)
                  │
                  v
        Phase 7 (Integration + E2E Tests)
                  │
        ┌─────────┴─────────┐
        v                   v
Phase 8 (Tables/Recaps)   Phase 9 (Cards/Cheat Sheets)
```

**Pipeline Flow (within Phase 4):**
```
ContextAssembler → GenerationOrchestrator → TrustAssigner → AcceptanceManager
```

**Critical Path:** 1 → 2 → 3 → 4 → 6 → 7 → (8 or 9)

**Parallel Work Possible:**
- Phase 3 and Phase 2 can overlap after 1.6 complete (pipeline types defined)
- Phase 5 can start after Phase 3 complete
- Frontend work (Phase 6) can begin UI scaffolding during Phase 4
- **Phase 8 and Phase 9 can run in parallel** after Phase 7 complete
- Phase 8 and 9 are independent of each other

**Architectural Invariants (must be enforced throughout):**
- All campaign truth lives in CampaignManager (single source of truth)
- Conversation suggestions produce drafts, never mutate state directly
- Generation results must pass through AcceptanceManager
- CampaignIntent is immutable after campaign creation

---

## Task Sizing Guide

| Size | Hours | Example |
|------|-------|---------|
| S | 1-2 | Add single migration, create model struct |
| M | 2-4 | Implement manager method with tests |
| L | 4-8 | Complete wizard step with validation |
| XL | 8+ | Full generation type (character background) |

Most sub-tasks in this plan are M or L sized.

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| LLM response parsing failures | Robust JSON extraction with fallback to raw text |
| Meilisearch downtime | Cache recent search results, allow skip of citation step |
| Context window exceeded | Implement conversation summarization at 75% capacity |
| Wizard state corruption | Validate state on load, offer reset to last valid step |
| Template versioning | Version templates, migrate on app update |

---

## Success Criteria

Phase complete when:
1. All sub-tasks in phase marked complete
2. Unit tests pass (>80% coverage for new code)
3. Integration tests pass
4. No critical bugs in manual testing
5. Performance targets met (per NFR)

Feature complete when:
1. All phases complete
2. E2E tests cover all user journeys
3. Documentation updated
4. Code review approved
