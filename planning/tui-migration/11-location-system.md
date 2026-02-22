# 11 — Location System

**Gap addressed:** #8 (MISSING — no segment)

## Overview

`core/location_gen/` (3 files) + `core/location_manager.rs` providing procedural and LLM-enhanced location generation with hierarchical management.

## Key Types

**LocationType** — 35+ variants: Tavern, Inn, Shop, Dungeon, Forest, Mountain, City, Town, Village, Castle, Tower, Tomb, Mine, Stronghold, Lair, Portal, Underwater, Aerial, etc.

**Location:**
- Basic: id, campaign_id, name, location_type, description, atmosphere, tags, notes, timestamps
- Features: notable_features (interactive/hidden with mechanical effects)
- Inhabitants: NPCs with role, disposition (Friendly/Neutral/Wary/Hostile/Varies), secrets, services
- Secrets: difficulty to discover, consequences, clues
- Encounters: trigger conditions, difficulty, rewards, optional flag
- Connections: linked locations via doors, paths, roads, stairs, portals, secrets, water, climb, flight
- Loot: treasure level (None/Poor/Modest/Average/Rich/Hoard/Legendary)

**Atmosphere:** lighting, sounds, smells, mood, weather, time_of_day_effects

## Generation Modes

1. **Quick generation** (`generate_quick()`) — template-based, instant. Generates name, description, atmosphere, features, inhabitants, secrets, encounters, loot based on type.
2. **Detailed generation** (`generate_detailed()`) — LLM-enhanced. Builds prompt, calls LLM, parses JSON response, falls back to quick generation on failure.

## Location Manager

CRUD + hierarchical tracking:
- Save/get/update/delete locations
- Campaign indexing (campaign_id → location_ids)
- Connection management with auto-cleanup on deletion
- Spatial relationships (connections are typed: path, portal, stairs, etc.)

## TUI Requirements

1. **Location browser** — list with search/filter by type, campaign
2. **Location generator** — type selector (35+ types) + quick/detailed mode toggle
3. **Location detail view** — rich display of description, atmosphere, inhabitants, secrets
4. **Inhabitant manager** — add/edit NPCs with disposition, services
5. **Connection editor** — add/remove connections to other locations with type
6. **Encounter manager** — trigger conditions, difficulty, rewards
7. **Map reference** — grid coords, floor number display
