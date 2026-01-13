# Requirements: Project Cleanup and Pruning

## Introduction
The current project structure contains significant clutter, including large binary files, scattered documentation, and abandoned research artifacts. This hampers maintainability and bloats the repository. This initiative aims to prune these unnecessary files and restructure the project for better organization.

## Requirements

### Requirement 1: Planning Directory Organization
**User Story:** As a developer, I want the `planning` directory to be organized so that I can easily find relevant documentation without wading through abandoned research.

#### Acceptance Criteria
1. WHEN the `planning` directory is viewed THEN it SHALL NOT contain loose research dumps in the root or `embeddings` subdirectory.
2. WHEN legacy research is identified THEN it SHALL be moved to `planning/archive` or deleted if confirmed safe.
3. WHEN active documentation is identified THEN it SHALL be categorized (e.g., `planning/docs`, `planning/active`).

### Requirement 2: Source Code Cleanliness
**User Story:** As a developer, I want the `src-tauri` directory to contain only source configuration and code, so that the build environment is clean.

#### Acceptance Criteria
1. WHEN the `src-tauri` directory is checked THEN it SHALL NOT contain markdown documentation files (e.g., `IPC_IMPLEMENTATION.md`).
2. WHEN temporary build artifacts (like `cargo_check_output.txt`) are present THEN they SHALL be removed and added to `.gitignore`.

### Requirement 3: Repository Size Optimization
**User Story:** As a developer, I want to remove large binaries from the version control to reduce repository size and improve clone times.

#### Acceptance Criteria
1. WHEN the `frontend` directory is checked THEN it SHALL NOT contain the `tailwindcss` binary (121MB).
2. WHEN the `tailwindcss` build process is run THEN it SHALL use `npx` or a managed dependency instead of a committed binary.

### Requirement 4: Legacy Dependency Pruning
**User Story:** As a developer, I want to remove unused or legacy dependencies to reduce compile times and security surface area.

#### Acceptance Criteria
1. IF `pdf-extract` is confirmed as unused legacy code THEN it SHALL be removed from `Cargo.toml` and source code.
2. IF `kreuzberg` is redundant THEN it SHALL be evaluated for removal.

## Non-Functional Requirements
- **Safety**: No active code or required documentation shall be deleted.
- **Build Integrity**: The project must still build and run successfully after changes.

## Constraints and Assumptions
- The `tailwindcss` binary was likely committed for convenience; removing it assumes the user has `npm` access to fetch it.
- `planning` artifacts are assumed to be mostly "read-only" research at this point.
