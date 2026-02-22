# 10 — Character Generation

**Gap addressed:** #7 (MISSING — not a single mention)

## Overview

`core/character_gen/` supports 10+ game systems with system-specific backstory generation and stat rules.

## Supported Game Systems

| System | Module | Notes |
|--------|--------|-------|
| D&D 5e | Built-in | Full class/race/background support |
| Pathfinder 2e | Built-in | Ancestry/heritage/class |
| Call of Cthulhu | Built-in | Occupation-based skills |
| Cyberpunk Red | Built-in | Lifepath + cyberware |
| Shadowrun 6e | Built-in | Priority system + edges |
| Fate Core | Built-in | Aspects + stunts |
| World of Darkness | Built-in | Clan/tribe + merits |
| Dungeon World | Built-in | PbtA moves |
| GURPS | Built-in | Point-buy advantages/disadvantages |
| Warhammer Fantasy | Built-in | Career paths + talents |
| Custom | `GameSystem::Custom(String)` | User-defined systems |

## Key Types

- **Character** — attributes, skills, traits, equipment, background, backstory
- **AttributeValue** — base score + modifier + temp bonuses (D&D-style)
- **CharacterTrait** — 15+ variants: Personality, Background, Racial, Class, Feat, Flaw, Bond, Ideal, Aspect, Stunt, Merit, Edge, Cyberware, Talent, Move, Advantage, Disadvantage
- **Equipment** — Weapon, Armor, Tool, Consumable, Magic, Tech, Vehicle
- **GenerationOptions** — name, concept, race, class, level, point_buy, random_stats, backstory_length, theme, campaign_setting
- **SystemGenerator** trait — generate, validate_options, list races/classes/backgrounds/attributes

## Generation Flow

1. User selects game system
2. `GeneratorRegistry::generate()` resolves system-specific generator
3. Generator validates options via `validate_options()`
4. Generator creates `Character` with system-specific mechanics
5. Optional backstory generation (trait-based narrative)

## TUI Requirements

1. **System selector** — pick from 10+ supported systems
2. **Character wizard** — multi-step form:
   - System → Race/Ancestry → Class/Profession → Background → Attributes → Equipment
   - System-dependent options (each system has different available choices)
3. **Character sheet display** — formatted stat block appropriate to system
4. **Backstory generator** — preview + regenerate narrative backstory
5. **Equipment manager** — add/edit/remove gear
6. **Character browser** — list existing characters with search/filter
7. **Export** — character sheet export/copy
