Below is a set of **Rust trait boundaries** that enforce your rewritten spec’s invariants in the type system:

* **Single Draft Truth during creation**
* **AI proposes; GM decides**
* **Only CampaignManager/SessionManager mutate canonical state**
* **Grounding + citations are attached to proposals, not silently applied**

I’m going to give you “interfaces first” (traits), then show how your existing modules implement them, and finally a dependency rule-of-thumb (“who can depend on whom”) so you don’t accidentally create a spaghetti deity.

---

## 1) Core types (small, boring, weaponized)

These are the types traits will talk about. Keep them in a `domain` crate/module.

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftSnapshot {
    pub draft_id: DraftId,
    pub partial_campaign: PartialCampaign,
    pub current_step: WizardStep,
    pub decisions: DecisionSummary, // distilled accept/reject history
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    pub field: String,              // e.g. "campaign.tone"
    pub value: serde_json::Value,   // new value
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchSet {
    pub patches: Vec<Patch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: Uuid,
    pub patches: PatchSet,          // “apply to draft” payload
    pub rationale: String,
    pub citations: Vec<Citation>,
    pub trust: TrustLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Decision {
    Accepted { proposal_id: Uuid, applied: PatchSet },
    Rejected { proposal_id: Uuid, reason: Option<String> },
    Modified { proposal_id: Uuid, applied: PatchSet },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactBundle {
    pub proposals: Vec<Proposal>,   // patch-style suggestions
    pub artifacts: Vec<Artifact>,   // NPCs, arcs, tables, etc. (draft artifacts)
    pub citations: Vec<Citation>,   // convenience aggregation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustLevel { Canonical, Derived, Creative, Unverified }
```

Key move: **everything AI produces is either a `Proposal` (patches) or an `Artifact`**. Nothing “just writes” into your state.

---

## 2) Draft truth boundaries (WizardManager becomes enforceable)

### DraftStore: the only thing allowed to mutate draft state

```rust
#[async_trait]
pub trait DraftStore: Send + Sync {
    async fn create_draft(&self) -> anyhow::Result<DraftSnapshot>;
    async fn load_draft(&self, id: DraftId) -> anyhow::Result<Option<DraftSnapshot>>;

    async fn apply_patches(&self, id: DraftId, patches: PatchSet) -> anyhow::Result<DraftSnapshot>;
    async fn set_step(&self, id: DraftId, step: WizardStep) -> anyhow::Result<DraftSnapshot>;

    async fn autosave_hint(&self, id: DraftId) -> anyhow::Result<()>;
    async fn list_incomplete(&self) -> anyhow::Result<Vec<DraftSnapshot>>;
    async fn delete_draft(&self, id: DraftId) -> anyhow::Result<()>;
}
```

### DraftValidator: keeps your invariants honest

```rust
pub trait DraftValidator: Send + Sync {
    fn validate_step(&self, snapshot: &DraftSnapshot) -> anyhow::Result<()>;
    fn validate_completion(&self, snapshot: &DraftSnapshot) -> anyhow::Result<()>;
}
```

WizardManager becomes an implementation of `DraftStore` + `DraftValidator` (or composes a validator).

---

## 3) Conversation as a decision ledger (not a magical brain)

### ConversationStore: persistence + branching + message retrieval

```rust
#[async_trait]
pub trait ConversationStore: Send + Sync {
    async fn create_thread(&self, draft_id: DraftId, purpose: ConversationPurpose)
        -> anyhow::Result<ThreadId>;

    async fn append_user_message(&self, thread: ThreadId, content: String)
        -> anyhow::Result<Uuid>;

    async fn append_assistant_message(&self, thread: ThreadId, content: String, proposals: Vec<Proposal>)
        -> anyhow::Result<Uuid>;

    async fn record_decision(&self, thread: ThreadId, decision: Decision)
        -> anyhow::Result<()>;

    async fn summarize_decisions(&self, thread: ThreadId)
        -> anyhow::Result<DecisionSummary>;

    async fn branch_from(&self, thread: ThreadId, message_id: Uuid)
        -> anyhow::Result<ThreadId>;
}
```

This makes ConversationManager a strict “ledger + memory,” not an authority.

---

## 4) Grounding boundaries (rules/lore retrieval + citations)

### KnowledgeIndex: your Meilisearch facade

```rust
#[async_trait]
pub trait KnowledgeIndex: Send + Sync {
    async fn hybrid_search(&self, query: &str, filters: SearchFilters)
        -> anyhow::Result<Vec<IndexedSnippet>>;
}
```

### Grounder: builds context packs + citations

```rust
#[async_trait]
pub trait Grounder: Send + Sync {
    async fn ground(&self, request: &GroundingRequest)
        -> anyhow::Result<GroundingPack>;
}

#[derive(Debug, Clone)]
pub struct GroundingPack {
    pub snippets: Vec<IndexedSnippet>,
    pub citations: Vec<Citation>,
}
```

### ReferenceResolver: for “Recursive Stat Blocks” and other expansions

```rust
#[async_trait]
pub trait ReferenceResolver: Send + Sync {
    async fn resolve_inline(&self, text: &str, system: &str)
        -> anyhow::Result<(String, Vec<Citation>)>;
}
```

RulebookLinker/FlavourSearcher implement `Grounder` (or compose into it). Recursive stat-block logic is a `ReferenceResolver`.

---

## 5) Generation boundaries (LLM is an adapter; orchestration is pure)

### LlmClient: the *dumb* streaming adapter

```rust
use std::pin::Pin;
use futures_core::Stream;

pub type TokenStream = Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>>;

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn stream_completion(&self, prompt: String) -> anyhow::Result<TokenStream>;
}
```

LLMRouter implements this.

### PromptRenderer: template registry boundary

```rust
pub trait PromptRenderer: Send + Sync {
    fn render(&self, template_id: &str, ctx: &serde_json::Value) -> anyhow::Result<String>;
}
```

TemplateRegistry implements this.

### Generator: the orchestrator boundary (the “engine”)

```rust
#[async_trait]
pub trait Generator: Send + Sync {
    async fn generate(&self, req: GenerationRequest) -> anyhow::Result<GenerationStream>;
}

pub struct GenerationStream {
    pub tokens: Box<TokenStream>,           // streaming text for UX
    pub finalize: Box<dyn Finalizer + Send + Sync>, // parse/structure after stream ends
}

#[async_trait]
pub trait Finalizer: Send + Sync {
    async fn finalize(self: Box<Self>) -> anyhow::Result<ArtifactBundle>;
}
```

This pattern is gold because you get:

* streaming UI immediately,
* structured results at the end,
* and you keep parsing separate from token streaming.

GenerationOrchestrator implements `Generator`.

---

## 6) Artifact generators: specialized, but same interface

Instead of one fat orchestrator method per artifact, give artifacts a trait:

```rust
#[async_trait]
pub trait ArtifactGenerator: Send + Sync {
    fn kind(&self) -> ArtifactKind;
    async fn generate(&self, snapshot: &DraftSnapshot, opts: serde_json::Value)
        -> anyhow::Result<ArtifactBundle>;
}
```

* PartyBalancer implements `ArtifactGenerator` (kind = PartyComposition)
* NPC generator implements `ArtifactGenerator` (kind = NPC)
* Arc generator implements `ArtifactGenerator` (kind = ArcOutline)
* RandomTableEngine implements `ArtifactGenerator` (kind = RandomTable) (can be offline)
* RecapGenerator implements `ArtifactGenerator` (kind = Recap)

If you want the LLM streaming UX for all of these, you can also define a `StreamingArtifactGenerator` that returns the same `GenerationStream`.

---

## 7) Canonical campaign boundaries (the “only writers” rule)

### CampaignWriter: the only thing allowed to commit canonical campaign data

```rust
#[async_trait]
pub trait CampaignWriter: Send + Sync {
    async fn create_campaign_from_draft(&self, snapshot: DraftSnapshot)
        -> anyhow::Result<CampaignId>;

    async fn apply_canonical_change(&self, campaign: CampaignId, change: CanonicalChange)
        -> anyhow::Result<()>;

    async fn snapshot(&self, campaign: CampaignId, note: String)
        -> anyhow::Result<()>;
}
```

CampaignManager implements this.

### SessionWriter: session lifecycle only

```rust
#[async_trait]
pub trait SessionWriter: Send + Sync {
    async fn create_session_plan(&self, campaign: CampaignId, plan: SessionPlan)
        -> anyhow::Result<Uuid>;
    async fn complete_session(&self, session_id: Uuid, summary: SessionSummary)
        -> anyhow::Result<()>;
}
```

SessionManager implements this.

---

## 8) The glue: “CreationFlow” as an application service

This is what your Tauri commands call. It composes the traits, but it does not own persistence formats.

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
    // High-level use cases go here:
    // - start draft
    // - send chat message -> stream response -> proposals
    // - accept proposal -> apply patches to draft
    // - complete -> commit campaign
}
```

This boundary is what keeps your modules from calling each other sideways.

---

## 9) Dependency rules (so the graph stays acyclic)

Think “inner domain is pure, outer layers depend inward.”

* **Domain types**: depend on nothing
* **Traits**: depend on domain types
* **Implementations**:

  * SqliteDraftStore depends on sqlx + domain + traits
  * MeilisearchGrounder depends on meilisearch + domain + traits
  * LlmRouterClient depends on llm router + traits
* **CreationFlow** depends on traits only (generic over impls)
* **Tauri commands** depend on CreationFlow

And the hard rule:

> Generators/Grounders never get a `CampaignWriter`.
> Only the application service (CreationFlow) holds writers.

That’s how you prevent “AI accidentally committed something.”

---

## 10) Minimal “trait → module” mapping (your original names)

* WizardManager ⇒ `DraftStore` (+ `DraftValidator`)
* ConversationManager ⇒ `ConversationStore`
* SearchClient/Meilisearch ⇒ `KnowledgeIndex`
* RulebookLinker + FlavourSearcher ⇒ `Grounder` (compose)
* CitationBuilder ⇒ part of `Grounder` / `ReferenceResolver` outputs
* LLMRouter ⇒ `LlmClient`
* TemplateRegistry ⇒ `PromptRenderer`
* GenerationOrchestrator ⇒ `Generator`
* PartyBalancer/RandomTableEngine/RecapGenerator ⇒ `ArtifactGenerator` (+ optional streaming variant)
* CampaignManager ⇒ `CampaignWriter`
* SessionManager ⇒ `SessionWriter`

---

If you want the extra-sharp version, I can convert this into a `crate` layout (`domain`, `ports`, `adapters`, `app`, `tauri`) with a compile-enforced “no forbidden dependencies” scheme (Rust module privacy + feature flags + `cargo deny` patterns). That’s the kind of nerdy prison that keeps big features honest.
