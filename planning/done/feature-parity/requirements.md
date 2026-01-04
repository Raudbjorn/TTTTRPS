# Feature Parity Requirements

This document defines the requirements for achieving feature parity between the original Python MCP server implementation and the Rust/Tauri desktop application.

> **See also**: `UXdesign/design.md` for detailed visual specifications and mockups.

## Document Information

| Field | Value |
|-------|-------|
| Version | 1.1.0 |
| Last Updated | 2026-01-02 |
| Status | Draft |

---

## 1. LLM Provider Requirements

### REQ-LLM-001: Multi-Provider Support
The system SHALL support multiple LLM providers with a unified interface.

**Acceptance Criteria:**
- Support for at least 10 providers (Ollama, Claude, OpenAI, Gemini, OpenRouter, Mistral, Groq, Together, Cohere, DeepSeek)
- Unified request/response format across providers
- Provider-specific error handling

**Current Status:** Implemented

### REQ-LLM-002: Dynamic Model Listing
The system SHALL fetch available models from provider APIs dynamically.

**Acceptance Criteria:**
- API-based model fetching for providers with public endpoints
- Fallback to hardcoded lists when API unavailable
- Model metadata including context window, pricing, capabilities

**Current Status:** Implemented

### REQ-LLM-003: Streaming Response Support
The system SHALL support streaming responses from LLM providers.

**Acceptance Criteria:**
- SSE/chunked response handling
- Real-time token delivery to UI
- Proper stream termination handling

**Current Status:** Partial (backend support, frontend needs streaming UI)

### REQ-LLM-004: Provider Health Monitoring
The system SHALL monitor provider health and availability.

**Acceptance Criteria:**
- Health check endpoint per provider
- Automatic failover on provider unavailability
- Health status display in UI

**Current Status:** Partial (health check exists, no auto-failover)

### REQ-LLM-005: Cost Tracking and Optimization
The system SHALL track token usage and costs per provider.

**Acceptance Criteria:**
- Token counting per request/response
- Cost calculation based on provider pricing
- Usage statistics and reporting
- Budget enforcement with warnings

**Current Status:** Not Implemented

### REQ-LLM-006: Tool/Function Calling
The system SHALL support tool/function calling across providers.

**Acceptance Criteria:**
- Unified tool definition format
- Provider-specific tool call translation
- Tool response handling and integration

**Current Status:** Not Implemented

---

## 2. Voice/TTS Requirements

### REQ-VOICE-001: Multi-Provider Voice Synthesis
The system SHALL support multiple voice synthesis providers.

**Acceptance Criteria:**
- ElevenLabs integration with API key
- OpenAI TTS with voice/model selection
- Ollama local TTS support
- Fish Audio integration
- System TTS fallback

**Current Status:** Partial (ElevenLabs, OpenAI, Ollama implemented)

### REQ-VOICE-002: Voice Profile System
The system SHALL support voice profiles for NPCs and narration.

**Acceptance Criteria:**
- Create/edit/delete voice profiles
- Link profiles to NPCs
- Profile metadata (age, gender, personality)
- 13+ preset DM/narrator personas

**Current Status:** Not Implemented

### REQ-VOICE-003: Audio Caching
The system SHALL cache synthesized audio for reuse.

**Acceptance Criteria:**
- Disk-based cache with configurable size limits
- Cache key based on text + voice + settings
- LRU eviction policy
- Cache statistics and management UI

**Current Status:** Partial (basic caching in VoiceManager)

### REQ-VOICE-004: Pre-Generation Queue
The system SHALL support pre-generating audio for upcoming content.

**Acceptance Criteria:**
- Background synthesis job queue
- Priority-based processing
- Progress tracking
- Session-based batch pre-generation

**Current Status:** Not Implemented

### REQ-VOICE-005: Audio Playback System
The system SHALL provide integrated audio playback.

**Acceptance Criteria:**
- Multi-sink support (voice, music, ambience, SFX)
- Volume control per sink
- Playback queue management
- Cross-fade support for music

**Current Status:** Partial (basic playback exists)

---

## 3. Document Ingestion Requirements

### REQ-DOC-001: Multi-Format Support
The system SHALL support ingestion of multiple document formats.

**Acceptance Criteria:**
- PDF with text extraction and table preservation
- EPUB e-book format
- MOBI/AZW/AZW3 Kindle formats
- DOCX Word documents
- TXT and Markdown plaintext

**Current Status:** Implemented

### REQ-DOC-002: Intelligent Chunking
The system SHALL chunk documents intelligently for search indexing.

**Acceptance Criteria:**
- Semantic-aware chunk boundaries
- Configurable chunk size and overlap
- Section hierarchy preservation
- Table extraction as searchable content

**Current Status:** Implemented

### REQ-DOC-003: Progress Reporting
The system SHALL report ingestion progress to the user.

**Acceptance Criteria:**
- Stage-based progress (parsing, chunking, indexing)
- Real-time progress updates via events
- Estimated time remaining
- Error reporting with context

**Current Status:** Implemented

### REQ-DOC-004: Adaptive Pattern Learning
The system SHALL learn document patterns for improved extraction.

**Acceptance Criteria:**
- Format pattern detection (spell blocks, stat blocks, tables)
- Pattern storage and reuse
- User feedback integration
- Pattern performance metrics

**Current Status:** Not Implemented

### REQ-DOC-005: Content Classification
The system SHALL classify document content by type.

**Acceptance Criteria:**
- Genre detection (rules, spells, creatures, lore)
- Source type tagging
- Campaign association
- Content enrichment metadata

**Current Status:** Partial (source_type field exists)

---

## 4. Campaign Management Requirements

### REQ-CAMP-001: Campaign CRUD Operations
The system SHALL support full campaign lifecycle management.

**Acceptance Criteria:**
- Create campaigns with name, system, description
- Read campaign details and contents
- Update campaign metadata and state
- Delete campaigns with confirmation

**Current Status:** Implemented

### REQ-CAMP-002: Campaign Versioning
The system SHALL maintain version history for campaigns.

**Acceptance Criteria:**
- Automatic snapshots on significant changes
- Manual snapshot creation with descriptions
- Version comparison view
- Rollback to previous versions

**Current Status:** Partial (snapshots exist, no comparison/rollback UI)

### REQ-CAMP-003: World State Management
The system SHALL track campaign world state.

**Acceptance Criteria:**
- In-game date tracking
- World events timeline
- Location state changes
- NPC relationship tracking
- Custom state fields

**Current Status:** Not Implemented

### REQ-CAMP-004: Campaign Export/Import
The system SHALL support campaign data portability.

**Acceptance Criteria:**
- JSON export of full campaign data
- Import with optional new ID generation
- Cross-version compatibility
- Selective export (characters only, NPCs only, etc.)

**Current Status:** Partial (basic export/import exists)

### REQ-CAMP-005: Multi-System Support
The system SHALL support multiple TTRPG systems.

**Acceptance Criteria:**
- System-specific character templates
- System-aware content generation
- House rules support
- System resource linking

**Current Status:** Partial (system field exists, no templates)

---

## 5. Session Management Requirements

### REQ-SESS-001: Session Lifecycle
The system SHALL manage game session lifecycle.

**Acceptance Criteria:**
- Start session with campaign association
- Track session duration and events
- End session with summary generation
- Session notes and timestamps

**Current Status:** Implemented

### REQ-SESS-002: Combat Tracking
The system SHALL provide combat encounter management.

**Acceptance Criteria:**
- Initiative order management
- HP/health tracking with max values
- Condition application and tracking
- Damage and healing operations
- Turn advancement

**Current Status:** Implemented

### REQ-SESS-003: Condition System
The system SHALL support TTRPG conditions and effects.

**Acceptance Criteria:**
- Common conditions (blinded, paralyzed, etc.)
- Custom condition creation
- Duration tracking
- Effect descriptions
- Auto-removal on duration expiry

**Current Status:** Partial (basic conditions, no duration tracking)

### REQ-SESS-004: Session Notes
The system SHALL capture and organize session notes.

**Acceptance Criteria:**
- Free-form note entry
- Tag-based organization
- Entity linking (NPCs, locations)
- Search within session notes
- AI-assisted categorization

**Current Status:** Partial (notes exist, no AI categorization)

### REQ-SESS-005: Session Timeline
The system SHALL maintain an event timeline per session.

**Acceptance Criteria:**
- Chronological event recording
- Event type classification
- Entity involvement tracking
- Timeline visualization

**Current Status:** Not Implemented

---

## 6. Search and RAG Requirements

### REQ-SEARCH-001: Hybrid Search
The system SHALL provide hybrid semantic and keyword search.

**Acceptance Criteria:**
- Vector-based semantic search
- BM25 keyword search
- Reciprocal Rank Fusion for result merging
- Configurable semantic/keyword weighting

**Current Status:** Partial (Meilisearch keyword search, no vector search)

### REQ-SEARCH-002: Search Filters
The system SHALL support filtered search queries.

**Acceptance Criteria:**
- Source type filtering
- Campaign ID filtering
- Index-specific search
- Date range filtering
- Metadata-based filters

**Current Status:** Partial (basic filters exist)

### REQ-SEARCH-003: Query Enhancement
The system SHALL enhance search queries intelligently.

**Acceptance Criteria:**
- TTRPG synonym expansion (HP -> hit points)
- Spell correction for typos
- Query completion suggestions
- Query clarification prompts

**Current Status:** Not Implemented

### REQ-SEARCH-004: RAG Integration
The system SHALL support retrieval-augmented generation.

**Acceptance Criteria:**
- Automatic context retrieval for chat
- Source citation in responses
- Configurable retrieval parameters
- Campaign-aware context selection

**Current Status:** Partial (Meilisearch Chat API, basic RAG)

### REQ-SEARCH-005: Search Analytics
The system SHALL track and report search usage.

**Acceptance Criteria:**
- Query frequency tracking
- Result click-through rates
- Cache hit/miss statistics
- Popular search terms

**Current Status:** Not Implemented

---

## 7. Character Generation Requirements

### REQ-CHAR-001: Multi-System Character Generation
The system SHALL generate characters for multiple TTRPG systems.

**Acceptance Criteria:**
- D&D 5e character generation
- Pathfinder 2e support
- At least 6 additional systems (e.g., Cyberpunk, Shadowrun, GURPS, Warhammer)
- System-specific stat arrays
- Class and race selection

**Current Status:** Partial (basic generation, limited systems)

### REQ-CHAR-002: NPC Generation
The system SHALL generate NPCs with full details.

**Acceptance Criteria:**
- Role-based generation (merchant, guard, noble)
- Appearance and personality traits
- Motivations and secrets
- Quest hooks and relationships
- Stats appropriate to role

**Current Status:** Implemented

### REQ-CHAR-003: AI-Powered Backstory
The system SHALL generate character backstories using AI.

**Acceptance Criteria:**
- LLM-based backstory generation
- Style matching to campaign setting
- Integration with character traits
- Editable generated content

**Current Status:** Not Implemented

### REQ-CHAR-004: Character Validation
The system SHALL validate characters against system rules.

**Acceptance Criteria:**
- Stat total validation
- Level-appropriate abilities
- Equipment restrictions
- Race/class compatibility

**Current Status:** Not Implemented

### REQ-CHAR-005: Location Generation
The system SHALL generate locations for campaigns.

**Acceptance Criteria:**
- Type-based generation (city, dungeon, forest)
- Notable features and inhabitants
- Connected locations
- Secrets and encounters
- Map reference support

**Current Status:** Not Implemented

---

## 8. Personality System Requirements

### REQ-PERS-001: Personality Profiles
The system SHALL support personality profiles for AI responses.

**Acceptance Criteria:**
- Create custom personality profiles
- Preset DM personality styles
- Tone and style parameters
- System-specific adaptations

**Current Status:** Partial (PersonalityStore exists, limited features)

### REQ-PERS-002: Personality Application
The system SHALL apply personalities to generated content.

**Acceptance Criteria:**
- Chat response styling
- NPC dialogue generation
- Narration tone matching
- Consistent voice across interactions

**Current Status:** Partial (basic system prompt integration)

### REQ-PERS-003: Active Personality Management
The system SHALL manage active personality context.

**Acceptance Criteria:**
- Set/switch active personality
- Personality per campaign/session
- Quick personality switching in UI
- Personality preview

**Current Status:** Not Implemented

---

## 9. Security and Privacy Requirements

### REQ-SEC-001: Credential Management
The system SHALL securely manage API credentials.

**Acceptance Criteria:**
- OS keychain integration
- Encrypted credential storage
- Credential isolation per provider
- No plaintext credential logging

**Current Status:** Implemented

### REQ-SEC-002: File Access Control
The system SHALL validate file access operations.

**Acceptance Criteria:**
- Path traversal prevention
- Allowed directory restrictions
- File type validation
- Size limits enforcement

**Current Status:** Partial (basic validation exists)

### REQ-SEC-003: Audit Logging
The system SHALL log security-relevant events.

**Acceptance Criteria:**
- Authentication attempts
- API key usage
- File operations
- Configuration changes

**Current Status:** Not Implemented

---

## 10. Performance Requirements

### REQ-PERF-001: Response Time
The system SHALL respond to user actions within acceptable limits.

**Acceptance Criteria:**
- UI interactions < 100ms
- Search queries < 500ms
- Document ingestion progress updates < 1s
- Chat responses start streaming < 2s

**Current Status:** Needs Testing

### REQ-PERF-002: Memory Efficiency
The system SHALL manage memory efficiently.

**Acceptance Criteria:**
- Bounded memory for document processing
- Cache size limits enforcement
- Memory cleanup on session end
- Large file streaming support

**Current Status:** Needs Testing

### REQ-PERF-003: Concurrent Operations
The system SHALL support concurrent operations.

**Acceptance Criteria:**
- Parallel document ingestion
- Background voice synthesis
- Non-blocking UI during operations
- Request queuing and prioritization

**Current Status:** Partial (basic async support)

---

## 11. User Interface Requirements

### REQ-UI-001: Settings Management
The system SHALL provide comprehensive settings UI.

**Acceptance Criteria:**
- LLM provider configuration with model dropdowns
- Voice provider configuration with voice selection
- Theme selection
- Search engine status

**Current Status:** Implemented

### REQ-UI-002: Campaign Dashboard
The system SHALL provide a campaign management dashboard.

**Acceptance Criteria:**
- Campaign list view
- Campaign creation wizard
- Campaign details view
- Quick actions (start session, add NPC)

**Current Status:** Partial (basic campaign list)

### REQ-UI-003: Chat Interface
The system SHALL provide a rich chat interface.

**Acceptance Criteria:**
- Message history with scroll
- Markdown rendering
- Code block syntax highlighting
- Token usage display
- Personality selector

**Current Status:** Partial (basic chat exists)

### REQ-UI-004: Library Browser
The system SHALL provide a document library browser.

**Acceptance Criteria:**
- Document list with metadata
- Source type categorization
- Search within library
- Document ingestion from browser

**Current Status:** Partial (ingestion exists, no browser)

### REQ-UI-005: Combat Tracker UI
The system SHALL provide a combat tracking interface.

**Acceptance Criteria:**
- Initiative order display
- Current combatant highlight
- HP bars and condition icons
- Quick actions (damage, heal, condition)
- Round counter

**Current Status:** Not Implemented (commands exist, no UI)

---

## 12. UX/Frontend Requirements

*Based on UX Design Specification v2.0.0 (see `UXdesign/design.md`)*

### Layout & Navigation

| ID | Requirement | Description |
|----|-------------|-------------|
| REQ-LAYOUT-1 | 5-Panel Architecture | Implement responsive grid: Icon Rail, Context Sidebar, Main Content, Info Panel, Media Bar |
| REQ-LAYOUT-2 | Collapsible Sidebars | Context Sidebar and Info Panel toggleable via hotkeys (`Cmd+.`, `Cmd+/`) |
| REQ-NAV-1 | Icon Rail | Fixed 48-64px left rail with tooltips for global navigation |
| REQ-NAV-2 | Context Switching | Sidebar content dynamically changes based on active view |

### Design Metaphors

| ID | Requirement | Description |
|----|-------------|-------------|
| REQ-META-SPOTIFY | Campaign/Session Presentation | Campaigns as "Albums" (cover art, genre); Sessions as "Tracks" (playable, duration) |
| REQ-META-SLACK | NPC Interaction | NPCs as contact list with presence dots, unread badges, thread-capable chat |
| REQ-META-OBSIDIAN | Knowledge Base | Library as graph with entity linking and backlink visualization |

### Dynamic Theming

| ID | Requirement | Description |
|----|-------------|-------------|
| REQ-THEME-1 | Extended Theme Set | 5 core themes: Fantasy, Cosmic, Terminal, Noir, Neon |
| REQ-THEME-2 | CSS Variable Architecture | Core palette (`--bg-deep`, `--accent`, etc.) across all themes |
| REQ-THEME-3 | Theme Interpolation | Weighted blending of themes (e.g., 60% Noir + 40% Cosmic) |
| REQ-THEME-4 | Visual Effects | CSS effects: Film Grain, CRT Scanlines, Text Glow, Redaction |
| REQ-THEME-5 | Auto-Adaptation | Default presets per game system with manual override |

### Components

| ID | Requirement | Description |
|----|-------------|-------------|
| REQ-COMP-MEDIA | Media Bar | Persistent 56px bottom bar with Play/Pause, Volume, "Now Speaking" |
| REQ-COMP-CARD | Campaign Card | Rich visual card with "Now Playing" pulse animation |
| REQ-COMP-CHAT | Chat Threading | Reply to specific messages with visual threading |

### Non-Functional (Frontend)

| ID | Requirement | Description |
|----|-------------|-------------|
| REQ-PERF-FE-1 | Animation | Purposeful, fast transitions (150-200ms). Respect `prefers-reduced-motion` |
| REQ-A11Y-1 | Accessibility | All themes meet WCAG contrast ratios |

---

## Requirement Priority Matrix

| Priority | Category | Count |
|----------|----------|-------|
| P0 - Critical | Core LLM, Basic UI | 8 |
| P1 - High | Voice, Search, Campaign | 15 |
| P2 - Medium | Generation, Sessions | 12 |
| P3 - Low | Analytics, Advanced | 10 |

---

## Feature Parity Summary

| Category | Python Original | Rust/Tauri Current | Gap |
|----------|-----------------|-------------------|-----|
| LLM Providers | 4 core + routing | 10 providers | Feature ahead, missing routing |
| Voice Providers | 4 + profiles | 3 basic | Missing profiles, caching |
| Document Formats | 4 formats | 5 formats | Feature ahead |
| Campaign Management | Full CRUD + versioning | Basic CRUD | Missing versioning UI |
| Session Management | Full combat + timeline | Basic combat | Missing timeline, advanced conditions |
| Search | Hybrid + analytics | Keyword only | Missing vector search |
| Character Gen | 8 systems + AI | Basic + NPC | Missing multi-system, AI backstory |
| Personality | Full system | Basic store | Missing application layer |
| Security | Full framework | Credentials only | Missing audit, access control |

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.1.0 | 2026-01-02 | Added UX/Frontend requirements section |
| 1.0.0 | 2025-12-29 | Initial requirements document |
