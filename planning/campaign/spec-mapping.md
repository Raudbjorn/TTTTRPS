Here’s the “nothing got lost” mapping from your **original modules/components** into the **new spine model** (Campaign Creation Loop + Creation Workspace). I’ll do it in two passes: (1) where each module lives in the spine, (2) what the module’s *contract* becomes in the rewritten model.

---

## 1) Spine mapping: where each original module fits

### Experience layer (Creation Workspace)

| Original thing      | New model home   | What it becomes                                        |
| ------------------- | ---------------- | ------------------------------------------------------ |
| `CampaignWizard`    | Guidance Surface | Step rail + structured fields editing the *same* draft |
| `ConversationPanel` | Dialogue Surface | Interview chat + suggestion patches + citations        |
| `GenerationPreview` | Commit Surface   | Artifact preview/edit + accept/reject/modify           |

### Draft & decision layer (Draft Truth + Decisions)

| Original thing                                  | New model home   | What it becomes                                                        |
| ----------------------------------------------- | ---------------- | ---------------------------------------------------------------------- |
| `WizardManager`                                 | Draft & Decision | Owner of draft lifecycle + step gating + autosave                      |
| `ConversationManager`                           | Draft & Decision | Owner of thread/message persistence + suggestion decisions + branching |
| `SuggestionStatus` (accepted/rejected/modified) | Draft & Decision | The “decision ledger” feeding future prompts + audit trail             |

### Intelligence layer (Context → Grounding → Generation → Normalization)

| Original thing                 | New model home     | What it becomes                                                       |
| ------------------------------ | ------------------ | --------------------------------------------------------------------- |
| `GenerationOrchestrator`       | Generation Engine  | The orchestrator for: context assembly → prompt → streaming → parsing |
| `TemplateRegistry` (YAML)      | Generation Engine  | Prompt rendering + schema expectations (what the model must output)   |
| `RulebookLinker`               | Grounding          | Reference detection + linking + confidence scoring                    |
| `FlavourSearcher` (in tasks)   | Grounding          | Lore/name/location retrieval + setting filters                        |
| `CitationBuilder`              | Normalization      | Turns “linked snippets” into first-class citations                    |
| “Personality Profiles”         | Context Assembly   | Style/tone vocabulary constraints injected into prompts               |
| `PartyBalancer`                | Artifact Generator | A specialized artifact generator (party suggestions + analysis)       |
| Recursive Stat Block Expansion | Normalization      | A post-process that resolves refs into inline content                 |

### Canonical campaign layer (Campaign Truth)

| Original thing               | New model home  | What it becomes                                           |
| ---------------------------- | --------------- | --------------------------------------------------------- |
| `CampaignManager` (extended) | Canonical layer | The only place allowed to mutate canonical campaign state |
| `SessionManager`             | Canonical layer | The only place allowed to mutate session state            |

### Storage layer

| Original thing                  | New model home      | What it becomes                                   |
| ------------------------------- | ------------------- | ------------------------------------------------- |
| `wizard_states`                 | Draft Truth         | persistent draft + progress for crash recovery    |
| `conversation_threads/messages` | Decision ledger     | conversation + suggestion history                 |
| `source_citations`              | Attribution memory  | what sources were used/accepted                   |
| `generation_history`            | Observability       | what was generated, when, and what happened to it |
| `party_compositions`            | Canonical metadata  | saved party plans for balancing                   |
| Meilisearch                     | Grounding substrate | hybrid search for rules + lore                    |

### “Advanced GM tools” (still fit cleanly as artifacts)

| Original thing              | New model home                 | What it becomes                                          |
| --------------------------- | ------------------------------ | -------------------------------------------------------- |
| `RandomTableEngine`         | Artifact Generator + Canonical | tables + rolling + session roll history                  |
| `RecapGenerator`            | Artifact Generator             | summaries derived from events/notes + style templates    |
| `CheatSheetBuilder`         | Artifact Renderer              | transforms canonical data into compact session reference |
| `QuickReferenceCardManager` | Artifact Renderer              | standard card views + pinned sets per session            |

---

## 2) Contract mapping: how each module’s responsibilities change under the new model

This is the important part: the new spec’s invariant is **AI proposes, GM decides, CampaignManager commits**. So each module has a tighter contract.

### WizardManager (draft owner)

**Old vibe:** “wizard state machine + persistence + completion.”
**New contract:** *the authoritative draft editor*.

* Owns `PartialCampaign` draft and step gating.
* Applies “patches” from:

  * forms,
  * accepted conversation suggestions,
  * accepted generation results.
* Never writes canonical campaign entities directly; it hands off to `CampaignManager` only on completion.

### ConversationManager (decision ledger)

**Old vibe:** “persist chat + branch + track acceptance.”
**New contract:** *the system’s memory of deliberation*.

* Stores messages and suggestions.
* Records decisions (accepted/rejected/modified).
* Provides a compact “decision summary” used in context assembly (“don’t suggest X again”).
* Still supports branching, but branching is now explicitly **exploration of alternatives** that don’t affect canonical state until accepted.

### GenerationOrchestrator (pipeline owner)

**Old vibe:** “template + LLM + grounding + streaming.”
**New contract:** *a deterministic pipeline with structured outputs*.

* Input: `DraftSnapshot + Purpose + Constraints + RecentDecisionSummary`.
* Output: `(streamed_text, structured_suggestions[], artifacts[], citations[])`.
* The orchestrator does not “apply” anything—ever. It only proposes.

### RulebookLinker / FlavourSearcher / CitationBuilder (grounding subsystem)

**Old vibe:** “citations are first-class.”
**New contract:** *grounding is a trust system*.

* Produce citations + confidence.
* Optionally provide “trust level” classification for UI (canonical/derived/creative/unverified).
* Track “used content” to avoid repetitive citations.

### PartyBalancer (artifact generator)

**Old vibe:** “party suggestions + gap analysis.”
**New contract:** *an artifact generator that can operate with or without the LLM*.

* Can run purely on rules heuristics if offline.
* Can optionally enrich suggestions via grounded LLM generation.
* Produces: `PartySuggestion[] + PartyAnalysis + citations`.

### RandomTableEngine / RecapGenerator / CheatSheetBuilder / QuickReferenceCardManager

**Old vibe:** “extra subsystems.”
**New contract:** *artifact generators/renderers fed by canonical campaign truth*.

* They consume canonical campaign state (and session state) and produce compact outputs.
* They only mutate canonical state when explicitly saving (e.g., storing a table definition, saving a recap).

---

## 3) Compatibility matrix: what stays identical vs what you’d rename/reframe

### Stays essentially identical

* Most structs and tables (`WizardState`, `ConversationThread`, `Suggestion`, `Citation`) are already aligned.
* Tauri streaming approach stays the same.
* YAML templates stay the same.

### Needs reframe (not rewrite)

* “Wizard vs Conversation” becomes “two inputs to one draft.”
* “Generation results” should be treated as *patches + artifacts*, not freeform text that magically becomes data.

### One suggested rename (purely conceptual)

* `WizardManager` → **CreationFlowManager** (optional)

  * Not required, but it matches the unified UX model and reduces confusion.

---

## 4) The one rule that makes the whole system sane

If you adopt only one structural rule, make it this:

> The draft (`PartialCampaign`) is the only mutable truth during creation.
> Canonical campaign state is only mutated through explicit acceptance and commit steps.

That single constraint prevents state fragmentation across Wizard, Chat, and Generation.


