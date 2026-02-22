# 09 — Archetype System

**Gap addressed:** #6 (MISSING — no segment)

## Overview

11 files in `core/archetype/` plus command files implementing a hierarchical resolution system with setting packs, vocabulary integration, and dependency-based cache invalidation.

## Resolution Hierarchy

```
Priority (lowest to highest):
  Role (10)     → e.g., "merchant"
  Race (20)     → e.g., "dwarf"
  Custom (25)   → user-defined
  Class (30)    → e.g., "fighter"
  Setting (40)  → setting pack override
  Direct ID     → explicit lookup (final precedence)
```

Resolution merges archetype properties up the chain — higher priority overrides lower.

## Key Types

**Archetype:**
- `id`: ArchetypeId (validated: starts with letter/underscore, alphanumeric+dash+underscore)
- `category`: Role | Race | Class | Setting | Custom
- `parent_id`: optional inheritance
- `personality_affinity`: trait weights (0.0-1.0) + default intensity (1-10)
- `npc_role_mappings`: role → probability weight
- `naming_cultures`: culture_id → selection weight
- `stat_tendencies`: optional character stat guidance
- `vocabulary_banks`: IDs for NPC dialogue generation

**SettingPack:**
- `id`, `name`, `game_system`, `version` (semver)
- `archetype_overrides`: per-archetype modifications (add/replace affinity, override stats)
- `custom_archetypes`: setting-scoped new archetypes
- `vocabulary_overrides`: per-bank modifications
- `naming_cultures`: custom cultural naming rules

**SettingPack lifecycle:** Load YAML → Validate → Register → Activate per campaign

## Integration Points

- **PersonalityBlender** → `blend_for_archetype("dwarf_merchant")` retrieves affinity weights
- **NPCGenerator** → `npc_context_for_archetype("guard")` retrieves role mappings, stat tendencies
- **NameGenerator** → `naming_context_for_archetype("elf")` retrieves cultural weights

## Cache System

LRU cache with 256 capacity, 3600s TTL:
- Dependency tracking: archetype ID changes invalidate affected entries
- Query component tracking (role, race, class, setting)
- Campaign-scoped entry tracking
- Stale-while-revalidate option

**Lock-free resolution pattern:**
1. Collect all inheritance chain IDs in single lock
2. Release lock
3. Resolve each archetype individually (no lock held)

## TUI Requirements

1. **Archetype browser** — list/search by category (Role/Race/Class/Setting/Custom)
2. **Resolution visualizer** — show inheritance chain for a given query (Role → Race → Class → Setting)
3. **Setting pack manager** — load/activate/deactivate YAML packs per campaign
4. **Archetype editor** — modify affinity weights, role mappings, culture selections
5. **Vocabulary bank viewer** — browse phrases by category/formality
6. **Cache stats** — hit rate, entry count, invalidation history
