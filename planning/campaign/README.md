# Campaign Generation Overhaul - Planning Documents

This directory contains the spec-driven development artifacts for the Campaign Generation & Management Overhaul feature.

## Feature Branch

`feature/campaign-generation-overhaul`

## Documents

| Document | Phase | Purpose |
|----------|-------|---------|
| [Requirements.md](./Requirements.md) | 1 | User stories, acceptance criteria (EARS format), constraints |
| [Design.md](./Design.md) | 2 | Technical architecture, components, data models, APIs |
| [Tasks.md](./Tasks.md) | 3 | Sequenced implementation tasks with requirements traceability |

## Overview

This overhaul transforms campaign creation from a manual form-filling exercise into an **interactive, AI-assisted collaborative process**. The GM engages in structured conversation with the LLM to craft campaigns, balanced parties, and rich character/NPC backstories—all grounded in indexed rulebooks and flavour sources.

### Key Features

1. **Campaign Intelligence Pipeline** - Single loop architecture: Input → Context → Generate → Normalize → Accept
2. **CampaignIntent Anchor** - Stable creative vision that unifies tone across all generation
3. **Interview-Style Campaign Creation** - Conversational discovery process, not form-filling
4. **Trust Levels** - Canonical/Derived/Creative/Unverified markers on all generated content
5. **Progressive Commitment** - Content flows through Draft → Approved → Canonical stages
6. **AI-Assisted Conversation** - Structured dialogue for exploring campaign concepts
7. **Balanced Party Suggestions** - System-aware composition recommendations
8. **Deep Character Backgrounds** - LLM-generated backstories with NPC connections
9. **Rich NPC Generation** - Role-based personalities, motivations, self-contained stat blocks
10. **Session Control Panel** - Two-column dashboard mimicking TTRPG two-page spread design
11. **Content Grounding** - All generation backed by indexed source material with citations
12. **Robust State Persistence** - Auto-save, crash recovery, versioning
13. **Quick Reference Cards** - Compact, pinnable entity views for at-table use
14. **Random Tables** - Probability-aware tables with dice notation support
15. **Session Recaps** - Auto-generated "Previously On..." summaries
16. **GM Cheat Sheets** - Single-page session summaries for quick reference

### Design Principles

These principles are enforced through trait boundaries in the architecture:

1. **Single Draft Truth** - All creation flows operate on the same `PartialCampaign` draft state
2. **AI as Advisor** - AI produces proposals (patches + rationale + citations), never mutates state directly
3. **Grounded Creativity** - Generation is creative but traceable; all output includes trust levels and citations
4. **Progressive Commitment** - Ideas move from Draft → Approved → Canonical intentionally
5. **Extensibility First** - Templates and modules evolve without breaking the core loop

### The Core Invariant

> **AI proposes; the GM decides. Only accepted changes mutate canonical campaign data.**

This is enforced architecturally: `Generator` traits return `ArtifactBundle` (proposals), only `CampaignWriter` can create canonical entities.

### Research Sources

- **MDMAI Codebase** - Analyzed for campaign data models, session management, NPC generation patterns, and ChromaDB persistence
- **Existing TTTRPS Code** - Existing `CampaignManager`, `SessionManager`, arc/phase/milestone types, LLM router

## Methodology

Following [Spec-Driven Development](../../knowledgebase/spec-driven-development/SPEC-DRIVEN-DEVELOPMENT.md):

1. **Requirements** → User stories + EARS acceptance criteria → **Approved?** →
2. **Design** → Architecture + components + data models → **Approved?** →
3. **Tasks** → Sequenced implementation steps → **Implementation**

## Approval Gates

- [ ] **Requirements → Design**: All requirements testable and complete
- [ ] **Design → Tasks**: All requirements addressed, technically feasible
- [ ] **Tasks → Implementation**: All design covered, dependencies sequenced

## Quick Stats

- **21 Requirements** (functional) + 4 NFR categories
- **Campaign Intelligence Pipeline** with 5 stages: Input → Context Assembly → Generation → Normalization → Acceptance
- **12 Trait Boundaries** enforcing architectural invariants (see Design.md § Trait Boundaries)
- **10 Backend Modules**: WizardManager, ConversationManager, ContextAssembler, GenerationOrchestrator, TrustAssigner, AcceptanceManager, RulebookLinker, PartyBalancer, RandomTableEngine, RecapGenerator
- **6 Frontend Components**: GuidedCreationFlow (wizard), ConversationPanel, GenerationPreview, QuickReferenceCards, SessionControlPanel, CheatSheetViewer
- **10 Database Tables**: wizard_states, campaign_intents, conversation_threads, conversation_messages, source_citations, generation_drafts, canon_status_log, party_compositions, random_tables, session_recaps
- **~80 Implementation Tasks** across 9 phases (or 5 conceptual phases: A-E)

## Implementation Roadmap (Simplified View)

| Phase | Goal | Key Deliverables |
|-------|------|------------------|
| **A - Core Loop** | Prove the campaign creation loop | Draft persistence, WizardManager, ConversationManager, basic generation, Creation Workspace UI |
| **B - Core Artifacts** | Artifact generation pipeline | NPC generation, party balancing, arc outlines, session plans |
| **C - Grounding** | Trust and citations | Improved rulebook linking, trust levels, basic contradiction detection |
| **D - GM Tools** | Advanced productivity | Random tables, recaps, cheat sheets, quick reference cards |
| **E - Refinement** | Scale and polish | Template customization, cross-campaign reuse, visualizations |

See [Tasks.md](./Tasks.md) for detailed 9-phase breakdown with ~80 sub-tasks.

## Additional Improvements Identified

Beyond the core requirements, the research and design process identified these potential future enhancements:

### Near-Term (Post-MVP)
1. **Campaign Template Library** - Save and share campaign frameworks as reusable blueprints
2. **Adventure Module Import** - Parse published adventures (PDF/text) into campaign structure
3. **Player Handout Generation** - Create in-world documents, maps, letters, wanted posters
4. **Encounter Calculator** - Real-time difficulty adjustment based on party composition and condition
5. **Rulebook Conflict Resolution** - When sources disagree, prompt GM for preference and remember

### Medium-Term
6. **Relationship Web Visualization** - Interactive NPC/faction relationship graph with zoom/filter
7. **Timeline View** - Visual campaign arc progression with session markers and branching paths
8. **Voice Profile Integration** - TTS with NPC-specific voice parameters for immersion
9. **Cross-Campaign Entity Library** - Reuse NPCs, locations, items across campaigns
10. **In-Session Music/Ambiance Cues** - Tie scenes to audio atmosphere profiles

### Long-Term / Experimental
11. **AI Dungeon Master Mode** - Fully AI-driven one-shot adventures for solo play
12. **Published Adventure Awareness** - Index official adventures and suggest tie-ins

### Deferred: Collaborative Features

Multi-user and collaborative features have been intentionally deferred to focus on single-GM excellence first. See [../collaborative-features/](../collaborative-features/) for:
- Collaborative Campaign Creation (multi-GM editing)
- Player-Facing Companion App
- Live Session Logging (voice transcription)
- Shared Campaign Templates (community hub)

## Next Steps

1. Review Requirements.md for completeness and clarity
2. Validate Design.md against technical constraints
3. Prioritize Tasks.md phases based on user value
4. Begin Phase 1 implementation after approval
