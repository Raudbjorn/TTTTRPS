# Tasks: Project Cleanup and Pruning

## Implementation Overview
This task list covers the reorganization of the `planning` directory, cleanup of `src-tauri`, and optimization of the `frontend` directory.

## Implementation Plan

- [ ] 1. Organization of Planning Directory
    - [ ] 1.1 Create `planning/archive` and `planning/docs` directories
    - [ ] 1.2 Move legacy research folders to `planning/archive`
        - Move: `embeddings`, `docling`, `ragflow`, `paddleNLP`, `open-parse`, `legacy_project`
    - [ ] 1.3 Move documentation from `src-tauri` to `planning/docs`
        - Move: `IPC_IMPLEMENTATION.md`, `SECURITY_IMPLEMENTATION.md`, `PERFORMANCE_OPTIMIZATION.md`
    - [ ] 1.4 Move root `planning` files to `planning/docs` if appropriate
        - Move: `IPC_IMPLEMENTATION.md` (if duplicate exists), `PERFORMANCE_OPTIMIZATION.md` (if duplicate exists)
        - _Requirements: 1.1, 1.2, 1.3_

- [ ] 2. Source Code Cleanup
    - [ ] 2.1 Clean `src-tauri` root
        - Remove `cargo_check_output.txt`
        - Add `cargo_check_output.txt` to `.gitignore`
    - [ ] 2.2 Prune dependencies
        - Remove `pdf-extract` from `src-tauri/Cargo.toml`
        - _Requirements: 2.1, 2.2, 4.1_

- [ ] 3. Frontend Optimization
    - [ ] 3.1 Remove large binaries
        - Remove `frontend/tailwindcss`
    - [ ] 3.2 Verify Tailwind build script
        - Check `frontend/package.json` for build script
        - Update to use `npx tailwindcss` if necessary
        - _Requirements: 3.1, 3.2_

- [ ] 4. Verification
    - [ ] 4.1 Verify Build
        - Run `cargo check` in `src-tauri`
        - Run `npm run build` in `frontend`

