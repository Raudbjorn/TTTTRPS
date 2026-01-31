# Requirements: Campaign Generation & Management Overhaul

## Introduction

The Campaign Generation & Management system is the cornerstone of the TTRPG Assistant (Sidecar DM) application. Currently, the system provides basic campaign CRUD operations, session tracking, and combat management. However, the campaign **creation** experience lacks guided assistance, and the system does not leverage the rich content available in indexed rulebooks and flavour sources.

This overhaul transforms campaign creation from a manual, form-filling exercise into an **interactive, AI-assisted collaborative process**. The Game Master (GM) engages in a structured conversation with the LLM assistant to craft bespoke campaigns, balanced parties, rich character backstories, and interconnected NPCs—all informed by the actual rules and lore from their indexed source material.

The business value is threefold:
1. **Reduced prep time** - GMs spend less time on mechanical party balancing and world-building scaffolding
2. **Richer narratives** - Deep, interconnected backstories emerge from AI-assisted generation
3. **Rules accuracy** - Suggestions are grounded in actual rulebook content, not hallucinated mechanics

### Requirements Summary (High-Level)

The 21 detailed requirements below map to 9 conceptual requirement areas:

| Concept | Detailed Requirements | Summary |
|---------|----------------------|---------|
| **R1. Guided Creation** | 1, 15 | Multi-step guided flow with structured and conversational inputs |
| **R2. AI-Assisted Dialogue** | 2, 11 | Context-aware questioning, grounded suggestions, adaptive behavior |
| **R3. Party Composition** | 3 | System-aware suggestions, gap analysis, archetype options |
| **R4. Character & NPC Generation** | 4, 5 | Deep backgrounds, relationships, recursive stat blocks, lore grounding |
| **R5. Arc and Session Planning** | 6, 10 | Structured arcs with milestones, session plans aligned with party/narrative |
| **R6. Content Grounding** | 7, 8, 20 | Hybrid search, citations, trust levels |
| **R7. Persistence and Recovery** | 9, 21 | Auto-save, crash recovery, versioning, progressive commitment |
| **R8. Visual Hierarchy** | 12, 16, 17, 18 | GM-only vs narrative, quick reference cards, progressive disclosure, cheat sheets |
| **R9. Randomization and Recaps** | 13, 14 | Random tables, roll tracking, recap generation |

*CampaignIntent (Req 19) is foundational to R2, R4, R5.*

---

## Requirements

### Requirement 1: Interactive Campaign Creation Wizard

**User Story:** As a Game Master, I want an interactive wizard that guides me through campaign creation step-by-step, so that I can create a well-structured campaign without forgetting essential elements.

#### Acceptance Criteria

1. WHEN user initiates campaign creation THEN system SHALL present a multi-step wizard interface with progress indication
2. WHEN wizard is active THEN system SHALL collect the following core information in sequence:
   - Campaign name and description
   - Game system selection (D&D 5e, Pathfinder 2e, etc.)
   - Campaign scope (one-shot, short arc, full campaign, ongoing/no planned end)
   - Target session count (1, 3-5, 10+, or "ongoing")
   - Number of players (1-8)
   - Tone and themes (heroic, dark, comedic, political, etc.)
   - Starting level and expected progression
3. WHEN user completes a wizard step THEN system SHALL persist partial campaign state to prevent data loss
4. WHEN user navigates backward in wizard THEN system SHALL preserve previously entered data
5. WHEN user cancels wizard mid-progress THEN system SHALL prompt for confirmation and offer to save draft
6. WHEN wizard completes THEN system SHALL create the campaign with all collected metadata

### Requirement 2: AI-Assisted Campaign Conversation

**User Story:** As a Game Master, I want to have a guided conversation with the AI assistant about my campaign concept, so that I can explore and refine ideas interactively rather than filling static forms.

#### Acceptance Criteria

1. WHEN user enables "AI-Assisted Mode" in campaign wizard THEN system SHALL initiate a structured conversation flow
2. WHEN conversation is active THEN system SHALL ask clarifying questions based on previous answers:
   - IF user selects "mystery" theme THEN system SHALL ask about red herrings, clue distribution, and revelation pacing
   - IF user selects "ongoing" campaign THEN system SHALL ask about overarching threats and faction dynamics
   - IF user specifies a setting THEN system SHALL query indexed flavour sources for relevant lore
3. WHEN AI generates suggestions THEN system SHALL cite source material when available (rulebook page, flavour source section)
4. WHEN user accepts an AI suggestion THEN system SHALL incorporate it into campaign data with source attribution
5. WHEN user rejects or modifies an AI suggestion THEN system SHALL record the decision and adapt future suggestions
6. WHILE conversation is active THEN system SHALL maintain context across the entire campaign creation session
7. WHEN conversation reaches natural completion THEN system SHALL summarize decisions and offer to proceed to party creation

### Requirement 3: Balanced Party Suggestions

**User Story:** As a Game Master, I want the system to suggest balanced party compositions based on player count and game system, so that I can help my players create a mechanically viable group.

#### Acceptance Criteria

1. WHEN user specifies player count THEN system SHALL generate party composition suggestions appropriate for that count
2. WHEN generating party suggestions THEN system SHALL consult indexed rulebooks for:
   - Class/role definitions and capabilities
   - Recommended party balance guidelines
   - System-specific party composition advice
3. WHEN party suggestion is generated THEN system SHALL display:
   - Suggested roles/classes with rationale
   - Gaps in party capability if any (e.g., "no dedicated healer")
   - Alternative compositions for different playstyles
4. IF player count is 1-2 THEN system SHALL suggest sidekick rules, gestalt options, or companion systems from rulebooks
5. IF player count is 6+ THEN system SHALL warn about potential pacing issues and suggest encounter scaling
6. WHEN user requests a specific party archetype THEN system SHALL suggest compositions matching that archetype:
   - "Combat-focused" → martial-heavy compositions
   - "Intrigue-focused" → skill-monkey and face characters
   - "Exploration-focused" → utility and survival specialists
7. WHEN party composition is finalized THEN system SHALL store it as campaign metadata for encounter balancing

### Requirement 4: Character Background Generation

**User Story:** As a Game Master, I want to generate deep, interconnected backgrounds for player characters, so that they have rich hooks into the campaign world from session one.

#### Acceptance Criteria

1. WHEN user requests character background generation THEN system SHALL prompt for:
   - Character name and basic concept
   - Class/race (if known)
   - Personality keywords or traits
   - Desired connection strength to campaign (light, moderate, deep)
2. WHEN generating background THEN system SHALL produce:
   - Personal history (origin, formative events, turning points)
   - Personality traits, ideals, bonds, flaws (system-appropriate)
   - At least 2 NPC connections (family, mentor, rival, enemy)
   - At least 1 unresolved plot hook
   - At least 1 secret (known to GM, optionally to player)
3. WHEN background includes NPCs THEN system SHALL create corresponding NPC stubs in the campaign
4. WHEN background includes locations THEN system SHALL offer to add them to campaign locations
5. WHEN multiple character backgrounds are generated THEN system SHALL offer to create inter-character connections:
   - Shared history options
   - Conflicting goals that create tension
   - Complementary backstory elements
6. WHEN background generation uses lore THEN system SHALL cite flavour source material
7. WHEN user edits generated background THEN system SHALL update linked NPCs and locations accordingly

### Requirement 5: NPC Generation with Depth

**User Story:** As a Game Master, I want to generate NPCs with personality, motivations, secrets, and relationships, so that my world feels alive and NPCs have consistent behavior.

#### Acceptance Criteria

1. WHEN user requests NPC generation THEN system SHALL prompt for:
   - NPC role (merchant, guard, villain, quest-giver, etc.)
   - Importance level (minor, supporting, major)
   - Location/faction association (optional)
   - Desired traits (optional)
2. WHEN generating NPC THEN system SHALL produce based on importance level:
   - **Minor**: Name, role, 1-2 personality traits, basic appearance
   - **Supporting**: Above + motivation, 2-3 relationships, a secret, voice/speech pattern
   - **Major**: Above + detailed backstory, multiple secrets, plot involvement, **Self-Contained Stat Block**
3. WHEN generating Stat Block THEN system SHALL resolve all cross-references (spells, traits, items) into inline descriptions (no "see PHB p.123")
4. WHEN NPC is associated with faction THEN system SHALL align personality and goals with faction values
5. WHEN NPC has secrets THEN system SHALL categorize them:
   - Plot-relevant secrets (advance main story)
   - Character secrets (personal drama)
   - World secrets (reveal lore/setting information)
6. WHEN NPC is created THEN system SHALL suggest relationships to existing campaign entities
7. WHEN multiple NPCs exist THEN system SHALL track relationship web and prevent contradictions

### Requirement 6: Session Planning & Running (The "Control Panel")

**User Story:** As a Game Master, I want a "Control Panel" view for running sessions that mimics a well-designed two-page spread, keeping narrative flow and mechanical tools visible simultaneously without page-flipping.

#### Acceptance Criteria

1. WHEN user creates session plan THEN system SHALL prompt for:
   - Session goals (what should be accomplished)
   - Expected duration
   - Pacing preference (action-heavy, roleplay-heavy, balanced)
2. WHEN session is active THEN system SHALL display a **Two-Column Dashboard**:
   - **Narrative Column (Left)**: Current scene description, "Boxed Text" (read-aloud), dialogue prompts, and story beats.
   - **Mechanical Column (Right)**: Active NPC stat blocks, initiative tracker, quick-reference rules, and pinned random tables.
3. WHEN generating encounters THEN system SHALL consult:
   - Party composition for appropriate difficulty
   - Campaign arc phase for narrative appropriateness
   - Rulebook monster/encounter guidelines
4. WHEN session plan references plot points THEN system SHALL track advancement toward arc milestones
5. WHEN session is completed THEN system SHALL prompt for:
   - Actual events summary
   - Plot points advanced/revealed
   - New NPCs introduced
   - Player decisions with consequences
6. WHEN multiple sessions are planned THEN system SHALL display timeline view showing narrative progression

### Requirement 7: Flavour Source Integration

**User Story:** As a Game Master, I want the system to draw upon my indexed setting books and adventure modules, so that generated content is consistent with my chosen setting.

#### Acceptance Criteria

1. WHEN campaign specifies a setting THEN system SHALL prioritize flavour sources matching that setting
2. WHEN generating any content THEN system SHALL search indexed flavour sources using a **hybrid search strategy** (keyword + semantic vector):
   - Relevant lore and worldbuilding details
   - Named NPCs, locations, and factions from source material
   - Setting-specific terminology and naming conventions
3. WHEN flavour source content is used THEN system SHALL:
   - Display source attribution (book title, page/section)
   - Offer to show full context passage
   - Allow user to mark content as "used" to prevent repetition
4. WHEN user adds new flavour sources mid-campaign THEN system SHALL re-index and offer retroactive integration
5. IF no flavour sources match THEN system SHALL fall back to generic generation with clear indication
6. WHEN generating names THEN system SHALL use setting-appropriate naming conventions from flavour sources

### Requirement 8: Rulebook-Grounded Mechanics

**User Story:** As a Game Master, I want mechanical suggestions to be grounded in actual rulebook content, so that I can trust the system's advice is rules-accurate.

#### Acceptance Criteria

1. WHEN system suggests game mechanics THEN system SHALL cite specific rulebook references
2. WHEN party balance is discussed THEN system SHALL reference class features from indexed rulebooks
3. WHEN encounter difficulty is calculated THEN system SHALL use system-specific formulas from rulebooks
4. WHEN conditions or effects are mentioned THEN system SHALL link to rulebook definitions
5. IF rulebook is not indexed THEN system SHALL indicate uncertainty and recommend manual verification
6. WHEN multiple rulebook sources conflict THEN system SHALL present options and ask user preference
7. WHEN house rules exist in campaign THEN system SHALL factor them into mechanical suggestions

### Requirement 9: State Persistence and Recovery

**User Story:** As a Game Master, I want my campaign creation progress and session states to be reliably persisted, so that I never lose work due to crashes or interruptions.

#### Acceptance Criteria

1. WHILE campaign wizard is active THEN system SHALL auto-save progress every 30 seconds
2. WHEN application restarts THEN system SHALL detect incomplete wizard states and offer to resume
3. WHEN AI conversation is in progress THEN system SHALL persist conversation history to database
4. WHEN user explicitly saves THEN system SHALL create timestamped snapshot
5. WHEN data corruption is detected THEN system SHALL attempt recovery from most recent valid snapshot
6. WHEN campaign is modified THEN system SHALL create version entry with change description
7. WHEN user requests rollback THEN system SHALL restore campaign to selected version
8. WHEN session ends unexpectedly THEN system SHALL preserve all unsaved changes in recovery storage

### Requirement 10: Campaign Arc Management

**User Story:** As a Game Master, I want to define narrative arcs with phases and milestones, so that I can track campaign progression and maintain dramatic structure.

#### Acceptance Criteria

1. WHEN user creates campaign arc THEN system SHALL prompt for:
   - Arc type (linear, branching, sandbox, mystery, heist)
   - Number of phases/acts
   - Key milestones per phase
   - Expected session count per phase
2. WHEN arc is defined THEN system SHALL generate:
   - Phase descriptions with dramatic purpose
   - Milestone criteria (what triggers completion)
   - Plot point suggestions per phase
   - Tension curve visualization
3. WHEN session completes THEN system SHALL prompt for milestone updates
4. WHEN milestone is achieved THEN system SHALL:
   - Mark it complete with timestamp
   - Suggest narrative consequences
   - Update phase progress indicator
   - Trigger any dependent plot points
5. WHEN arc type is "branching" THEN system SHALL track multiple possible paths
6. WHEN campaign has multiple arcs THEN system SHALL track parallel progression and intersections

### Requirement 11: AI Persona Adaptation

**User Story:** As a Game Master, I want the AI assistant to adopt a persona and tone appropriate for the chosen game system and setting, so that the assistance feels immersive and thematically consistent.

#### Acceptance Criteria

1. WHEN a game system is selected THEN system SHALL load a corresponding **Personality Profile**:
   - **Tone**: e.g., "Grim & Gritty" (Warhammer), "Heroic High Fantasy" (D&D 5e), "Cosmic Horror" (CoC)
   - **Perspective**: e.g., "Omniscient Narrator", "Rules Lawyer", "Helpful Assistant"
   - **Vocabulary**: System-specific terms (e.g., "Sanity" vs "Morale", "Credits" vs "Gold")
2. WHEN AI generates text THEN it SHALL conform to the active Personality Profile's style guides
3. WHEN user interacts with the assistant THEN the AI SHALL maintain the persona (e.g., a "Keeper of Arcane Lore" for CoC)
4. WHEN generating content THEN system SHALL use the profile to bias creative choices (e.g., suggestion of tentacles in CoC vs dragons in D&D)
5. WHEN user explicitly overrides tone THEN system SHALL respect user preference over default profile

### Requirement 12: Visual Information Hierarchy

**User Story:** As a Game Master, I want the interface to use distinct visual styles for different types of information (narrative, rules, GM secrets), so that I can scan the screen quickly during play without confusion.

#### Acceptance Criteria

1. WHEN displaying content THEN system SHALL use distinct typographic styles for:
   - **Read-Aloud Text**: Boxed, serif font (e.g., "You see a dark cavern...")
   - **GM Secrets/Notes**: Distinct background (e.g., yellow/red), sans-serif, marked "GM ONLY"
   - **Mechanics/Rules**: Monospace or technical font, high contrast
2. WHEN displaying Stat Blocks THEN system SHALL use a self-contained "Card" layout:
   - All spells, abilities, and conditions MUST be inline or hover-expandable (Recursive Stat Blocks)
   - No external page flipping required
3. WHEN displaying Lists/Tables THEN system SHALL optimize for vertical scanning (alternating rows, clear headers)

### Requirement 13: Random Tables and Probability Tools

**User Story:** As a Game Master, I want to create and use random tables with proper dice notation and weighted probabilities, so that I can generate emergent content during play using the classic TTRPG randomization patterns.

#### Acceptance Criteria

1. WHEN user creates random table THEN system SHALL support dice notation:
   - Standard dice: d4, d6, d8, d10, d12, d20, d100
   - Compound dice: 2d6, 3d6, d66 (read as tens/ones)
   - Modifiers: d20+5, 2d6-1
2. WHEN defining table entries THEN system SHALL support:
   - Single roll result (e.g., "1-3: Goblin")
   - Range results (e.g., "01-65: Common, 66-85: Uncommon, 86-100: Rare")
   - Weighted probability visualization showing likelihood
3. WHEN rolling on table THEN system SHALL:
   - Display the dice roll result
   - Highlight the selected entry
   - Support "reroll" and "keep" actions
4. WHEN table result references another table THEN system SHALL support nested/cascading rolls
5. WHEN user requests AI table generation THEN system SHALL produce tables with:
   - Setting-appropriate content
   - Balanced probability distributions
   - No "nothing happens" entries (every result is usable)
6. WHEN table is used THEN system SHALL track roll history for the session

### Requirement 14: Session Recap Generation

**User Story:** As a Game Master, I want the system to automatically generate "Previously On..." recaps from session notes and timeline events, so that I can quickly remind players of past events without extensive prep.

#### Acceptance Criteria

1. WHEN session ends THEN system SHALL prompt for key events to include in recap
2. WHEN generating recap THEN system SHALL produce:
   - **Read-Aloud Version**: 2-3 paragraphs of narrative prose (under 200 words)
   - **Bullet Summary**: Key events, decisions, and consequences
   - **Cliffhanger Hook**: How the last session ended
3. WHEN recap is generated THEN system SHALL distinguish:
   - Facts the players know
   - Secrets players haven't discovered
   - Events players were present for vs. heard about
4. WHEN multiple sessions exist THEN system SHALL support:
   - Recap of last session
   - Recap of story arc so far
   - Full campaign timeline summary
5. WHEN player characters have different knowledge THEN system SHALL allow per-PC recap filtering
6. WHEN recap references NPCs or locations THEN system SHALL link to their entries

### Requirement 15: Interview-Style Campaign Creation

**User Story:** As a Game Master, I want campaign creation to feel like a guided conversation where I discover my campaign through dialogue, not form-filling, so that the creative process feels collaborative and emergent.

#### Acceptance Criteria

1. WHEN user starts campaign creation THEN system SHALL default to conversational mode (form-fill available as fallback)
2. WHEN in conversational mode THEN AI assistant SHALL:
   - Ask one focused question at a time
   - Offer 2-4 suggested answers as chips (with "other" option)
   - Build on previous answers to ask deeper follow-up questions
   - Occasionally offer surprising creative prompts ("What if the villain was actually...?")
3. WHEN gathering campaign basics THEN system SHALL use interview questions like:
   - "What's the one sentence you'd use to pitch this campaign to players?"
   - "Describe a scene that captures the tone you're going for."
   - "Who or what is the big threat? How do they operate?"
4. WHEN user provides vague answers THEN system SHALL ask clarifying questions (not assume)
5. WHEN user seems stuck THEN system SHALL offer:
   - Random inspiration from indexed content
   - "Here's what other GMs have done..." examples
   - "Would you like me to suggest three directions?"
6. WHEN interview reaches natural conclusion THEN system SHALL:
   - Summarize all decisions in structured form
   - Allow editing of any answer
   - Show the campaign as it would appear when created

### Requirement 16: Quick Reference Cards

**User Story:** As a Game Master, I want any entity (NPC, location, item, plot point) to be viewable as a compact "card" that shows essential information at a glance, so that I can reference it quickly during play without navigating away from my current context.

#### Acceptance Criteria

1. WHEN entity is displayed THEN system SHALL offer a "Card View" mode showing:
   - **NPCs**: Name, role, 2-3 key traits, current disposition, stat summary
   - **Locations**: Name, type, atmosphere (1-2 lines), notable features (bullets)
   - **Items**: Name, type, properties, current holder
   - **Plot Points**: Title, status, urgency, next trigger
2. WHEN card is displayed THEN system SHALL fit all essential info without scrolling
3. WHEN card is clicked/tapped THEN system SHALL expand to full detail view
4. WHEN multiple cards are needed THEN system SHALL support:
   - Pinning cards to a "card tray" (persists during session)
   - Arranging cards in a grid or row
   - Maximum 6 pinned cards visible simultaneously
5. WHEN in combat THEN system SHALL auto-display cards for active combatants
6. WHEN user hovers/focuses on entity name in text THEN system SHALL show card preview

### Requirement 17: Progressive Disclosure Interface

**User Story:** As a Game Master, I want the interface to show minimal information by default and reveal more on demand, so that I'm not overwhelmed during play but can drill down when needed.

#### Acceptance Criteria

1. WHEN displaying any complex content THEN system SHALL show:
   - **Level 1 (Default)**: Title/name + most critical detail only
   - **Level 2 (Expanded)**: Full summary with all key fields
   - **Level 3 (Complete)**: Every detail including history and metadata
2. WHEN user clicks/expands content THEN system SHALL animate smoothly between levels
3. WHEN displaying NPC THEN progressive levels SHALL be:
   - L1: Name + role + disposition
   - L2: Above + appearance + motivation + key relationships
   - L3: Above + full backstory + secrets + stat block
4. WHEN displaying scene/location THEN progressive levels SHALL be:
   - L1: Name + atmosphere tag
   - L2: Above + read-aloud text + notable features
   - L3: Above + GM notes + secrets + connected locations
5. WHEN user preference is "compact mode" THEN system SHALL default to L1 everywhere
6. WHEN user preference is "detailed mode" THEN system SHALL default to L2

### Requirement 18: GM Cheat Sheet Generation

**User Story:** As a Game Master, I want to generate a single-page "cheat sheet" for each session containing only what I need to run that session, so that I can reference the most critical information without navigating the full interface.

#### Acceptance Criteria

1. WHEN user requests cheat sheet for session THEN system SHALL generate:
   - Session goals and estimated pacing
   - Active NPCs with card-style summaries
   - Key locations with read-aloud text
   - Plot points that might trigger
   - Relevant rules quick-reference
   - Random table shortcuts
2. WHEN cheat sheet is generated THEN it SHALL fit on one screen (or one printed page)
3. WHEN cheat sheet content exceeds space THEN system SHALL:
   - Prioritize by GM-specified importance
   - Allow manual include/exclude toggles
   - Warn that some content is truncated
4. WHEN session has planned encounters THEN cheat sheet SHALL include:
   - Monster stat cards (self-contained)
   - Initiative pre-roll suggestions
   - Difficulty warnings based on party
5. WHEN user marks items as "always include" THEN system SHALL remember for future sheets
6. WHEN cheat sheet is viewed THEN system SHALL support:
   - Print-friendly view
   - Export to PDF
   - Quick-access during session (floating panel)

### Requirement 19: Campaign Intent (Creative Vision Anchor)

**User Story:** As a Game Master, I want to define my campaign's core creative vision early, so that all AI-generated content maintains a consistent tone, theme, and style throughout the campaign.

#### Acceptance Criteria

1. WHEN user creates a campaign THEN system SHALL capture CampaignIntent including:
   - Core fantasy statement (one sentence: "grim political thriller", "heroic dungeon crawl")
   - Desired player experiences (mystery, power fantasy, tragedy, discovery)
   - Hard constraints (low magic, urban only, PG-13, no character death)
   - Themes to weave through content (corruption of power, found family, redemption)
   - Tone keywords (dark, humorous, epic, intimate, gritty)
   - Content to avoid (graphic violence, romantic subplots, real-world politics)
2. WHEN generating any content (NPCs, sessions, arcs, recaps) THEN system SHALL include CampaignIntent in generation context
3. WHEN generated content contradicts CampaignIntent THEN system SHALL flag the contradiction for GM review
4. WHEN CampaignIntent is not yet defined THEN system SHALL prompt for it before first generation
5. WHEN campaign is active (sessions have occurred) THEN CampaignIntent SHALL be immutable without explicit "migration" action
6. WHEN user requests intent change mid-campaign THEN system SHALL:
   - Warn that tone shift may affect existing content
   - Offer to regenerate affected content with new intent
   - Log the intent change for campaign history

### Requirement 20: Trust Levels for Generated Content

**User Story:** As a Game Master, I want to immediately understand how reliable AI-generated content is, so that I can trust canonical rules but verify creative additions.

#### Acceptance Criteria

1. WHEN content is generated THEN system SHALL assign a trust level:
   - **Canonical** - Directly from indexed rulebooks/sourcebooks (spell stats, monster CR, class features)
   - **Derived** - Logically derived from rules/lore ("a cleric would likely...")
   - **Creative** - Pure AI invention with no source backing
   - **Unverified** - Attempted to cite source but couldn't verify
2. WHEN displaying content THEN system SHALL indicate trust level:
   - Canonical: No indicator (assumed reliable)
   - Derived: Subtle badge, expandable to show reasoning chain
   - Creative: "AI-generated" badge, always visible
   - Unverified: Warning indicator, suggests manual verification
3. WHEN trust level is Canonical or Derived THEN system SHALL include source citation
4. WHEN user filters content THEN system SHALL support filtering by trust level
5. WHEN content is later verified by user THEN system SHALL allow promotion to higher trust level
6. WHEN calculating trust THEN system SHALL use confidence thresholds:
   - Canonical requires ≥95% confidence match to indexed source
   - Derived requires ≥75% confidence in reasoning chain
   - Below 75% defaults to Creative

### Requirement 21: Progressive Commitment (Draft Lifecycle)

**User Story:** As a Game Master, I want generated content to go through stages before becoming "real" campaign data, so that I can iterate freely during creation without accidentally committing unreviewed content.

#### Acceptance Criteria

1. WHEN content is generated THEN it SHALL have status `Draft`
2. WHEN user explicitly approves content THEN status SHALL transition to `Approved`
3. WHEN approved content is used in a session THEN status SHALL auto-transition to `Canonical`
4. WHEN user retcons content THEN status SHALL transition to `Deprecated`
5. WHEN content status is Draft or Approved THEN system SHALL allow free editing
6. WHEN content status is Canonical THEN system SHALL:
   - Warn that content has been used in play
   - Require explicit "retcon" action to edit
   - Log original version for history
7. WHEN multiple drafts exist THEN system SHALL support bulk approval ("approve all")
8. WHEN wizard completes THEN system SHALL prompt review of all Draft content before campaign creation
9. WHEN showing content THEN system SHALL visually distinguish by status:
   - Draft: Dashed border or "draft" watermark
   - Approved: Normal appearance
   - Canonical: Subtle "locked" indicator
   - Deprecated: Strikethrough or muted appearance

---

## Non-Functional Requirements

### Performance

1. WHEN wizard step is submitted THEN system SHALL respond within 500ms for non-AI operations
2. WHEN AI generation is requested THEN system SHALL begin streaming response within 3 seconds
3. WHEN searching flavour sources THEN system SHALL return results within 2 seconds
4. WHEN loading campaign THEN system SHALL display content within 1 second for campaigns with < 100 entities

### Reliability

1. WHEN auto-save triggers THEN system SHALL complete without blocking user interaction
2. WHEN database write fails THEN system SHALL retry with exponential backoff (max 3 attempts)
3. WHEN LLM provider is unavailable THEN system SHALL fall back to alternative provider or offline mode
4. WHEN offline THEN system SHALL queue changes and sync when connection restored

### Usability

1. WHEN user encounters unfamiliar term THEN system SHALL provide tooltip explanation
2. WHEN AI generates content THEN system SHALL provide edit controls for all generated fields
3. WHEN form validation fails THEN system SHALL display specific error message adjacent to field
4. WHEN operation is in progress THEN system SHALL display appropriate loading state

### Security

1. WHEN campaign data is persisted THEN system SHALL encrypt sensitive fields (player real names, custom secrets)
2. WHEN API keys are used THEN system SHALL never include them in logs or error messages
3. WHEN campaign is exported THEN system SHALL strip API keys and sensitive configuration

---

## Constraints and Assumptions

### Constraints

- Must work within existing Tauri v2.1 + Leptos v0.7 architecture
- Must use SQLite for persistence (existing database)
- Must integrate with existing Meilisearch for content search
- LLM providers limited to those already supported (Claude, Gemini, OpenAI, Ollama, Groq, Mistral, Cohere, Together)
- UI must fit within existing 5-panel grid layout

### Assumptions

- Users have at least one rulebook indexed before using party balance features
- Users understand basic TTRPG terminology (session, campaign, NPC, etc.)
- LLM providers are available for AI-assisted features (graceful degradation otherwise)
- Meilisearch service is running for content search features
- Campaign data size will remain under 10MB per campaign for typical use

### Dependencies

- Existing `CampaignManager` for campaign CRUD
- Existing `SessionManager` for session operations
- Existing `LLMRouter` for AI provider abstraction
- Existing `SearchClient` for Meilisearch integration
- Existing arc/phase/milestone types in `core/campaign/`

---

## Glossary

| Term | Definition |
|------|------------|
| **Arc** | A narrative storyline spanning multiple sessions with defined beginning, middle, and end |
| **Boxed Text** | Read-aloud descriptive text for players, typically sensory-focused and under 70 words |
| **Card View** | Compact, single-screen representation of an entity showing essential info at a glance |
| **Cheat Sheet** | Single-page summary of everything needed to run a specific session |
| **Dice Notation** | Standard format for describing random rolls (e.g., 2d6+3, d100, d66) |
| **Flavour Source** | Setting books, adventure modules, and lore documents indexed for search |
| **GM Notes** | Information visible only to the Game Master, not players |
| **Interview Mode** | Campaign creation through conversational Q&A rather than form-filling |
| **Milestone** | A significant story beat or accomplishment within a campaign arc |
| **One-shot** | A single-session complete adventure |
| **Phase** | A subdivision of an arc representing a distinct narrative stage |
| **Plot Point** | A specific story element, quest, or event that can be tracked |
| **Progressive Disclosure** | UI pattern showing minimal info by default, expanding on demand |
| **Quick Reference** | Compact view of entity designed for fast scanning during play |
| **Random Table** | Probability-weighted list of results rolled with dice |
| **Recap** | Summary of previous session(s) for player reminder |
| **Rulebook** | Core rules, supplements, and mechanical reference documents |
| **Self-Contained Stat Block** | Monster/NPC stats where all abilities are inline (no external references) |
| **Session** | A single play session, typically 2-4 hours |
| **Sidekick** | NPC companion rules for small parties (D&D 5e specific) |
| **Two-Page Spread** | Design principle where all info for one task fits in view without navigation |

---

## Open Questions

### Resolved in This Spec
- ~~Session Recap Generation~~ → Now Requirement 14
- ~~Quick reference during play~~ → Now Requirements 16, 17, 18
- ~~Interview vs form-fill creation~~ → Now Requirement 15

### Still Open

1. **Adventure Module Import**: Should the wizard support importing campaign frameworks from published adventures? What formats (PDF text extraction, official digital formats)?

2. **Backstory Conflicts**: How should conflicting AI suggestions be handled when multiple players have backstory conflicts? Options:
   - Flag conflicts for GM resolution
   - Suggest conflict as intentional dramatic tension
   - Require explicit GM approval when conflicts detected

3. **Conversation History Limits**: What is the maximum conversation history length before summarization is needed?
   - Claude: ~100k tokens but degrades at high context
   - Suggestion: Summarize at 50k tokens, archive full history

4. **Cross-Campaign Templates**: Should generated content (NPCs, locations, plot structures) be shareable across the user's own campaigns as templates?
   - "Campaign-specific" vs "reusable" flag needed
   - Note: Public sharing deferred to collaborative features phase

5. **Random Table Scope**: Should random tables be:
   - Campaign-specific only?
   - Personal library across campaigns?
   - Imported from indexed rulebooks automatically?

6. **Cheat Sheet Customization**: How much control should GMs have over cheat sheet layout?
   - Fixed template (simpler) vs drag-and-drop (complex)?
   - Multiple cheat sheet templates for different play styles?

7. **Player Knowledge Tracking**: Should the system track what information each PC has learned?
   - Per-PC "known facts" list
   - Auto-filter recaps by PC knowledge
   - Complexity vs value tradeoff

### Deferred to Collaborative Phase

- **Collaborative Creation**: Multi-GM editing → See `../collaborative-features/`
- **Shared Public Templates**: Community sharing → See `../collaborative-features/`
