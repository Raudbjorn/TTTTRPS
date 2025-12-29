# Feature Parity Requirements
Based on UX Design Specification v2.0.0

## 1. Layout & Navigation
| ID | Requirement | Description | Source |
|----|-------------|-------------|--------|
| **REQ-LAYOUT-1** | **5-Panel Architecture** | Implement the specific responsive grid: Icon Rail, Context Sidebar, Main Content, Info Panel, Media Bar. | Design 2.0 (Layout) |
| **REQ-LAYOUT-2** | **Collapsible Sidebars** | Context Sidebar and Info Panel must be toggleable (hotkeys `Cmd+.`, `Cmd+/`). | Design 2.0 (Layout) |
| **REQ-NAV-1** | **Icon Rail** | Fixed 48-64px left rail with tooltips for global navigation (Chat, Campaigns, Library, Graph, Settings). | Design 2.0 (Panels) |
| **REQ-NAV-2** | **Context Switching** | Sidebar content must dynamically change based on active view (Campaign -> Session List; Library -> Tree). | Design 2.0 (Panels) |

## 2. Metaphor Implementation
| ID | Requirement | Description | Source |
|----|-------------|-------------|--------|
| **REQ-META-SPOTIFY** | **Campaign/Session Presentation** | Campaigns displayed as "Albums" (Cover Art, Genre). Sessions listed as "Tracks" (Playable, Duration). | Design 2.0 (Spotify) |
| **REQ-META-SLACK** | **NPC Interaction** | NPCs displayed as a persistent contact list with presence dots and unread badges. Chat interface is thread-capable. | Design 2.0 (Slack) |
| **REQ-META-OBSIDIAN** | **Knowledge Base** | Library data structured as a graph; entity linking and backlink visualization. | Design 2.0 (Obsidian) |

## 3. Dynamic Immersion (Advanced Theming)
| ID | Requirement | Description | Source |
|----|-------------|-------------|--------|
| **REQ-THEME-1** | **Extended Theme Set** | Implement 5 core themes: `Fantasy`, `Cosmic`, `Terminal`, `Noir`, `Neon`. | Design 2.0 (Themes) |
| **REQ-THEME-2** | **CSS Variable Architecture** | Define strict core palette (`--bg-deep`, `--accent`, etc.) across all themes for hot-swapping. | Design 2.0 (Base Tokens) |
| **REQ-THEME-3** | **Theme Interpolation** | **CRITICAL**: Support weighted blending of themes (e.g., 60% Noir + 40% Cosmic). Backend/Frontend must calculate interpolated hex values or CSS variables. | Design 2.0 (Interpolation) |
| **REQ-THEME-4** | **Visual Effects** | Implement CSS-based effects: Film Grain, CRT Scanlines, Text Glow, Redaction. Intensity controlled by theme variables. | Design 2.0 (Effects) |
| **REQ-THEME-5** | **Auto-Adaptation** | System must default to reasonable presets (e.g. Cthulhu -> Cosmic) but allow manual override/blending. | Design 2.0 (Interpolation) |

## 4. Components
| ID | Requirement | Description | Source |
|----|-------------|-------------|--------|
| **REQ-COMP-MEDIA** | **Media Bar** | Persistent bottom bar (56px) with Play/Pause, Volume, and "Now Speaking" indicator. | Design 2.0 (Media Bar) |
| **REQ-COMP-CARD** | **Campaign Card** | Rich visual card with "Now Playing" pulse animation if session active. | Design 2.0 (Campaign Card) |
| **REQ-COMP-CHAT** | **Chat Threading** | UI support for replying to specific messages (visual threading). | Design 2.0 (Chat) |

## 5. Non-Functional
| ID | Requirement | Description | Source |
|----|-------------|-------------|--------|
| **REQ-PERF-1** | **Animation** | Transitions must be purposeful and fast (150-200ms). Respect `prefers-reduced-motion`. | Design 2.0 (Animation) |
| **REQ-A11Y-1** | **Accessibility** | All themes must meet contrast ratios. | Design 2.0 (A11Y) |
