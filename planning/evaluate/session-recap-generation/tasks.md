# Session Recap Generation - Tasks

## Phase 1: Basic Summary

### Backend (Rust)

- [ ] **Database schema**
  - [ ] Create `session_recaps` table migration
  - [ ] Add indexes for session_id and campaign_id

- [ ] **Core data structures**
  - [ ] `SessionRecap` struct
  - [ ] `RecapStyle` enum (BulletPoints, Narrative, Dramatic, Comedic)
  - [ ] `RecapRequest` for generation params

- [ ] **Transcript processing**
  - [ ] Collect chat history for session
  - [ ] Chunk transcript by message count or token limit
  - [ ] Extract speaker information
  - [ ] Filter out system messages:
    - [ ] Application-generated status messages (combat start/end, dice rolls)
    - [ ] Out-of-character messages (prefixed with "OOC:" or bracketed [OOC])
    - [ ] System prompts and LLM instructions
    - [ ] Error messages and warnings
    - [ ] Define `is_system_message(msg)` predicate with configurable rules

- [ ] **Multi-stage summarization**
  - [ ] Stage 1: Summarize each chunk independently
  - [ ] Stage 2: Combine chunk summaries
  - [ ] Stage 3: Extract key events list
  - [ ] Stage 4: Format final output

- [ ] **Key event extraction**
  - [ ] Combat outcomes (who won, casualties)
  - [ ] NPC introductions
  - [ ] Quest updates (accepted, completed, failed)
  - [ ] Location changes
  - [ ] Major player decisions
  - [ ] Item acquisitions

  **Phase 1 approach (MVP):** Use LLM-based extraction via structured prompt (see Prompt Engineering section). While this has cost/latency, it provides the best accuracy for identifying narrative significance. Simpler regex/keyword approaches miss context-dependent events.

  **Future optimization:** Cache extraction results per chunk; only re-extract on session edit.

- [ ] **Tauri commands**
  - [ ] `generate_session_recap(session_id, style)` - Create recap
  - [ ] `get_session_recap(session_id)` - Retrieve existing
  - [ ] `regenerate_recap(session_id, style)` - New generation
  - [ ] `update_recap(recap_id, content)` - Edit recap
  - [ ] `delete_recap(recap_id)`

### Frontend (Leptos)

- [ ] **RecapGenerator component**
  - [ ] Session selector (if generating for past session)
  - [ ] Style dropdown
  - [ ] Generate button
  - [ ] Progress indicator during generation

- [ ] **RecapViewer component**
  - [ ] Markdown rendering of recap
  - [ ] Edit mode toggle
  - [ ] Regenerate button
  - [ ] Copy to clipboard

- [ ] **Session detail integration**
  - [ ] "Generate Recap" button in session panel
  - [ ] Show existing recap if available
  - [ ] Recap badge on session list

---

## Phase 2: Style Options

### Backend

- [ ] **Style-specific prompts**
  - [ ] Bullet points: Concise, factual, organized
  - [ ] Narrative: Prose style, story flow
  - [ ] Dramatic: Heightened language, tension
  - [ ] Comedic: Light-hearted, highlight funny moments

- [ ] **Character perspective mode**
  - [ ] Filter events to character's knowledge
  - [ ] First-person narration option
  - [ ] Character voice matching

- [ ] **Metadata inclusion**
  - [ ] Session date and duration
  - [ ] Characters present
  - [ ] NPCs featured
  - [ ] Locations visited

- [ ] **Unresolved threads extraction**
  - [ ] Identify open quests
  - [ ] Note unanswered questions
  - [ ] Flag cliffhangers

### Frontend

- [ ] **StyleSelector component**
  - [ ] Style preview descriptions
  - [ ] Style comparison view
  - [ ] Remember last used style

- [ ] **CharacterPerspective selector**
  - [ ] Character dropdown
  - [ ] Preview perspective before generating

- [ ] **RecapMetadata component**
  - [ ] Display session info
  - [ ] Featured NPCs list
  - [ ] Unresolved threads section

---

## Phase 3: Export & Archive

### Backend

- [ ] **Export formats**
  - [ ] Markdown export (default)
  - [ ] Plain text export
  - [ ] HTML export (for PDF via print)
  - [ ] JSON export (structured data)

- [ ] **Campaign chronicle**
  - [ ] Aggregate all recaps for campaign
  - [ ] Generate chapter structure
  - [ ] Table of contents
  - [ ] Cast of characters (NPCs across sessions)

- [ ] **Tauri commands**
  - [ ] `export_recap(recap_id, format)` - Single recap
  - [ ] `export_campaign_chronicle(campaign_id, format)`
  - [ ] `get_campaign_recaps(campaign_id)` - List all

### Frontend

- [ ] **RecapExport modal**
  - [ ] Format selector
  - [ ] Include metadata toggle
  - [ ] Download button
  - [ ] Copy to clipboard option

- [ ] **CampaignChronicle component**
  - [ ] Timeline view of all sessions
  - [ ] Expandable recap previews
  - [ ] "Export All" button
  - [ ] Search within recaps

- [ ] **ChronicleViewer component**
  - [ ] Book-like reading experience
  - [ ] Chapter navigation
  - [ ] Character index with appearances

---

## Prompt Engineering

- [ ] **Base recap prompt**
  ```
  You are a skilled fantasy narrator creating a session recap.

  SESSION: [name] - Session #[number]
  DATE: [date]
  CHARACTERS: [list]

  TRANSCRIPT SUMMARY:
  [chunk summaries]

  Generate a [style] recap that:
  1. Opens with a "last time" hook
  2. Covers major plot developments
  3. Highlights memorable moments
  4. Notes unresolved threads
  5. Ends with a teaser

  Keep under [word_limit] words.
  ```

- [ ] **Style variations**
  - [ ] Bullet: "Use concise bullet points, one per event"
  - [ ] Narrative: "Write flowing prose as if telling a story"
  - [ ] Dramatic: "Emphasize tension, conflict, and stakes"
  - [ ] Comedic: "Highlight humor, use light tone"

- [ ] **Key event extraction prompt**
  ```
  Extract key events from this session transcript.
  Categories: combat, dialogue, discovery, decision, quest, travel
  Format: JSON array with type, description, importance (1-5)
  ```

---

## Performance Optimization

- [ ] **Caching**
  - [ ] Cache chunk summaries
  - [ ] Invalidate on session edit
  - [ ] Pre-compute on session end (optional)

- [ ] **Token management**
  - [ ] Track token usage per generation
  - [ ] Chunk size tuning for model limits
  - [ ] Truncate very long sessions intelligently

---

## Dependencies

- LLM client (existing)
- Chat history storage (existing)
- Session management (existing)

## Testing

- [ ] Unit tests for transcript chunking
- [ ] Unit tests for key event extraction parsing
- [ ] Integration tests for recap CRUD
- [ ] Manual evaluation of recap quality (human rubric: coherence, coverage, style, faithfulness; optionally ROUGE/BLEU if reference recaps exist)
- [ ] Test with various session lengths
