# Leptos Migration Overview

This document provides the rationale and high-level plan for migrating the frontend from Dioxus 0.7 to Leptos 0.7.

## Document Information

| Field | Value |
|-------|-------|
| Version | 1.1.0 |
| Created | 2026-01-01 |
| Updated | 2026-01-01 |
| Status | Draft |

---

## 1. Executive Summary

This migration replaces the Dioxus frontend framework with Leptos while preserving:
- All 69+ Tauri backend commands (unchanged)
- Complete type safety between frontend and backend
- Single-language (Rust) codebase
- Existing Tailwind + themes.css styling system
- All current application features

---

## 2. Motivation

### 2.1 Why Migrate from Dioxus?

| Concern | Dioxus 0.7 | Leptos 0.7 |
|---------|------------|------------|
| **API Stability** | Breaking changes between versions | More stable, mature API |
| **Documentation** | Decent, gaps in advanced topics | Excellent, comprehensive |
| **Community Size** | Growing | Larger, more active |
| **Ecosystem** | Smaller component library | More community components |
| **Performance** | Good | Excellent (often benchmark winner) |
| **Tooling** | `dx` CLI (improving) | `cargo-leptos` (mature) |
| **SSR Support** | Limited | First-class (not needed for Tauri, but indicates maturity) |

### 2.2 Why Stay in Rust (vs. SvelteKit)?

| Factor | Rust/Leptos | TypeScript/SvelteKit |
|--------|-------------|----------------------|
| **Type Safety** | Full Rust types end-to-end | TypeScript (good, not Rust-level) |
| **Shared Types** | Same structs frontend + backend | Must duplicate or generate |
| **Shared Logic** | Direct import of validation, etc. | Reimplement in TS |
| **Language Context** | Single language | Split Rust + TypeScript |
| **Compile-time Guarantees** | Maximum | Good |
| **Team Skills** | Rust-focused | Requires TS expertise |

### 2.3 Preserved Advantages

By choosing Leptos over a web framework:

1. **Type Safety**: `bindings.rs` types remain Rust structs, shared with backend
2. **Single Language**: No context switching, no type duplication
3. **Shared Validation**: Same validation logic frontend and backend
4. **No FFI Boundary Issues**: Rust-to-Rust is seamless
5. **Compile-time Errors**: Catch UI bugs at compile time

---

## 3. Current Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Tauri Application                         │
├─────────────────────────────────────────────────────────────┤
│  Frontend (WASM)           │  Backend (Rust)                │
│  ┌───────────────────────┐ │  ┌──────────────────────────┐  │
│  │ Dioxus 0.7            │ │  │ Tauri Commands (69+)     │  │
│  │ - #[component] macro  │ │  │ - LLM, Voice, Search     │  │
│  │ - use_signal()        │ │  │ - Campaigns, Sessions    │  │
│  │ - rsx! macro          │ │  │ - Characters, NPCs       │  │
│  │ - Dioxus Router       │ │  │ - Document Ingestion     │  │
│  └───────────────────────┘ │  └──────────────────────────┘  │
│            │               │             │                   │
│  ┌─────────▼─────────────┐ │             │                   │
│  │ bindings.rs (928 LOC) │◄┼─────────────┘                   │
│  │ - wasm-bindgen FFI    │ │  IPC via window.__TAURI__       │
│  │ - Type-safe wrappers  │ │                                 │
│  └───────────────────────┘ │                                 │
└─────────────────────────────────────────────────────────────┘
```

### Current Frontend Stats

| Metric | Value |
|--------|-------|
| Total Component LOC | ~4,500 |
| Page Components | 6 |
| Layout Components | 4 (MainShell, IconRail, MediaBar, DragHandle) |
| Design System Components | 12+ |
| Services | 2 (LayoutState, ThemeService) |
| Views (ViewType) | 5 (Campaigns, Chat, Library, Graph, Settings) |
| IPC Bindings | ~1,150 LOC |
| Tailwind Classes | Extensive |

---

## 4. Target Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Tauri Application                         │
├─────────────────────────────────────────────────────────────┤
│  Frontend (WASM)           │  Backend (Rust) - UNCHANGED    │
│  ┌───────────────────────┐ │  ┌──────────────────────────┐  │
│  │ Leptos 0.7            │ │  │ Tauri Commands (69+)     │  │
│  │ - #[component] macro  │ │  │ - LLM, Voice, Search     │  │
│  │ - create_signal()     │ │  │ - Campaigns, Sessions    │  │
│  │ - view! macro         │ │  │ - Characters, NPCs       │  │
│  │ - Leptos Router       │ │  │ - Document Ingestion     │  │
│  └───────────────────────┘ │  └──────────────────────────┘  │
│            │               │             │                   │
│  ┌─────────▼─────────────┐ │             │                   │
│  │ bindings.rs (updated) │◄┼─────────────┘                   │
│  │ - wasm-bindgen FFI    │ │  IPC via window.__TAURI__       │
│  │ - Type-safe wrappers  │ │  (same mechanism)               │
│  └───────────────────────┘ │                                 │
└─────────────────────────────────────────────────────────────┘
```

**Key Insight**: The Tauri backend is completely unchanged. Only the frontend framework changes.

---

## 5. Migration Scope

### 5.1 What Changes

| Component | Action |
|-----------|--------|
| `frontend/Cargo.toml` | Replace dioxus deps with leptos |
| `frontend/src/main.rs` | Rewrite app entry point |
| `frontend/src/components/*.rs` | Rewrite all components (~4,500 LOC) |
| `frontend/src/services/*.rs` | Migrate services (LayoutState, ThemeService) |
| `frontend/Dioxus.toml` | Remove (replaced by Trunk.toml or cargo-leptos) |
| `src-tauri/tauri.conf.json` | Update build commands |
| Navigation | Migrate ViewType-based navigation pattern |
| Layout System | Port MainShell, IconRail, MediaBar, resizable panels |

### 5.2 What Stays the Same

| Component | Status |
|-----------|--------|
| `src-tauri/src/**` | Unchanged (all 69+ commands) |
| `frontend/src/bindings.rs` | Types preserved, wasm-bindgen same |
| `frontend/public/*.css` | Tailwind + themes unchanged |
| `frontend/index.html` | Minor updates only |
| All backend logic | Completely unchanged |
| Meilisearch integration | Unchanged |
| Database layer | Unchanged |

---

## 6. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Component translation errors | Medium | Medium | Incremental migration, testing |
| Build tooling differences | Low | Low | cargo-leptos is well-documented |
| Performance regression | Low | Medium | Leptos typically faster |
| Missing Leptos equivalents | Low | Low | Leptos has mature ecosystem |
| Extended development time | Medium | Medium | Phase-based approach |

---

## 7. Success Criteria

- [ ] All 5 ViewType views functional (Campaigns, Chat, Library, Graph, Settings)
- [ ] Layout system working (MainShell, IconRail, MediaBar)
- [ ] Resizable panels functional (sidebar, info panel)
- [ ] All design system components ported
- [ ] All Tauri IPC bindings working
- [ ] Theme service with OKLCH interpolation working
- [ ] NPC conversation feature functional
- [ ] No regression in features
- [ ] Build size similar or smaller
- [ ] Performance maintained or improved

---

## 8. Decision

**Recommendation**: Proceed with Leptos migration if:

1. API stability and documentation quality are priorities
2. Team is committed to Rust-only frontend
3. Development time for migration is acceptable

**Alternative**: Stay on Dioxus if:

1. Current functionality is sufficient
2. Dioxus 0.7+ addresses stability concerns
3. Migration effort is too high for current roadmap

---

## Related Documents

- [architecture.md](./architecture.md) - Detailed technical architecture
- [component-mapping.md](./component-mapping.md) - Dioxus to Leptos translation guide
- [tasks.md](./tasks.md) - Actionable migration tasks

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.1.0 | 2026-01-01 | Updated for UX overhaul: new layout system, services, ViewType navigation |
| 1.0.0 | 2026-01-01 | Initial overview document |
