# 12 — Campaign Generation Pipeline

**Gap addressed:** #10 (MISSING — not addressed)

## Overview

`core/campaign/generation/` (11 files) provides a multi-step AI generation pipeline coordinated by a central orchestrator.

## Orchestrator Pattern

```
GenerationRequest
    │
    ▼
GenerationOrchestrator
    │
    ├── TemplateRegistry (load prompt templates)
    ├── ContextAssembler (gather RAG context)
    ├── LLMRouter (execute generation)
    └── TrustAssigner (score generated content)
    │
    ▼
GenerationResult + Citations
```

## Generation Types

| Type | Module | Output |
|------|--------|--------|
| ArcOutline | `arc_gen.rs` | Arc name, summary, 3-5 phases, antagonist, outcome scenarios |
| CharacterBackground | `character_gen.rs` | LLM-enhanced character backstory |
| Npc | `npc_gen.rs` | NPC with personality, relationships, hidden goals |
| SessionPlan | `session_gen.rs` | Session overview, encounters, plot developments, rewards |
| PartyAnalysis | `party_gen.rs` | Gap analysis, role coverage, recommended additions |
| Location | — | LLM-enhanced location description |
| QuestHook | — | Quest starters with stakes |
| Encounter | — | Combat/social encounters with difficulty |
| Custom | — | Freeform generation with custom template |

## Arc Generation Detail

**ArcDraft:**
- Name, summary, overview (hook, stakes, climax)
- 3-5 ArcPhases, each with: title, description, plot_points, encounters, NPCs, locations, milestone
- Antagonist description
- ArcOutcome scenarios: victory conditions, failure consequences, alternative paths

**Arc types:** Main, Side, Personal

## Party Composition Analysis

**PartyRole:** Tank, Healer, DamageDealer, Support, Controller, Utility, Face, Scout

Analyzes party members (class, level, role) against expected challenges, identifies coverage gaps, suggests additions.

## Template System

- `TemplateRegistry` loads YAML templates from disk, caches them
- 9 template types covering all generation types
- `GenerationTemplate`: system prompt + user prompt template + variable definitions + output schema
- Variable substitution via handlebars-style `{{var}}`

## Context Assembly

- `ContextAssembler` gathers campaign context, party info, previous decisions
- Fetches RAG context from SurrealDB storage
- Token budget-aware context window management

## Campaign Grounding (`core/campaign/grounding/`)

- `CitationBuilder` — attaches source references to generated content
- `FlavourSearcher` — retrieves thematic elements from indexed content
- `RulebookLinker` — links generated text to page/section citations
- `UsageTracker` — tracks token usage and citation frequency

## TUI Requirements (Generation view)

1. **Generation wizard** — step-by-step:
   - Select generation type (arc, NPC, session, party analysis, etc.)
   - Configure options (arc type, themes, party level, etc.)
   - Select/customize prompt template
   - Preview assembled context (what will be sent to LLM)
   - Stream generation with live preview
   - Accept/reject/regenerate result
2. **Template browser** — search/select/edit YAML templates
3. **Context preview** — show RAG context + token budget usage
4. **Citation display** — source references for generated content
5. **Generation history** — list of previous generations with reuse option
6. **Batch generation** — queue multiple generation requests
7. **Arc timeline** — visual representation of generated arc phases
8. **Party dashboard** — role coverage matrix, gap highlights
