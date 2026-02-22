# 08 — Personality System

**Gap addressed:** #5 (MISSING — no segment)

## Overview

14 files in `core/personality/` implementing a 4-phase system: foundation types → template storage → weighted blending → contextual integration.

## Module Structure

**Phase 1 — Foundation:**
- `types.rs` — Newtype IDs (TemplateId, PersonalityId, BlendRuleId), document types
- `errors.rs` — Structured errors for templates, blends, context detection
- `context.rs` — `GameplayContext` enum: CombatEncounter, SocialInteraction, Exploration, Investigation, LoreExposition, Downtime, RulesClarification, Unknown
- `context_keywords.rs` — Weighted keyword detector for auto-context switching

**Phase 2 — Templates:**
- `templates.rs` — `SettingTemplate` with validation rules
- `template_store.rs` — Meilisearch-backed CRUD + LRU cache (100 capacity)
- `template_loader.rs` — YAML file loading (built-in + user directory)

**Phase 3 — Blending:**
- `blender.rs` — `PersonalityBlender` with weighted interpolation, LRU cache (100 entries)
- `context_detector.rs` — Keyword analysis + session state signals
- `blend_rules.rs` — Context-based blend rule storage

**Phase 4 — Integration:**
- `contextual.rs` — `ContextualPersonalityManager` combining all components
- `application.rs` — UI layer types (NarrativeTone, VocabularyLevel, NarrativeStyle)

## Key Types

**PersonalityProfile** (from `personality_base`):
- Traits with numerical intensities (0-10)
- Speech patterns (vocabulary, formality, accent, rate)
- Behavioral tendencies (stubbornness, sociability, trustworthiness)
- Tone scores (dramatic, humorous, dark, etc.) — normalized to sum=1.0

**BlendSpec** — weighted combination of personalities:
```
BlendSpec::new(vec![
    BlendComponent { personality_id, weight: 0.6 },
    BlendComponent { personality_id, weight: 0.4 },
])
```
- Numeric fields: weighted average
- Categorical fields: highest-weight component wins
- List fields: proportional selection (2x multiplier)
- Tone scores: normalized post-blend

**GameplayContext** — auto-detected from conversation:
- Keyword analysis with confidence scoring (0.0-1.0)
- Session state signals: `combat_active`, `initiative_count`, `downed_characters`, `active_npc_id`, `scene_tag`
- History smoothing (last 5 detections)
- Combat boost: +0.3 when combat signals present

**NarrativeTone** — 10 options: Neutral, Dramatic, Casual, Mysterious, Humorous, Epic, Gritty, Whimsical, Horror, Romantic

**VocabularyLevel** — 5 levels: Simple, Standard, Elevated, Archaic, Technical

## Context Detection Flow

```
Chat message text → KeywordDetector → ┬─ Keyword confidence
                                        │
Session state (combat, NPCs) ──────────┤
                                        │
Last 5 detections ─────────────────────┤
                                        ▼
                                   GameplayContext + confidence
                                        │
                                   BlendRule lookup
                                        │
                                   Adjusted personality weights
```

## Setting Templates

Builder pattern for setting-specific DM personality:
```
SettingTemplate::builder("Forgotten Realms Sage", "storyteller")
    .game_system("dnd5e")
    .vocabulary("ancient texts", 0.05)
    .common_phrase("As the annals of Candlekeep record")
    .deity_reference("Mystra")
    .build()
```

Validation presets: minimal(), lenient(), strict(), default().
Stored in Meilisearch with full-text search across name, vocabulary, phrases.
YAML import/export supported.

## TUI Requirements (Personality view)

1. **Profile selector** — browse/search personality profiles
2. **Blend composer** — multi-slider for blending weights between profiles
3. **Blend preview** — show resulting personality traits after blending
4. **Context rule editor** — define blend adjustments per gameplay context
5. **Context indicator** — live display of detected context + confidence
6. **Template browser** — search/select setting templates, YAML import
7. **Template editor** — create/edit templates with vocabulary, phrases, deity references
8. **Tone selector** — NarrativeTone dropdown with preview
9. **Vocabulary level** — slider/dropdown for output complexity
10. **Narrative style** — perspective selection (first/second/third person)
