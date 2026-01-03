# Map Generation - Tasks

## Phase 1: Battle Map Foundation

### Backend (Rust)

- [ ] **Define core data structures**
  - [ ] `BattleMap` struct (id, name, width, height, grid_type, cells, metadata)
  - [ ] `MapCell` struct (terrain, elevation, movement_cost, decoration)
    - [ ] `movement_cost: f32` - 1.0 = normal, 2.0 = difficult terrain, f32::INFINITY = impassable
    - [ ] Derive walkable from movement_cost (walkable = cost < INFINITY)
  - [ ] `MapToken` struct (entity_id, entity_type, x, y, size)
  - [ ] `TerrainType` enum (Grass, Stone, Water, DifficultTerrain, Void)
  - [ ] `GridType` enum (Square, Hex)

- [ ] **Database schema**
  - [ ] Create `maps` table migration
  - [ ] Create `map_tokens` table migration (persistent decorations/objects only)
  - [ ] Add indexes for campaign_id lookups

  **Note on token storage:**
  - `map_tokens` table: Persistent map decorations (furniture, traps, loot markers)
  - `CombatState.combatant_positions`: Active combatant positions during combat (source of truth for combat)
  - On combat start: Initialize combatant positions from auto-placement or manual
  - On combat end: Combatant positions discarded; map_tokens persist

- [ ] **Procedural generation algorithms**
  - [ ] Arena generator (simple rectangular room with optional pillars)
  - [ ] L-shaped room generator
  - [ ] Corridor generator (straight, L-bend, T-junction)
  - [ ] Random terrain scatter (rocks, vegetation)

- [ ] **Tauri commands**
  - [ ] `create_map` - Create empty map with dimensions
  - [ ] `generate_battle_map` - Procedural generation with params
  - [ ] `get_map` / `list_maps` - CRUD operations
  - [ ] `update_map` - Save map state
  - [ ] `delete_map`
  - [ ] `export_map_png` - Render to image
  - [ ] `export_map_svg` - Vector export

- [ ] **Map rendering utilities**
  - [ ] Grid coordinate system (square and hex math)
  - [ ] Cell-to-pixel conversion
  - [ ] PNG rendering via `image` crate
  - [ ] SVG generation for vector export

### Frontend (Leptos)

- [ ] **MapCanvas component**
  - [ ] Canvas2D grid rendering
  - [ ] Pan and zoom controls
  - [ ] Cell highlighting on hover
  - [ ] Click-to-select cell

- [ ] **MapToolbar component**
  - [ ] Generation trigger button
  - [ ] Grid size controls
  - [ ] Terrain brush selector
  - [ ] Export buttons (PNG/SVG)

- [ ] **MapGenerator modal**
  - [ ] Map type selector (arena, corridor, room)
  - [ ] Dimension inputs (width, height)
  - [ ] Grid type toggle (square/hex)
  - [ ] Generate preview
  - [ ] Save to campaign

- [ ] **Map library browser**
  - [ ] List maps for campaign
  - [ ] Thumbnail previews
  - [ ] Delete/duplicate actions

---

## Phase 2: Combat Integration

### Backend

- [ ] **Link maps to combat encounters**
  - [ ] Add optional `map_id` to `CombatState`
  - [ ] Store token positions in combat state
  - [ ] Validate token positions against map bounds

- [ ] **Token management commands**
  - [ ] `place_token` - Add combatant to map position
  - [ ] `move_token` - Update position
  - [ ] `remove_token` - Remove from map
  - [ ] `get_tokens_for_map` - List all tokens

- [ ] **Distance calculations**
  - [ ] Square grid distance modes:
    - [ ] Chebyshev (D&D 4e/simple: diagonal = 1)
    - [ ] Manhattan (no diagonals)
    - [ ] D&D 5e alternating (5ft/10ft/5ft/10ft for diagonals)
    - [ ] Configurable per campaign/system
  - [ ] Hex grid distance
  - [ ] Line-of-sight check (basic, no obstacles)

### Frontend

- [ ] **CombatMapPanel component**
  - [ ] Embed MapCanvas in combat tracker
  - [ ] Show initiative order as token labels
  - [ ] Highlight current turn token
  - [ ] Click token to select in initiative

- [ ] **Token interaction**
  - [ ] Drag-and-drop token movement
  - [ ] Right-click context menu (attack, move, etc.)
  - [ ] Show movement range overlay
  - [ ] Distance measurement tool

- [ ] **Combat map quick actions**
  - [ ] "Add all combatants to map" button
  - [ ] Auto-place tokens in starting positions
  - [ ] Clear all tokens

---

## Phase 3: Dungeon Generation

### Backend

- [ ] **BSP tree dungeon generator**
  - [ ] Recursive space partitioning
  - [ ] Room placement within partitions
  - [ ] Corridor connection between rooms

- [ ] **Cellular automata cave generator**
  - [ ] Initial random fill
  - [ ] Smoothing iterations
  - [ ] Flood fill to ensure connectivity

- [ ] **Dungeon features**
  - [ ] Door placement at room entrances
  - [ ] Trap markers
  - [ ] Chest/loot markers
  - [ ] Stairs up/down for multi-floor

- [ ] **Theme presets**
  - [ ] Castle (stone walls, wooden doors)
  - [ ] Cave (irregular walls, natural features)
  - [ ] Sewer (water channels, grates)
  - [ ] Crypt (tombs, coffins, altars)
  - [ ] Temple (pillars, sanctuaries)

- [ ] **Multi-floor support**
  - [ ] Dungeon as collection of floor maps
  - [ ] Stair connections between floors
  - [ ] Floor navigation in UI

### Frontend

- [ ] **DungeonGenerator modal**
  - [ ] Algorithm selector (BSP, cellular)
  - [ ] Theme preset dropdown
  - [ ] Size and room count params
  - [ ] Multi-floor toggle

- [ ] **Dungeon viewer**
  - [ ] Floor selector tabs
  - [ ] Stair connection indicators
  - [ ] Room annotation labels

---

## Phase 4: World Maps (Stretch)

### Backend

- [ ] **Hex grid terrain generation**
  - [ ] Simplex noise for elevation
  - [ ] Biome assignment based on elevation + moisture
  - [ ] River generation (flow downhill)
  - [ ] Coastline detection

- [ ] **Biome types**
  - [ ] Ocean, Lake, River
  - [ ] Plains, Forest, Dense Forest
  - [ ] Hills, Mountains, Snow Peaks
  - [ ] Desert, Swamp, Tundra

- [ ] **Settlement placement**
  - [ ] Poisson disk sampling for distribution
  - [ ] Settlement size categories (village, town, city)
  - [ ] Name generation (tie to future name generator)
  - [ ] Link to Location system (planned feature)

- [ ] **Travel routes**
  - [ ] Road generation between settlements
  - [ ] Pathfinding for travel time calculation
  - [ ] Terrain difficulty modifiers

### Frontend

- [ ] **WorldMapCanvas component**
  - [ ] Hex grid rendering at scale
  - [ ] Biome color palette
  - [ ] Settlement icons
  - [ ] Road/path overlay

- [ ] **WorldMapGenerator modal**
  - [ ] World size presets
  - [ ] Climate parameters
  - [ ] Settlement density
  - [ ] Seed for reproducibility

- [ ] **World map interactions**
  - [ ] Click settlement for details
  - [ ] Draw travel route
  - [ ] Add custom markers (POIs)

---

## Dependencies

- `noise` crate for procedural generation
- `image` crate for PNG export
- Canvas2D via web-sys (existing)

## Testing

- [ ] Unit tests for grid math (distance, coordinates)
- [ ] Unit tests for each generator algorithm
- [ ] Integration tests for map CRUD operations
- [ ] Visual regression tests for rendered maps (optional)
