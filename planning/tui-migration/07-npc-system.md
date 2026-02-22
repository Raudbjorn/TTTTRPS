# 07 — NPC System

**Gap addressed:** #4 (MISSING — entire feature not mentioned)

## Overview

~15 files across `core/npc_gen/` implementing NPC generation with vocabulary banks, cultural naming, dialect transformations, and two conversation modes.

## Module Structure

| Module | Purpose |
|--------|---------|
| `generator.rs` | NPC generation with stats, appearance, personality, relationships, plot hooks |
| `vocabulary.rs` | Frequency-weighted phrase banks by formality (formal/casual/hostile) |
| `names.rs` | Cultural naming system (8 name structures) |
| `dialects.rs` | Phonetic + grammatical transformations (3 intensity levels) |
| `indexes.rs` | Meilisearch indexes for vocabulary, names, exclamations |
| `file_utils.rs` | Async YAML loading for cultural data |

## Key Types

**NPC** — full character profile:
- Appearance: age, height, build, hair, eyes, skin, distinguishing features
- Personality: traits, ideals, bonds, flaws, mannerisms, speech patterns, motivations, fears
- Voice: pitch, pace, accent, vocabulary, sample phrases
- Relationships: directed edges with disposition (-100 to +100)
- Plot hooks: Quest, Rumor, Secret, Conflict, Opportunity, Warning
- Stats: optional Character struct from character_gen
- Role: 12 predefined (Ally, Enemy, Merchant, QuestGiver, Authority, Informant, Rival, Mentor, Minion, Boss, Bystander) + Custom

**Name Structures** (8 patterns):
GivenFamily, FamilyGiven, GivenEpithet, PrefixRootSuffix, ClanDescriptor, Patronymic, Matronymic, SingleName, TitleBased

**Dialect Intensity:** Light, Moderate, Heavy — controls probability of rule application

## Conversation Modes

| Mode | System Prompt | Use Case |
|------|---------------|----------|
| `"about"` | DM assistant | Develop NPC backstory, personality, story hooks |
| `"voice"` | Roleplay as NPC | AI speaks in first person using NPC's voice |

Both modes integrate with the personality system for consistent NPC voice.

## Integration Points

- **Personality:** `NPCVoiceConfig` specifies vocabulary_bank_id, dialect_id, intensity, culture_id, formality
- **Archetype:** NPC roles map to archetypes; vocabulary banks overridable via setting packs
- **Session Manager:** NPC relationships and plot hooks tracked in campaign state
- **Voice:** `VoiceProfile` linked to NPC for speech synthesis
- **Meilisearch indexes:** `ttrpg_vocabulary_banks`, `ttrpg_name_components`, `ttrpg_exclamation_templates`

## TUI Requirements

1. **NPC browser** — list with search/filter by role, campaign, name
2. **NPC generator** — role selection (12 + custom), appearance customization, personality config
3. **NPC card view** — summary showing appearance, personality, sample phrases, relationships
4. **Conversation view** — chat with NPC in "about" or "voice" mode (reuse Chat view pattern)
5. **Voice config** — vocabulary bank selector, dialect picker, intensity slider, culture selection
6. **Relationship editor** — add/edit disposition to other NPCs
7. **Plot hook manager** — attach/edit quest hooks, rumors, secrets
8. **Name generator** — cultural name generation with structure preview
