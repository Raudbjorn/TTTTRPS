# Feature Parity - Remaining Tasks

## Status: IN PROGRESS

**Last Updated:** 2026-01-03

This document consolidates all remaining tasks to bring the Rust/Tauri implementation into feature parity with the original Python/MCP project.

---

## Legend

| Priority | Description |
|----------|-------------|
| P0 | Critical - Core functionality |
| P1 | High - Important for production |
| P2 | Medium - Enhances user experience |
| P3 | Low - Nice to have |

---

## Backend Tasks

### 1. AI Providers Module

#### OpenAI Provider (P0)
**File:** `src-tauri/src/core/llm/openai.rs`

- [ ] Create `OpenAIClient` struct with configuration
- [ ] Implement `chat_completion()` for GPT-4o, GPT-4o-mini, GPT-3.5-turbo
- [ ] Implement `stream_completion()` with SSE parsing
- [ ] Add `generate_embeddings()` using text-embedding-3-small/large
- [ ] Implement rate limit header extraction
- [ ] Add vision support for GPT-4o (image input)
- [ ] Add tool/function calling support
- [ ] Handle API errors: 429, 500, 503, context length exceeded

#### Streaming Response Handler (P0)
**File:** `src-tauri/src/core/llm/streaming.rs`

- [ ] Create `StreamingManager` struct
- [ ] Define `StreamingChunk` struct (content, finish_reason, usage)
- [ ] Implement provider-specific stream parsing (Claude, OpenAI, Gemini, Ollama)
- [ ] Add chunk aggregation for final response
- [ ] Implement stream cancellation
- [ ] Add Tauri event emission for frontend streaming

---

### 2. Cost Optimization Module

#### Budget Enforcer (P1)
**File:** `src-tauri/src/core/budget.rs`

- [ ] Create `BudgetEnforcer` struct
- [ ] Define `BudgetLimit` struct (amount, period: hourly/daily/monthly/total)
- [ ] Define `BudgetAction` enum (warn, throttle, degrade, reject)
- [ ] Implement spending velocity monitoring
- [ ] Add soft/hard limit tiers
- [ ] Implement automatic model downgrade when approaching limits
- [ ] Create budget status API for frontend

#### Cost Predictor (P2)
**File:** `src-tauri/src/core/cost_predictor.rs`

- [ ] Create `CostPredictor` struct
- [ ] Implement usage pattern detection
- [ ] Add simple moving average forecasting
- [ ] Generate cost projections
- [ ] Persist historical data

#### Alert System (P1)
**File:** `src-tauri/src/core/alerts.rs`

- [ ] Create `AlertSystem` struct
- [ ] Define alert types (BudgetApproaching, BudgetExceeded, ProviderDown)
- [ ] Implement threshold-based triggering
- [ ] Add system notification delivery
- [ ] Implement alert deduplication

---

### 3. Search Module

#### Query Expansion (P1)
**File:** `src-tauri/src/core/query_expansion.rs`

- [ ] Create `QueryExpander` struct
- [ ] Add TTRPG-specific synonym map (HP, AC, DC, etc.)
- [ ] Implement related term suggestion
- [ ] Add stemming/lemmatization

#### Spell Correction (P2)
**File:** `src-tauri/src/core/spell_correction.rs`

- [ ] Create `SpellCorrector` struct
- [ ] Implement Levenshtein distance calculation
- [ ] Build TTRPG vocabulary dictionary
- [ ] Implement "did you mean?" suggestions

#### Search Analytics (P3)
**File:** `src-tauri/src/core/search_analytics.rs`

- [ ] Track query frequency
- [ ] Track zero-result queries
- [ ] Persist analytics to database

---

### 4. Document Processing Module

#### MOBI/AZW Parser (P2)
**File:** `src-tauri/src/ingestion/mobi_parser.rs`

- [ ] Add MOBI crate dependency
- [ ] Create `MobiParser` struct
- [ ] Extract text content from MOBI/AZW/AZW3
- [ ] Extract metadata (title, author)
- [ ] Handle DRM-free files only

---

### 5. Campaign Module

#### Location Management (P2)
**File:** `src-tauri/src/core/location_manager.rs`

- [ ] Create `LocationManager` struct
- [ ] Define `Location` struct (id, name, description, type, parent, connections, npcs)
- [ ] Implement CRUD operations
- [ ] Add location hierarchy traversal
- [ ] Add database table and migration

#### Plot Point Tracking (P2)
**File:** `src-tauri/src/core/plot_manager.rs`

- [ ] Create `PlotManager` struct
- [ ] Define `PlotPoint` struct (id, title, description, status, priority, involved entities)
- [ ] Implement CRUD operations
- [ ] Add status transitions
- [ ] Add database table and migration

---

### 6. Session Module

#### Session Summary Generation (P2)
**File:** `src-tauri/src/core/session_summary.rs`

- [ ] Create `SessionSummarizer` struct
- [ ] Implement LLM-based summary generation
- [ ] Extract key events, combat outcomes
- [ ] Generate "previously on..." recap
- [ ] Add Tauri command

---

### 7. Character Generation Module

#### Extended Genre Support (P2)
**File:** `src-tauri/src/core/character_gen.rs`

Add support for 13+ additional genres:
- [ ] Cyberpunk-specific data (cyberware, neural interface, street cred)
- [ ] Cosmic Horror data (sanity, forbidden knowledge, trauma)
- [ ] Post-Apocalyptic data (mutations, radiation resistance)
- [ ] Superhero data (powers, weaknesses, origin)
- [ ] Additional genre templates

#### Name Generator (P3)
**File:** `src-tauri/src/core/name_gen.rs`

- [ ] Create `NameGenerator` struct
- [ ] Add race-specific name tables
- [ ] Support gender variants
- [ ] Add surname/clan generation

---

### 8. Voice Synthesis Module

#### Pre-generation Queue (P3)
**File:** `src-tauri/src/core/voice_queue.rs`

- [ ] Create `PreGenerationQueue` struct
- [ ] Implement background job processing
- [ ] Add batch session preparation
- [ ] Add job status tracking

---

### 9. Security Module

#### Input Validator (P1)
**File:** `src-tauri/src/core/input_validator.rs`

- [ ] Create `InputValidator` struct
- [ ] Implement XSS prevention
- [ ] Implement path traversal prevention
- [ ] Add input length limits
- [ ] Create validation for Tauri commands

#### Audit Logger (P1)
**File:** `src-tauri/src/core/audit.rs`

- [ ] Create `AuditLogger` struct
- [ ] Define audit event types
- [ ] Implement structured audit logging
- [ ] Persist to SQLite
- [ ] Add audit log rotation

---

### 10. Database Module

#### Additional Tables (P2)
**File:** `src-tauri/src/database/migrations.rs`

- [ ] Add `locations` table
- [ ] Add `plot_points` table
- [ ] Add `audit_log` table

---

## Frontend Tasks

### Chat Interface

#### Streaming Response Display (P0)
**File:** `frontend/src/components/chat.rs`

- [ ] Implement real-time token streaming display
- [ ] Add typing indicator during generation
- [ ] Show partial response as it streams
- [ ] Add stop generation button

#### Message Formatting (P1)
**File:** `frontend/src/components/chat.rs`

- [ ] Add Markdown rendering for messages
- [ ] Implement code block syntax highlighting
- [ ] Add message copy button

#### Chat History (P1)
**File:** `frontend/src/components/chat_history.rs`

- [ ] Implement conversation persistence
- [ ] Add conversation list sidebar
- [ ] Implement conversation search
- [ ] Add conversation export

---

### Settings Interface

#### OpenAI Configuration (P1)
**File:** `frontend/src/components/settings.rs`

- [ ] Add OpenAI API key input
- [ ] Add OpenAI model selection
- [ ] Add API key test button

#### Budget Settings (P1)
**File:** `frontend/src/components/budget_settings.rs`

- [ ] Create budget configuration panel
- [ ] Add daily/weekly/monthly limit inputs
- [ ] Show current spending display
- [ ] Show spending projection

---

### Library Interface

#### Document Management (P1)
**File:** `frontend/src/components/document_list.rs`

- [ ] Create document list with sorting
- [ ] Add document search/filter
- [ ] Implement document deletion with confirmation
- [ ] Show indexing status

#### Search Results Enhancement (P1)
**File:** `frontend/src/components/search_results.rs`

- [ ] Add relevance score display
- [ ] Add highlighted match snippets
- [ ] Implement pagination

---

### Campaign Interface

#### Campaign Dashboard (P2)
**File:** `frontend/src/components/campaign_dashboard.rs`

- [ ] Create campaign overview page
- [ ] Show campaign stats (sessions, NPCs)
- [ ] Add quick action buttons

#### Location Manager UI (P2)
**File:** `frontend/src/components/location_manager.rs`

- [ ] Create location hierarchy view
- [ ] Add location creation form
- [ ] Show NPCs at location

#### Plot Tracker UI (P2)
**File:** `frontend/src/components/plot_tracker.rs`

- [ ] Create plot point kanban board
- [ ] Add plot point creation form
- [ ] Implement drag-drop status changes

---

### Session Interface

#### Combat Tracker Enhancement (P1)
**File:** `frontend/src/components/session.rs`

- [ ] Add HP bars with visual health status
- [ ] Add condition badges on combatant cards
- [ ] Show current turn highlight
- [ ] Add combat log sidebar

#### Session Notes UI (P1)
**File:** `frontend/src/components/session_notes.rs`

- [ ] Create note-taking panel during session
- [ ] Add quick note buttons (plot, combat, NPC)
- [ ] Show note history for session

#### Session Summary View (P2)
**File:** `frontend/src/components/session_summary.rs`

- [ ] Create post-session summary page
- [ ] Display AI-generated recap
- [ ] Show combat statistics

---

### Character Interface

#### Character Sheet View (P1)
**File:** `frontend/src/components/character_sheet.rs`

- [ ] Create full character sheet layout
- [ ] Implement editable fields
- [ ] Add character export (JSON)

---

### Analytics Dashboard

#### Usage Analytics (P2)
**File:** `frontend/src/components/analytics.rs`

- [ ] Create analytics dashboard page
- [ ] Add token usage chart
- [ ] Add cost breakdown chart
- [ ] Implement date range selector

---

### Design System

#### Design System Polish (P3)
**File:** `frontend/src/components/design_system.rs`

- [ ] Create `Button` component (variants: primary, secondary, danger, ghost)
- [ ] Create `Input` and `Select` components with consistent styling
- [ ] Create `Modal` component (reusable portal-based dialog)
- [ ] Create `Card` component (header, body, footer)
- [ ] Create `Badge` component (for conditions, tags)
- [ ] Create `LoadingSpinner` component
- [ ] Refactor existing views to use these primitives

---

## Priority Summary

| Priority | Backend | Frontend | Total |
|----------|---------|----------|-------|
| P0 | 2 | 1 | 3 |
| P1 | 5 | 9 | 14 |
| P2 | 8 | 5 | 13 |
| P3 | 3 | 1 | 4 |
| **Total** | **18** | **16** | **34** |

---

## Reference Documents

- `requirements.md` - Full requirements specification
- `design.md` - Architecture and API design

---

*Consolidated from FEATURE_PARITY_TASKS.md and feature-parity/tasks.md on 2026-01-03*
