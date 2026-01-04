# UX Overhaul Task List

## Infrastructure
- [ ] Create `feature/ux-overhaul` branch
- [ ] Implement `NotificationService` and `Toast` components in frontend
- [ ] Integrate `NotificationProvider` in `app.rs`

## Sidebar Redesign
- [x] Refactor `icon_rail.rs`
    - [x] Update SVG icons to modern style
    - [x] Remove "stone pebble" background styling
    - [x] Improve hover effects

## Library Page
- [x] Fix "Supported Formats" display in `library/mod.rs`
    - [x] Ensure proper spacing/wrapping
    - [x] Consider grid layout for readability

## Error Handling
- [x] Audit user-facing errors
- [x] Replace ad-hoc error strings with structured `Toast` calls
- [x] **Constraint**: Ensure every error includes a suggested actionable resolution (Retry, Check permissions, etc.)
- [x] Add informative tooltips to UI elements (Library, etc.)

## Theming
- [x] Review `theme_service.rs` presets
- [x] Ensure error colors are high-contrast and accessible
- [x] Overhaul `index.html` structure and meta tags
- [x] Improve dynamic theming (add smooth transitions, refine palettes)

## Finalization
- [x] Run `coderabbit review --prompt-only`
- [x] Address review issues
- [x] Create PR to main
