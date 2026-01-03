# NPC Memory System - Feature Evaluation

## Status: PROPOSED

**Priority:** HIGH - Deep immersion through persistent character relationships

## Overview

Enable NPCs to remember past interactions, track relationship dynamics, and reference previous conversations. The DM personality adapts NPC dialogue based on accumulated history with players.

## Why This Feature

- **Immersion breakthrough**: NPCs that remember create emotional investment
- **Narrative continuity**: "Didn't you help me find my lost ring?" callbacks
- **DM assistance**: Reduces cognitive load tracking NPC relationships
- **Personality synergy**: Builds on existing PersonalityProfile system

## Feature Scope

### Tier 1: Interaction Memory (MVP)
- Record NPC interactions (who, when, what happened, sentiment)
- Surface recent interactions in NPC detail view
- Include interaction summary in LLM context for NPC dialogue
- Basic relationship score (hostile -> neutral -> friendly -> allied)

### Tier 2: Relationship Dynamics
- Relationship changes based on actions (helped/harmed/ignored)
- NPC attitude affects dialogue tone (PersonalityProfile modulation)
- Faction relationships (helping one NPC affects faction standing)
- Rumors spread (NPCs share info about party)

### Tier 3: Memory-Aware Generation
- LLM generates callbacks to past events
- NPCs ask about unfinished quests
- Relationship milestones (first meeting, saved their life, betrayed them)
- Emotional arcs (grudging respect -> genuine friendship)

## Technical Approach

### Memory Architecture

```
┌─────────────────────────────────────────────┐
│              NPC Memory Store               │
├─────────────────────────────────────────────┤
│  Interaction Log    │  Relationship State   │
│  - Events           │  - Score (-100 to 100)│
│  - Timestamps       │  - Milestones         │
│  - Sentiment        │  - Faction links      │
│  - Key quotes       │  - Last seen          │
└─────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────┐
│         LLM Context Builder                 │
│  (Personality + Voice + Memory Summary)     │
└─────────────────────────────────────────────┘
```

### Memory Compression

Raw interaction logs grow unbounded. Use LLM summarization:

1. **Recent interactions**: Full detail (last 3-5)
2. **Older interactions**: Summarized to key points
3. **Ancient history**: Single paragraph overview
4. **Milestones**: Always preserved (first meeting, major events)

### Implementation Plan

### Phase 1: Interaction Logging
- `NpcInteraction` struct: npc_id, session_id, timestamp, summary, sentiment
- Capture interactions from chat (when NPC is active speaker)
- Manual "log interaction" action for non-chat events
- View interaction history in NPC detail panel

### Phase 2: Relationship System
- `NpcRelationship` struct: npc_id, character_id, score, milestones
- Score adjustment commands (+10 helped, -20 attacked, etc.)
- Relationship status labels (Hostile/Unfriendly/Neutral/Friendly/Allied)
- Visual relationship indicator in NPC cards

### Phase 3: Context Integration
- Build memory context for LLM when NPC speaks
- Include relationship status + recent interactions + milestones
- Prompt engineering: "Remember: this NPC [relationship] the party because [reason]"
- Test dialogue consistency

### Phase 4: Smart Callbacks (Stretch)
- LLM-generated references to past events
- Quest reminder system ("Have you found the artifact yet?")
- Rumor propagation between connected NPCs

## Database Schema

```sql
-- NPC interactions log
CREATE TABLE npc_interactions (
    id TEXT PRIMARY KEY,
    npc_id TEXT NOT NULL REFERENCES npcs(id),
    session_id TEXT REFERENCES sessions(id),
    campaign_id TEXT NOT NULL REFERENCES campaigns(id),
    interaction_type TEXT NOT NULL, -- 'dialogue', 'trade', 'combat', 'quest', 'social'
    summary TEXT NOT NULL,
    sentiment TEXT, -- 'positive', 'negative', 'neutral'
    key_quotes TEXT, -- JSON array of notable quotes
    participants TEXT, -- JSON array of character IDs
    created_at TEXT NOT NULL
);

-- NPC relationships with party/characters
CREATE TABLE npc_relationships (
    id TEXT PRIMARY KEY,
    npc_id TEXT NOT NULL REFERENCES npcs(id),
    character_id TEXT REFERENCES characters(id), -- NULL = party-wide
    campaign_id TEXT NOT NULL REFERENCES campaigns(id),
    score INTEGER DEFAULT 0, -- -100 (hostile) to 100 (allied)
    status TEXT NOT NULL, -- 'hostile', 'unfriendly', 'neutral', 'friendly', 'allied'
    milestones TEXT, -- JSON array of significant events
    last_interaction TEXT,
    updated_at TEXT NOT NULL,
    UNIQUE(npc_id, character_id, campaign_id)
);

-- Enforce a single party-wide relationship per NPC (NULL character_id)
-- SQLite treats NULLs as distinct, so UNIQUE constraint won't catch this
CREATE UNIQUE INDEX idx_npc_relationships_party_wide
    ON npc_relationships(npc_id, campaign_id)
    WHERE character_id IS NULL;

-- Indexes
CREATE INDEX idx_npc_interactions_npc ON npc_interactions(npc_id);
CREATE INDEX idx_npc_relationships_npc ON npc_relationships(npc_id);
```

## Data Structures

```rust
pub struct NpcInteraction {
    pub id: String,
    pub npc_id: String,
    pub session_id: Option<String>,
    pub campaign_id: String,
    pub interaction_type: InteractionType,
    pub summary: String,
    pub sentiment: Sentiment,
    pub key_quotes: Vec<String>,
    pub participants: Vec<String>,
    pub created_at: DateTime<Utc>,
}

pub enum InteractionType {
    Dialogue,
    Trade,
    Combat,
    Quest,
    Social,
}

pub enum Sentiment {
    Positive,
    Negative,
    Neutral,
}

pub struct NpcRelationship {
    pub npc_id: String,
    pub character_id: Option<String>,
    pub score: i32,
    pub status: RelationshipStatus,
    pub milestones: Vec<Milestone>,
    pub last_interaction: Option<DateTime<Utc>>,
}

pub enum RelationshipStatus {
    Hostile,    // -100 to -60
    Unfriendly, // -59 to -20
    Neutral,    // -19 to 19
    Friendly,   // 20 to 59
    Allied,     // 60 to 100
}

pub struct Milestone {
    pub event: String,
    pub timestamp: DateTime<Utc>,
    pub score_change: i32,
}
```

## LLM Context Building

When generating NPC dialogue:

```
You are roleplaying as [NPC Name], a [personality traits].

RELATIONSHIP WITH PARTY:
- Status: Friendly (+45)
- First met: Session 3 at the tavern
- Key events: Party saved them from bandits (+30), helped find lost ring (+15)
- Recent: Last session, discussed the missing merchant

RECENT INTERACTIONS:
1. [2 days ago] Party asked about rumors in town - provided info about strange lights
2. [1 week ago] Traded supplies, gave fair prices
3. [2 weeks ago] Party saved NPC from bandit attack - very grateful

Remember: This NPC is grateful to the party and willing to help. They might reference the bandit rescue.
```

## UI Components

- `NpcRelationshipBadge.rs` - Visual indicator (color-coded)
- `InteractionHistory.rs` - Timeline view in NPC detail
- `RelationshipEditor.rs` - Manual score adjustment
- `MemorySummary.rs` - Condensed view for quick reference

## Effort Estimate

| Phase | Complexity | Notes |
|-------|------------|-------|
| Interaction Logging | Low | CRUD + UI |
| Relationship System | Medium | Score logic, status thresholds |
| Context Integration | Medium | Prompt engineering, testing |
| Smart Callbacks | High | LLM reliability challenges |

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Context window bloat | Aggressive summarization, recency bias |
| Inconsistent callbacks | Explicit memory in prompt, not implicit |
| Manual logging burden | Auto-capture from chat where possible |
| Score gaming | Meaningful thresholds, DM override |

## Success Metrics

- NPCs reference past events in 30%+ of extended dialogues
- Users report increased emotional engagement
- Reduced "who is this NPC again?" questions

## Related Features

- NPC system (existing) - Entity storage, personality
- Personality system (existing) - Dialogue generation
- Session notes (existing) - Manual memory augmentation
- Campaign versioning (existing) - Relationship snapshots

## Recommendation

**High value, medium effort.** The NPC system exists; this extends it with temporal awareness. Start with Tier 1 (interaction logging) to build the data foundation, then add relationship scoring. Smart callbacks can come later as LLM capabilities mature.
