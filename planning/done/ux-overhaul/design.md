# UX Design Specification

## Document Info
- **Version**: 2.0.0
- **Branch**: `feature/ux-overhaul`
- **Last Updated**: 2025-12-29

---

## Overview

Modernize the TTRPG Assistant into an immersive, adaptive interface that feels native to whatever world the DM is running. The UI should disappear into the experience, enhancing rather than interrupting gameplay.

---

## Design Metaphor: Spotify × Slack × Obsidian

### Spotify: The Library
| Spotify Concept | TTRPG Application |
|-----------------|-------------------|
| Albums | Campaigns - cover art, track listing, genre |
| Tracks | Sessions - numbered, playable, duration shown |
| Playlists | Session groups by status (Past/Current/Planned) |
| Artist Pages | Personality profiles - voice, traits, linked NPCs |
| Genres/Moods | Setting themes - Fantasy, Horror, Sci-Fi |
| Now Playing | Active session indicator, media bar |

### Slack: The Conversations
| Slack Concept | TTRPG Application |
|---------------|-------------------|
| Channels | NPCs as conversation threads |
| Direct Messages | 1:1 NPC dialogue history |
| Threads | Multi-turn exchanges within an NPC conversation |
| Presence dots | NPC availability/activity indicators |
| Unread badges | Pending NPC responses |
| Reactions | Quick emotion/note annotations on messages |

### Obsidian: The Knowledge Base
| Obsidian Concept | TTRPG Application |
|------------------|-------------------|
| Vault | Campaign - all knowledge scoped to one world |
| Notes | Entities - NPCs, locations, plot threads |
| Links | Relationships - who knows whom, what connects |
| Graph View | Visual relationship map |
| Tags | Entity classification (faction, region, status) |
| Backlinks | "Referenced by" - see all mentions |

---

## Layout Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  [Icon Rail]  │  [Context Sidebar]  │    [Main Content]    │ [Info Panel] │
│    48-64px    │     200-280px       │        flex          │   280-350px  │
│               │                     │                      │   (toggle)   │
│   ○ Chat      │  ▾ Current          │                      │              │
│   ○ Campaigns │    ● Session 12     │   Chat / Notes /     │  NPC Detail  │
│   ○ Library   │  ▾ Planned          │   Combat Tracker     │  Initiative  │
│   ○ Graph     │    ○ Session 13     │                      │  Dice Roller │
│   ○ Settings  │  ▾ Past             │                      │              │
│               │    ○ Session 1-11   │                      │              │
├───────────────┴─────────────────────┴──────────────────────┴──────────────┤
│                           [Media Bar - 56px]                              │
│  ▶ ❚❚ ■  │  ═══════○═══  │  🔊━━━━━  │  "Grondar the Wise"  │  🎤 ○     │
└───────────────────────────────────────────────────────────────────────────┘
```

### Panel Breakdown

#### Icon Rail (Left Edge)
- Fixed 48-64px width
- Icon-only navigation (Slack-style)
- Sections: Chat, Campaigns, Library, Graph, Settings
- Active indicator: vertical bar or fill
- Tooltip on hover

#### Context Sidebar
- 200-280px, resizable
- Content changes by view:
  - **Campaign View**: Session list grouped by status
  - **Library View**: Document tree (Obsidian-style)
  - **Graph View**: Entity filter/search
- Collapsible with `Cmd+.`

#### Main Content
- Flexible width, min 400px
- Primary workspace: chat, notes, combat
- Header shows context (campaign name, session title)
- Scrollable with sticky header

#### Info Panel (Right)
- 280-350px, toggleable with `Cmd+/`
- Contextual:
  - **Chat View**: Active NPC details
  - **Combat View**: Initiative tracker, HP bars
  - **Note View**: Backlinks, related entities

#### Media Bar (Bottom)
- Fixed 56px height
- Always visible
- Controls: Play/Pause, Stop, Progress, Volume
- Now Speaking: NPC name + avatar
- Transcription toggle with waveform

---

## Component Designs

### Campaign Card (Album Cover)

```
┌─────────────────────────┐
│   ╔═══════════════╗     │
│   ║               ║     │  Cover image (AI-generated or placeholder)
│   ║   [ARTWORK]   ║     │  Genre badge overlay (top-right)
│   ║               ║     │  "Now Playing" pulse (if active)
│   ╚═══════════════╝     │
│                         │
│   Curse of Strahd       │  Campaign name
│   D&D 5e • 12 sessions  │  System • session count
│   Last played: 2 days   │  Recency
└─────────────────────────┘
```

### Session List (Track List)

```
┌─────────────────────────────┐
│ ▾ CURRENT                   │
│ ┌─────────────────────────┐ │
│ │ ● 12  Dinner with Death │ │  Active: pulsing dot, bold
│ │       2h 34m • Dec 22   │ │
│ └─────────────────────────┘ │
│                             │
│ ▾ PLANNED                   │
│ ┌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┐   │
│ ╎ ○ 13  The Ritual       ╎   │  Planned: dashed border
│ ╎       Outline only     ╎   │
│ └╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┘   │
│                             │
│ ▸ PAST (11 sessions)        │  Collapsed by default
└─────────────────────────────┘
```

### NPC Sidebar (Slack DMs)

```
┌─────────────────────────────┐
│ NPCs                    [+] │
│─────────────────────────────│
│ ┌─┐                         │
│ │◉│ Strahd von Zarovich  •2 │  Avatar, name, unread badge
│ └─┘ "You dare..."           │  Last message preview
│                             │
│ ┌─┐                         │
│ │○│ Ireena Kolyana          │  No unread: hollow dot
│ └─┘ *sighs* "Again?"        │
│                             │
│ ┌─┐                         │
│ │○│ Ismark the Lesser       │
│ └─┘ "My sister needs..."    │
└─────────────────────────────┘
```

### Chat Message (Threaded)

```
┌─────────────────────────────────────────────────────┐
│ [◉] Strahd von Zarovich              12:34 PM   ▶  │
│─────────────────────────────────────────────────────│
│ "You think you can escape my domain? How...        │
│ *amusing*."                                        │
│                                                    │
│     ┌─────────────────────────────────────────┐    │
│     │ ↳ 3 replies                         [+] │    │  Thread preview
│     └─────────────────────────────────────────┘    │
│                                                    │
│ [📌] [💀] [📝 Note]                                │  Quick actions
└─────────────────────────────────────────────────────┘
```

### Media Bar

```
┌─────────────────────────────────────────────────────────────────────┐
│  ▶  ❚❚  ■  │  ════════════○══════════  │  2:34 / 5:12  │           │
│            │                            │               │           │
│  [◉] Strahd von Zarovich               │  🔊━━━━━━━    │  🎤 [○]   │
│      Speaking...                        │  Volume       │  Mic      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Dynamic Theme System

### Philosophy

The UI should feel **native to the world** being played. A horror game should feel oppressive; a fantasy game, magical; a sci-fi game, clinical. This isn't just colors—it's typography, spacing, animations, and effects.

### Base Theme Tokens

Each theme defines these CSS custom properties:

```css
:root {
  /* Core palette */
  --bg-deep: ...;      /* Deepest background (app shell) */
  --bg-surface: ...;   /* Cards, panels */
  --bg-elevated: ...;  /* Modals, popovers */
  --text-primary: ...; /* Main text */
  --text-muted: ...;   /* Secondary text */
  --accent: ...;       /* Interactive elements */
  --accent-hover: ...; /* Hover state */
  --danger: ...;       /* Destructive actions, HP loss */

  /* Borders & effects */
  --border-subtle: ...;
  --border-strong: ...;
  --glow-color: ...;
  --shadow-color: ...;

  /* Typography */
  --font-body: ...;
  --font-header: ...;
  --font-mono: ...;

  /* Spacing & shape */
  --radius-sm: ...;
  --radius-md: ...;
  --radius-lg: ...;

  /* Animation */
  --transition-fast: ...;
  --transition-base: ...;

  /* Effects (0-1 intensity) */
  --effect-grain: ...;
  --effect-scanline: ...;
  --effect-glow: ...;
  --effect-blur: ...;
}
```

### Theme Definitions

#### `fantasy` — Arcane Glassmorphism
**For**: D&D, Pathfinder, Warhammer Fantasy, 13th Age

```css
.theme-fantasy {
  --bg-deep: oklch(15% 0.02 280);      /* Deep purple-black */
  --bg-surface: oklch(20% 0.03 280 / 0.8);
  --bg-elevated: oklch(25% 0.04 280 / 0.9);
  --text-primary: oklch(95% 0.01 60);  /* Warm white */
  --text-muted: oklch(60% 0.02 280);
  --accent: oklch(75% 0.15 45);        /* Gold */
  --accent-hover: oklch(80% 0.18 45);
  --danger: oklch(60% 0.20 25);        /* Blood red */

  --border-subtle: oklch(30% 0.03 280 / 0.5);
  --border-strong: oklch(75% 0.12 45 / 0.6); /* Gold border */
  --glow-color: oklch(75% 0.15 45 / 0.3);

  --font-body: 'Inter', system-ui, sans-serif;
  --font-header: 'Cinzel', 'Merriweather', serif;
  --font-mono: 'Iosevka', monospace;

  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 16px;

  --effect-blur: 12px;  /* Glassmorphism */
  --effect-grain: 0;
  --effect-scanline: 0;
  --effect-glow: 0.4;
}
```

#### `cosmic` — Eldritch Dread
**For**: Call of Cthulhu, Delta Green (partial), Kult, Vaesen

```css
.theme-cosmic {
  --bg-deep: oklch(8% 0.02 160);       /* Abyss green-black */
  --bg-surface: oklch(12% 0.03 160 / 0.85);
  --bg-elevated: oklch(16% 0.04 150 / 0.9);
  --text-primary: oklch(85% 0.02 100); /* Sickly off-white */
  --text-muted: oklch(50% 0.04 160);
  --accent: oklch(55% 0.12 160);       /* Toxic green */
  --accent-hover: oklch(60% 0.15 160);
  --danger: oklch(50% 0.15 320);       /* Void purple */

  --border-subtle: oklch(20% 0.05 160 / 0.4);
  --border-strong: oklch(55% 0.10 160 / 0.5);
  --glow-color: oklch(55% 0.12 160 / 0.2);

  --font-body: 'Crimson Text', Georgia, serif;
  --font-header: 'IM Fell English', serif;
  --font-mono: 'Courier Prime', monospace;

  --radius-sm: 2px;
  --radius-md: 4px;
  --radius-lg: 6px;  /* Less rounded = more unsettling */

  --effect-grain: 0.15;    /* Film grain overlay */
  --effect-blur: 4px;
  --effect-scanline: 0;
  --effect-glow: 0.2;
}
```

#### `terminal` — Nostromo Console
**For**: Mothership, Alien RPG, Traveller, Stars Without Number

```css
.theme-terminal {
  --bg-deep: oklch(5% 0 0);            /* True black */
  --bg-surface: oklch(10% 0.01 145);   /* Faint green tint */
  --bg-elevated: oklch(15% 0.02 145);
  --text-primary: oklch(85% 0.15 145); /* Phosphor green */
  --text-muted: oklch(55% 0.08 145);
  --accent: oklch(75% 0.18 80);        /* Amber warning */
  --accent-hover: oklch(80% 0.20 80);
  --danger: oklch(65% 0.20 25);        /* Red alert */

  --border-subtle: oklch(25% 0.05 145 / 0.3);
  --border-strong: oklch(70% 0.12 145 / 0.5);
  --glow-color: oklch(75% 0.15 145 / 0.4);

  --font-body: 'VT323', 'Fira Code', monospace;
  --font-header: 'Share Tech Mono', monospace;
  --font-mono: 'VT323', monospace;

  --radius-sm: 0;
  --radius-md: 0;
  --radius-lg: 2px;  /* Sharp corners */

  --effect-scanline: 0.3;  /* CRT lines */
  --effect-grain: 0.05;
  --effect-blur: 0;
  --effect-glow: 0.6;      /* Text glow */
}
```

#### `noir` — 90s Office Paranoia
**For**: Delta Green (primary), Night's Black Agents, Cold War games

```css
.theme-noir {
  --bg-deep: oklch(20% 0.01 80);       /* Dark manila */
  --bg-surface: oklch(28% 0.02 75);    /* Folder tan */
  --bg-elevated: oklch(35% 0.03 70);
  --text-primary: oklch(90% 0.01 90);  /* Paper white */
  --text-muted: oklch(55% 0.02 80);
  --accent: oklch(45% 0.08 25);        /* Dried blood / stamp red */
  --accent-hover: oklch(50% 0.10 25);
  --danger: oklch(55% 0.15 25);

  --border-subtle: oklch(40% 0.02 80 / 0.3);
  --border-strong: oklch(30% 0.03 80 / 0.6);
  --glow-color: none;

  --font-body: 'IBM Plex Mono', 'Courier', monospace;
  --font-header: 'Special Elite', 'Courier', monospace; /* Typewriter */
  --font-mono: 'IBM Plex Mono', monospace;

  --radius-sm: 0;
  --radius-md: 2px;
  --radius-lg: 4px;

  --effect-grain: 0.08;    /* Paper texture */
  --effect-scanline: 0;
  --effect-blur: 0;
  --effect-glow: 0;
  --effect-redact: 1;      /* Special: redacted text effect available */
}
```

#### `neon` — Cyberpunk Chrome
**For**: Cyberpunk RED, Shadowrun, The Sprawl, Neon City Overdrive

```css
.theme-neon {
  --bg-deep: oklch(8% 0.01 270);       /* Deep purple-black */
  --bg-surface: oklch(12% 0.02 280);
  --bg-elevated: oklch(18% 0.03 290);
  --text-primary: oklch(95% 0.02 200); /* Cool white */
  --text-muted: oklch(60% 0.05 280);
  --accent: oklch(70% 0.25 330);       /* Hot pink/magenta */
  --accent-hover: oklch(75% 0.28 330);
  --danger: oklch(65% 0.22 25);

  /* Secondary accent for variety */
  --accent-alt: oklch(75% 0.20 195);   /* Cyan */

  --border-subtle: oklch(25% 0.08 280 / 0.3);
  --border-strong: oklch(70% 0.20 330 / 0.5);
  --glow-color: oklch(70% 0.25 330 / 0.4);

  --font-body: 'Rajdhani', 'Orbitron', sans-serif;
  --font-header: 'Orbitron', sans-serif;
  --font-mono: 'Fira Code', monospace;

  --radius-sm: 0;
  --radius-md: 4px;
  --radius-lg: 8px;

  --effect-grain: 0.03;
  --effect-scanline: 0.1;
  --effect-blur: 8px;
  --effect-glow: 0.8;      /* Strong neon glow */
}
```

### Theme Interpolation

For settings that blend genres, themes can be mathematically interpolated:

```rust
/// Theme weight configuration stored per campaign
pub struct ThemeWeights {
    pub fantasy: f32,   // 0.0 - 1.0
    pub cosmic: f32,
    pub terminal: f32,
    pub noir: f32,
    pub neon: f32,
}

impl ThemeWeights {
    /// Normalize weights to sum to 1.0
    pub fn normalize(&mut self) { ... }

    /// Interpolate CSS custom property values
    pub fn interpolate_color(&self, property: &str) -> String { ... }
}
```

**Example Presets**:
```
Delta Green:     { cosmic: 0.4, noir: 0.6 }
Shadowrun:       { neon: 0.6, fantasy: 0.3, terminal: 0.1 }
Blades in Dark:  { noir: 0.5, fantasy: 0.3, cosmic: 0.2 }
Starfinder:      { fantasy: 0.5, terminal: 0.4, neon: 0.1 }
```

**Custom Blend UI**:
- Settings panel with 5 sliders (0-100 each)
- Auto-normalize to 100% total
- Live preview as user adjusts
- Save as campaign setting

### Effect Implementations

#### Film Grain (Cosmic, Noir)
```css
.effect-grain::before {
  content: '';
  position: fixed;
  inset: 0;
  background-image: url('data:image/svg+xml,...'); /* Noise pattern */
  opacity: var(--effect-grain);
  pointer-events: none;
  mix-blend-mode: overlay;
}
```

#### CRT Scanlines (Terminal)
```css
.effect-scanlines::after {
  content: '';
  position: fixed;
  inset: 0;
  background: repeating-linear-gradient(
    0deg,
    transparent,
    transparent 2px,
    oklch(0% 0 0 / 0.1) 2px,
    oklch(0% 0 0 / 0.1) 4px
  );
  opacity: var(--effect-scanline);
  pointer-events: none;
}
```

#### Text Glow (Terminal, Neon)
```css
.glow-text {
  text-shadow:
    0 0 2px var(--glow-color),
    0 0 8px var(--glow-color),
    0 0 16px var(--glow-color);
}
```

#### Redacted Text (Noir)
```css
.redacted {
  background: currentColor;
  color: transparent;
  user-select: none;
}
.redacted:hover {
  background: transparent;
  color: inherit;
}
```

---

## Responsive Behavior

### Breakpoints

| Width | Layout |
|-------|--------|
| ≥1400px | Full 4-panel layout |
| 1200-1399px | Hide info panel by default |
| 900-1199px | Collapse context sidebar to icons |
| <900px | Single column, drawer navigation |

### Gesture Support (Future)
- Swipe right: Open context sidebar
- Swipe left: Open info panel
- Long press: Context menu
- Pinch: Zoom graph view

---

## Animation Guidelines

### Principles
- **Purposeful**: Animation conveys meaning (state change, attention)
- **Quick**: Base transitions 150-200ms
- **Interruptible**: User can act during animation
- **Reduced Motion**: Respect `prefers-reduced-motion`

### Key Animations
| Element | Animation | Duration |
|---------|-----------|----------|
| Panel toggle | Slide + fade | 200ms ease-out |
| Theme change | Crossfade | 300ms |
| Message appear | Fade up | 150ms |
| Button hover | Scale 1.02 | 100ms |
| Modal open | Scale up + fade | 200ms |
| Toast notification | Slide in | 150ms |

---

## Accessibility

### Requirements
- **Contrast**: WCAG AA minimum (4.5:1 normal text, 3:1 large)
- **Focus**: Visible focus ring on all interactive elements
- **Screen Reader**: ARIA labels, live regions for updates
- **Keyboard**: Full navigation without mouse
- **Motion**: `prefers-reduced-motion` disables animations

### Theme-Specific Considerations
- Terminal theme: Ensure green text passes contrast
- Cosmic theme: Avoid pure black on dark backgrounds
- All themes: Test with color blindness simulators

