# Feature Parity Architecture Design

This document describes the architecture and design decisions for achieving feature parity between the original Python MCP server and the Rust/Tauri desktop application.

## Document Information

| Field | Value |
|-------|-------|
| Version | 1.1.0 |
| Last Updated | 2025-12-29 |
| Status | Draft |

---

## 1. Architecture Overview

### 1.1 Current Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Tauri Application                         │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────────┐    ┌──────────────────┐                   │
│  │   Frontend       │    │   Backend        │                   │
│  │   (Dioxus/WASM)  │◄──►│   (Rust)         │                   │
│  │                  │    │                  │                   │
│  │  - Components    │    │  - Commands      │                   │
│  │  - Bindings      │    │  - Core Modules  │                   │
│  │  - State         │    │  - State Mgmt    │                   │
│  └──────────────────┘    └────────┬─────────┘                   │
│                                   │                              │
│                    ┌──────────────┴──────────────┐              │
│                    ▼                             ▼              │
│           ┌───────────────┐            ┌─────────────────┐      │
│           │  Meilisearch  │            │  External APIs  │      │
│           │  (Sidecar)    │            │  (LLM, Voice)   │      │
│           └───────────────┘            └─────────────────┘      │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Target Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           Tauri Application                              │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌────────────────────────┐         ┌────────────────────────────┐      │
│  │      Frontend          │         │         Backend            │      │
│  │      (Dioxus/WASM)     │◄───────►│         (Rust)             │      │
│  │                        │   IPC   │                            │      │
│  │  ┌──────────────────┐  │         │  ┌──────────────────────┐  │      │
│  │  │ Chat Component   │  │         │  │  LLM Provider Router │  │      │
│  │  │ Campaign Dash    │  │         │  │  - Multi-provider    │  │      │
│  │  │ Library Browser  │  │         │  │  - Fallback logic    │  │      │
│  │  │ Combat Tracker   │  │         │  │  - Cost optimization │  │      │
│  │  │ Settings Panel   │  │         │  └──────────────────────┘  │      │
│  │  └──────────────────┘  │         │                            │      │
│  │                        │         │  ┌──────────────────────┐  │      │
│  │  ┌──────────────────┐  │         │  │  Voice Manager       │  │      │
│  │  │ State Management │  │         │  │  - Provider routing  │  │      │
│  │  │ - Signals        │  │         │  │  - Profile system    │  │      │
│  │  │ - Context        │  │         │  │  - Caching layer     │  │      │
│  │  └──────────────────┘  │         │  └──────────────────────┘  │      │
│  └────────────────────────┘         │                            │      │
│                                     │  ┌──────────────────────┐  │      │
│                                     │  │  Campaign Manager    │  │      │
│                                     │  │  - CRUD + Versioning │  │      │
│                                     │  │  - World state       │  │      │
│                                     │  │  - Entity relations  │  │      │
│                                     │  └──────────────────────┘  │      │
│                                     │                            │      │
│                                     │  ┌──────────────────────┐  │      │
│                                     │  │  Session Manager     │  │      │
│                                     │  │  - Combat tracking   │  │      │
│                                     │  │  - Timeline events   │  │      │
│                                     │  │  - Notes system      │  │      │
│                                     │  └──────────────────────┘  │      │
│                                     │                            │      │
│                                     │  ┌──────────────────────┐  │      │
│                                     │  │  Search Engine       │  │      │
│                                     │  │  - Hybrid search     │  │      │
│                                     │  │  - RAG integration   │  │      │
│                                     │  │  - Query enhancement │  │      │
│                                     │  └──────────────────────┘  │      │
│                                     │                            │      │
│                                     │  ┌──────────────────────┐  │      │
│                                     │  │  Generator Engine    │  │      │
│                                     │  │  - Character gen     │  │      │
│                                     │  │  - NPC gen           │  │      │
│                                     │  │  - Location gen      │  │      │
│                                     │  └──────────────────────┘  │      │
│                                     └────────────────────────────┘      │
│                                                  │                       │
│                    ┌─────────────────────────────┼───────────────────┐  │
│                    ▼                             ▼                   ▼  │
│           ┌───────────────┐            ┌─────────────┐      ┌────────┐  │
│           │  Meilisearch  │            │  SQLite     │      │External│  │
│           │  (Sidecar)    │            │  (Local DB) │      │  APIs  │  │
│           │  - Search     │            │  - Campaigns│      │  - LLM │  │
│           │  - Vectors    │            │  - Sessions │      │  - TTS │  │
│           │  - RAG        │            │  - Entities │      │  - etc │  │
│           └───────────────┘            └─────────────┘      └────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Module Designs

### 2.1 LLM Provider Router

**Purpose:** Unified interface for multiple LLM providers with intelligent routing.

**Location:** `src-tauri/src/core/llm/router.rs`

**Design:**

```rust
pub struct LLMRouter {
    providers: HashMap<String, Box<dyn LLMProvider>>,
    config: RouterConfig,
    health_tracker: HealthTracker,
    cost_tracker: CostTracker,
}

pub trait LLMProvider: Send + Sync {
    fn id(&self) -> &str;
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;
    async fn stream_chat(&self, request: ChatRequest) -> Result<Box<dyn Stream<Item = Result<ChatChunk>> + Send + Unpin>>;
    async fn health_check(&self) -> bool;
    fn pricing(&self) -> ProviderPricing;
}

impl LLMRouter {
    /// Route request to best available provider
    pub async fn route(&self, request: ChatRequest) -> Result<ChatResponse> {
        // 1. Check if specific provider requested
        // 2. Apply cost optimization if enabled
        // 3. Check provider health
        // 4. Execute with fallback on failure
    }

    /// Get all healthy providers
    pub fn healthy_providers(&self) -> Vec<&str>;

    /// Get cost estimates for request
    pub fn estimate_cost(&self, request: &ChatRequest) -> HashMap<String, f64>;
}
```

**Satisfies:** REQ-LLM-001, REQ-LLM-004, REQ-LLM-005

### 2.2 Voice Profile System

**Purpose:** Manage voice profiles linked to NPCs and personalities.

**Location:** `src-tauri/src/core/voice/profiles.rs`

**Design:**

```rust
pub struct VoiceProfile {
    pub id: String,
    pub name: String,
    pub provider: VoiceProviderType,
    pub voice_id: String,
    pub settings: VoiceSettings,
    pub metadata: ProfileMetadata,
}

pub struct ProfileMetadata {
    pub age_range: AgeRange,      // Child, Adult, Elderly
    pub gender: Gender,            // Male, Female, Neutral
    pub personality_traits: Vec<String>,
    pub linked_npc_ids: Vec<String>,
}

pub struct VoiceProfileManager {
    profiles: HashMap<String, VoiceProfile>,
    cache: AudioCache,
    presets: Vec<VoiceProfile>,  // 13+ DM personas
}

impl VoiceProfileManager {
    pub fn create_profile(&mut self, profile: VoiceProfile) -> Result<String>;
    pub fn get_profile(&self, id: &str) -> Option<&VoiceProfile>;
    pub fn link_to_npc(&mut self, profile_id: &str, npc_id: &str) -> Result<()>;
    pub fn get_profile_for_npc(&self, npc_id: &str) -> Option<&VoiceProfile>;
    pub fn list_presets(&self) -> &[VoiceProfile];
}
```

**Satisfies:** REQ-VOICE-002

### 2.3 Audio Cache System

**Purpose:** Cache synthesized audio with intelligent eviction.

**Location:** `src-tauri/src/core/voice/cache.rs`

**Design:**

```rust
pub struct AudioCache {
    cache_dir: PathBuf,
    max_size_bytes: u64,
    current_size: AtomicU64,
    entries: RwLock<HashMap<String, CacheEntry>>,
}

pub struct CacheEntry {
    pub path: PathBuf,
    pub size: u64,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u32,
    pub tags: Vec<String>,  // session_id, npc_id, etc.
}

impl AudioCache {
    /// Get or synthesize audio
    pub async fn get_or_synthesize<F, Fut>(
        &self,
        key: &str,
        synthesize: F,
    ) -> Result<PathBuf>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Vec<u8>>>;

    /// Evict entries using LRU policy
    fn evict_lru(&self, bytes_needed: u64);

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats;

    /// Clear entries by tag
    pub fn clear_by_tag(&self, tag: &str);
}
```

**Satisfies:** REQ-VOICE-003

### 2.4 Hybrid Search Engine

**Purpose:** Combine vector and keyword search with result fusion.

**Location:** `src-tauri/src/core/search/hybrid.rs`

**Design:**

```rust
pub struct HybridSearchEngine {
    meilisearch: SearchClient,
    embedding_provider: Box<dyn EmbeddingProvider>,
    config: HybridConfig,
}

pub struct HybridConfig {
    pub semantic_weight: f32,      // 0.0 - 1.0
    pub keyword_weight: f32,       // 0.0 - 1.0
    pub rrf_k: u32,               // RRF constant (typically 60)
    pub query_expansion: bool,
    pub spell_correction: bool,
}

pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
}

impl HybridSearchEngine {
    /// Perform hybrid search with RRF
    pub async fn search(&self, query: &str, options: SearchOptions) -> Result<Vec<SearchResult>> {
        // 1. Expand query with TTRPG synonyms
        // 2. Run keyword search
        // 3. Embed query and run vector search
        // 4. Fuse results using RRF
        // 5. Apply filters and return
    }

    /// Expand query with TTRPG knowledge
    fn expand_query(&self, query: &str) -> String {
        // HP -> "HP OR hit points OR health"
        // AC -> "AC OR armor class OR defense"
        // etc.
    }
}
```

**Satisfies:** REQ-SEARCH-001, REQ-SEARCH-003

### 2.5 Campaign Versioning System

**Purpose:** Track campaign history with rollback capability.

**Location:** `src-tauri/src/core/campaign/versioning.rs`

**Design:**

```rust
pub struct CampaignVersion {
    pub id: String,
    pub campaign_id: String,
    pub version_number: u32,
    pub created_at: DateTime<Utc>,
    pub description: String,
    pub snapshot_type: SnapshotType,
    pub data: CampaignSnapshot,
    pub diff_from_previous: Option<CampaignDiff>,
}

pub enum SnapshotType {
    Manual,
    AutoSave,
    PreEdit,
    SessionStart,
    SessionEnd,
}

pub struct CampaignDiff {
    pub additions: Vec<EntityChange>,
    pub modifications: Vec<EntityChange>,
    pub deletions: Vec<EntityChange>,
}

pub struct VersionManager {
    storage: Box<dyn VersionStorage>,
    config: VersionConfig,
}

impl VersionManager {
    /// Create a new version snapshot
    pub async fn create_version(
        &self,
        campaign_id: &str,
        snapshot_type: SnapshotType,
        description: &str,
    ) -> Result<String>;

    /// Compare two versions
    pub fn compare(&self, v1: &str, v2: &str) -> Result<CampaignDiff>;

    /// Rollback to a previous version
    pub async fn rollback(&self, campaign_id: &str, version_id: &str) -> Result<()>;

    /// List versions for a campaign
    pub fn list_versions(&self, campaign_id: &str) -> Result<Vec<CampaignVersion>>;
}
```

**Satisfies:** REQ-CAMP-002

### 2.6 Entity Relationship System

**Purpose:** Track many-to-many relationships between campaign entities (NPCs, Characters, Locations, Quests).

**Location:** `src-tauri/src/core/campaign/relationships.rs`

**Design:**

```rust
pub struct EntityRelationship {
    pub id: String,
    pub campaign_id: String,
    pub source_entity_type: EntityType,
    pub source_entity_id: String,
    pub target_entity_type: EntityType,
    pub target_entity_id: String,
    pub relationship_type: RelationshipType,
    pub description: Option<String>,
    pub strength: f32,          // 0.0 to 1.0
    pub bidirectional: bool,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Npc,
    Character,
    Location,
    Quest,
}

#[derive(Clone, PartialEq, Eq)]
pub enum RelationshipType {
    // Social
    Ally,
    Enemy,
    Neutral,
    Family,
    Friend,
    Rival,
    // Professional
    Employee,
    Employer,
    Colleague,
    Mentor,
    Student,
    // Location
    LocatedAt,
    OriginatesFrom,
    Controls,
    // Quest
    QuestGiver,
    QuestTarget,
    QuestLocation,
    // Custom
    Custom(String),
}

pub struct RelationshipManager {
    db: Arc<Database>,
}

impl RelationshipManager {
    /// Create a relationship between entities
    pub async fn create(&self, relationship: EntityRelationship) -> Result<String>;

    /// Get all relationships for an entity
    pub async fn get_for_entity(
        &self,
        entity_type: EntityType,
        entity_id: &str,
    ) -> Result<Vec<EntityRelationship>>;

    /// Get relationships of a specific type
    pub async fn get_by_type(
        &self,
        campaign_id: &str,
        relationship_type: &RelationshipType,
    ) -> Result<Vec<EntityRelationship>>;

    /// Delete a relationship
    pub async fn delete(&self, id: &str) -> Result<()>;

    /// Get entity graph for visualization
    pub async fn get_entity_graph(&self, campaign_id: &str) -> Result<EntityGraph>;
}

pub struct EntityGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

pub struct GraphNode {
    pub entity_type: EntityType,
    pub entity_id: String,
    pub label: String,
}

pub struct GraphEdge {
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: RelationshipType,
    pub bidirectional: bool,
}
```

**Satisfies:** REQ-CAMP-003

### 2.7 Session Timeline

**Purpose:** Track chronological events within sessions.

**Location:** `src-tauri/src/core/session/timeline.rs`

**Design:**

```rust
pub struct TimelineEvent {
    pub id: String,
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub description: String,
    pub entities_involved: Vec<EntityReference>,
    pub metadata: serde_json::Value,
}

pub enum EventType {
    CombatStart,
    CombatEnd,
    CombatantAction { combatant_id: String, action: String },
    NoteAdded { note_id: String },
    NPCInteraction { npc_id: String },
    LocationChange { location_id: String },
    CustomEvent { category: String },
}

pub struct SessionTimeline {
    events: Vec<TimelineEvent>,
    session_id: String,
}

impl SessionTimeline {
    /// Add event to timeline
    pub fn add_event(&mut self, event: TimelineEvent);

    /// Get events by type
    pub fn events_by_type(&self, event_type: &EventType) -> Vec<&TimelineEvent>;

    /// Get events in time range
    pub fn events_in_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<&TimelineEvent>;

    /// Generate session summary from timeline
    pub fn generate_summary(&self) -> SessionSummary;
}
```

**Satisfies:** REQ-SESS-005

### 2.8 Character Generation Engine

**Purpose:** Generate characters for multiple TTRPG systems.

**Location:** `src-tauri/src/core/character_gen/engine.rs`

**Design:**

```rust
pub trait SystemGenerator: Send + Sync {
    fn system_id(&self) -> &str;
    fn generate(&self, options: &GenerationOptions) -> Result<Character>;
    fn validate(&self, character: &Character) -> Vec<ValidationError>;
    fn stat_template(&self) -> StatTemplate;
}

pub struct GenerationEngine {
    systems: HashMap<String, Box<dyn SystemGenerator>>,
    llm_client: Arc<LLMClient>,
}

// System implementations
pub struct DnD5eGenerator;
pub struct Pathfinder2eGenerator;
pub struct DungeonWorldGenerator;
pub struct GURPSGenerator;
pub struct WarhammerGenerator;
pub struct ShadowrunGenerator;
pub struct CyberpunkGenerator;
pub struct CustomGenerator { template: SystemTemplate };

impl GenerationEngine {
    /// Generate character with AI backstory
    pub async fn generate_with_backstory(
        &self,
        system: &str,
        options: GenerationOptions,
    ) -> Result<Character> {
        let character = self.generate(system, &options)?;
        let backstory = self.generate_backstory(&character).await?;
        Ok(character.with_backstory(backstory))
    }

    /// Generate NPC appropriate for role
    pub async fn generate_npc(
        &self,
        role: NPCRole,
        options: NPCOptions,
    ) -> Result<NPC>;

    /// Generate location with features
    pub async fn generate_location(
        &self,
        location_type: LocationType,
        options: LocationOptions,
    ) -> Result<Location>;
}
```

**Satisfies:** REQ-CHAR-001, REQ-CHAR-003, REQ-CHAR-005

---

## 3. Data Models

### 3.1 Database Schema

**Storage:** SQLite for structured data, Meilisearch for search

```sql
-- Campaigns
CREATE TABLE campaigns (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    system TEXT NOT NULL,
    description TEXT,
    setting TEXT,
    current_in_game_date TEXT,
    house_rules TEXT,  -- JSON
    world_state TEXT,  -- JSON
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);

-- Campaign Versions
CREATE TABLE campaign_versions (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL REFERENCES campaigns(id),
    version_number INTEGER NOT NULL,
    snapshot_type TEXT NOT NULL,
    description TEXT,
    data TEXT NOT NULL,  -- JSON snapshot
    diff_data TEXT,      -- JSON diff
    created_at TEXT NOT NULL
);

-- Characters (PCs)
CREATE TABLE characters (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL REFERENCES campaigns(id),
    name TEXT NOT NULL,
    system TEXT NOT NULL,
    class TEXT,
    level INTEGER,
    race TEXT,
    background TEXT,
    stats TEXT NOT NULL,  -- JSON
    equipment TEXT,       -- JSON
    backstory TEXT,
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- NPCs
CREATE TABLE npcs (
    id TEXT PRIMARY KEY,
    campaign_id TEXT REFERENCES campaigns(id),
    name TEXT NOT NULL,
    role TEXT,
    location_id TEXT REFERENCES locations(id) ON DELETE SET NULL,
    description TEXT,
    personality TEXT,
    motivations TEXT,
    stats TEXT,          -- JSON
    voice_profile_id TEXT REFERENCES voice_profiles(id) ON DELETE SET NULL,
    quest_hooks TEXT,    -- JSON
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Entity Relationships (many-to-many between campaign entities)
CREATE TABLE entity_relationships (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL REFERENCES campaigns(id),
    source_entity_type TEXT NOT NULL,  -- 'npc', 'character', 'location', 'quest'
    source_entity_id TEXT NOT NULL,
    target_entity_type TEXT NOT NULL,
    target_entity_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL,   -- 'ally', 'enemy', 'family', 'employee', 'located_at', etc.
    description TEXT,
    strength REAL DEFAULT 1.0,         -- 0.0 to 1.0, relationship strength
    bidirectional INTEGER DEFAULT 0,   -- 1 if relationship applies both ways
    metadata TEXT,                     -- JSON for additional properties
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(source_entity_type, source_entity_id, target_entity_type, target_entity_id, relationship_type)
);

-- Locations
CREATE TABLE locations (
    id TEXT PRIMARY KEY,
    campaign_id TEXT REFERENCES campaigns(id),
    name TEXT NOT NULL,
    location_type TEXT,
    description TEXT,
    notable_features TEXT,  -- JSON array
    connected_locations TEXT,  -- JSON array of IDs
    npcs TEXT,              -- JSON array of IDs
    secrets TEXT,           -- JSON
    map_reference TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Sessions
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL REFERENCES campaigns(id),
    session_number INTEGER NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    duration_mins INTEGER,
    summary TEXT
);

-- Session Events (Timeline)
CREATE TABLE session_events (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    description TEXT,
    entities TEXT,   -- JSON array
    metadata TEXT    -- JSON
);

-- Combat State
CREATE TABLE combat_states (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    round INTEGER NOT NULL DEFAULT 1,
    current_turn INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    combatants TEXT NOT NULL,  -- JSON array
    created_at TEXT NOT NULL
);

-- Voice Profiles
CREATE TABLE voice_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider TEXT NOT NULL,
    voice_id TEXT NOT NULL,
    settings TEXT,      -- JSON
    age_range TEXT,
    gender TEXT,
    personality_traits TEXT,  -- JSON array
    is_preset INTEGER DEFAULT 0,
    created_at TEXT NOT NULL
);

-- Session Notes
CREATE TABLE session_notes (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    campaign_id TEXT NOT NULL REFERENCES campaigns(id),
    content TEXT NOT NULL,
    tags TEXT,          -- JSON array
    entity_links TEXT,  -- JSON array of {type, id}
    created_at TEXT NOT NULL
);

-- Indexes
CREATE INDEX idx_characters_campaign ON characters(campaign_id);
CREATE INDEX idx_npcs_campaign ON npcs(campaign_id);
CREATE INDEX idx_npcs_location ON npcs(location_id);
CREATE INDEX idx_locations_campaign ON locations(campaign_id);
CREATE INDEX idx_sessions_campaign ON sessions(campaign_id);
CREATE INDEX idx_events_session ON session_events(session_id);
CREATE INDEX idx_notes_session ON session_notes(session_id);
CREATE INDEX idx_versions_campaign ON campaign_versions(campaign_id);
CREATE INDEX idx_relationships_campaign ON entity_relationships(campaign_id);
CREATE INDEX idx_relationships_source ON entity_relationships(source_entity_type, source_entity_id);
CREATE INDEX idx_relationships_target ON entity_relationships(target_entity_type, target_entity_id);
CREATE INDEX idx_relationships_type ON entity_relationships(relationship_type);
```

---

## 4. API Design

### 4.1 New Tauri Commands

```rust
// Campaign Versioning
#[tauri::command]
pub async fn create_campaign_version(
    campaign_id: String,
    description: String,
    state: State<'_, AppState>,
) -> Result<String, String>;

#[tauri::command]
pub fn list_campaign_versions(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<CampaignVersion>, String>;

#[tauri::command]
pub fn compare_versions(
    campaign_id: String,
    version1_id: String,
    version2_id: String,
    state: State<'_, AppState>,
) -> Result<CampaignDiff, String>;

#[tauri::command]
pub async fn rollback_campaign(
    campaign_id: String,
    version_id: String,
    state: State<'_, AppState>,
) -> Result<(), String>;

// Voice Profiles
#[tauri::command]
pub fn create_voice_profile(
    profile: VoiceProfile,
    state: State<'_, AppState>,
) -> Result<String, String>;

#[tauri::command]
pub fn list_voice_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<VoiceProfile>, String>;

#[tauri::command]
pub fn link_voice_to_npc(
    profile_id: String,
    npc_id: String,
    state: State<'_, AppState>,
) -> Result<(), String>;

// Session Timeline
#[tauri::command]
pub fn add_timeline_event(
    session_id: String,
    event: TimelineEvent,
    state: State<'_, AppState>,
) -> Result<String, String>;

#[tauri::command]
pub fn get_session_timeline(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<TimelineEvent>, String>;

// Hybrid Search
#[tauri::command]
pub async fn hybrid_search(
    query: String,
    options: SearchOptions,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, String>;

// Character Generation
#[tauri::command]
pub fn list_supported_systems() -> Vec<SystemInfo>;

#[tauri::command]
pub async fn generate_character_with_backstory(
    system: String,
    options: GenerationOptions,
    state: State<'_, AppState>,
) -> Result<Character, String>;

#[tauri::command]
pub async fn generate_location(
    options: LocationOptions,
    state: State<'_, AppState>,
) -> Result<Location, String>;
```

---

## 5. Frontend Components

### 5.1 New Components Needed

```
frontend/src/components/
├── campaign/
│   ├── campaign_dashboard.rs      # REQ-UI-002
│   ├── version_history.rs         # REQ-CAMP-002
│   ├── world_state_editor.rs      # REQ-CAMP-003
│   └── entity_browser.rs          # Characters, NPCs, Locations
├── session/
│   ├── combat_tracker.rs          # REQ-UI-005
│   ├── timeline_view.rs           # REQ-SESS-005
│   └── notes_panel.rs             # REQ-SESS-004
├── library/
│   ├── document_browser.rs        # REQ-UI-004
│   ├── search_panel.rs            # REQ-SEARCH-001
│   └── source_manager.rs          # Document management
├── generation/
│   ├── character_generator.rs     # REQ-CHAR-001
│   ├── npc_generator.rs           # REQ-CHAR-002
│   └── location_generator.rs      # REQ-CHAR-005
├── voice/
│   ├── profile_manager.rs         # REQ-VOICE-002
│   ├── playback_controls.rs       # REQ-VOICE-005
│   └── synthesis_queue.rs         # REQ-VOICE-004
└── analytics/
    ├── usage_dashboard.rs         # REQ-LLM-005
    └── cost_tracker.rs            # Cost visualization
```

---

## 6. Migration Strategy

### Phase 1: Core Infrastructure (P0)
1. SQLite database setup with migrations
2. LLM Router with multi-provider support
3. Basic voice profile system
4. Streaming response UI

### Phase 2: Campaign Enhancement (P1)
1. Campaign versioning backend
2. World state tracking
3. Entity relationships
4. Version comparison UI

### Phase 3: Search & RAG (P1)
1. Embedding provider integration
2. Hybrid search implementation
3. Query expansion
4. RAG context selection

### Phase 4: Session Features (P2)
1. Session timeline
2. Advanced condition system
3. Combat tracker UI
4. Session notes with AI

### Phase 5: Generation & Personality (P2)
1. Multi-system character generation
2. AI backstory generation
3. Location generation
4. Personality application layer

### Phase 6: Polish & Analytics (P3)
1. Usage tracking
2. Cost optimization
3. Security audit logging
4. Performance monitoring

---

## 7. Testing Strategy

### Unit Tests
- Each module should have >70% coverage
- Mock external APIs for deterministic tests
- Property-based testing for generators

### Integration Tests
- End-to-end command testing
- Database migration testing
- Provider failover testing

### UI Tests
- Component rendering tests
- User flow tests
- Accessibility testing

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.1.0 | 2025-12-29 | Add entity_relationships table (replaces JSON in npcs), add EntityRelationship module design |
| 1.0.0 | 2025-12-29 | Initial design document |
