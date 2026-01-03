# Map Generation - Feature Evaluation

## Status: PROPOSED

**Priority:** HIGH - User requested, natural extension of immersion stack

## Overview

Add procedural and AI-assisted map generation for battle encounters, dungeon exploration, and world/region visualization. Maps integrate with existing session/combat system.

## Why This Feature

- **Immersion alignment**: Personality + Voice already proven; visual maps complete the sensory experience
- **Gameplay utility**: Combat encounters need spatial context for tactical decisions
- **DM productivity**: Reduce prep time with procedural generation
- **User demand**: Explicitly requested by project owner

## Feature Scope

### Tier 1: Battle Maps (MVP)
- Grid-based tactical maps (square/hex)
- Terrain types: grass, stone, water, difficult terrain, elevation
- Simple procedural generation (arena, corridor, room)
- Token placement for combatants (tie to existing combat tracker)
- Fog of war (optional)
- Export to PNG/SVG

### Tier 2: Dungeon Maps
- Room + corridor generation (BSP, cellular automata, graph-based)
- Door/trap/chest markers
- Multi-floor support with stairs/ladders
- Theme presets: cave, castle, sewer, crypt, temple
- Integration with session notes (discovered rooms)

### Tier 3: World/Region Maps
- Hex-based overworld generation
- Terrain biomes: forest, mountain, desert, ocean, plains
- Settlement markers with names (tie to location system)
- Travel route visualization
- Weather overlay (ties to future weather system)

## Technical Approach

### Option A: Pure Rust Generation (Recommended)
- **Pros**: No external deps, works offline, fast, integrates cleanly with Tauri
- **Cons**: More initial dev work
- **Libraries**: `noise` (procedural), `image` (rendering), custom SVG generation
- **Pattern**: Generate data model -> render to canvas/SVG in frontend

### Option B: AI Image Generation
- **Pros**: High visual quality, flexible styles
- **Cons**: Requires API (cost, latency), inconsistent sizing, harder to edit
- **Services**: DALL-E, Midjourney API, Stable Diffusion (local via Ollama)
- **Use case**: Better for world maps, worse for tactical grids

### Option C: Hybrid
- Procedural for battle/dungeon (needs precision)
- AI-generated for world maps and artistic backdrops

## Implementation Plan

### Phase 1: Battle Map Foundation
- Define `BattleMap` struct: grid, cells, tokens, metadata
- Implement basic procedural generators (arena, L-shaped room)
- Create Leptos canvas component for rendering
- Tauri commands: `generate_battle_map`, `export_map`, `save_map`

### Phase 2: Combat Integration
- Link maps to combat encounters (CombatState gets optional map_id)
- Token placement reflects initiative order
- Click-to-move tokens (update positions)
- Distance calculation for range attacks

### Phase 3: Dungeon Generation
- BSP tree dungeon generator
- Corridor connection algorithm
- Theme-based tile sets
- Room annotation support

### Phase 4: World Maps (Stretch)
- Hex grid terrain generation
- Simplex noise for biome distribution
- Settlement placement with Poisson disk sampling
- Export for VTT platforms

## Database Changes

```sql
CREATE TABLE maps (
    id TEXT PRIMARY KEY,
    campaign_id TEXT REFERENCES campaigns(id),
    name TEXT NOT NULL,
    map_type TEXT NOT NULL, -- 'battle', 'dungeon', 'world'
    width INTEGER,
    height INTEGER,
    grid_type TEXT, -- 'square', 'hex'
    data BLOB NOT NULL, -- JSON: cells, tokens, decorations
    thumbnail BLOB,
    created_at TEXT,
    updated_at TEXT
);

CREATE TABLE map_tokens (
    id TEXT PRIMARY KEY,
    map_id TEXT REFERENCES maps(id),
    entity_id TEXT, -- NPC or character ID
    entity_type TEXT, -- 'npc', 'character', 'object'
    x INTEGER,
    y INTEGER,
    size TEXT, -- 'small', 'medium', 'large', 'huge'
);
```

## UI Components

- `MapCanvas.rs` - WebGL/Canvas2D grid renderer
- `MapToolbar.rs` - Generation controls, export, token tools
- `MapGenerator.rs` - Generation wizard modal
- `CombatMapPanel.rs` - Integrated view in combat tracker

## Dependencies

**Rust:**
- `noise = "0.9"` - Simplex/Perlin noise for terrain
- `image = "0.25"` - PNG export
- `svg = "0.17"` - SVG generation (optional)

**Frontend:**
- Canvas2D API (already available via web-sys)
- Optional: WebGL for larger maps

## Effort Estimate

| Phase | Complexity | Notes |
|-------|------------|-------|
| Battle Maps | Medium | Core feature, most value |
| Combat Integration | Medium | Builds on existing combat tracker |
| Dungeon Generation | Medium-High | Algorithms well-documented |
| World Maps | High | Stretch goal |

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Performance with large maps | Use WebGL, tile-based rendering, viewport culling |
| Procedural maps look generic | Multiple algorithms + theming + decoration layers |
| Token management complexity | Start simple (position only), add features incrementally |

## Success Metrics

- Battle map generation < 500ms
- Users create maps for 50%+ of combat encounters
- Positive feedback on tactical utility

## Related Features

- Combat tracker (existing) - Token positions
- Session notes (existing) - Map annotations
- Location system (planned) - World map markers
- NPC system (existing) - Token entity linking

## Recommendation

**Start with Tier 1 (Battle Maps)** - Highest utility-to-effort ratio. The combat tracker already exists; adding spatial context is the natural next step. World maps are impressive but less critical for gameplay.
