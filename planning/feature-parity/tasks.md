# Feature Parity: Remaining Tasks

**Status:** Active development
**Updated:** 2026-01-03

Tasks remaining to achieve full feature parity between the original Python/MCP implementation and the Rust/Tauri application.

**Priority Legend:**
- 🔴 **Critical** - Core functionality
- 🟠 **High** - Important for production
- 🟡 **Medium** - Enhanced UX
- 🟢 **Low** - Nice to have

---

## Phase 1: Core Parity (Critical) 🔴

### 1.1 OpenAI Provider
**File:** `src-tauri/src/core/llm/openai.rs`

- [ ] Create `OpenAIClient` struct with configuration
- [ ] Implement `chat_completion()` for GPT-4o, GPT-4o-mini, GPT-3.5-turbo
- [ ] Implement `stream_completion()` with SSE parsing
- [ ] Add `generate_embeddings()` using text-embedding-3-small/large
- [ ] Implement rate limit header extraction
- [ ] Add vision support for GPT-4o
- [ ] Add tool/function calling support
- [ ] Handle API errors: 429, 500, 503, context length exceeded

### 1.2 Streaming Response Handler
**File:** `src-tauri/src/core/llm/streaming.rs` (new)

- [ ] Create `StreamingManager` struct
- [ ] Define `StreamingChunk` struct (content, finish_reason, usage)
- [ ] Implement provider-specific stream parsing (Claude, OpenAI, Gemini, Ollama)
- [ ] Add chunk aggregation for final response
- [ ] Implement stream cancellation
- [ ] Add Tauri event emission for frontend streaming

### 1.3 Streaming Response Display (Frontend)
**File:** `frontend/src/components/chat.rs`

- [ ] Implement real-time token streaming display
- [ ] Add typing indicator during generation
- [ ] Show partial response as it streams
- [ ] Add stop generation button

---

## Phase 2: Enhanced Experience (High) 🟠

### 2.1 Budget Enforcer
**File:** `src-tauri/src/core/budget.rs` (new)

- [ ] Create `BudgetEnforcer` struct
- [ ] Define `BudgetLimit` (amount, period: hourly/daily/monthly/total)
- [ ] Define `BudgetAction` enum (warn, throttle, degrade, reject)
- [ ] Implement spending velocity monitoring
- [ ] Add soft/hard limit tiers
- [ ] Implement automatic model downgrade when approaching limits
- [ ] Create budget status API for frontend

### 2.2 Alert System
**File:** `src-tauri/src/core/alerts.rs` (new)

- [ ] Create `AlertSystem` struct
- [ ] Define alert types (BudgetApproaching, BudgetExceeded, ProviderDown)
- [ ] Implement threshold-based triggering
- [ ] Add system notification delivery
- [ ] Implement alert deduplication

### 2.3 Query Expansion
**File:** `src-tauri/src/core/query_expansion.rs` (new)

- [ ] Create `QueryExpander` struct
- [ ] Add TTRPG-specific synonym map (HP→hit points, AC→armor class, DC→difficulty class)
- [ ] Implement related term suggestion
- [ ] Add stemming/lemmatization

### 2.4 Input Validator
**File:** `src-tauri/src/core/input_validator.rs` (new)

- [ ] Create `InputValidator` struct
- [ ] Implement XSS prevention
- [ ] Implement path traversal prevention
- [ ] Add input length limits
- [ ] Create validation for Tauri commands

### 2.5 Audit Logger
**File:** `src-tauri/src/core/audit.rs` (new)

- [ ] Create `AuditLogger` struct
- [ ] Define audit event types
- [ ] Implement structured audit logging
- [ ] Persist to SQLite
- [ ] Add audit log rotation

### 2.6 Message Formatting (Frontend)
**File:** `frontend/src/components/chat.rs`

- [ ] Add Markdown rendering for messages
- [ ] Implement code block syntax highlighting
- [ ] Add message copy button

### 2.7 Chat History (Frontend)
**File:** `frontend/src/components/chat_history.rs` (new)

- [ ] Implement conversation persistence
- [ ] Add conversation list sidebar
- [ ] Implement conversation search
- [ ] Add conversation export

### 2.8 OpenAI Configuration (Frontend)
**File:** `frontend/src/components/settings.rs`

- [ ] Add OpenAI API key input
- [ ] Add OpenAI model selection
- [ ] Add API key test button

### 2.9 Budget Settings (Frontend)
**File:** `frontend/src/components/budget_settings.rs` (new)

- [ ] Create budget configuration panel
- [ ] Add daily/weekly/monthly limit inputs
- [ ] Show current spending display
- [ ] Show spending projection

### 2.10 Document Management (Frontend)
**File:** `frontend/src/components/document_list.rs` (new)

- [ ] Create document list with sorting
- [ ] Add document search/filter
- [ ] Implement document deletion with confirmation
- [ ] Show indexing status

### 2.11 Search Results Enhancement (Frontend)
**File:** `frontend/src/components/search_results.rs`

- [ ] Add relevance score display
- [ ] Add highlighted match snippets
- [ ] Implement pagination

### 2.12 Combat Tracker Enhancement (Frontend)
**File:** `frontend/src/components/session.rs`

- [ ] Add HP bars with visual health status
- [ ] Add condition badges on combatant cards
- [ ] Show current turn highlight
- [ ] Add combat log sidebar

### 2.13 Session Notes UI (Frontend)
**File:** `frontend/src/components/session_notes.rs` (new)

- [ ] Create note-taking panel during session
- [ ] Add quick note buttons (plot, combat, NPC)
- [ ] Show note history for session

### 2.14 Character Sheet View (Frontend)
**File:** `frontend/src/components/character_sheet.rs` (new)

- [ ] Create full character sheet layout
- [ ] Implement editable fields
- [ ] Add character export (JSON)

---

## Phase 3: Advanced Features (Medium) 🟡

### 3.1 Cost Predictor
**File:** `src-tauri/src/core/cost_predictor.rs` (new)

- [ ] Create `CostPredictor` struct
- [ ] Implement usage pattern detection
- [ ] Add simple moving average forecasting
- [ ] Generate cost projections
- [ ] Persist historical data

### 3.2 Spell Correction
**File:** `src-tauri/src/core/spell_correction.rs` (new)

- [ ] Create `SpellCorrector` struct
- [ ] Implement Levenshtein distance calculation
- [ ] Build TTRPG vocabulary dictionary
- [ ] Implement "did you mean?" suggestions

### 3.3 MOBI/AZW Parser
**File:** `src-tauri/src/ingestion/mobi_parser.rs` (new)

- [ ] Add MOBI crate dependency
- [ ] Create `MobiParser` struct
- [ ] Extract text content from MOBI/AZW/AZW3
- [ ] Extract metadata (title, author)
- [ ] Handle DRM-free files only

### 3.4 Location Management
**File:** `src-tauri/src/core/location_manager.rs` (new)

- [ ] Create `LocationManager` struct
- [ ] Define `Location` struct (id, name, description, type, parent_id, connections, npcs_present)
- [ ] Implement CRUD operations
- [ ] Add location hierarchy traversal
- [ ] Add database table and migration

### 3.5 Plot Point Tracking
**File:** `src-tauri/src/core/plot_manager.rs` (new)

- [ ] Create `PlotManager` struct
- [ ] Define `PlotPoint` struct (id, title, description, status, priority, involved NPCs/locations)
- [ ] Implement CRUD operations
- [ ] Add status transitions
- [ ] Add database table and migration

### 3.6 Session Summary Generation
**File:** `src-tauri/src/core/session_summary.rs` (new)

- [ ] Create `SessionSummarizer` struct
- [ ] Implement LLM-based summary generation
- [ ] Extract key events, combat outcomes
- [ ] Generate "previously on..." recap
- [ ] Add Tauri command

### 3.7 Extended Genre Support
**File:** `src-tauri/src/core/character_gen.rs` (enhance)

Original Python supports 21+ genres. Add:
- [ ] Cyberpunk-specific data (cyberware, neural interface, street cred)
- [ ] Cosmic Horror data (sanity, forbidden knowledge, trauma)
- [ ] Post-Apocalyptic data (mutations, radiation resistance)
- [ ] Superhero data (powers, weaknesses, origin)
- [ ] Add 13+ additional genre support

### 3.8 Additional Database Tables
**File:** `src-tauri/src/database/migrations.rs`

- [ ] Add `locations` table
- [ ] Add `plot_points` table
- [ ] Add `audit_log` table

### 3.9 Design System Polish (Frontend)
**File:** `frontend/src/components/design_system.rs` (new)

- [ ] Create `Button` component (variants: primary, secondary, danger, ghost)
- [ ] Create `Input` and `Select` components
- [ ] Create `Modal` component (portal-based dialog)
- [ ] Create `Card` component (header, body, footer)
- [ ] Create `Badge` component (for conditions, tags)
- [ ] Create `LoadingSpinner` component
- [ ] Refactor existing views to use these primitives

### 3.10 Campaign Dashboard (Frontend)
**File:** `frontend/src/components/campaign_dashboard.rs` (new)

- [ ] Create campaign overview page
- [ ] Show campaign stats (sessions, NPCs)
- [ ] Add quick action buttons

### 3.11 Location Manager UI (Frontend)
**File:** `frontend/src/components/location_manager.rs` (new)

- [ ] Create location hierarchy view
- [ ] Add location creation form
- [ ] Show NPCs at location

### 3.12 Plot Tracker UI (Frontend)
**File:** `frontend/src/components/plot_tracker.rs` (new)

- [ ] Create plot point kanban board
- [ ] Add plot point creation form
- [ ] Implement drag-drop status changes

### 3.13 Session Summary View (Frontend)
**File:** `frontend/src/components/session_summary.rs` (new)

- [ ] Create post-session summary page
- [ ] Display AI-generated recap
- [ ] Show combat statistics

### 3.14 Analytics Dashboard (Frontend)
**File:** `frontend/src/components/analytics.rs` (new)

- [ ] Create analytics dashboard page
- [ ] Add token usage chart
- [ ] Add cost breakdown chart
- [ ] Implement date range selector

---

## Phase 4: Polish (Low) 🟢

### 4.1 Search Analytics
**File:** `src-tauri/src/core/search_analytics.rs` (new)

- [ ] Track query frequency
- [ ] Track zero-result queries
- [ ] Persist analytics to database

### 4.2 Name Generator
**File:** `src-tauri/src/core/name_gen.rs` (new)

- [ ] Create `NameGenerator` struct
- [ ] Add race-specific name tables
- [ ] Support gender variants
- [ ] Add surname/clan generation

### 4.3 Pre-generation Queue
**File:** `src-tauri/src/core/voice_queue.rs` (new)

- [ ] Create `PreGenerationQueue` struct
- [ ] Implement background job processing
- [ ] Add batch session preparation
- [ ] Add job status tracking

---

## Testing Requirements

Each feature implementation should include appropriate testing:

### Backend Testing
- [ ] Unit tests for all new structs/managers (BudgetEnforcer, AlertSystem, etc.)
- [ ] Integration tests for Tauri commands
- [ ] Integration tests for database migrations (locations, plot_points, audit_log)
- [ ] E2E tests for streaming response flow with cancellation
- [ ] Performance benchmarks for StreamingManager throughput

### Frontend Testing
- [ ] Component tests for new UI components (chat, settings, combat tracker)
- [ ] Integration tests for Tauri invoke calls
- [ ] Visual regression tests for design system components

### QA Validation
- [ ] Cross-platform testing (Linux, macOS, Windows)
- [ ] Provider integration testing (OpenAI, Claude, Gemini, Ollama)
- [ ] Budget limit behavior validation
- [ ] Streaming UX testing (latency, cancellation, error states)

---

## Effort Estimates

| Phase | Tasks | Est. Hours |
|-------|-------|------------|
| Phase 1 (Critical) | 3 | 40-55 |
| Phase 2 (High) | 14 | 85-115 |
| Phase 3 (Medium) | 14 | 90-120 |
| Phase 4 (Low) | 3 | 20-30 |
| Testing | - | 30-50 |
| **Total** | **34** | **265-370** |
