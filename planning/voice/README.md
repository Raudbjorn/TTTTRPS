# Voice Module Extraction Plan

This directory contains the specification documents for extracting voice-related Tauri commands from `commands_legacy.rs` into the organized `commands/voice/` module structure.

## Documents

| Document | Purpose | Status |
|----------|---------|--------|
| [Requirements.md](./Requirements.md) | Formal requirements using EARS format | ✅ Complete |
| [Design.md](./Design.md) | Technical architecture and component specs | ✅ Complete |
| [Tasks.md](./Tasks.md) | Sequenced implementation tasks | ✅ Complete |

## Quick Reference

### Scope
- **Source**: `src-tauri/src/commands_legacy.rs` (~1,400 lines of voice code)
- **Target**: `src-tauri/src/commands/voice/` (8 submodules)
- **Commands**: ~56 Tauri commands total

### Module Structure

```
commands/voice/
├── mod.rs              # Module declarations + re-exports
├── config.rs           # 4 commands - Provider configuration
├── synthesis.rs        # 6 commands - Core TTS operations
├── providers.rs        # 6 commands - Provider installation
├── presets.rs          # 3 commands - Built-in DM presets
├── profiles.rs         # 6 commands - User voice profiles
├── cache.rs            # 6 commands - Audio cache management
├── queue.rs            # 3 commands - Basic voice queue
└── synthesis_queue.rs  # 22 commands - Advanced priority queue
```

### Key Issues to Fix
1. **Secret persistence** (`config.rs`): API keys written to disk in plaintext
2. **Fallback path** (`providers.rs`): `get_models_dir()` falls back to CWD
3. **Queue race condition** (`queue.rs`): Multiple concurrent processors possible
4. **TOCTOU race** (`queue.rs`): Item selection and claim not atomic

### Implementation Order
1. Module structure preparation
2. Extract presets (3 commands)
3. Extract profiles (6 commands)
4. Extract cache (6 commands)
5. Extract synthesis queue (22 commands)
6. Verify existing extracted commands
7. Fix known issues
8. Update main.rs registration
9. Remove code from commands_legacy.rs
10. Documentation and verification

## Getting Started

```bash
# Create feature branch (already done)
git checkout feature/voice-module-extraction

# After implementing, verify build
cd src-tauri && cargo check

# Run tests
cargo test

# Check for warnings
cargo clippy -- -D warnings
```

## Related Files

- **Core voice logic**: `src-tauri/src/core/voice/`
- **Existing skeleton**: `src-tauri/src/commands/voice/`
- **Legacy code**: `src-tauri/src/commands_legacy.rs` (lines ~2400-6600)
- **Command registration**: `src-tauri/src/main.rs`

## Methodology

This specification follows the [Spec-Driven Development](../spec-driven-development/SPEC-DRIVEN-DEVELOPMENT.md) methodology:

1. **Requirements** (EARS format) → Define testable acceptance criteria
2. **Design** → Technical architecture and component interfaces
3. **Tasks** → Sequenced implementation steps with traceability

Each phase must be reviewed before proceeding to implementation.
