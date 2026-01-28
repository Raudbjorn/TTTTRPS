# Design: Campaign Generation & Management Overhaul

## Overview

This design translates the campaign generation requirements into a technical architecture that integrates with the existing TTTRPS codebase.

### Core Value Proposition

> **Help a GM go from idea → playable campaign faster than they could alone, with traceable sources and consistent lore.**

Every architectural decision flows from this.

### Design Principles

1. **Single Intelligence Loop** - All generation flows through one pipeline, not parallel subsystems
2. **CampaignManager Owns Truth** - All canonical campaign state lives in one place; everything else is advisory
3. **Progressive Commitment** - Content moves through stages (Draft → Approved → Canonical) before becoming "real"
4. **Trust is Explicit** - Generated content carries trust levels (Canonical, Derived, Creative, Unverified)
5. **Intent Anchors Generation** - A stable `CampaignIntent` prevents tone drift across NPCs, arcs, sessions

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Spine architecture (CIP) | Single pipeline reduces cognitive load; all features are pipeline stages |
| CampaignIntent as anchor | Unifies tone across all generation; simplifies prompt engineering |
| Trust levels on content | GMs instantly understand reliability; enables filtering by confidence |
| CanonStatus lifecycle | Prevents premature commitment; supports retconning and iteration |
| Wizard + Chat share PartialCampaign | Two entry points, one data model; users choose preferred UX |
| Streaming for all AI responses | Perceived performance; enables early cancellation |

---

## Architecture

### Campaign Intelligence Pipeline (CIP)

The **Campaign Intelligence Pipeline** is the architectural spine. All features plug into stages of this pipeline rather than existing as peer subsystems.

```
┌──────────────────────────────────────────────────────────────────────────┐
│                    CAMPAIGN INTELLIGENCE PIPELINE                         │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   ┌─────────────┐                                                        │
│   │   INPUT     │  Wizard Steps, Chat Messages, Direct Edits             │
│   └──────┬──────┘                                                        │
│          │                                                               │
│          ▼                                                               │
│   ┌─────────────────────┐                                                │
│   │  CONTEXT ASSEMBLY   │  Campaign + CampaignIntent + Lore + Rules      │
│   │                     │  + Preferences + Session History               │
│   └──────────┬──────────┘                                                │
│              │                                                           │
│              ▼                                                           │
│   ┌─────────────────────┐                                                │
│   │  GENERATION ENGINE  │  LLM + Templates + Streaming                   │
│   └──────────┬──────────┘                                                │
│              │                                                           │
│              ▼                                                           │
│   ┌─────────────────────┐                                                │
│   │   NORMALIZATION     │  Structuring, Validation, Trust Assignment,    │
│   │                     │  Citation Linking, Consistency Check           │
│   └──────────┬──────────┘                                                │
│              │                                                           │
│              ▼                                                           │
│   ┌─────────────────────┐                                                │
│   │  ACCEPTANCE LAYER   │  Preview → Edit → Approve → Apply              │
│   │                     │  (CanonStatus: Draft → Approved → Canonical)   │
│   └──────────┬──────────┘                                                │
│              │                                                           │
│              ▼                                                           │
│   ┌─────────────────────┐                                                │
│   │     ARTIFACTS       │  NPCs, Arcs, Sessions, Locations, Tables,      │
│   │                     │  Recaps, Plot Points                           │
│   └─────────────────────┘                                                │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### Module Mapping to Pipeline

Existing and new modules map onto the pipeline stages:

| Pipeline Stage | Module(s) | Purpose |
|----------------|-----------|---------|
| **Input** | `WizardManager`, `ConversationManager` | Capture user input via structured steps or free-form chat |
| **Context Assembly** | `RulebookLinker`, `FlavourSearcher`, `CampaignIntent` | Build rich context from indexed sources + campaign state |
| **Generation Engine** | `GenerationOrchestrator`, `LLMRouter`, `TemplateRegistry` | Execute LLM calls with streaming |
| **Normalization** | `CitationBuilder`, `TrustAssigner`, `Validator` | Structure output, assign trust, link citations |
| **Acceptance Layer** | `AcceptanceManager` (new) | Preview/edit/approve flow with CanonStatus transitions |
| **Artifacts** | `CampaignManager`, `SessionManager`, `PartyBalancer` | Final storage of canonical campaign entities |

### System Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              FRONTEND (Leptos WASM)                          │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────────────┐   │
│  │  Guided Creation │  │  Conversation    │  │  Generation Preview      │   │
│  │  Flow (wizard)   │  │  Panel           │  │  + Acceptance UI         │   │
│  └────────┬─────────┘  └────────┬─────────┘  └────────────┬─────────────┘   │
│           │                     │                         │                 │
│           └─────────────────────┼─────────────────────────┘                 │
│                                 │                                           │
│  ┌──────────────────────────────┴────────────────────────────────────────┐   │
│  │                    PartialCampaign (shared draft state)                │   │
│  │           Both wizard and chat converge on the same model              │   │
│  └────────────────────────────────┬──────────────────────────────────────┘   │
└───────────────────────────────────┼──────────────────────────────────────────┘
                                    │ Tauri IPC
┌───────────────────────────────────┼──────────────────────────────────────────┐
│                              BACKEND (Rust)                                  │
├───────────────────────────────────┼──────────────────────────────────────────┤
│  ┌────────────────────────────────┴──────────────────────────────────────┐   │
│  │                         Campaign Commands                              │   │
│  │     /commands/campaign/pipeline.rs, wizard.rs, conversation.rs        │   │
│  └────────────────────────────────┬──────────────────────────────────────┘   │
│                                   │                                         │
│  ┌────────────────────────────────┴──────────────────────────────────────┐   │
│  │                    Campaign Intelligence Pipeline                      │   │
│  │  ┌─────────┐ ┌─────────────┐ ┌────────────┐ ┌───────────┐ ┌─────────┐ │   │
│  │  │  Input  │→│  Context    │→│ Generation │→│ Normalize │→│ Accept  │ │   │
│  │  │         │ │  Assembly   │ │  Engine    │ │           │ │         │ │   │
│  │  └─────────┘ └─────────────┘ └────────────┘ └───────────┘ └─────────┘ │   │
│  └────────────────────────────────┬──────────────────────────────────────┘   │
│                                   │                                         │
│  ┌────────────────────────────────┴──────────────────────────────────────┐   │
│  │                      Supporting Services                               │   │
│  │  ┌───────────┐ ┌───────────────┐ ┌──────────────┐ ┌────────────────┐  │   │
│  │  │ LLMRouter │ │ SearchClient  │ │ TemplateReg  │ │ TrustAssigner  │  │   │
│  │  └───────────┘ └───────────────┘ └──────────────┘ └────────────────┘  │   │
│  └────────────────────────────────┬──────────────────────────────────────┘   │
│                                   │                                         │
│  ┌────────────────────────────────┴──────────────────────────────────────┐   │
│  │              CampaignManager (SINGLE SOURCE OF TRUTH)                  │   │
│  │       All canonical campaign state lives here. Period.                 │   │
│  └────────────────────────────────┬──────────────────────────────────────┘   │
│                                   │                                         │
│  ┌────────────────────────────────┴──────────────────────────────────────┐   │
│  │                         SQLite Database                                │   │
│  │   campaigns, campaign_intents, sessions, wizard_states, conversations,│   │
│  │   source_citations, generation_drafts, canon_status_log               │   │
│  └───────────────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Architectural Invariants

These invariants prevent state fragmentation as the system scales:

1. **All campaign truth lives in CampaignManager.** Everything else is advisory.
2. **Conversation suggestions never mutate state directly.** They produce drafts.
3. **Generation results must pass through Acceptance.** No auto-commit.
4. **Wizard only edits PartialCampaign.** Never touches canonical campaign.
5. **CampaignIntent is immutable once campaign starts.** Edit requires explicit migration.

### Technology Stack

| Layer | Technology | Rationale |
|-------|------------|-----------|
| Pipeline Orchestration | Rust async + channels | Type-safe stage composition |
| Frontend State | Leptos signals + context | Existing pattern; reactive updates |
| Draft State | SQLite table | Persistence across restarts |
| AI Integration | Existing LLMRouter | Multi-provider support already built |
| Content Search | Meilisearch | Existing semantic + BM25 hybrid search |
| Streaming | Tauri events | Low-latency bidirectional streaming |
| Templates | YAML/JSON | User-editable without recompilation |

---

## Core Concepts

### CampaignIntent

**Purpose:** Stable anchor that unifies tone across all generation. Prevents drift.

```rust
/// The creative vision for a campaign - set once, referenced everywhere.
/// Immutable after campaign creation; changes require explicit migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignIntent {
    /// Core fantasy: "grim political thriller", "heroic dungeon crawl", "cosmic horror"
    pub fantasy: String,

    /// Desired player experiences: "mystery", "power fantasy", "tragedy", "discovery"
    pub player_experiences: Vec<String>,

    /// Hard constraints: "low magic", "urban only", "PG-13", "no character death"
    pub constraints: Vec<String>,

    /// Themes to weave through: "corruption of power", "found family", "redemption"
    pub themes: Vec<String>,

    /// Tone keywords: "dark", "humorous", "epic", "intimate", "gritty"
    pub tone_keywords: Vec<String>,

    /// What to avoid: "graphic violence", "romantic subplots", "real-world politics"
    pub avoid: Vec<String>,
}
```

**Usage:** Every generation request includes CampaignIntent in context assembly. Prompts reference it to maintain consistency.

### Trust Levels

**Purpose:** GMs instantly understand how reliable generated content is.

```rust
/// How much to trust a piece of generated content.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrustLevel {
    /// Directly from indexed rulebooks/sourcebooks (e.g., spell stats, monster CR)
    Canonical,

    /// Logically derived from rules/lore (e.g., "a cleric would likely...")
    Derived,

    /// Pure AI invention with no source backing
    Creative,

    /// Generation attempted to cite source but couldn't verify
    Unverified,
}

impl TrustLevel {
    /// Returns true if this content can be used without GM review
    pub fn is_reliable(&self) -> bool {
        matches!(self, TrustLevel::Canonical | TrustLevel::Derived)
    }
}
```

**UI Integration:**
- Canonical: No indicator (assumed reliable)
- Derived: Subtle "derived" badge, expandable to show reasoning
- Creative: "AI-generated" badge, always shown
- Unverified: Warning indicator, suggests manual verification

### CanonStatus (Progressive Commitment)

**Purpose:** Content moves through stages before becoming "real" campaign data.

```rust
/// Lifecycle status of generated content.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CanonStatus {
    /// Initial generation, not yet reviewed
    Draft,

    /// GM has reviewed and approved, but not yet used in play
    Approved,

    /// Used in a session - now part of campaign history
    Canonical,

    /// Retconned or replaced - kept for history but not active
    Deprecated,
}

impl CanonStatus {
    /// Can this content be freely edited?
    pub fn is_editable(&self) -> bool {
        matches!(self, CanonStatus::Draft | CanonStatus::Approved)
    }

    /// Is this content "locked" by play history?
    pub fn is_locked(&self) -> bool {
        matches!(self, CanonStatus::Canonical | CanonStatus::Deprecated)
    }
}
```

**Transitions:**
- `Draft → Approved`: GM clicks "Accept" in preview
- `Approved → Canonical`: Entity used in a session (auto-promotion)
- `Canonical → Deprecated`: Explicit retcon action
- `Approved → Draft`: GM decides to re-edit

### Guided Creation Flow

**Purpose:** Unified abstraction for campaign creation that supports both structured (wizard) and free-form (chat) input.

The frontend offers two entry points:
1. **Wizard Mode**: Step-by-step forms with AI suggestions
2. **Conversation Mode**: Free-form chat that populates the same fields

Both modes update the same `PartialCampaign` draft. Users can switch between modes freely.

```rust
/// Shared draft state for campaign creation.
/// Both wizard steps and conversation suggestions update this.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartialCampaign {
    pub name: Option<String>,
    pub system: Option<String>,
    pub description: Option<String>,
    pub intent: Option<CampaignIntent>,  // Core creative vision
    pub session_scope: Option<SessionScope>,
    pub player_count: Option<u8>,
    pub party_composition: Option<PartyComposition>,
    pub arc_structure: Option<ArcStructure>,
    pub initial_npcs: Vec<EntityDraft<NpcData>>,
    pub initial_locations: Vec<EntityDraft<LocationData>>,
    pub initial_plot_points: Vec<EntityDraft<PlotPointData>>,
}

/// Wrapper that tracks draft status and trust for any entity type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDraft<T> {
    pub id: Uuid,
    pub data: T,
    pub status: CanonStatus,
    pub trust: TrustLevel,
    pub citations: Vec<Citation>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Patch and Proposal Model

**Purpose:** All AI output is structured as patches (field updates) and proposals (patches + rationale + citations). This enforces "AI proposes, GM decides."

```rust
/// A single field update to the draft.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    pub field: String,              // e.g. "campaign.tone", "npcs[0].motivation"
    pub value: serde_json::Value,   // new value
}

/// A collection of patches to apply atomically.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PatchSet {
    pub patches: Vec<Patch>,
}

/// A structured suggestion from the AI with traceability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: Uuid,
    pub patches: PatchSet,          // "apply to draft" payload
    pub rationale: String,          // why this was suggested
    pub citations: Vec<Citation>,   // source backing
    pub trust: TrustLevel,          // reliability indicator
}

/// GM's decision on a proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Decision {
    Accepted { proposal_id: Uuid, applied: PatchSet },
    Rejected { proposal_id: Uuid, reason: Option<String> },
    Modified { proposal_id: Uuid, applied: PatchSet },  // GM edited before accepting
}

/// Compact summary of past decisions for context assembly.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecisionSummary {
    pub accepted_count: usize,
    pub rejected_count: usize,
    pub rejected_topics: Vec<String>,  // "don't suggest X again"
    pub last_decisions: Vec<Decision>, // recent decisions for context
}

/// Output bundle from any generation operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactBundle {
    pub proposals: Vec<Proposal>,   // patch-style suggestions
    pub artifacts: Vec<Artifact>,   // NPCs, arcs, tables, etc. (draft artifacts)
    pub citations: Vec<Citation>,   // convenience aggregation
}

/// Generic artifact wrapper for generated content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: Uuid,
    pub kind: ArtifactKind,
    pub data: serde_json::Value,    // type-specific payload
    pub trust: TrustLevel,
    pub citations: Vec<Citation>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArtifactKind {
    Npc,
    Location,
    PlotPoint,
    Arc,
    Session,
    PartyComposition,
    RandomTable,
    Recap,
    CheatSheet,
}
```

### Draft Snapshot

**Purpose:** Immutable snapshot of draft state for passing to generators. Generators never mutate the draft directly.

```rust
/// Immutable snapshot for generation context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftSnapshot {
    pub draft_id: Uuid,
    pub partial_campaign: PartialCampaign,
    pub current_step: WizardStep,
    pub decisions: DecisionSummary,  // distilled accept/reject history
}
```

---

## Trait Boundaries

The system enforces architectural invariants through trait boundaries. Each trait defines a clear responsibility and prevents unauthorized state mutation.

### Design Principles (Enforced by Traits)

1. **Single Draft Truth**: Only `DraftStore` can mutate draft state during creation
2. **AI as Advisor**: Generators produce `ArtifactBundle`, never mutate state directly
3. **Explicit Commitment**: Only `CampaignWriter` can create canonical campaign entities
4. **Grounded Creativity**: All generation flows through `Grounder` for citation attachment

### Dependency Rules

```
Domain types     →  depend on nothing
Traits           →  depend on domain types only
Implementations  →  depend on traits + external crates (sqlx, meilisearch, etc.)
CreationFlow     →  depends on traits only (generic over implementations)
Tauri commands   →  depend on CreationFlow
```

**Hard rule:** Generators and Grounders never receive a `CampaignWriter`. Only the application service (`CreationFlow`) holds writers.

### DraftStore (implements: WizardManager)

The only trait allowed to mutate draft state.

```rust
#[async_trait]
pub trait DraftStore: Send + Sync {
    async fn create_draft(&self) -> Result<DraftSnapshot>;
    async fn load_draft(&self, id: Uuid) -> Result<Option<DraftSnapshot>>;

    async fn apply_patches(&self, id: Uuid, patches: PatchSet) -> Result<DraftSnapshot>;
    async fn set_step(&self, id: Uuid, step: WizardStep) -> Result<DraftSnapshot>;

    async fn autosave_hint(&self, id: Uuid) -> Result<()>;
    async fn list_incomplete(&self) -> Result<Vec<DraftSnapshot>>;
    async fn delete_draft(&self, id: Uuid) -> Result<()>;
}
```

### DraftValidator

Validates draft state at step transitions and completion.

```rust
pub trait DraftValidator: Send + Sync {
    fn validate_step(&self, snapshot: &DraftSnapshot) -> Result<()>;
    fn validate_completion(&self, snapshot: &DraftSnapshot) -> Result<()>;
}
```

### ConversationStore (implements: ConversationManager)

The decision ledger - stores conversation history and GM decisions.

```rust
#[async_trait]
pub trait ConversationStore: Send + Sync {
    async fn create_thread(&self, draft_id: Uuid, purpose: ConversationPurpose)
        -> Result<Uuid>;

    async fn append_user_message(&self, thread: Uuid, content: String)
        -> Result<Uuid>;

    async fn append_assistant_message(
        &self,
        thread: Uuid,
        content: String,
        proposals: Vec<Proposal>,
    ) -> Result<Uuid>;

    async fn record_decision(&self, thread: Uuid, decision: Decision)
        -> Result<()>;

    async fn summarize_decisions(&self, thread: Uuid)
        -> Result<DecisionSummary>;

    async fn branch_from(&self, thread: Uuid, message_id: Uuid)
        -> Result<Uuid>;
}
```

### KnowledgeIndex (implements: SearchClient/Meilisearch)

Facade for indexed rulebook and lore search.

```rust
#[async_trait]
pub trait KnowledgeIndex: Send + Sync {
    async fn hybrid_search(&self, query: &str, filters: SearchFilters)
        -> Result<Vec<IndexedSnippet>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedSnippet {
    pub id: String,
    pub content: String,
    pub source_name: String,
    pub location: SourceLocation,
    pub relevance_score: f32,
}
```

### Grounder (implements: RulebookLinker + FlavourSearcher)

Builds grounded context with citations for generation.

```rust
#[async_trait]
pub trait Grounder: Send + Sync {
    async fn ground(&self, request: &GroundingRequest) -> Result<GroundingPack>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingRequest {
    pub query: String,
    pub system: String,
    pub campaign_id: Option<Uuid>,
    pub filters: SearchFilters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingPack {
    pub snippets: Vec<IndexedSnippet>,
    pub citations: Vec<Citation>,
}
```

### ReferenceResolver (implements: Recursive Stat Block Expansion)

Resolves inline references (spells, traits, etc.) in generated text.

```rust
#[async_trait]
pub trait ReferenceResolver: Send + Sync {
    async fn resolve_inline(&self, text: &str, system: &str)
        -> Result<(String, Vec<Citation>)>;
}
```

### LlmClient (implements: LLMRouter)

Dumb streaming adapter for LLM providers.

```rust
use futures_core::Stream;

pub type TokenStream = Pin<Box<dyn Stream<Item = Result<String>> + Send>>;

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn stream_completion(&self, prompt: String) -> Result<TokenStream>;
}
```

### PromptRenderer (implements: TemplateRegistry)

Renders prompts from templates.

```rust
pub trait PromptRenderer: Send + Sync {
    fn render(&self, template_id: &str, ctx: &serde_json::Value) -> Result<String>;
}
```

### Generator (implements: GenerationOrchestrator)

The pipeline orchestrator. Returns streaming tokens + structured finalization.

```rust
#[async_trait]
pub trait Generator: Send + Sync {
    async fn generate(&self, req: GenerationRequest) -> Result<GenerationStream>;
}

pub struct GenerationStream {
    pub tokens: TokenStream,                        // streaming text for UX
    pub finalize: Box<dyn Finalizer + Send + Sync>, // parse/structure after stream ends
}

#[async_trait]
pub trait Finalizer: Send + Sync {
    async fn finalize(self: Box<Self>) -> Result<ArtifactBundle>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub purpose: GenerationPurpose,
    pub snapshot: DraftSnapshot,
    pub grounding: GroundingPack,
    pub template_id: String,
    pub options: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GenerationPurpose {
    CampaignCreation,
    CharacterBackground,
    NpcGeneration,
    SessionPlanning,
    ArcOutline,
    PartyComposition,
    Recap,
}
```

### ArtifactGenerator (implements: PartyBalancer, RecapGenerator, etc.)

Specialized artifact generators share this interface.

```rust
#[async_trait]
pub trait ArtifactGenerator: Send + Sync {
    fn kind(&self) -> ArtifactKind;

    async fn generate(
        &self,
        snapshot: &DraftSnapshot,
        opts: serde_json::Value,
    ) -> Result<ArtifactBundle>;
}
```

### CampaignWriter (implements: CampaignManager)

The **only** trait that can create canonical campaign entities.

```rust
#[async_trait]
pub trait CampaignWriter: Send + Sync {
    async fn create_campaign_from_draft(&self, snapshot: DraftSnapshot)
        -> Result<Uuid>;

    async fn apply_canonical_change(&self, campaign: Uuid, change: CanonicalChange)
        -> Result<()>;

    async fn snapshot(&self, campaign: Uuid, note: String)
        -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CanonicalChange {
    AddEntity { kind: ArtifactKind, data: serde_json::Value },
    UpdateEntity { id: Uuid, patches: PatchSet },
    DeprecateEntity { id: Uuid, reason: String },
}
```

### SessionWriter (implements: SessionManager)

Session lifecycle management.

```rust
#[async_trait]
pub trait SessionWriter: Send + Sync {
    async fn create_session_plan(&self, campaign: Uuid, plan: SessionPlan)
        -> Result<Uuid>;

    async fn complete_session(&self, session_id: Uuid, summary: SessionSummary)
        -> Result<()>;
}
```

### CreationFlow (Application Service)

The glue that composes all traits. Tauri commands call this.

```rust
pub struct CreationFlow<S, C, G, Gen, CW>
where
    S: DraftStore,
    C: ConversationStore,
    G: Grounder,
    Gen: Generator,
    CW: CampaignWriter,
{
    pub drafts: S,
    pub convo: C,
    pub grounder: G,
    pub gen: Gen,
    pub campaigns: CW,
}

impl<S, C, G, Gen, CW> CreationFlow<S, C, G, Gen, CW>
where
    S: DraftStore,
    C: ConversationStore,
    G: Grounder,
    Gen: Generator,
    CW: CampaignWriter,
{
    /// Start a new campaign creation draft
    pub async fn start_draft(&self) -> Result<DraftSnapshot>;

    /// Process a chat message: ground → generate → return proposals
    pub async fn process_message(
        &self,
        draft_id: Uuid,
        thread_id: Uuid,
        message: String,
    ) -> Result<GenerationStream>;

    /// Accept a proposal: apply patches to draft
    pub async fn accept_proposal(
        &self,
        draft_id: Uuid,
        thread_id: Uuid,
        proposal_id: Uuid,
    ) -> Result<DraftSnapshot>;

    /// Reject a proposal: record decision, update context
    pub async fn reject_proposal(
        &self,
        draft_id: Uuid,
        thread_id: Uuid,
        proposal_id: Uuid,
        reason: Option<String>,
    ) -> Result<()>;

    /// Complete the draft: validate → create canonical campaign
    pub async fn complete_draft(&self, draft_id: Uuid) -> Result<Uuid>;
}
```

### Trait → Module Mapping

| Trait | Implementing Module |
|-------|---------------------|
| `DraftStore` + `DraftValidator` | WizardManager |
| `ConversationStore` | ConversationManager |
| `KnowledgeIndex` | SearchClient (Meilisearch) |
| `Grounder` | RulebookLinker + FlavourSearcher (composed) |
| `ReferenceResolver` | StatBlockExpander |
| `LlmClient` | LLMRouter |
| `PromptRenderer` | TemplateRegistry |
| `Generator` | GenerationOrchestrator |
| `ArtifactGenerator` | PartyBalancer, RecapGenerator, RandomTableEngine, etc. |
| `CampaignWriter` | CampaignManager |
| `SessionWriter` | SessionManager |

---

## Components and Interfaces

### WizardManager

**Purpose:** Manages the campaign creation wizard state machine, persisting progress and enabling recovery.

**Responsibilities:**
- Track wizard step progression
- Persist partial campaign data
- Validate step transitions
- Generate wizard state summaries
- Handle draft recovery

**Interface:**
```rust
pub struct WizardManager {
    db: Arc<SqlitePool>,
}

impl WizardManager {
    // Lifecycle
    pub async fn start_wizard(&self, user_id: Option<String>) -> Result<WizardState>;
    pub async fn get_wizard(&self, wizard_id: Uuid) -> Result<Option<WizardState>>;
    pub async fn list_incomplete_wizards(&self) -> Result<Vec<WizardSummary>>;
    pub async fn delete_wizard(&self, wizard_id: Uuid) -> Result<()>;

    // Step management
    pub async fn advance_step(&self, wizard_id: Uuid, step_data: StepData) -> Result<WizardState>;
    pub async fn go_back(&self, wizard_id: Uuid) -> Result<WizardState>;
    pub async fn skip_step(&self, wizard_id: Uuid) -> Result<WizardState>;

    // Completion
    pub async fn complete_wizard(&self, wizard_id: Uuid) -> Result<Campaign>;
    pub async fn cancel_wizard(&self, wizard_id: Uuid, save_draft: bool) -> Result<()>;

    // Auto-save
    pub async fn auto_save(&self, wizard_id: Uuid, partial_data: PartialWizardData) -> Result<()>;
}
```

**Implementation Notes:**
- Wizard state stored in `wizard_states` table with JSON blob for step data
- Auto-save debounced to 30-second intervals via frontend
- Step transitions validated against allowed state machine paths

### GenerationOrchestrator

**Purpose:** Coordinates content generation by combining LLM capabilities with content grounding from indexed sources.

**Responsibilities:**
- Build context-aware prompts using **Personality Profiles**
- Search and inject relevant source material using **Hybrid Search** (Vector + BM25)
- Manage streaming responses
- Parse and structure generated content
- Track generation history

**Interface:**
```rust
pub struct GenerationOrchestrator {
    llm: Arc<LLMRouter>,
    search: Arc<SearchClient>,
    templates: TemplateRegistry,
}

impl GenerationOrchestrator {
    // Context Management
    pub async fn load_personality_profile(&self, system: &str) -> Result<PersonalityProfile>;

    // Character generation
    pub async fn generate_character_background(
        &self,
        request: CharacterBackgroundRequest,
        campaign_context: &CampaignContext,
    ) -> Result<impl Stream<Item = GenerationChunk>>;

    // NPC generation
    pub async fn generate_npc(
        &self,
        request: NpcGenerationRequest,
        campaign_context: &CampaignContext,
    ) -> Result<impl Stream<Item = GenerationChunk>>;

    // Session planning
    pub async fn generate_session_plan(
        &self,
        request: SessionPlanRequest,
        campaign_context: &CampaignContext,
    ) -> Result<impl Stream<Item = GenerationChunk>>;

    // Party suggestions
    pub async fn suggest_party_composition(
        &self,
        request: PartyRequest,
    ) -> Result<Vec<PartySuggestion>>;

    // Recursive Stat Block Expansion
    pub async fn expand_stat_block(
        &self,
        stat_block: &StatBlock,
    ) -> Result<StatBlock>; // Resolves refs to inline text

    // Arc generation
    pub async fn generate_arc_outline(
        &self,
        request: ArcRequest,
        campaign_context: &CampaignContext,
    ) -> Result<impl Stream<Item = GenerationChunk>>;
}
```

**Implementation Notes:**
- Uses template system for prompt construction (YAML files in `resources/templates/`)
- Searches Meilisearch for relevant rulebook/flavour content before generation
- All generation returns streaming for responsive UX
- Results include source citations where applicable

### ConversationManager

**Purpose:** Persists and manages AI conversation threads for campaign creation assistance.

**Responsibilities:**
- Create and manage conversation threads
- Persist messages with metadata
- Retrieve conversation history with pagination
- Support conversation branching (explore alternatives)
- Track which suggestions were accepted/rejected

**Interface:**
```rust
pub struct ConversationManager {
    db: Arc<SqlitePool>,
}

impl ConversationManager {
    // Thread lifecycle
    pub async fn create_thread(
        &self,
        campaign_id: Option<Uuid>,
        purpose: ConversationPurpose,
    ) -> Result<ConversationThread>;

    pub async fn get_thread(&self, thread_id: Uuid) -> Result<Option<ConversationThread>>;
    pub async fn list_threads(&self, campaign_id: Uuid) -> Result<Vec<ThreadSummary>>;
    pub async fn archive_thread(&self, thread_id: Uuid) -> Result<()>;

    // Message management
    pub async fn add_message(
        &self,
        thread_id: Uuid,
        message: ConversationMessage,
    ) -> Result<Uuid>;

    pub async fn get_messages(
        &self,
        thread_id: Uuid,
        limit: usize,
        before: Option<Uuid>,
    ) -> Result<Vec<ConversationMessage>>;

    // Suggestion tracking
    pub async fn mark_suggestion_accepted(&self, message_id: Uuid, field: &str) -> Result<()>;
    pub async fn mark_suggestion_rejected(&self, message_id: Uuid, field: &str) -> Result<()>;

    // Branching
    pub async fn branch_from(&self, message_id: Uuid) -> Result<ConversationThread>;
}
```

**Implementation Notes:**
- Messages stored in `conversation_messages` table with thread_id FK
- Suggestions stored as JSON within message, with acceptance status flags
- Conversation context built from recent messages (configurable window)

### RulebookLinker

**Purpose:** Extracts rulebook references from text and links them to indexed content for citation.

**Responsibilities:**
- Detect rulebook references in text (page numbers, section names)
- Search indexed rulebooks for matching content
- Build citation objects with confidence scores
- Validate that referenced rules still exist
- Track which rulebook content has been used in campaign

**Interface:**
```rust
pub struct RulebookLinker {
    search: Arc<SearchClient>,
}

impl RulebookLinker {
    // Reference detection
    pub async fn find_references(&self, text: &str) -> Result<Vec<RulebookReference>>;

    // Linking
    pub async fn link_to_rulebook(
        &self,
        query: &str,
        rulebook_filter: Option<Vec<String>>,
    ) -> Result<Vec<LinkedContent>>;

    // Citation building
    pub fn build_citation(&self, reference: &RulebookReference) -> Citation;

    // Validation
    pub async fn validate_references(&self, references: &[RulebookReference]) -> ValidationReport;

    // Usage tracking
    pub async fn mark_content_used(&self, campaign_id: Uuid, content_id: &str) -> Result<()>;
    pub async fn get_used_content(&self, campaign_id: Uuid) -> Result<Vec<String>>;
}
```

**Implementation Notes:**
- Uses regex patterns for common citation formats (PHB p.123, DMG Chapter 5, etc.)
- Semantic search for fuzzy matching when exact page not found
- Confidence threshold of 0.7 for automatic linking

### PartyBalancer

**Purpose:** Analyzes player count and game system to suggest balanced party compositions.

**Responsibilities:**
- Load system-specific party balance rules
- Generate composition suggestions
- Identify capability gaps
- Suggest adjustments for unusual party sizes
- Factor in campaign tone/themes

**Interface:**
```rust
pub struct PartyBalancer {
    search: Arc<SearchClient>,
    system_rules: SystemRulesCache,
}

impl PartyBalancer {
    // Analysis
    pub async fn analyze_party(
        &self,
        party: &[CharacterSummary],
        system: &str,
    ) -> PartyAnalysis;

    // Suggestions
    pub async fn suggest_compositions(
        &self,
        player_count: u8,
        system: &str,
        preferences: Option<PartyPreferences>,
    ) -> Result<Vec<PartySuggestion>>;

    // Gap analysis
    pub fn identify_gaps(&self, party: &[CharacterSummary], system: &str) -> Vec<CapabilityGap>;

    // Small party support
    pub async fn suggest_small_party_options(
        &self,
        player_count: u8,
        system: &str,
    ) -> Vec<SmallPartyOption>;
}
```

**Implementation Notes:**
- System rules loaded from indexed rulebooks + fallback defaults
- Composition templates configurable via YAML
- Gaps categorized: healing, tank, damage, utility, social, exploration

### RandomTableEngine

**Purpose:** Create, manage, and roll on probability-weighted random tables with standard dice notation.

**Responsibilities:**
- Parse dice notation (d6, 2d6, d100, d66, etc.)
- Execute dice rolls with proper probability
- Support weighted result ranges
- Handle nested/cascading table references
- Track roll history per session

**Interface:**
```rust
pub struct RandomTableEngine {
    db: Arc<SqlitePool>,
    rng: Arc<Mutex<StdRng>>,
}

impl RandomTableEngine {
    // Table management
    pub async fn create_table(&self, table: RandomTableDefinition) -> Result<RandomTable>;
    pub async fn get_table(&self, table_id: Uuid) -> Result<Option<RandomTable>>;
    pub async fn list_tables(&self, campaign_id: Uuid) -> Result<Vec<RandomTableSummary>>;
    pub async fn update_table(&self, table_id: Uuid, updates: TableUpdate) -> Result<RandomTable>;
    pub async fn delete_table(&self, table_id: Uuid) -> Result<()>;

    // Rolling
    pub fn roll_on_table(&self, table: &RandomTable) -> RollResult;
    pub fn roll_dice(&self, notation: &str) -> Result<DiceRollResult>;
    pub async fn resolve_nested_roll(&self, result: &RollResult) -> Result<ResolvedRollResult>;

    // History
    pub async fn record_roll(&self, session_id: Uuid, roll: &RollResult) -> Result<()>;
    pub async fn get_roll_history(&self, session_id: Uuid) -> Result<Vec<RollResult>>;
}
```

**Implementation Notes:**
- Dice notation parsed via regex: `(\d*)d(\d+)([+-]\d+)?`
- d66 tables rolled as 2d6 read as tens/ones (11-66, skipping 7-9)
- Seeded RNG for reproducibility in testing

### RecapGenerator

**Purpose:** Generate session and arc recaps from timeline events and session notes.

**Responsibilities:**
- Aggregate session events into narrative summaries
- Distinguish player-known vs GM-only information
- Generate read-aloud and bullet-point formats
- Track cliffhangers and unresolved hooks
- Support per-PC knowledge filtering

**Interface:**
```rust
pub struct RecapGenerator {
    llm: Arc<LLMRouter>,
    db: Arc<SqlitePool>,
    templates: TemplateRegistry,
}

impl RecapGenerator {
    // Generation
    pub async fn generate_session_recap(
        &self,
        session_id: Uuid,
        options: RecapOptions,
    ) -> Result<SessionRecap>;

    pub async fn generate_arc_recap(
        &self,
        arc_id: Uuid,
        options: RecapOptions,
    ) -> Result<ArcRecap>;

    pub async fn generate_campaign_summary(
        &self,
        campaign_id: Uuid,
    ) -> Result<CampaignSummary>;

    // Filtering
    pub fn filter_by_pc_knowledge(
        &self,
        recap: &SessionRecap,
        pc_id: Uuid,
    ) -> SessionRecap;

    // Persistence
    pub async fn save_recap(&self, recap: &SessionRecap) -> Result<()>;
    pub async fn get_recap(&self, session_id: Uuid) -> Result<Option<SessionRecap>>;
}
```

**Implementation Notes:**
- Uses timeline events as primary source, supplemented by session notes
- LLM summarization with prompt template for narrative style
- Cliffhanger detection via unresolved plot points at session end

### CheatSheetBuilder

**Purpose:** Assemble single-page GM reference sheets for specific sessions.

**Responsibilities:**
- Aggregate relevant entities for a session
- Prioritize and truncate to fit space constraints
- Generate print-friendly and screen-optimized views
- Remember GM preferences for future sheets

**Interface:**
```rust
pub struct CheatSheetBuilder {
    db: Arc<SqlitePool>,
}

impl CheatSheetBuilder {
    // Building
    pub async fn build_cheat_sheet(
        &self,
        session_plan_id: Uuid,
        options: CheatSheetOptions,
    ) -> Result<CheatSheet>;

    pub async fn build_from_selection(
        &self,
        campaign_id: Uuid,
        selection: CheatSheetSelection,
    ) -> Result<CheatSheet>;

    // Customization
    pub async fn save_preferences(
        &self,
        campaign_id: Uuid,
        preferences: CheatSheetPreferences,
    ) -> Result<()>;

    pub async fn get_preferences(
        &self,
        campaign_id: Uuid,
    ) -> Result<CheatSheetPreferences>;

    // Export
    pub fn render_to_html(&self, sheet: &CheatSheet) -> String;
    pub fn estimate_print_pages(&self, sheet: &CheatSheet) -> u8;
}
```

**Implementation Notes:**
- Content prioritization based on session plan encounters and plot points
- Space estimation based on content type (NPC cards ~100px, locations ~150px)
- Truncation warns user and suggests what was excluded

### QuickReferenceCardManager

**Purpose:** Render entities as compact cards and manage pinned card collections.

**Responsibilities:**
- Render any entity type as a standardized card
- Manage pinned card tray per session
- Support card expansion to full view
- Generate hover previews

**Interface:**
```rust
pub struct QuickReferenceCardManager {
    db: Arc<SqlitePool>,
}

impl QuickReferenceCardManager {
    // Card rendering
    pub fn render_npc_card(&self, npc: &Npc) -> NpcCard;
    pub fn render_location_card(&self, location: &Location) -> LocationCard;
    pub fn render_item_card(&self, item: &Item) -> ItemCard;
    pub fn render_plot_card(&self, plot: &PlotPoint) -> PlotCard;

    // Card tray
    pub async fn pin_card(&self, session_id: Uuid, entity_ref: EntityRef) -> Result<()>;
    pub async fn unpin_card(&self, session_id: Uuid, entity_ref: EntityRef) -> Result<()>;
    pub async fn get_pinned_cards(&self, session_id: Uuid) -> Result<Vec<PinnedCard>>;
    pub async fn reorder_cards(&self, session_id: Uuid, order: Vec<Uuid>) -> Result<()>;

    // Preview
    pub fn generate_preview(&self, entity_ref: &EntityRef) -> CardPreview;
}
```

**Implementation Notes:**
- Cards use fixed-height templates per entity type
- Maximum 6 pinned cards enforced at database level
- Hover preview uses same card template with pointer-events disabled

### AcceptanceManager

**Purpose:** The Acceptance Layer of the CIP - manages the preview/edit/approve flow for all generated content.

**Pipeline Stage:** Acceptance Layer

**Responsibilities:**
- Present generated drafts for GM review
- Track acceptance/rejection/modification decisions
- Manage CanonStatus transitions (Draft → Approved → Canonical)
- Apply approved content to CampaignManager
- Log all acceptance decisions for audit

**Interface:**
```rust
pub struct AcceptanceManager {
    db: Arc<SqlitePool>,
    campaign_manager: Arc<CampaignManager>,
}

impl AcceptanceManager {
    // Draft management
    pub async fn create_draft<T: Serialize>(
        &self,
        entity_type: EntityType,
        data: T,
        trust: TrustLevel,
        citations: Vec<Citation>,
    ) -> Result<EntityDraft<T>>;

    pub async fn get_draft(&self, draft_id: Uuid) -> Result<Option<DraftEnvelope>>;
    pub async fn list_pending_drafts(&self, campaign_id: Uuid) -> Result<Vec<DraftSummary>>;

    // Acceptance flow
    pub async fn approve_draft(&self, draft_id: Uuid) -> Result<CanonStatus>;
    pub async fn reject_draft(&self, draft_id: Uuid, reason: Option<String>) -> Result<()>;
    pub async fn modify_draft<T: Serialize>(
        &self,
        draft_id: Uuid,
        modifications: T,
    ) -> Result<EntityDraft<T>>;

    // Promotion to canonical
    pub async fn apply_to_campaign(
        &self,
        draft_id: Uuid,
        campaign_id: Uuid,
    ) -> Result<Uuid>; // Returns canonical entity ID

    // Bulk operations
    pub async fn approve_all(&self, draft_ids: Vec<Uuid>) -> Result<Vec<CanonStatus>>;

    // Audit
    pub async fn get_acceptance_history(
        &self,
        campaign_id: Uuid,
        limit: usize,
    ) -> Result<Vec<AcceptanceEvent>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceEvent {
    pub draft_id: Uuid,
    pub entity_type: EntityType,
    pub decision: AcceptanceDecision,
    pub previous_status: CanonStatus,
    pub new_status: CanonStatus,
    pub modifications: Option<serde_json::Value>,
    pub reason: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AcceptanceDecision {
    Approved,
    Rejected,
    Modified,
    AppliedToCampaign,
}
```

**Implementation Notes:**
- All acceptance decisions are logged for audit trail
- `apply_to_campaign` is the only path to canonical campaign state
- Drafts expire after 30 days if not acted upon
- Batch approval available for initial campaign creation (review-all-then-accept flow)

### TrustAssigner

**Purpose:** Normalization stage component that assigns trust levels to generated content based on citation analysis.

**Pipeline Stage:** Normalization

**Responsibilities:**
- Analyze citations attached to generated content
- Assign appropriate TrustLevel based on source quality
- Detect unsupported claims that need verification
- Aggregate trust metrics for generation quality tracking

**Interface:**
```rust
pub struct TrustAssigner {
    search: Arc<SearchClient>,
    thresholds: TrustThresholds,
}

impl TrustAssigner {
    // Core assignment
    pub fn assign_trust(
        &self,
        content: &str,
        citations: &[Citation],
    ) -> TrustAssignment;

    // Detailed analysis
    pub async fn analyze_claims(
        &self,
        content: &str,
        campaign_context: &CampaignContext,
    ) -> Vec<ClaimAnalysis>;

    // Verification
    pub async fn verify_citation(
        &self,
        citation: &Citation,
    ) -> VerificationResult;

    // Metrics
    pub fn calculate_trust_score(&self, assignment: &TrustAssignment) -> f32;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustAssignment {
    pub overall_trust: TrustLevel,
    pub claim_breakdown: Vec<ClaimTrust>,
    pub unverified_claims: Vec<String>,
    pub confidence: f32,  // 0.0-1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimTrust {
    pub claim: String,
    pub trust: TrustLevel,
    pub supporting_citation: Option<Uuid>,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustThresholds {
    pub canonical_confidence: f32,  // Default 0.95
    pub derived_confidence: f32,    // Default 0.75
    pub creative_confidence: f32,   // Default 0.0 (anything ungrounded)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationResult {
    Verified { confidence: f32 },
    PartialMatch { confidence: f32, issues: Vec<String> },
    NotFound,
    SourceUnavailable,
}
```

**Implementation Notes:**
- Trust assignment runs automatically in Normalization stage
- Canonical requires exact match to indexed source
- Derived requires logical derivation from indexed rules (e.g., "clerics can cast healing spells")
- Creative is default for pure AI invention
- Unverified flags content that attempted but failed to cite sources

### ContextAssembler

**Purpose:** Context Assembly stage of the CIP - builds rich context for generation from all available sources.

**Pipeline Stage:** Context Assembly

**Responsibilities:**
- Snapshot current PartialCampaign or Campaign state
- Retrieve relevant rulebook/lore content via hybrid search
- Include CampaignIntent for tone consistency
- Build conversation context window
- Assemble personality profile for system tone

**Interface:**
```rust
pub struct ContextAssembler {
    search: Arc<SearchClient>,
    campaign_manager: Arc<CampaignManager>,
}

impl ContextAssembler {
    // Full context build
    pub async fn assemble_context(
        &self,
        request: &GenerationRequest,
        intent: &CampaignIntent,
        conversation_history: Option<&[ConversationMessage]>,
    ) -> Result<AssembledContext>;

    // Partial context (for specific needs)
    pub async fn get_relevant_rules(
        &self,
        query: &str,
        system: &str,
        limit: usize,
    ) -> Result<Vec<GroundedContent>>;

    pub async fn get_relevant_lore(
        &self,
        query: &str,
        campaign_id: Uuid,
        limit: usize,
    ) -> Result<Vec<GroundedContent>>;

    // Campaign snapshot
    pub async fn snapshot_campaign(
        &self,
        campaign_id: Uuid,
    ) -> Result<CampaignSnapshot>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledContext {
    pub campaign_snapshot: CampaignSnapshot,
    pub intent: CampaignIntent,
    pub grounded_rules: Vec<GroundedContent>,
    pub grounded_lore: Vec<GroundedContent>,
    pub conversation_window: Vec<ConversationMessage>,
    pub personality_profile: Option<PersonalityProfile>,
    pub token_budget: TokenBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundedContent {
    pub content: String,
    pub source: Citation,
    pub relevance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    pub total: usize,
    pub used_by_context: usize,
    pub available_for_generation: usize,
}
```

**Implementation Notes:**
- Context assembly is the first pipeline stage that touches external data
- Token budget management prevents context overflow
- Relevance scoring prioritizes most useful content
- Campaign snapshot includes all entities, not just current focus

---

## Data Models

### WizardState

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardState {
    pub id: Uuid,
    pub current_step: WizardStep,
    pub completed_steps: Vec<WizardStep>,
    pub campaign_draft: PartialCampaign,
    pub conversation_thread_id: Option<Uuid>,
    pub ai_assisted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub auto_saved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WizardStep {
    Basics,           // name, description, system
    Intent,           // CampaignIntent: fantasy, themes, constraints, tone (NEW)
    Scope,            // session count, ongoing vs finite
    Players,          // player count, party preferences
    PartyComposition, // suggested/custom party
    ArcStructure,     // arc type, phases, milestones
    InitialContent,   // starting NPCs, locations, plot hooks
    Review,           // final review before creation
}

/// Shared draft state for campaign creation.
/// Both wizard steps and conversation suggestions update this.
/// See also: Core Concepts > Guided Creation Flow
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartialCampaign {
    // Core identity
    pub name: Option<String>,
    pub system: Option<String>,
    pub description: Option<String>,

    // Creative vision (anchor for all generation)
    pub intent: Option<CampaignIntent>,

    // Scope and structure
    pub session_scope: Option<SessionScope>,
    pub player_count: Option<u8>,
    pub starting_level: Option<u8>,
    pub party_composition: Option<PartyComposition>,
    pub arc_structure: Option<ArcStructure>,

    // Initial content (wrapped with trust/status tracking)
    pub initial_npcs: Vec<EntityDraft<NpcDraft>>,
    pub initial_locations: Vec<EntityDraft<LocationDraft>>,
    pub initial_plot_points: Vec<EntityDraft<PlotPointDraft>>,
}

/// Wrapper that tracks draft status, trust level, and citations for any entity.
/// Used throughout the Acceptance Layer of the CIP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDraft<T> {
    pub id: Uuid,
    pub data: T,
    pub status: CanonStatus,
    pub trust: TrustLevel,
    pub citations: Vec<Citation>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl<T: Default> Default for EntityDraft<T> {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            data: T::default(),
            status: CanonStatus::Draft,
            trust: TrustLevel::Creative,
            citations: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionScope {
    OneShot,
    ShortArc { sessions: u8 },  // 3-5 sessions
    FullCampaign { sessions: u8 }, // 10+ sessions
    Ongoing,  // no planned end
}
```

**Validation Rules:**
- `name` required, 1-100 characters
- `system` must match known game system
- `player_count` range 1-12
- `starting_level` system-dependent range

### ConversationThread

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationThread {
    pub id: Uuid,
    pub campaign_id: Option<Uuid>,  // None for pre-campaign conversations
    pub wizard_id: Option<Uuid>,    // Link to wizard if in creation flow
    pub purpose: ConversationPurpose,
    pub title: String,
    pub message_count: u32,
    pub branched_from: Option<Uuid>,  // Parent thread if branched
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConversationPurpose {
    CampaignCreation,
    CharacterBackground,
    NpcGeneration,
    SessionPlanning,
    WorldBuilding,
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub suggestions: Vec<Suggestion>,
    pub citations: Vec<Citation>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: Uuid,
    pub field: String,        // What this suggests (e.g., "campaign.description")
    pub value: serde_json::Value,
    pub rationale: String,
    pub status: SuggestionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionStatus {
    Pending,
    Accepted,
    Rejected,
    Modified,
}
```

**Relationships:**
- Thread belongs_to Campaign (optional)
- Thread has_many Messages
- Message has_many Suggestions
- Message has_many Citations

### Citation

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub id: Uuid,
    pub source_type: SourceType,
    pub source_id: String,           // Document ID in Meilisearch
    pub source_name: String,         // Human-readable (e.g., "Player's Handbook")
    pub location: SourceLocation,
    pub excerpt: String,             // Relevant snippet
    pub confidence: f32,             // 0.0-1.0
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    Rulebook,
    FlavourSource,
    Adventure,
    Homebrew,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub page: Option<u32>,
    pub section: Option<String>,
    pub chapter: Option<String>,
}
```

### PartySuggestion

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartySuggestion {
    pub id: Uuid,
    pub name: String,                  // e.g., "Classic Balanced Party"
    pub description: String,
    pub roles: Vec<SuggestedRole>,
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
    pub playstyle: String,             // e.g., "combat-focused", "roleplay-heavy"
    pub source_citations: Vec<Citation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedRole {
    pub role_name: String,            // e.g., "Tank", "Healer", "DPS"
    pub suggested_classes: Vec<String>,
    pub priority: RolePriority,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RolePriority {
    Essential,
    Recommended,
    Optional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyAnalysis {
    pub overall_balance: BalanceRating,
    pub capability_coverage: HashMap<String, CoverageLevel>,
    pub gaps: Vec<CapabilityGap>,
    pub redundancies: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BalanceRating {
    WellBalanced,
    SlightlyUnbalanced,
    Specialized,
    Problematic,
}
```

### GeneratedCharacterBackground

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCharacterBackground {
    pub character_id: Option<Uuid>,
    pub name: String,
    pub personal_history: PersonalHistory,
    pub personality: PersonalityTraits,
    pub connections: Vec<NpcConnection>,
    pub plot_hooks: Vec<PlotHook>,
    pub secrets: Vec<CharacterSecret>,
    pub citations: Vec<Citation>,
    pub generation_metadata: GenerationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalHistory {
    pub origin: String,
    pub formative_events: Vec<String>,
    pub turning_point: String,
    pub current_situation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityTraits {
    pub traits: Vec<String>,
    pub ideals: Vec<String>,
    pub bonds: Vec<String>,
    pub flaws: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcConnection {
    pub npc_name: String,
    pub relationship: String,        // e.g., "mentor", "rival", "family"
    pub description: String,
    pub potential_npc_id: Option<Uuid>,  // If NPC already exists
    pub create_npc: bool,            // Should system create this NPC?
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotHook {
    pub title: String,
    pub description: String,
    pub hook_type: PlotHookType,
    pub urgency: HookUrgency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlotHookType {
    Unfinished Business,
    Secret,
    Goal,
    Fear,
    Debt,
    Prophecy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSecret {
    pub content: String,
    pub known_to_player: bool,
    pub revelation_trigger: Option<String>,
}
```

### GeneratedNpc

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedNpc {
    pub name: String,
    pub role: NpcRole,
    pub importance: NpcImportance,
    pub appearance: String,
    pub personality: NpcPersonality,
    pub motivation: String,
    pub secrets: Vec<NpcSecret>,
    pub relationships: Vec<NpcRelationship>,
    pub voice_notes: Option<VoiceNotes>,
    pub stat_block_ref: Option<StatBlockReference>,
    pub faction_id: Option<Uuid>,
    pub location_id: Option<Uuid>,
    pub citations: Vec<Citation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NpcRole {
    QuestGiver,
    Merchant,
    Guard,
    Noble,
    Criminal,
    Scholar,
    Artisan,
    Innkeeper,
    Villain,
    Henchman,
    Ally,
    Neutral,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NpcImportance {
    Minor,      // Name + 1-2 traits
    Supporting, // Full personality + motivation
    Major,      // Everything including stat block
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcPersonality {
    pub demeanor: String,
    pub traits: Vec<String>,
    pub quirks: Vec<String>,
    pub speech_pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcSecret {
    pub content: String,
    pub secret_type: SecretType,
    pub discovery_condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretType {
    PlotRelevant,
    Personal,
    WorldLore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceNotes {
    pub accent: Option<String>,
    pub pitch: Option<String>,       // "high", "low", "gravelly"
    pub pace: Option<String>,        // "rapid", "measured", "drawling"
    pub verbal_tics: Vec<String>,
    pub sample_phrases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityProfile {
    pub system: String,
    pub tone: String,                // e.g., "Grim & Gritty", "High Fantasy"
    pub perspective: String,         // e.g., "Narrator", "Rules Lawyer"
    pub vocabulary: HashMap<String, f32>, // Term -> Frequency weight
    pub style_descriptors: Vec<String>,
    pub common_phrases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub status: SessionStatus,
    pub initiative_state: Option<InitiativeState>,
    pub active_encounters: Vec<EncounterState>,
    pub notes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Planned,
    Active,
    Paused,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiativeState {
    pub round: u32,
    pub current_turn_index: usize,
    pub combatants: Vec<CombatantState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncounterState {
    pub id: Uuid,
    pub name: String,
    pub monsters: Vec<MonsterState>,
    pub round: u32,
}
```

### RandomTable

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomTable {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub dice_notation: String,              // e.g., "d100", "2d6", "d66"
    pub entries: Vec<TableEntry>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableEntry {
    pub id: Uuid,
    pub range_start: u32,                   // e.g., 1 for "1-3"
    pub range_end: u32,                     // e.g., 3 for "1-3"
    pub result: String,
    pub nested_table_id: Option<Uuid>,      // For cascading rolls
    pub weight: Option<f32>,                // For visualization (calculated from range)
    pub used_count: u32,                    // Track how often rolled
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiceRollResult {
    pub notation: String,
    pub dice: Vec<u32>,                     // Individual die results
    pub modifier: i32,
    pub total: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollResult {
    pub id: Uuid,
    pub table_id: Uuid,
    pub roll: DiceRollResult,
    pub entry: TableEntry,
    pub nested_results: Vec<RollResult>,    // For cascading tables
    pub timestamp: DateTime<Utc>,
}
```

**Validation Rules:**
- `dice_notation` must match pattern `(\d*)d(\d+)([+-]\d+)?`
- Entry ranges must be contiguous and cover full dice range
- Nested table references must exist

### SessionRecap

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecap {
    pub id: Uuid,
    pub session_id: Uuid,
    pub campaign_id: Uuid,
    pub session_number: u32,
    pub read_aloud_text: String,            // Narrative prose (max 200 words)
    pub bullet_summary: Vec<String>,        // Key events as bullets
    pub cliffhanger: Option<String>,        // How session ended
    pub key_events: Vec<RecapEvent>,
    pub unresolved_hooks: Vec<Uuid>,        // Plot point IDs
    pub player_decisions: Vec<PlayerDecision>,
    pub generated_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecapEvent {
    pub description: String,
    pub event_type: RecapEventType,
    pub involved_entities: Vec<EntityRef>,
    pub is_player_known: bool,              // False = GM-only knowledge
    pub pc_witnesses: Vec<Uuid>,            // Which PCs saw this
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecapEventType {
    Combat,
    Discovery,
    Dialogue,
    Decision,
    Revelation,
    Travel,
    Milestone,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerDecision {
    pub description: String,
    pub consequences: Vec<String>,
    pub plot_impact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecapOptions {
    pub include_gm_secrets: bool,
    pub filter_by_pc: Option<Uuid>,
    pub max_word_count: usize,
    pub style: RecapStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecapStyle {
    Narrative,                              // Prose paragraphs
    Dramatic,                               // "Previously on..." style
    Factual,                                // Just the facts
}
```

### CheatSheet

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatSheet {
    pub id: Uuid,
    pub session_plan_id: Option<Uuid>,
    pub campaign_id: Uuid,
    pub title: String,
    pub sections: Vec<CheatSheetSection>,
    pub warnings: Vec<String>,              // e.g., "Content truncated"
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatSheetSection {
    pub section_type: CheatSheetSectionType,
    pub title: String,
    pub content: CheatSheetContent,
    pub priority: u8,                       // 1-10, higher = more important
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheatSheetSectionType {
    SessionGoals,
    ActiveNpcs,
    KeyLocations,
    PlotPoints,
    Encounters,
    RulesReference,
    RandomTables,
    PlayerNotes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheatSheetContent {
    Text(String),
    NpcCards(Vec<NpcCard>),
    LocationCards(Vec<LocationCard>),
    PlotCards(Vec<PlotCard>),
    EncounterSummaries(Vec<EncounterSummary>),
    RuleSnippets(Vec<RuleSnippet>),
    TableShortcuts(Vec<TableShortcut>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatSheetPreferences {
    pub always_include: Vec<EntityRef>,
    pub never_include: Vec<EntityRef>,
    pub section_order: Vec<CheatSheetSectionType>,
    pub max_npcs: usize,
    pub max_locations: usize,
    pub include_stat_blocks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatSheetSelection {
    pub npcs: Vec<Uuid>,
    pub locations: Vec<Uuid>,
    pub plot_points: Vec<Uuid>,
    pub encounters: Vec<Uuid>,
    pub tables: Vec<Uuid>,
    pub custom_notes: Option<String>,
}
```

### QuickReferenceCard

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRef {
    pub entity_type: EntityType,
    pub entity_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    Npc,
    Location,
    Item,
    PlotPoint,
    Faction,
    Session,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcCard {
    pub id: Uuid,
    pub name: String,
    pub role: String,
    pub key_traits: Vec<String>,            // Max 3
    pub disposition: String,                // Current attitude
    pub stat_summary: Option<String>,       // "AC 15, HP 45"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationCard {
    pub id: Uuid,
    pub name: String,
    pub location_type: String,
    pub atmosphere: String,                 // 1-2 line mood
    pub notable_features: Vec<String>,      // Max 4 bullets
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotCard {
    pub id: Uuid,
    pub title: String,
    pub status: String,
    pub urgency: String,
    pub next_trigger: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedCard {
    pub entity_ref: EntityRef,
    pub pinned_at: DateTime<Utc>,
    pub position: u8,                       // 0-5 for ordering
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardPreview {
    pub entity_ref: EntityRef,
    pub preview_html: String,               // Pre-rendered card HTML
    pub estimated_height: u32,              // In pixels
}
```

---

## API Design

### Tauri Commands

#### Wizard Commands

```rust
#[tauri::command]
pub async fn start_campaign_wizard(
    state: State<'_, AppState>,
    ai_assisted: bool,
) -> Result<WizardState, String>;

#[tauri::command]
pub async fn get_wizard_state(
    state: State<'_, AppState>,
    wizard_id: Uuid,
) -> Result<WizardState, String>;

#[tauri::command]
pub async fn advance_wizard_step(
    state: State<'_, AppState>,
    wizard_id: Uuid,
    step_data: StepData,
) -> Result<WizardState, String>;

#[tauri::command]
pub async fn wizard_go_back(
    state: State<'_, AppState>,
    wizard_id: Uuid,
) -> Result<WizardState, String>;

#[tauri::command]
pub async fn complete_wizard(
    state: State<'_, AppState>,
    wizard_id: Uuid,
) -> Result<Campaign, String>;

#[tauri::command]
pub async fn cancel_wizard(
    state: State<'_, AppState>,
    wizard_id: Uuid,
    save_draft: bool,
) -> Result<(), String>;

#[tauri::command]
pub async fn list_incomplete_wizards(
    state: State<'_, AppState>,
) -> Result<Vec<WizardSummary>, String>;
```

#### Generation Commands

```rust
#[tauri::command]
pub async fn generate_character_background(
    state: State<'_, AppState>,
    window: Window,
    request: CharacterBackgroundRequest,
    campaign_id: Option<Uuid>,
) -> Result<String, String>;  // Returns stream ID

#[tauri::command]
pub async fn generate_npc(
    state: State<'_, AppState>,
    window: Window,
    request: NpcGenerationRequest,
    campaign_id: Uuid,
) -> Result<String, String>;  // Returns stream ID

#[tauri::command]
pub async fn generate_session_plan(
    state: State<'_, AppState>,
    window: Window,
    request: SessionPlanRequest,
    campaign_id: Uuid,
) -> Result<String, String>;  // Returns stream ID

#[tauri::command]
pub async fn suggest_party_composition(
    state: State<'_, AppState>,
    player_count: u8,
    system: String,
    preferences: Option<PartyPreferences>,
) -> Result<Vec<PartySuggestion>, String>;

#[tauri::command]
pub async fn accept_generated_content(
    state: State<'_, AppState>,
    generation_id: Uuid,
    modifications: Option<serde_json::Value>,
) -> Result<(), String>;

#[tauri::command]
pub async fn reject_generated_content(
    state: State<'_, AppState>,
    generation_id: Uuid,
) -> Result<(), String>;
```

#### Conversation Commands

```rust
#[tauri::command]
pub async fn create_conversation_thread(
    state: State<'_, AppState>,
    campaign_id: Option<Uuid>,
    purpose: ConversationPurpose,
) -> Result<ConversationThread, String>;

#[tauri::command]
pub async fn send_conversation_message(
    state: State<'_, AppState>,
    window: Window,
    thread_id: Uuid,
    content: String,
) -> Result<String, String>;  // Returns stream ID for response

#[tauri::command]
pub async fn get_conversation_messages(
    state: State<'_, AppState>,
    thread_id: Uuid,
    limit: usize,
    before: Option<Uuid>,
) -> Result<Vec<ConversationMessage>, String>;

#[tauri::command]
pub async fn accept_suggestion(
    state: State<'_, AppState>,
    message_id: Uuid,
    suggestion_id: Uuid,
) -> Result<(), String>;

#[tauri::command]
pub async fn reject_suggestion(
    state: State<'_, AppState>,
    message_id: Uuid,
    suggestion_id: Uuid,
) -> Result<(), String>;
```

#### Random Table Commands

```rust
#[tauri::command]
pub async fn create_random_table(
    state: State<'_, AppState>,
    campaign_id: Uuid,
    table: RandomTableDefinition,
) -> Result<RandomTable, String>;

#[tauri::command]
pub async fn get_random_table(
    state: State<'_, AppState>,
    table_id: Uuid,
) -> Result<RandomTable, String>;

#[tauri::command]
pub async fn list_random_tables(
    state: State<'_, AppState>,
    campaign_id: Uuid,
) -> Result<Vec<RandomTableSummary>, String>;

#[tauri::command]
pub async fn roll_on_table(
    state: State<'_, AppState>,
    table_id: Uuid,
    session_id: Option<Uuid>,
) -> Result<RollResult, String>;

#[tauri::command]
pub async fn roll_dice(
    state: State<'_, AppState>,
    notation: String,
) -> Result<DiceRollResult, String>;

#[tauri::command]
pub async fn get_roll_history(
    state: State<'_, AppState>,
    session_id: Uuid,
) -> Result<Vec<RollResult>, String>;

#[tauri::command]
pub async fn generate_random_table(
    state: State<'_, AppState>,
    window: Window,
    request: TableGenerationRequest,
) -> Result<String, String>;  // Returns stream ID
```

#### Recap Commands

```rust
#[tauri::command]
pub async fn generate_session_recap(
    state: State<'_, AppState>,
    window: Window,
    session_id: Uuid,
    options: RecapOptions,
) -> Result<String, String>;  // Returns stream ID

#[tauri::command]
pub async fn get_session_recap(
    state: State<'_, AppState>,
    session_id: Uuid,
) -> Result<Option<SessionRecap>, String>;

#[tauri::command]
pub async fn update_session_recap(
    state: State<'_, AppState>,
    recap_id: Uuid,
    updates: RecapUpdate,
) -> Result<SessionRecap, String>;

#[tauri::command]
pub async fn generate_arc_recap(
    state: State<'_, AppState>,
    window: Window,
    arc_id: Uuid,
    options: RecapOptions,
) -> Result<String, String>;  // Returns stream ID

#[tauri::command]
pub async fn filter_recap_by_pc(
    state: State<'_, AppState>,
    recap_id: Uuid,
    pc_id: Uuid,
) -> Result<SessionRecap, String>;
```

#### Cheat Sheet Commands

```rust
#[tauri::command]
pub async fn build_cheat_sheet(
    state: State<'_, AppState>,
    session_plan_id: Uuid,
    options: CheatSheetOptions,
) -> Result<CheatSheet, String>;

#[tauri::command]
pub async fn build_custom_cheat_sheet(
    state: State<'_, AppState>,
    campaign_id: Uuid,
    selection: CheatSheetSelection,
) -> Result<CheatSheet, String>;

#[tauri::command]
pub async fn export_cheat_sheet_html(
    state: State<'_, AppState>,
    sheet_id: Uuid,
) -> Result<String, String>;

#[tauri::command]
pub async fn save_cheat_sheet_preferences(
    state: State<'_, AppState>,
    campaign_id: Uuid,
    preferences: CheatSheetPreferences,
) -> Result<(), String>;

#[tauri::command]
pub async fn get_cheat_sheet_preferences(
    state: State<'_, AppState>,
    campaign_id: Uuid,
) -> Result<CheatSheetPreferences, String>;
```

#### Quick Reference Card Commands

```rust
#[tauri::command]
pub async fn get_entity_card(
    state: State<'_, AppState>,
    entity_ref: EntityRef,
) -> Result<CardPreview, String>;

#[tauri::command]
pub async fn pin_card(
    state: State<'_, AppState>,
    session_id: Uuid,
    entity_ref: EntityRef,
) -> Result<PinnedCard, String>;

#[tauri::command]
pub async fn unpin_card(
    state: State<'_, AppState>,
    session_id: Uuid,
    entity_ref: EntityRef,
) -> Result<(), String>;

#[tauri::command]
pub async fn get_pinned_cards(
    state: State<'_, AppState>,
    session_id: Uuid,
) -> Result<Vec<PinnedCard>, String>;

#[tauri::command]
pub async fn reorder_pinned_cards(
    state: State<'_, AppState>,
    session_id: Uuid,
    order: Vec<Uuid>,
) -> Result<(), String>;
```

### Streaming Events

Generation and conversation responses stream via Tauri events:

```rust
// Event: "generation-chunk"
#[derive(Serialize)]
pub struct GenerationChunkEvent {
    pub stream_id: String,
    pub chunk_type: ChunkType,
    pub content: String,
    pub is_final: bool,
}

#[derive(Serialize)]
pub enum ChunkType {
    Text,
    Citation,
    Suggestion,
    Error,
}

// Event: "generation-complete"
#[derive(Serialize)]
pub struct GenerationCompleteEvent {
    pub stream_id: String,
    pub generation_id: Uuid,
    pub result: GenerationResult,
}
```

---

## Error Handling

| Category | Error Type | User Action |
|----------|-----------|-------------|
| Wizard state not found | `WizardNotFound` | Start new wizard |
| Step validation failed | `ValidationError` | Fix input and retry |
| LLM unavailable | `LLMUnavailable` | Retry or continue without AI |
| Search service down | `SearchUnavailable` | Continue with limited features |
| Generation timeout | `GenerationTimeout` | Retry or simplify request |
| Draft corrupted | `DraftCorrupted` | Offer recovery options |
| Conversation too long | `ContextLimitExceeded` | Start new thread or summarize |

**Error Response Format:**
```rust
#[derive(Serialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
    pub suggestions: Vec<String>,
}
```

---

## Testing Strategy

### Unit Testing

**Coverage Targets:** 80% for new modules

**Focus Areas:**
- Wizard state machine transitions
- Party balance calculations
- Citation parsing and linking
- Conversation context building
- Template rendering

### Integration Testing

**Key Scenarios:**
1. Complete wizard flow (all steps, no AI)
2. Complete wizard flow (AI-assisted)
3. Generation with source citations
4. Conversation persistence across restart
5. Draft recovery after simulated crash

### E2E Testing

**User Journeys:**
1. New user creates first campaign via wizard
2. Experienced user uses AI to generate complex backstory
3. User generates balanced party for 4 players
4. User plans session with encounter suggestions

---

## Migration Plan

### Database Migrations

**New Tables:**

*Core Pipeline Tables:*
1. `wizard_states` - Wizard progress persistence
2. `campaign_intents` - CampaignIntent anchor for each campaign (NEW)
3. `conversation_threads` - AI conversation threads
4. `conversation_messages` - Individual messages
5. `source_citations` - Citation tracking
6. `generation_drafts` - Draft content with trust/status (NEW)
7. `canon_status_log` - CanonStatus transition audit trail (NEW)
8. `acceptance_events` - Acceptance layer decisions (NEW)
9. `party_compositions` - Stored party compositions

*Phase 2 Tables (Random Tables & Recaps):*
10. `random_tables` - User-defined random tables
11. `random_table_entries` - Table entries with ranges
12. `roll_history` - Session roll log
13. `session_recaps` - Generated session summaries
14. `pinned_cards` - Per-session card tray
15. `cheat_sheet_preferences` - Per-campaign cheat sheet settings

**Schema:**
```sql
-- Migration: 20240125_campaign_pipeline_core

-- CampaignIntent: Stable creative vision anchor (Requirement 19)
CREATE TABLE campaign_intents (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL UNIQUE,
    fantasy TEXT NOT NULL,              -- Core fantasy statement
    player_experiences TEXT NOT NULL,   -- JSON array
    constraints TEXT NOT NULL,          -- JSON array
    themes TEXT NOT NULL,               -- JSON array
    tone_keywords TEXT NOT NULL,        -- JSON array
    avoid TEXT NOT NULL,                -- JSON array
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    migrated_from TEXT,                 -- Previous intent ID if migrated
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id)
);

CREATE INDEX idx_intent_campaign ON campaign_intents(campaign_id);

-- Generation drafts with trust and status (Requirements 20, 21)
CREATE TABLE generation_drafts (
    id TEXT PRIMARY KEY,
    campaign_id TEXT,
    wizard_id TEXT,
    entity_type TEXT NOT NULL,          -- 'npc', 'location', 'plot_point', 'arc', etc.
    data TEXT NOT NULL,                 -- JSON blob of entity data
    status TEXT NOT NULL DEFAULT 'draft', -- draft, approved, canonical, deprecated
    trust_level TEXT NOT NULL DEFAULT 'creative', -- canonical, derived, creative, unverified
    trust_confidence REAL,
    citations TEXT,                     -- JSON array of Citation
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    applied_entity_id TEXT,             -- ID of canonical entity after apply_to_campaign
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id),
    FOREIGN KEY (wizard_id) REFERENCES wizard_states(id)
);

CREATE INDEX idx_drafts_campaign ON generation_drafts(campaign_id, status);
CREATE INDEX idx_drafts_wizard ON generation_drafts(wizard_id);

-- CanonStatus transition log (audit trail for Requirement 21)
CREATE TABLE canon_status_log (
    id TEXT PRIMARY KEY,
    draft_id TEXT NOT NULL,
    previous_status TEXT NOT NULL,
    new_status TEXT NOT NULL,
    reason TEXT,
    triggered_by TEXT NOT NULL,         -- 'user', 'system', 'session_start'
    timestamp TEXT NOT NULL,
    FOREIGN KEY (draft_id) REFERENCES generation_drafts(id)
);

CREATE INDEX idx_canon_log_draft ON canon_status_log(draft_id, timestamp);

-- Acceptance layer events (AcceptanceManager audit)
CREATE TABLE acceptance_events (
    id TEXT PRIMARY KEY,
    draft_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    decision TEXT NOT NULL,             -- approved, rejected, modified, applied_to_campaign
    modifications TEXT,                 -- JSON diff if modified
    reason TEXT,
    timestamp TEXT NOT NULL,
    FOREIGN KEY (draft_id) REFERENCES generation_drafts(id)
);

CREATE INDEX idx_acceptance_draft ON acceptance_events(draft_id);

-- Migration: 20240125_campaign_wizard_tables

CREATE TABLE wizard_states (
    id TEXT PRIMARY KEY,
    current_step TEXT NOT NULL,
    completed_steps TEXT NOT NULL,  -- JSON array
    campaign_draft TEXT NOT NULL,   -- JSON blob
    conversation_thread_id TEXT,
    ai_assisted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    auto_saved_at TEXT,
    FOREIGN KEY (conversation_thread_id) REFERENCES conversation_threads(id)
);

CREATE TABLE conversation_threads (
    id TEXT PRIMARY KEY,
    campaign_id TEXT,
    wizard_id TEXT,
    purpose TEXT NOT NULL,
    title TEXT NOT NULL,
    active_personality TEXT, -- JSON blob of PersonalityProfile
    message_count INTEGER NOT NULL DEFAULT 0,
    branched_from TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id),
    FOREIGN KEY (wizard_id) REFERENCES wizard_states(id),
    FOREIGN KEY (branched_from) REFERENCES conversation_threads(id)
);

CREATE TABLE conversation_messages (
    id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    suggestions TEXT,  -- JSON array
    citations TEXT,    -- JSON array
    created_at TEXT NOT NULL,
    FOREIGN KEY (thread_id) REFERENCES conversation_threads(id)
);

CREATE INDEX idx_messages_thread ON conversation_messages(thread_id, created_at);

CREATE TABLE source_citations (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    source_type TEXT NOT NULL,
    source_id TEXT NOT NULL,
    source_name TEXT NOT NULL,
    location TEXT,     -- JSON
    excerpt TEXT NOT NULL,
    confidence REAL NOT NULL,
    used_in TEXT,      -- What entity used this citation
    created_at TEXT NOT NULL,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id)
);

CREATE TABLE party_compositions (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    name TEXT NOT NULL,
    composition TEXT NOT NULL,  -- JSON
    analysis TEXT,              -- JSON
    created_at TEXT NOT NULL,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id)
);

CREATE TABLE generation_history (
    id TEXT PRIMARY KEY,
    campaign_id TEXT,
    generation_type TEXT NOT NULL,
    request TEXT NOT NULL,      -- JSON
    result TEXT NOT NULL,       -- JSON
    status TEXT NOT NULL,       -- pending, accepted, rejected, modified
    created_at TEXT NOT NULL,
    resolved_at TEXT,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id)
);

-- Migration: 20240126_random_tables_and_recaps

CREATE TABLE random_tables (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    dice_notation TEXT NOT NULL,
    tags TEXT,                  -- JSON array
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id)
);

CREATE TABLE random_table_entries (
    id TEXT PRIMARY KEY,
    table_id TEXT NOT NULL,
    range_start INTEGER NOT NULL,
    range_end INTEGER NOT NULL,
    result TEXT NOT NULL,
    nested_table_id TEXT,
    used_count INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (table_id) REFERENCES random_tables(id) ON DELETE CASCADE,
    FOREIGN KEY (nested_table_id) REFERENCES random_tables(id)
);

CREATE INDEX idx_table_entries ON random_table_entries(table_id);

CREATE TABLE roll_history (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    table_id TEXT NOT NULL,
    roll_data TEXT NOT NULL,    -- JSON (DiceRollResult)
    entry_id TEXT NOT NULL,
    nested_results TEXT,        -- JSON array of roll IDs
    timestamp TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    FOREIGN KEY (table_id) REFERENCES random_tables(id),
    FOREIGN KEY (entry_id) REFERENCES random_table_entries(id)
);

CREATE INDEX idx_roll_history_session ON roll_history(session_id, timestamp);

CREATE TABLE session_recaps (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL UNIQUE,
    campaign_id TEXT NOT NULL,
    session_number INTEGER NOT NULL,
    read_aloud_text TEXT NOT NULL,
    bullet_summary TEXT NOT NULL,   -- JSON array
    cliffhanger TEXT,
    key_events TEXT NOT NULL,       -- JSON array
    unresolved_hooks TEXT,          -- JSON array of plot point IDs
    player_decisions TEXT,          -- JSON array
    generated_at TEXT NOT NULL,
    edited_at TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id)
);

CREATE TABLE pinned_cards (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    pinned_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    UNIQUE (session_id, entity_type, entity_id),
    CHECK (position >= 0 AND position <= 5)
);

CREATE INDEX idx_pinned_cards_session ON pinned_cards(session_id, position);

CREATE TABLE cheat_sheet_preferences (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL UNIQUE,
    always_include TEXT,        -- JSON array of EntityRefs
    never_include TEXT,         -- JSON array of EntityRefs
    section_order TEXT,         -- JSON array of section types
    max_npcs INTEGER DEFAULT 5,
    max_locations INTEGER DEFAULT 3,
    include_stat_blocks INTEGER DEFAULT 1,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id)
);
```

---

## Open Design Questions

1. **Context Window Management:** How to handle conversations that exceed LLM context limits? Options: rolling summary, hierarchical compression, or hard thread length limit.

2. **Multi-Model Generation:** Should different generation types (backstory vs NPC vs encounter) use different models based on task complexity?

3. **Offline Templates:** Should we ship default generation templates that work without LLM for basic functionality?

4. **Cross-Campaign Templates:** Should generated NPCs/locations be saveable as templates for reuse across the user's own campaigns?

5. **Card Layout Responsiveness:** How should quick reference cards adapt to different screen sizes? Fixed size vs responsive?

6. **Cheat Sheet Print Layout:** Should we support different paper sizes (US Letter, A4) and orientations?

### Deferred Questions

The following questions relate to collaborative features and are deferred to that phase:
- Data structures for multi-GM editing → See `../collaborative-features/`
- Conflict resolution for concurrent edits → See `../collaborative-features/`
