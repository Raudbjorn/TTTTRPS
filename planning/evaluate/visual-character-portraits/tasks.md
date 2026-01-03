# Visual Character Portraits - Tasks

## Phase 1: Manual Portraits

### Backend (Rust)

- [ ] **Database schema updates**
  - [ ] Add `portrait_url` column to `characters` table
  - [ ] Add `portrait_url` column to `npcs` table
  - [ ] Add `portrait_prompt` column (for AI regeneration)

- [ ] **Portrait storage**
  - [ ] Define portraits directory: `data/portraits/`
  - [ ] File naming convention: `{entity_type}_{id}_{hash}.{ext}`
    - [ ] `hash`: SHA256 of image content (first 12 chars) - prevents duplicate storage and enables cache-busting on frontend
  - [ ] Supported formats: PNG, JPG, WebP

- [ ] **Image processing**
  - [ ] Image validation (format, dimensions, file size)
  - [ ] Resize to standard dimensions (e.g., 256x256, 512x512)
  - [ ] Crop to square aspect ratio
  - [ ] Thumbnail generation for lists

- [ ] **Tauri commands**
  - [ ] `upload_portrait(entity_type, entity_id, image_data)` - Save portrait
  - [ ] `get_portrait(entity_type, entity_id)` - Retrieve portrait URL
  - [ ] `delete_portrait(entity_type, entity_id)` - Remove portrait
  - [ ] `crop_portrait(image_data, x, y, width, height)` - Crop image

- [ ] **Default avatars**
  - [ ] Generate silhouette by race (human, elf, dwarf, etc.)
  - [ ] Color variation by class or role
  - [ ] Bundle default avatar set

### Frontend (Leptos)

- [ ] **PortraitDisplay component**
  - [ ] Responsive image display
  - [ ] Fallback to default avatar
  - [ ] Loading placeholder
  - [ ] Size variants (small, medium, large)

- [ ] **PortraitUploader component**
  - [ ] Drag-and-drop upload
  - [ ] File picker button
  - [ ] Format validation feedback
  - [ ] Size limit warning

- [ ] **PortraitCropper component**
  - [ ] Interactive crop area
  - [ ] Maintain square aspect ratio
  - [ ] Preview before save
  - [ ] Zoom/pan controls

- [ ] **Character sheet integration**
  - [ ] Portrait display area
  - [ ] Click to upload/change
  - [ ] Remove portrait option

- [ ] **NPC card integration**
  - [ ] Portrait in card header
  - [ ] Edit portrait on NPC detail view

- [ ] **Combat tracker integration**
  - [ ] Token-style portraits in initiative
  - [ ] Small circular portraits

- [ ] **Chat integration**
  - [ ] Speaker portrait next to messages
  - [ ] NPC portrait when NPC speaks

---

## Phase 2: AI Generation

### Backend

- [ ] **Provider abstraction**
  - [ ] `PortraitProvider` trait
  - [ ] Provider selection logic
  - [ ] Error handling and fallback

- [ ] **Cloud provider integration (primary)**
  - [ ] Stability AI API client
  - [ ] DALL-E 3 API client (optional)
  - [ ] Request/response handling
  - [ ] Cost tracking per generation

- [ ] **Local Stable Diffusion (experimental)**
  - [ ] ComfyUI API client
  - [ ] Model configuration
  - [ ] Queue management for slow generation
  - [ ] Fallback to cloud if local fails
  - [ ] **User documentation for ComfyUI setup:**
    - [ ] Installation guide (ComfyUI + dependencies)
    - [ ] Required models (SD 1.5/SDXL recommendations)
    - [ ] API mode configuration (`--listen --port 8188`)
    - [ ] In-app setup wizard with connection test
    - [ ] Troubleshooting common issues

- [ ] **Prompt building**
  - [ ] Character data -> structured prompt
  - [ ] Race-specific descriptors
  - [ ] Class-specific elements (armor, weapons, robes)
  - [ ] Style modifiers (realistic, painterly, anime)
  - [ ] Negative prompts (avoid common issues)

- [ ] **Prompt template**
  ```
  Portrait of a [race] [class], [age] years old, [gender].
  Physical: [build], [skin_tone], [hair_color] [hair_style], [eye_color] eyes.
  Features: [distinguishing_features].
  Expression: [expression].
  Style: fantasy character portrait, [style], detailed, dramatic lighting.
  Bust shot, slight angle, dark background.
  ```

- [ ] **Content filtering**
  - [ ] Block copyrighted character references
  - [ ] Block artist name references
  - [ ] Prompt sanitization
  - [ ] Flag and log suspicious prompts

- [ ] **Tauri commands**
  - [ ] `generate_portrait(config)` - AI generation
  - [ ] `regenerate_portrait(entity_id, seed?)` - New generation
  - [ ] `get_generation_cost_estimate(provider)` - Cost preview
  - [ ] `list_portrait_providers()` - Available providers

### Frontend

- [ ] **PortraitGenerator modal**
  - [ ] Form fields from character data (pre-filled)
  - [ ] Style selector dropdown
  - [ ] Provider selector (cloud/local)
  - [ ] Cost estimate display
  - [ ] Generate button with loading state

- [ ] **GenerationPreview component**
  - [ ] Display generated image
  - [ ] Accept/reject buttons
  - [ ] Regenerate with tweaks
  - [ ] Seed display for reproducibility

- [ ] **PortraitConfig form**
  - [ ] Race/class (from character)
  - [ ] Physical attributes
  - [ ] Expression selector
  - [ ] Style selector
  - [ ] Custom prompt additions

- [ ] **Settings integration**
  - [ ] Default provider selection
  - [ ] API key configuration
  - [ ] Local SD endpoint URL
  - [ ] Generation budget/limits

---

## Phase 3: Polish

### Backend

- [ ] **Portrait library**
  - [ ] Save generated portraits to library
  - [ ] Tag portraits for reuse
  - [ ] Search library by tags
  - [ ] Share portraits across campaigns
  - [ ] **Storage strategy:**
    - [ ] Local storage in `data/portraits/library/` (default)
    - [ ] Portraits stored as files with metadata in SQLite `portrait_library` table
    - [ ] Future consideration: Optional cloud sync for multi-device (out of scope for MVP)

- [ ] **Token export**
  - [ ] Circular crop for VTT tokens
  - [ ] Border/frame options
  - [ ] Size presets (VTT standard sizes)
  - [ ] Batch export for party

- [ ] **Style consistency**
  - [ ] Save generation parameters per campaign
  - [ ] Apply consistent style to batch generations
  - [ ] Seed management for similar looks

- [ ] **Tauri commands**
  - [ ] `save_to_library(portrait_data, tags)`
  - [ ] `search_library(tags)`
  - [ ] `export_token(portrait_id, format, size)`
  - [ ] `batch_generate_portraits(npc_ids)`

### Frontend

- [ ] **PortraitLibrary component**
  - [ ] Grid view of saved portraits
  - [ ] Tag filtering
  - [ ] Search by name/tags
  - [ ] Apply portrait to character/NPC

- [ ] **TokenExport modal**
  - [ ] Border style selector
  - [ ] Size preset buttons
  - [ ] Preview with border
  - [ ] Download button

- [ ] **BatchGenerator component**
  - [ ] Select multiple NPCs
  - [ ] Consistent style settings
  - [ ] Progress indicator
  - [ ] Review all before saving

- [ ] **CampaignStyleSettings component**
  - [ ] Default portrait style
  - [ ] Seed/consistency preferences
  - [ ] Provider preferences

---

## Content Safety

- [ ] **Blocked descriptors list**
  - [ ] Copyrighted characters (Gandalf, Geralt, etc.)
  - [ ] Artist names (Artgerm, Greg Rutkowski, etc.)
  - [ ] Trademarked terms

- [ ] **Prompt validation**
  - [ ] Check against blocked list
  - [ ] Sanitize before sending to provider
  - [ ] Log rejected prompts

- [ ] **User agreement**
  - [ ] TOS acceptance for AI generation
  - [ ] Attribution requirements display
  - [ ] Content ownership disclaimer

---

## Dependencies

- `image` crate for processing
- `reqwest` for API calls (existing)
- Cloud provider API keys (user-provided)
- Optional: ComfyUI for local generation

## Testing

- [ ] Unit tests for image validation
- [ ] Unit tests for prompt building
- [ ] Integration tests for upload/retrieve cycle
- [ ] Test blocked descriptor filtering
- [ ] Manual testing of generation quality
- [ ] Test fallback when provider unavailable
