# Visual Character Portraits - Feature Evaluation

## Status: PROPOSED

**Priority:** MEDIUM-HIGH - Visual immersion, but high complexity with AI generation

## Overview

Add visual portraits for player characters and NPCs through AI generation, image upload, or curated library. Portraits appear in character sheets, combat tracker, chat, and NPC cards.

## Why This Feature

- **Visual identity**: Faces create emotional connection
- **Quick recognition**: Combat tracker benefits from visual tokens
- **Player investment**: Characters feel more real with portraits
- **NPC consistency**: Same face across sessions

## Feature Scope

### Tier 1: Manual Portraits (MVP)
- Upload image for character/NPC
- Crop/resize tool
- Display in character sheet, combat tracker, chat bubbles
- Default silhouette by race/class

### Tier 2: AI Portrait Generation
- Generate from text description (race, class, age, features)
- Style options: realistic, painterly, anime, pixel art
- Regenerate with tweaks
- Save favorites to library

### Tier 3: Advanced Features
- Expression variants (happy, angry, injured, dead)
- Token generation from portraits (circular, framed)
- Consistent style across campaign
- Portrait evolution (aging, scars, equipment changes)

## Technical Approach

### Image Generation Options

**Option A: Local Stable Diffusion (Experimental/Aspirational)**
- **Pros**: Free, offline, customizable
- **Cons**: Requires GPU, large models (~2-7GB), quality varies
- **Integration**: Via ComfyUI API or diffusers-rs
- **Feasibility caveats**:
  - Arc A770 (16GB) can run quantized models, but 7GB+ full-precision models may strain inference memory
  - oneAPI/IPEX support for ComfyUI and diffusers-rs is immature with limited optimizations
  - Generation times may exceed 30 seconds on Arc without heavy optimization work
  - Consider INT8 quantization or CPU fallback for broader hardware support
- **Recommendation**: Treat local SD as optional experimental fallback; cloud-hosted SD (Stability API) as primary generation option

**Option B: Cloud APIs**
- **Pros**: High quality, no local resources
- **Cons**: Cost per image, latency, content policies
- **Services**: DALL-E 3, Midjourney (unofficial), Leonardo.ai, Stability AI
- **Cost**: ~$0.02-0.08 per image

**Option C: Hybrid**
- User choice: local (free) or cloud (quality)
- Fall back to upload for users without either

### Portrait Prompt Engineering

Character data -> structured prompt:

```
Portrait of a [race] [class], [age] years old.
Physical: [height], [build], [skin tone], [hair], [eyes], [distinguishing features]
Expression: [default expression]
Style: [fantasy portrait, painterly, detailed, dramatic lighting]
Bust shot, facing slightly left, dark background.
```

### Implementation Plan

### Phase 1: Manual Portraits
- Add `portrait_url` field to characters and NPCs
- Image upload with validation (size, format)
- Crop/resize component (use existing image crate)
- Display portraits in existing UI components
- Default avatars by race/class

### Phase 2: Generation Integration
- Portrait generator modal with form fields
- Text description -> prompt builder
- Provider selection (local SD / cloud API)
- Generation preview with regenerate option
- Save to character/NPC

### Phase 3: Polish
- Token export (circular crop for VTT)
- Batch generation for NPC groups
- Style consistency settings per campaign
- Portrait gallery/library

## Database Changes

```sql
-- Add to existing tables
ALTER TABLE characters ADD COLUMN portrait_url TEXT;
ALTER TABLE characters ADD COLUMN portrait_prompt TEXT;

ALTER TABLE npcs ADD COLUMN portrait_url TEXT;
ALTER TABLE npcs ADD COLUMN portrait_prompt TEXT;

-- Portrait library for reuse
CREATE TABLE portrait_library (
    id TEXT PRIMARY KEY,
    campaign_id TEXT REFERENCES campaigns(id),
    name TEXT,
    image_path TEXT NOT NULL,
    prompt TEXT,
    tags TEXT, -- JSON array: ["human", "warrior", "male"]
    created_at TEXT
);
```

## Data Structures

```rust
pub struct PortraitConfig {
    pub race: String,
    pub class: Option<String>,
    pub age: Option<String>,
    pub gender: Option<String>,
    pub skin_tone: Option<String>,
    pub hair: Option<String>,
    pub eyes: Option<String>,
    pub distinguishing_features: Vec<String>,
    pub expression: String,
    pub style: PortraitStyle,
}

pub enum PortraitStyle {
    Realistic,
    Painterly,
    Anime,
    PixelArt,
    Sketch,
}

pub struct GeneratedPortrait {
    pub image_data: Vec<u8>,
    pub prompt: String,
    pub seed: Option<u64>,
    pub provider: String,
}
```

## UI Components

- `PortraitUploader.rs` - Upload + crop interface
- `PortraitGenerator.rs` - AI generation wizard
- `PortraitDisplay.rs` - Responsive portrait component
- `PortraitLibrary.rs` - Campaign portrait gallery

## Effort Estimate

| Phase | Complexity | Notes |
|-------|------------|-------|
| Manual Portraits | Low | Upload + display |
| AI Generation | High | Provider integration, prompt tuning |
| Polish | Medium | Token export, consistency |

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| AI generates inappropriate content | Content filters, style constraints |
| Inconsistent character appearance | Save seed, use same prompt base |
| Generation costs add up | Local option, generation limits |
| Copyright concerns | Prohibit prompts referencing copyrighted characters/artists (e.g., "like Artgerm", "Geralt of Rivia"); prompt filtering with blocked descriptors; require user TOS acceptance for generated content; log flagged prompts for review; obtain legal review before launch |
| GPU requirements for local | Cloud fallback, upload option |

## Success Metrics

- 70%+ characters have portraits within 3 sessions
- Portrait generation < 30 seconds
- User satisfaction with visual quality

## Related Features

- Character system (existing) - Portrait storage
- NPC system (existing) - Portrait storage
- Combat tracker (existing) - Token display
- Chat interface (existing) - Speaker portraits
- Map generation (proposed) - Token integration

## Recommendation

**Medium-high priority, but start with Tier 1.** Manual upload provides immediate value with minimal complexity. AI generation is a separate phase that can be added based on user demand. The user's Arc A770 makes local generation feasible, which is a nice advantage.

Consider implementing after map generation, as portraits naturally become tokens on maps.
