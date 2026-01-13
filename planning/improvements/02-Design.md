# Design: Project Cleanup and Pruning

## Overview
This design outlines the specific file operations and configuration changes needed to meet the cleanup requirements. The focus is on non-destructive moves where possible (archiving) and removing clearly regeneratable or unused binary artifacts.

## Architecture

### Directory Structure Changes

#### Planning Directory
The `planning` directory will be reorganized to separate active documentation from legacy research.

- **`planning/archive/` (NEW)**
  - Destination for: `embeddings`, `docling`, `ragflow`, `paddleNLP`, `open-parse`, `legacy_project`.
- **`planning/docs/` (NEW)**
  - Destination for docs currently in `src-tauri` (e.g., `IPC_IMPLEMENTATION.md`).

#### Source Directory (`src-tauri`)
- Removel: `IPC_IMPLEMENTATION.md`, `SECURITY_IMPLEMENTATION.md`, `PERFORMANCE_OPTIMIZATION.md` (move to `planning/docs`).
- Gitignore: Add `cargo_check_output.txt`.

#### Frontend Directory (`frontend`)
- Removal: `tailwindcss` binary.
- Replacement: Ensure `package.json` has `tailwindcss` dev dependency, or use `npx`.

## Components and Interfaces

### Tailwind CSS Build Process
- **Current**: Relies on a checked-in binary `./tailwindcss`.
- **New**: Use `npx tailwindcss` or `npm run build:css`.
  - **Action**: Verify `package.json` scripts. If `tailwindcss` script exists, ensure it uses the npm version.

### Dependency Management
- **PDF Extraction**:
  - Remove `pdf-extract` (legacy) from `Cargo.toml`.
  - Verify `kreuzberg` usage. If it's the core PDF engine now, keep it. Ideally, remove if `docling` is the new standard, but `docling` seems to be in `planning/research`, implying it's not fully integrated yet.
  - **Decision**: Keep `kreuzberg` for now as it seems active. Remove `pdf-extract`.

## Review & Verification Strategy

### Verification Steps
1. **Build Check**: Run `./build.sh` or `cargo build` to ensure no broken paths.
2. **Frontend Check**: Run `npm run build` in `frontend` to ensure Tailwind still compiles.
3. **File Check**: Verify `planning` directory structure looks clean.

### Safety
- "Archiving" instead of "Deleting" for the `planning` folder content ensures we don't lose potentially valuable research.
