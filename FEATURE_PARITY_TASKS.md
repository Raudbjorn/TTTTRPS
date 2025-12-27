# Feature Parity Tasks: Original Python → Rust/Tauri

**Last Updated:** 2025-12-27

This document contains detailed, actionable tasks to bring the Rust/Tauri implementation into feature parity with the original Python/MCP project.

**Legend:**
- 🔴 **Critical** - Core functionality, blocks other features
- 🟠 **High** - Important for production use
- 🟡 **Medium** - Enhances user experience
- 🟢 **Low** - Nice to have, can defer
- ✅ **Complete** - Already implemented

---

# IMPLEMENTATION STATUS SUMMARY

## 🚀 IMMEDIATE FRONTEND ACTION PLAN
To address user-facing parity efficiently:

1.  **Refactor Design System** (`components/design_system.rs`): Extract inline styles/components from `chat.rs`, `session.rs` into reusable UI primitives (Button, Card, Modal, Badge).
2.  **Chat Polish** (`components/chat.rs`): Implement Markdown rendering (pulldown-cmark) and syntax highlighting.
3.  **Visual Feedback**: Add typing indicators and stream simulation (until backend streaming is ready).
4.  **Campaign Dashboard**: Create the missing dashboard view to aggregate campaign stats and quick actions.

---

## What's Already Implemented ✅

The Rust implementation has achieved significant feature parity:

| Category | Status | Details |
|----------|--------|---------|
| **LLM Providers** | 75% | Claude, Gemini, Ollama (missing: OpenAI) |
| **Voice Synthesis** | 100% | ElevenLabs, Fish Audio, Ollama, System TTS |
| **Search** | 100% | Hybrid (LanceDB vector + Tantivy BM25 + RRF) |
| **Document Ingestion** | 85% | PDF, EPUB, chunking (missing: MOBI/AZW) |
| **Campaign Management** | 90% | CRUD, snapshots, notes, export (missing: locations, plots) |
| **Session Management** | 95% | Sessions, combat, initiative, conditions, HP tracking |
| **Character Generation** | 80% | 8 systems, full stats (missing: extended genres) |
| **NPC Generation** | 100% | Roles, appearance, personality, relationships, plot hooks |
| **Voice Features** | 100% | Caching, profiles, multi-provider |
| **Audio** | 100% | Multi-sink playback (voice, music, ambience, SFX) |
| **Database** | 100% | SQLite + migrations + backup/restore |
| **Security** | 70% | Keyring credentials (missing: audit, rate limiting) |
| **UI Theme** | 100% | Dark/light adaptive themes |
| **Tauri Commands** | 100% | 54 commands covering all features |

---

# BACKEND TASKS (Remaining)

## 1. AI Providers Module

### 1.1 OpenAI Provider 🔴
**File:** `src-tauri/src/core/llm/openai.rs`

- [ ] Create `OpenAIClient` struct with configuration
- [ ] Implement `chat_completion()` for GPT-4o, GPT-4o-mini, GPT-3.5-turbo
- [ ] Implement `stream_completion()` with Server-Sent Events (SSE) parsing
- [ ] Add `generate_embeddings()` using text-embedding-3-small/large
- [ ] Implement rate limit header extraction (x-ratelimit-remaining, x-ratelimit-reset)
- [ ] Add vision support for GPT-4o (image input)
- [ ] Add tool/function calling support matching OpenAI format
- [ ] Handle API errors: 429 (rate limit), 500, 503, context length exceeded

### ~~1.2 Enhanced Provider Router~~ ✅ IMPLEMENTED
Located in `src-tauri/src/core/llm_router.rs` with circuit breaker pattern.

### ~~1.3 Load Balancer~~ ✅ PARTIAL
Basic health-aware routing exists. Enhanced load balancing is optional.

### 1.4 Streaming Response Handler 🔴
**File:** `src-tauri/src/core/llm/streaming.rs` (new)

- [ ] Create `StreamingManager` struct
- [ ] Define `StreamingChunk` struct (content, finish_reason, usage)
- [ ] Implement provider-specific stream parsing:
  - Claude: event-stream format
  - OpenAI: SSE format
  - Gemini: JSON stream format
  - Ollama: newline-delimited JSON
- [ ] Add chunk aggregation for final response
- [ ] Implement stream cancellation
- [ ] Add Tauri event emission for frontend streaming

### ~~1.5 Token Estimator~~ ✅ IMPLEMENTED
Token counting exists in usage tracking.

### ~~1.6 Rate Limiter~~ ✅ PARTIAL
Provider-level rate limit detection exists.

### ~~1.7 Health Monitor~~ ✅ IMPLEMENTED
Circuit breaker with health checks exists.

---

## 2. Cost Optimization Module

### 2.1 Budget Enforcer 🟠
**File:** `src-tauri/src/core/budget.rs` (new)

- [ ] Create `BudgetEnforcer` struct
- [ ] Define `BudgetLimit` struct (amount, period: hourly/daily/monthly/total)
- [ ] Define `BudgetAction` enum (warn, throttle, degrade, reject)
- [ ] Implement spending velocity monitoring
- [ ] Add soft/hard limit tiers
- [ ] Implement automatic model downgrade when approaching limits
- [ ] Create budget status API for frontend

### 2.2 Cost Predictor 🟡
**File:** `src-tauri/src/core/cost_predictor.rs` (new)

- [ ] Create `CostPredictor` struct
- [ ] Implement usage pattern detection
- [ ] Add simple moving average forecasting
- [ ] Generate cost projections
- [ ] Persist historical data

### 2.3 Alert System 🟠
**File:** `src-tauri/src/core/alerts.rs` (new)

- [ ] Create `AlertSystem` struct
- [ ] Define alert types (BudgetApproaching, BudgetExceeded, ProviderDown)
- [ ] Implement threshold-based triggering
- [ ] Add system notification delivery
- [ ] Implement alert deduplication

### ~~2.4 Pricing Engine~~ ✅ IMPLEMENTED
Cost estimation exists in `database/models.rs`.

---

## 3. Search Module

### 3.1 Query Expansion 🟠
**File:** `src-tauri/src/core/query_expansion.rs` (new)

- [ ] Create `QueryExpander` struct
- [ ] Add TTRPG-specific synonym map:
  - "HP" → "hit points", "health"
  - "AC" → "armor class", "defense"
  - "DC" → "difficulty class", "check"
- [ ] Implement related term suggestion
- [ ] Add stemming/lemmatization

### 3.2 Spell Correction 🟡
**File:** `src-tauri/src/core/spell_correction.rs` (new)

- [ ] Create `SpellCorrector` struct
- [ ] Implement Levenshtein distance calculation
- [ ] Build TTRPG vocabulary dictionary
- [ ] Implement "did you mean?" suggestions

### 3.3 Search Analytics 🟢
**File:** `src-tauri/src/core/search_analytics.rs` (new)

- [ ] Track query frequency
- [ ] Track zero-result queries
- [ ] Persist analytics to database

### ~~3.4 Hybrid Search~~ ✅ IMPLEMENTED
Full hybrid search with RRF exists.

---

## 4. Document Processing Module

### 4.1 MOBI/AZW Parser 🟡
**File:** `src-tauri/src/ingestion/mobi_parser.rs` (new)

- [ ] Add MOBI crate dependency
- [ ] Create `MobiParser` struct
- [ ] Extract text content from MOBI/AZW/AZW3
- [ ] Extract metadata (title, author)
- [ ] Handle DRM-free files only

### ~~4.2 PDF Parser~~ ✅ IMPLEMENTED
Full PDF parsing with metadata extraction exists.

### ~~4.3 EPUB Parser~~ ✅ IMPLEMENTED
Full EPUB parsing with chapter extraction exists.

### ~~4.4 Semantic Chunker~~ ✅ IMPLEMENTED
Intelligent chunking with sentence/paragraph awareness exists.

---

## 5. Campaign Module

### ~~5.1 Campaign CRUD~~ ✅ IMPLEMENTED
Full campaign management with snapshots exists.

### ~~5.2 Campaign Notes~~ ✅ IMPLEMENTED
Notes with tagging and search exists.

### ~~5.3 Campaign Export/Import~~ ✅ IMPLEMENTED
Full backup with snapshots and notes exists.

### 5.4 Location Management 🟡
**File:** `src-tauri/src/core/location_manager.rs` (new)

- [ ] Create `LocationManager` struct
- [ ] Define `Location` struct:
  - id, name, description
  - location_type (city, dungeon, wilderness, building)
  - parent_location_id (for hierarchy)
  - connections (list of connected location IDs)
  - npcs_present (list of NPC IDs)
- [ ] Implement CRUD operations
- [ ] Add location hierarchy traversal
- [ ] Add database table and migration

### 5.5 Plot Point Tracking 🟡
**File:** `src-tauri/src/core/plot_manager.rs` (new)

- [ ] Create `PlotManager` struct
- [ ] Define `PlotPoint` struct:
  - id, title, description
  - status (active, completed, failed, pending)
  - priority (main, side, background)
  - involved_npcs, involved_locations
- [ ] Implement CRUD operations
- [ ] Add status transitions
- [ ] Add database table and migration

---

## 6. Session Module

### ~~6.1 Session Management~~ ✅ IMPLEMENTED
Full session tracking with status exists.

### ~~6.2 Combat System~~ ✅ IMPLEMENTED
Initiative, HP, combatant types all exist.

### ~~6.3 Condition Tracking~~ ✅ IMPLEMENTED
6 condition duration types exist.

### 6.4 Session Summary Generation 🟡
**File:** `src-tauri/src/core/session_summary.rs` (new)

- [ ] Create `SessionSummarizer` struct
- [ ] Implement LLM-based summary generation
- [ ] Extract key events, combat outcomes
- [ ] Generate "previously on..." recap
- [ ] Add Tauri command

---

## 7. Character Generation Module

### ~~7.1 Multi-System Support~~ ✅ IMPLEMENTED
8 game systems supported.

### 7.2 Extended Genre Support 🟡
**File:** `src-tauri/src/core/character_gen.rs` (enhance)

Original Python supports 21+ genres. Add:
- [ ] Cyberpunk-specific data (cyberware, neural interface, street cred)
- [ ] Cosmic Horror data (sanity, forbidden knowledge, trauma)
- [ ] Post-Apocalyptic data (mutations, radiation resistance)
- [ ] Superhero data (powers, weaknesses, origin)
- [ ] Add 13+ additional genre support

### 7.3 Name Generator 🟢
**File:** `src-tauri/src/core/name_gen.rs` (new)

- [ ] Create `NameGenerator` struct
- [ ] Add race-specific name tables
- [ ] Support gender variants
- [ ] Add surname/clan generation

---

## 8. Voice Synthesis Module

### ~~8.1 ElevenLabs Provider~~ ✅ IMPLEMENTED
### ~~8.2 Fish Audio Provider~~ ✅ IMPLEMENTED
### ~~8.3 Ollama TTS Provider~~ ✅ IMPLEMENTED
### ~~8.4 System TTS Provider~~ ✅ IMPLEMENTED
### ~~8.5 Voice Caching~~ ✅ IMPLEMENTED
### ~~8.6 Voice Profiles~~ ✅ IMPLEMENTED (via personality module)

### 8.7 Pre-generation Queue 🟢
**File:** `src-tauri/src/core/voice_queue.rs` (new)

- [ ] Create `PreGenerationQueue` struct
- [ ] Implement background job processing
- [ ] Add batch session preparation
- [ ] Add job status tracking

---

## 9. Security Module

### ~~9.1 Credential Storage~~ ✅ IMPLEMENTED
System keyring storage exists.

### 9.2 Input Validator 🟠
**File:** `src-tauri/src/core/input_validator.rs` (new)

- [ ] Create `InputValidator` struct
- [ ] Implement XSS prevention
- [ ] Implement path traversal prevention
- [ ] Add input length limits
- [ ] Create validation for Tauri commands

### 9.3 Audit Logger 🟠
**File:** `src-tauri/src/core/audit.rs` (new)

- [ ] Create `AuditLogger` struct
- [ ] Define audit event types
- [ ] Implement structured audit logging
- [ ] Persist to SQLite
- [ ] Add audit log rotation

---

## 10. Database Module

### ~~10.1 SQLite with SQLx~~ ✅ IMPLEMENTED
### ~~10.2 Connection Pooling~~ ✅ IMPLEMENTED
### ~~10.3 Migrations~~ ✅ IMPLEMENTED
### ~~10.4 Backup/Restore~~ ✅ IMPLEMENTED

### 10.5 Additional Tables 🟡
**File:** `src-tauri/src/database/migrations.rs` (enhance)

- [ ] Add `locations` table
- [ ] Add `plot_points` table
- [ ] Add `audit_log` table

---

# FRONTEND TASKS (Remaining)

## 11. Theme & Styling

### ~~11.1 Dark/Light Theme~~ ✅ IMPLEMENTED
Adaptive UI themes exist.

### 11.2 Design System Polish 🟡
**Target:** extract to `frontend/src/components/design_system.rs`
*Current State:* UI elements are scattered in `chat.rs` and `session.rs`.

- [ ] Create `Button` component (variants: primary, secondary, danger, ghost)
- [ ] Create `Input` and `Select` components with consistent styling
- [ ] Create `Modal` component (reusable portal-based dialog)
- [ ] Create `Card` component (header, body, footer)
- [ ] Create `Badge` component (for conditions, tags)
- [ ] Create `LoadingSpinner` component
- [ ] Refactor existing views to use these primitives

---

## 12. Chat Interface

### 12.1 Streaming Response Display 🔴
**File:** `frontend/src/components/chat.rs` (enhance)

- [ ] Implement real-time token streaming display
- [ ] Add typing indicator during generation
- [ ] Show partial response as it streams
- [ ] Add stop generation button

### 12.2 Message Formatting 🟠
**File:** `frontend/src/components/chat.rs` (enhance)

- [ ] Add Markdown rendering for messages
- [ ] Implement code block syntax highlighting
- [ ] Add message copy button

### 12.3 Chat History 🟠
**File:** `frontend/src/components/chat_history.rs` (new)

- [ ] Implement conversation persistence
- [ ] Add conversation list sidebar
- [ ] Implement conversation search
- [ ] Add conversation export

---

## 13. Settings Interface

### ~~13.1 Provider Configuration~~ ✅ PARTIAL
Basic LLM configuration exists.

### 13.2 OpenAI Configuration 🟠
**File:** `frontend/src/components/settings.rs` (enhance)

- [ ] Add OpenAI API key input
- [ ] Add OpenAI model selection
- [ ] Add API key test button

### 13.3 Budget Settings 🟠
**File:** `frontend/src/components/budget_settings.rs` (new)

- [ ] Create budget configuration panel
- [ ] Add daily/weekly/monthly limit inputs
- [ ] Show current spending display
- [ ] Show spending projection

---

## 14. Library Interface

### ~~14.1 Document Ingestion~~ ✅ IMPLEMENTED
Drag-drop with ingestion exists.

### 14.2 Document Management 🟠
**File:** `frontend/src/components/document_list.rs` (new)

- [ ] Create document list with sorting
- [ ] Add document search/filter
- [ ] Implement document deletion with confirmation
- [ ] Show indexing status

### 14.3 Search Results Enhancement 🟠
**File:** `frontend/src/components/search_results.rs` (enhance)

- [ ] Add relevance score display
- [ ] Add highlighted match snippets
- [ ] Implement pagination

---

## 15. Campaign Interface

### ~~15.1 Campaign List~~ ✅ IMPLEMENTED
### ~~15.2 Campaign Create/Edit~~ ✅ IMPLEMENTED

### 15.3 Campaign Dashboard 🟡
**File:** `frontend/src/components/campaign_dashboard.rs` (new)

- [ ] Create campaign overview page
- [ ] Show campaign stats (sessions, NPCs)
- [ ] Add quick action buttons

### 15.4 Location Manager UI 🟡
**File:** `frontend/src/components/location_manager.rs` (new)

- [ ] Create location hierarchy view
- [ ] Add location creation form
- [ ] Show NPCs at location

### 15.5 Plot Tracker UI 🟡
**File:** `frontend/src/components/plot_tracker.rs` (new)

- [ ] Create plot point kanban board
- [ ] Add plot point creation form
- [ ] Implement drag-drop status changes

---

## 16. Session Interface

### ~~16.1 Combat Tracker~~ ✅ IMPLEMENTED
Initiative, HP, conditions all exist.

### 16.2 Combat Tracker Enhancement 🟠
**File:** `frontend/src/components/session.rs` (enhance)

- [ ] Add HP bars with visual health status
- [ ] Add condition badges on combatant cards
- [ ] Show current turn highlight
- [ ] Add combat log sidebar

### 16.3 Session Notes UI 🟠
**File:** `frontend/src/components/session_notes.rs` (new)

- [ ] Create note-taking panel during session
- [ ] Add quick note buttons (plot, combat, NPC)
- [ ] Show note history for session

### 16.4 Session Summary View 🟡
**File:** `frontend/src/components/session_summary.rs` (new)

- [ ] Create post-session summary page
- [ ] Display AI-generated recap
- [ ] Show combat statistics

---

## 17. Character Interface

### ~~17.1 Character Generation~~ ✅ IMPLEMENTED
Multi-system character generator exists.

### 17.2 Character Sheet View 🟠
**File:** `frontend/src/components/character_sheet.rs` (new)

- [ ] Create full character sheet layout
- [ ] Implement editable fields
- [ ] Add character export (JSON)

---

## 18. Analytics Dashboard

### 18.1 Usage Analytics 🟡
**File:** `frontend/src/components/analytics.rs` (new)

- [ ] Create analytics dashboard page
- [ ] Add token usage chart
- [ ] Add cost breakdown chart
- [ ] Implement date range selector

---

# TASK PRIORITIZATION

## Phase 1: Core Parity (Critical) 🔴
1. OpenAI Provider
2. Streaming Response Handler
3. Streaming Response Display (Frontend)

## Phase 2: Enhanced Experience (High) 🟠
1. Budget Enforcer
2. Alert System
3. Query Expansion
4. Input Validator
5. Audit Logger
6. Chat History
7. Message Formatting
8. Combat Tracker Enhancement
9. Session Notes UI
10. Character Sheet View

## Phase 3: Advanced Features (Medium) 🟡
1. Cost Predictor
2. Spell Correction
3. MOBI Parser
4. Location Management
5. Plot Point Tracking
6. Session Summary Generation
7. Extended Genre Support
8. Campaign Dashboard
9. Analytics Dashboard

## Phase 4: Polish (Low) 🟢
1. Search Analytics
2. Name Generator
3. Pre-generation Queue
4. Design System Polish

---

# REVISED ESTIMATED EFFORT

| Category | Tasks | Est. Hours | Notes |
|----------|-------|------------|-------|
| AI Providers | 2 | 20-30 | Only OpenAI + streaming remain |
| Cost Optimization | 3 | 25-35 | Budget, alerts, predictor |
| Search | 3 | 20-25 | Query expansion, spell check, analytics |
| Document Processing | 1 | 8-12 | MOBI parser only |
| Campaign | 2 | 20-25 | Locations, plot points |
| Session | 1 | 8-12 | Summary generation |
| Character | 2 | 15-20 | Extended genres, name gen |
| Voice | 1 | 8-10 | Pre-generation queue |
| Security | 2 | 15-20 | Input validation, audit |
| Database | 1 | 5-8 | Additional tables |
| **Backend Total** | **18** | **144-197** | |
| | | | |
| Chat Interface | 3 | 25-35 | Streaming, formatting, history |
| Settings | 2 | 15-20 | OpenAI config, budget |
| Library | 2 | 15-20 | Document management, search |
| Campaign UI | 3 | 25-30 | Dashboard, locations, plots |
| Session UI | 3 | 20-25 | Enhancements, notes, summary |
| Character UI | 1 | 15-20 | Character sheet |
| Analytics | 1 | 10-15 | Usage dashboard |
| Design System | 1 | 8-12 | Polish |
| **Frontend Total** | **16** | **133-177** | |
| | | | |
| **GRAND TOTAL** | **34** | **277-374** | Reduced from 85 tasks / 630-810 hrs |

---

# COMPARISON: What Was Already Done

The following items from the original task list are now **complete**:

### Backend ✅
- ~~Enhanced Provider Router~~ → Circuit breaker exists
- ~~Load Balancer~~ → Health-aware routing exists
- ~~Token Estimator~~ → Usage tracking exists
- ~~Rate Limiter~~ → Provider rate limiting exists
- ~~Health Monitor~~ → Circuit breaker health checks exist
- ~~Pricing Engine~~ → Cost estimation exists
- ~~Hybrid Search~~ → RRF implementation exists
- ~~PDF Parser~~ → lopdf implementation exists
- ~~EPUB Parser~~ → epub crate implementation exists
- ~~Semantic Chunker~~ → Sentence/paragraph aware chunking exists
- ~~Campaign CRUD~~ → Full implementation exists
- ~~Campaign Notes~~ → Tagged notes with search exist
- ~~Campaign Snapshots~~ → Manual + auto + rollback exist
- ~~Campaign Export/Import~~ → Full backup exists
- ~~Session Management~~ → Full session tracking exists
- ~~Combat System~~ → Initiative + HP + combatants exist
- ~~Condition Tracking~~ → 6 duration types exist
- ~~Multi-System Character Gen~~ → 8 systems exist
- ~~ElevenLabs Provider~~ → Full implementation exists
- ~~Fish Audio Provider~~ → Full implementation exists
- ~~Ollama TTS Provider~~ → Full implementation exists
- ~~Voice Caching~~ → Hash-based file cache exists
- ~~Voice Profiles~~ → Via personality module
- ~~Credential Storage~~ → System keyring exists
- ~~Database Pool~~ → SQLx pool exists
- ~~Migrations~~ → Versioned migrations exist
- ~~Backup/Restore~~ → Full implementation exists

### Frontend ✅
- ~~Dark/Light Theme~~ → Adaptive themes exist
- ~~Campaign List~~ → Full implementation exists
- ~~Campaign Create/Edit~~ → Modal forms exist
- ~~Combat Tracker~~ → Initiative/HP/conditions exist
- ~~Character Generation~~ → Multi-system wizard exists
- ~~Document Ingestion~~ → Drag-drop exists
- ~~Provider Configuration~~ → Basic config exists

---

*Generated by Claude Code for TTTRPS feature parity planning*
*Updated: 2025-12-27 with accurate implementation status*
