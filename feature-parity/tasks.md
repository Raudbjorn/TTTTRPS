# Feature Parity Tasks

## Phase 1: Foundation & Layout
- [ ] **Task 1.1**: Create `MainShell` layout component with CSS Grid implementation. (Ref: REQ-LAYOUT-1)
- [ ] **Task 1.2**: Implement `IconRail` component with navigation links and tooltip support. (Ref: REQ-NAV-1)
- [ ] **Task 1.3**: Implement `LayoutService` signal to handle Sidebar/InfoPanel toggling logic. (Ref: REQ-LAYOUT-2)
- [ ] **Task 1.4**: Create `MediaBar` component skeleton (visual only initially) anchored to bottom. (Ref: REQ-COMP-MEDIA)

## Phase 2: Dynamic Theme Engine
- [ ] **Task 2.1**: Define base `ThemeDefinition` structs and Color math util in Rust. (Ref: REQ-THEME-2)
- [ ] **Task 2.2**: Implement `ThemeState` and interpolation logic (`to_css_vars`). (Ref: REQ-THEME-3)
- [ ] **Task 2.3**: Define the 5 core themes (Fantasy, Cosmic, Terminal, Noir, Neon) in Rust code. (Ref: REQ-THEME-1)
- [ ] **Task 2.4**: Create `ThemeDebug` component (temporary UI) to test slider-based interpolation. (Ref: REQ-THEME-5)
- [ ] **Task 2.5**: update `App.rs` to inject the calculated CSS variables into the root style. (Ref: REQ-THEME-3)

## Phase 3: Content Migration (Metaphors)
- [ ] **Task 3.1**: Refactor `SessionList` to live inside the `ContextSidebar` when in Campaign View. (Ref: REQ-NAV-2, REQ-META-SPOTIFY)
- [ ] **Task 3.2**: Refactor `NPCList` into the `InfoPanel` slot. (Ref: REQ-META-SLACK)
- [ ] **Task 3.3**: Implement "Campaign Card" visual design for the dashboard. (Ref: REQ-COMP-CARD)
- [ ] **Task 3.4**: Implement "Obsidian-like" graph view placeholder in Main Content. (Ref: REQ-META-OBSIDIAN)

## Phase 4: Polish & Effects
- [ ] **Task 4.1**: Implement CSS for Film Grain, Scanlines, and Glow effects using pseudo-elements. (Ref: REQ-THEME-4)
- [ ] **Task 4.2**: Verify Accessibility (Contrast check) for all 5 core themes. (Ref: REQ-A11Y-1)
