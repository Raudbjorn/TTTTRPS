# Session Recap Generation - Feature Evaluation

## Status: PROPOSED

**Priority:** MEDIUM - Quality-of-life feature, lower immersion impact than others

## Overview

Automatically generate "Previously on..." style summaries from session transcripts and notes. Summaries help players recall events, onboard absent players, and create campaign archives.

## Why This Feature

- **Player continuity**: Sessions weeks apart, memory fades
- **DM relief**: Manual recaps are tedious
- **Narrative polish**: LLM can write cinematic summaries
- **Archive value**: Campaign history becomes readable story

## Feature Scope

### Tier 1: Basic Recap (MVP)
- Generate summary from session chat history
- Bullet-point format: key events, NPC interactions, combat outcomes
- Manual trigger: "Generate Recap" button post-session
- Export to markdown/text

### Tier 2: Styled Narratives
- Multiple styles: bullet points, narrative prose, dramatic, comedic
- Include session metadata (date, duration, characters present)
- Character-specific perspectives ("What Thorin remembers")
- Highlight unresolved plot threads

### Tier 3: Campaign Chronicle
- Link recaps across sessions into continuous narrative
- Chapter structure with session boundaries
- Automatic "cast of characters" with NPC appearances
- Export to PDF/EPUB for campaign archive

## Technical Approach

### Input Sources
1. **Chat transcript** - Full DM/player exchange
2. **Session notes** - Manual annotations
3. **Combat logs** - Encounter outcomes
4. **NPC interactions** - Who was involved (ties to memory system)

### Summarization Strategy

**Challenge:** Chat transcripts can be 50K+ tokens; LLM context windows have limits.

**Solution:** Multi-stage summarization
1. Chunk transcript into segments (by time or topic)
2. Summarize each chunk independently
3. Combine chunk summaries into final recap
4. Apply style/persona for output format

### Prompt Engineering

```
You are a skilled fantasy narrator creating a session recap.

SESSION METADATA:
- Campaign: [name]
- Session #: [number]
- Date: [date]
- Characters present: [list]

EVENTS TO SUMMARIZE:
[chunked summaries or key events]

Generate a [style] recap that:
1. Opens with a brief "last time" hook
2. Covers major plot developments
3. Highlights memorable moments
4. Notes unresolved threads
5. Ends with a teaser for next session

Keep it under [word_limit] words.
```

### Implementation Plan

### Phase 1: Basic Summary
- Collect chat history for session
- Chunk into manageable segments
- LLM summarizes each chunk
- Combine into bullet-point recap
- Display in session detail view

### Phase 2: Style Options
- Add style selector (bullets, narrative, dramatic)
- Adjust prompt per style
- Character perspective option
- Edit/regenerate capability

### Phase 3: Export & Archive
- Markdown export
- PDF generation (via printable HTML)
- Campaign chronicle view (all recaps)
- Search within recaps

## Data Structures

```rust
pub struct SessionRecap {
    pub id: String,
    pub session_id: String,
    pub campaign_id: String,
    pub style: RecapStyle,
    pub content: String,
    pub word_count: u32,
    pub key_events: Vec<String>,
    pub npcs_featured: Vec<String>,
    pub unresolved_threads: Vec<String>,
    pub generated_at: DateTime<Utc>,
}

pub enum RecapStyle {
    BulletPoints,
    Narrative,
    Dramatic,
    Comedic,
    CharacterPerspective(String), // character_id
}
```

## Database Schema

```sql
CREATE TABLE session_recaps (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    campaign_id TEXT NOT NULL REFERENCES campaigns(id),
    style TEXT NOT NULL,
    content TEXT NOT NULL,
    word_count INTEGER,
    key_events TEXT, -- JSON array
    npcs_featured TEXT, -- JSON array of NPC IDs
    unresolved_threads TEXT, -- JSON array
    generated_at TEXT NOT NULL,
    edited_at TEXT
);
```

## UI Components

- `RecapGenerator.rs` - Generation wizard with style options
- `RecapViewer.rs` - Display + edit recap
- `CampaignChronicle.rs` - All recaps timeline view
- `RecapExport.rs` - Export options modal

## Effort Estimate

| Phase | Complexity | Notes |
|-------|------------|-------|
| Basic Summary | Medium | Chunking strategy key |
| Style Options | Low | Prompt variations |
| Export & Archive | Low | Markdown/HTML generation |

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Long transcripts exceed context | Multi-stage summarization |
| Missing important events | Key event extraction step |
| Hallucinated details | Base summary on explicit events |
| Bland/generic recaps | Strong style prompts, user editing |

## Success Metrics

- Recap generation < 30 seconds
- **Key event accuracy**: Average F1 score >= 0.80 on DM-annotated key events
  - **Protocol**: Sample 20 sessions; DM annotates key events (combat outcomes, major dialogue turns, discoveries, plot advances); compare LLM-extracted events vs annotations; compute precision/recall/F1
  - **Key event types**: Combat outcomes, NPC introductions, quest updates, location changes, major player decisions, item acquisitions
  - **Threshold**: DM completeness rating >= 4/5 OR F1 >= 0.80
- Users use recaps at session start (70%+ adoption after 5 sessions)

## Related Features

- Session management (existing) - Source data
- Chat history (existing) - Primary input
- NPC memory (proposed) - Featured NPCs
- Export system (partial) - Output formats

## Recommendation

**Medium priority.** Useful quality-of-life feature but doesn't drive immersion like personality/voice/maps. Implement after higher-impact features. Good candidate for stretch goals or community requests.
