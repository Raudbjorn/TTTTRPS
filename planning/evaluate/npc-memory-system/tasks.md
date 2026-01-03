# NPC Memory System - Tasks

## Phase 1: Interaction Logging

### Backend (Rust)

- [ ] **Database schema**
  - [ ] Create `npc_interactions` table migration
  - [ ] Create `npc_relationships` table migration
  - [ ] Add partial unique index for party-wide relationships
  - [ ] Add indexes for efficient lookups

- [ ] **Core data structures**
  - [ ] `NpcInteraction` struct
  - [ ] `InteractionType` enum (Dialogue, Trade, Combat, Quest, Social)
  - [ ] `Sentiment` enum (Positive, Negative, Neutral)
  - [ ] `NpcRelationship` struct
  - [ ] `RelationshipStatus` enum with score thresholds

- [ ] **Interaction capture**
  - [ ] Manual interaction logging
  - [ ] Auto-capture from chat when NPC is active speaker
  - [ ] Extract key quotes from dialogue
  - [ ] Sentiment detection (basic keyword matching or LLM)

- [ ] **Tauri commands**
  - [ ] `log_npc_interaction` - Record new interaction
  - [ ] `get_npc_interactions(npc_id)` - List interaction history
  - [ ] `get_recent_interactions(npc_id, limit)` - Recent only
  - [ ] `delete_interaction` - Remove entry
  - [ ] `update_interaction` - Edit summary/sentiment

### Frontend (Leptos)

- [ ] **InteractionHistory component**
  - [ ] Timeline view of NPC interactions
  - [ ] Filter by interaction type
  - [ ] Filter by session
  - [ ] Expandable entries with full details

- [ ] **LogInteractionModal component**
  - [ ] Interaction type selector
  - [ ] Summary text input
  - [ ] Sentiment picker
  - [ ] Key quotes input (optional)
  - [ ] Participant selector

- [ ] **NPC detail panel extension**
  - [ ] "History" tab showing interactions
  - [ ] Quick log interaction button
  - [ ] Interaction count badge

---

## Phase 2: Relationship System

### Backend

- [ ] **Relationship management**
  - [ ] Create relationship on first interaction
  - [ ] Score adjustment functions (+/- with reason)
  - [ ] Status calculation from score thresholds
  - [ ] Milestone recording

- [ ] **Score thresholds**
  ```
  Hostile:    -100 to -60
  Unfriendly: -59 to -20
  Neutral:    -19 to 19
  Friendly:   20 to 59
  Allied:     60 to 100
  ```
  - [ ] Clamp scores to [-100, 100] range on all adjustments
  - [ ] Status derived from clamped score

- [ ] **Preset score adjustments**
  - [ ] Helped in combat: +15
  - [ ] Completed quest for NPC: +20
  - [ ] Gave gift/gold: +5 to +15
  - [ ] Insulted/threatened: -10
  - [ ] Attacked/harmed: -30
  - [ ] Betrayed: -50

- [ ] **Tauri commands**
  - [ ] `get_npc_relationship(npc_id, character_id?)` - Get relationship
  - [ ] `adjust_relationship(npc_id, character_id?, delta, reason)`
  - [ ] `set_relationship_score` - Direct set (DM override)
  - [ ] `add_relationship_milestone`
  - [ ] `get_party_relationships(campaign_id)` - Overview

### Frontend

- [ ] **RelationshipBadge component**
  - [ ] Color-coded status indicator
  - [ ] Score tooltip on hover
  - [ ] Compact for NPC cards

- [ ] **RelationshipPanel component**
  - [ ] Current status and score
  - [ ] Score adjustment controls (+/- buttons)
  - [ ] Milestone timeline
  - [ ] Relationship history graph (optional)

- [ ] **RelationshipOverview component**
  - [ ] Party-wide relationship summary
  - [ ] Sort by status (hostile first, etc.)
  - [ ] Filter by faction (future)

- [ ] **NPC card updates**
  - [ ] Show relationship badge
  - [ ] Quick adjust buttons
  - [ ] Last interaction date

---

## Phase 3: Context Integration

### Backend

- [ ] **Memory context builder**
  - [ ] Build relationship summary for LLM prompt
  - [ ] Include recent interactions (last 3-5)
  - [ ] Include milestones
  - [ ] Format as structured context block

- [ ] **Memory summarization**
  - [ ] Summarize old interactions (keep recent in detail)
  - [ ] Preserve milestone events always
  - [ ] Token budget management
  - [ ] **Summarization triggers:**
    - [ ] Background job after session end (preferred - no dialogue latency)
    - [ ] On-demand when interaction count exceeds threshold (e.g., >10 unsummarized)
    - [ ] Never during dialogue generation to avoid latency

- [ ] **Prompt injection**
  - [ ] Integrate memory context into NPC dialogue prompts
  - [ ] Adjust personality based on relationship status
  - [ ] Include behavioral hints ("grateful", "suspicious", etc.)

- [ ] **Context template**
  ```
  RELATIONSHIP WITH PARTY:
  - Status: [status] ([score])
  - First met: [date/session]
  - Key events: [milestones]

  RECENT INTERACTIONS:
  1. [summary] ([sentiment])
  2. ...

  Remember: [behavioral hint based on relationship]
  ```

- [ ] **Tauri commands**
  - [ ] `build_npc_context(npc_id)` - Get formatted context
  - [ ] `summarize_interactions(npc_id)` - Compress history

### Frontend

- [ ] **MemorySummary component**
  - [ ] Condensed view of NPC memory
  - [ ] Key facts at a glance
  - [ ] Expand for full history

- [ ] **Context preview** (debug/DM tool)
  - [ ] Show what context LLM receives
  - [ ] Edit/override before generation

---

## Phase 4: Smart Callbacks (Stretch)

### Backend

- [ ] **Callback generation**
  - [ ] Prompt LLM to reference past events
  - [ ] Quest reminder system
  - [ ] Emotional callbacks ("I still remember when you...")

- [ ] **Rumor propagation** (advanced)
  - [ ] Define NPC connection graph
  - [ ] Spread information between connected NPCs
  - [ ] Time-delayed propagation

- [ ] **Tauri commands**
  - [ ] `generate_callback_hint(npc_id)` - Suggest callback
  - [ ] `propagate_rumor(source_npc, info)` - Spread info

### Frontend

- [ ] **CallbackSuggestion component**
  - [ ] DM tool showing callback opportunities
  - [ ] "Use this callback" quick insert
  - [ ] Dismiss/archive suggestions

---

## Data Migration

- [ ] **Existing NPC enhancement**
  - [ ] Add relationship fields to existing NPCs
  - [ ] Initialize neutral relationships for existing campaigns
  - [ ] Migration script for existing data

---

## Dependencies

- Existing NPC system
- Existing personality system
- LLM client for summarization/callbacks

## Testing

- [ ] Unit tests for score calculations and thresholds
- [ ] Unit tests for context building
- [ ] Integration tests for interaction CRUD
- [ ] Test relationship cascade on NPC delete
- [ ] Test context token budget limits
