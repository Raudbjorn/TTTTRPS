# Design Insights: TTRPG Document Processing

Extracted from research materials:
- `/home/svnbjrn/dev/AI-RPG/embeddings/planning/embeddings/The-Design-Language-of-Tabletop-Roleplaying-Game-Books.md`
- `/home/svnbjrn/dev/knowledgebase/embeddings/Unique-Features-of-TTRPG-Rulebooks-and-Sourcebooks.pdf`
- `/home/svnbjrn/dev/knowledgebase/embeddings/MDMAI` (original codebase)

## Core Insight: The Triple Mandate

TTRPG books must simultaneously:
1. **Teach** - Tutorial for new players
2. **Reference** - Quick lookup during active play
3. **Inspire** - Creative world-building and imagination

This creates unique design constraints not found in any other document type.

## Key Design Patterns to Preserve in Chunking

### 1. Two-Page Spread as Fundamental Unit

> "The single most influential layout principle in modern TTRPG design is confining content to visible two-page spreads."

**Implication for chunking**: Even-odd page pairs (2-3, 4-5, etc.) should be considered as potential semantic units. Content that spans a physical spread is likely designed to be read together.

```rust
// Proposed: SpreadAwareChunker
fn merge_spread_pages(pages: &[(u32, String)]) -> Vec<(u32, u32, String)> {
    // Merge even-odd pairs: page 2+3, 4+5, etc.
    pages.chunks(2).map(|pair| {
        let start = pair[0].0;
        let end = pair.get(1).map(|p| p.0).unwrap_or(start);
        let content = pair.iter().map(|p| &p.1).join("\n");
        (start, end, content)
    }).collect()
}
```

### 2. Fluff vs Crunch Classification

TTRPG content has two modes:
- **Crunch** (rules): Mechanical, procedural, precise
- **Fluff** (lore): Narrative, evocative, inspirational

Both are valid rules - fluff tells you *how to play* thematically, crunch tells you *how to play* mechanically.

**Detection heuristics**:

| Content Type | Indicators |
|--------------|------------|
| Crunch | Dice notation, "DC", "modifier", "roll", numbered lists, "must", "can", ability scores |
| Fluff | Past tense verbs, quotation marks, first-person, no dice notation, adjective-heavy |
| Mixed | In-character sidebars with mechanical effects |

```rust
#[derive(Debug, Clone, Copy)]
pub enum ContentMode {
    Crunch,      // Pure mechanical rules
    Fluff,       // Pure narrative/lore
    Mixed,       // Rules with flavor text
    Example,     // "Example of play" blocks
    Optional,    // Variant/optional rules
}
```

### 3. Stat Block Evolution

The stat block format has evolved significantly:

| Era | Format | Characteristics |
|-----|--------|-----------------|
| OD&D (1974) | Inline | "Black Bear: AC 7; HD 3+3; hp 25" |
| AD&D 2e | Dense blocks | Compressed, requires cross-reference |
| D&D 4e | Self-contained cards | Everything needed, no external reference |
| D&D 5e | Readable blocks | Balanced between 4e and 2e styles |

**Implication**: Stat blocks are functional units that must NEVER be split. They should be detected and preserved as atomic chunks regardless of size.

### 4. Random Table Conventions

Dice notation signals probability design:

| Notation | Results | Use Case |
|----------|---------|----------|
| d6 | 6 | Quick binary-weighted outcomes |
| d20 | 20 | Balanced probability |
| d100 | 100 | Fine-grained probability control |
| d66 | 36 | 2d6 read as tens/ones |
| 2d6 | 2-12 | Bell curve distribution |

**Detect and preserve**:
- Table headers with dice notation
- Roll ranges (1-3, 4-6, 01-65)
- Associated result text

### 5. Boxed Text / Read-Aloud Conventions

Tournament play origins: standardized text for fair play across different GMs.

**Best practices** (which affect chunking):
- Under 70 words (50-70 optimal)
- Focus on immediate sensory details
- No game mechanical terms
- No predicting player actions

**Detection**:
- `>` blockquote markers
- Border/shading indicators
- "Read the following aloud"
- Italic blocks (varies by publisher)

### 6. GM vs Player Information Splits

**DM Only** sections appeared in 1976 (Palace of the Vampire Queen).

Types of GM-only content:
- Monster stats (players shouldn't see AC/HP)
- Room contents (descriptions without spoilers)
- Trap mechanics
- NPC motivations

**Detection**:
- "GM Only", "DM Section", "For the Referee"
- Hidden stats in adventure modules
- Parenthetical notes in room descriptions

## Content Types Missing from Current Implementation

### A. Diegetic Fiction

White Wolf's World of Darkness books include in-universe fiction vignettes at chapter starts.

**Characteristics**:
- Written in-character
- No rules content
- Sets mood/tone
- Often formatted differently (different font, border)

**Chunking behavior**:
- Should be classified as `Fiction` type
- Lower retrieval priority for rules queries
- Higher priority for setting/mood queries

### B. Optional Rules and Variants

> "RPG texts often include optional rules, variants, and designer notes about different ways to play. These are sometimes set apart in sidebars or clearly marked as non-core rules."

**Detection patterns**:
- "Optional Rule:", "Variant:", "House Rule:"
- Sidebar placement
- "You may choose to...", "GMs may allow..."

**Chunking behavior**:
- Tag as `optional: true` in metadata
- Include in searches but with lower default weight

### C. Example of Play Blocks

Dialogic examples showing sample game scenes.

**Pattern**:
```
GM: "You see a door ahead."
Player: "I check for traps."
GM: "Roll Investigation."
```

**Detection**:
- Alternating speaker labels
- "GM:", "DM:", "Player:", character names
- Quotation marks with dice outcomes

### D. Dice Notation in Text

> "RPG rules are full of expressions like 'roll 2d6 and add your Strength modifier' or 'DC 15 Wisdom save.'"

**Patterns to extract**:
- `\d*d\d+` - Basic dice (2d6, d20)
- `DC\s*\d+` - Difficulty class
- `\+\d+` / `-\d+` - Modifiers
- `(\d+)d(\d+)\s*([+-]\s*\d+)?` - Full expression

Store as `dice_expressions: Vec<String>` in chunk metadata.

## Genre Classification Patterns

From MDMAI's `genre_classifier.py`, comprehensive keyword sets for:

| Genre | Key Indicators |
|-------|----------------|
| Fantasy | wizard, spell, dragon, elf, dwarf, dungeon |
| Sci-Fi | spaceship, laser, alien, planet, hyperspace |
| Cyberpunk | hacker, megacorp, chrome, neural, matrix |
| Cosmic Horror | eldritch, madness, sanity, tentacle, mythos |
| Post-Apocalyptic | wasteland, survivor, radiation, mutant |
| Steampunk | clockwork, brass, airship, automaton |
| Urban Fantasy | vampire, werewolf, modern magic, masquerade |

**Usage**: Store detected genre in chunk metadata for faceted search.

## Layout Detection Heuristics

### Typography-Based Hierarchy (from Design Language doc)

Standard TTRPG typography:
- Body text: 10-11pt serif, 12pt leading
- Sans-serif: 9pt on 11pt leading
- Headers: Distinct typeface, larger size
- Examples: Italic or distinct typeface
- Sidebars: Inverted colors or contrasting font

**Without font extraction**, detect hierarchy through:
- Line length patterns (headers usually shorter)
- All-caps lines (section headers)
- Indentation patterns
- Blank line patterns

### Multi-Column Detection

> "The standard convention uses two columns for US Letter/A4 formats and single columns for digest-sized books."

**Implication**: Text extraction may need column reordering for reading order.

## Cross-Reference Patterns

> "See page 47", "refer to Chapter 3", "see the 'Combat' section"

**Patterns**:
```regex
(?i)(?:see|refer to|check|consult)\s+(?:page|p\.?)\s*(\d+)
(?i)(?:see|refer to)\s+chapter\s+(\d+|[IVXLC]+)
(?i)(?:see|refer to)\s+(?:the\s+)?["']([^"']+)["']\s+section
(?i)as\s+described\s+(?:on|in)\s+page\s*(\d+)
```

**Store as**:
```rust
struct CrossReference {
    ref_type: RefType,        // Page, Chapter, Section
    ref_target: String,       // "47", "Combat", "III"
    ref_text: String,         // Full match text
    source_chunk_id: String,  // Which chunk contains this ref
}
```

## Additional Tasks for Implementation

Based on this research, add to tasks.md:

### Task X.1: Fluff/Crunch Classification

**File**: `src/ingestion/ttrpg/classifier.rs` [EXTEND]

```rust
#[derive(Debug, Clone, Copy)]
pub enum ContentMode {
    Crunch,
    Fluff,
    Mixed,
    Example,
    Optional,
    Fiction,
}

impl TTRPGClassifier {
    pub fn classify_content_mode(&self, text: &str) -> ContentMode {
        let dice_count = count_dice_notation(text);
        let narrative_score = count_narrative_indicators(text);

        if dice_count > 3 && narrative_score < 2 {
            ContentMode::Crunch
        } else if narrative_score > 5 && dice_count == 0 {
            ContentMode::Fluff
        } else {
            ContentMode::Mixed
        }
    }
}
```

### Task X.2: Dice Expression Extraction

**File**: `src/ingestion/ttrpg/attribute_extractor.rs` [EXTEND]

```rust
pub fn extract_dice_expressions(text: &str) -> Vec<DiceExpression> {
    let dice_re = Regex::new(r"(\d*)d(\d+)(?:\s*([+-])\s*(\d+))?").unwrap();
    let dc_re = Regex::new(r"DC\s*(\d+)").unwrap();

    let mut expressions = Vec::new();

    for cap in dice_re.captures_iter(text) {
        expressions.push(DiceExpression {
            count: cap.get(1).map(|m| m.as_str().parse().unwrap_or(1)).unwrap_or(1),
            die: cap[2].parse().unwrap(),
            modifier: cap.get(3).zip(cap.get(4)).map(|(sign, val)| {
                let v: i32 = val.as_str().parse().unwrap();
                if sign.as_str() == "-" { -v } else { v }
            }),
        });
    }

    expressions
}

pub struct DiceExpression {
    pub count: u32,      // Number of dice (2 in "2d6")
    pub die: u32,        // Die type (6 in "2d6")
    pub modifier: Option<i32>,
}
```

### Task X.3: Optional Rules Detection

**File**: `src/ingestion/ttrpg/classifier.rs` [EXTEND]

```rust
fn is_optional_rule(text: &str, context: &ClassificationContext) -> bool {
    let optional_patterns = [
        r"(?i)optional\s*rule",
        r"(?i)variant\s*rule",
        r"(?i)house\s*rule",
        r"(?i)you\s+may\s+(?:choose|opt|decide)",
        r"(?i)gms?\s+may\s+(?:allow|permit)",
        r"(?i)at\s+(?:the\s+)?(?:gm|dm)'?s?\s+discretion",
    ];

    optional_patterns.iter().any(|p| {
        Regex::new(p).unwrap().is_match(text)
    }) || context.in_sidebar
}
```

### Task X.4: Genre Detection Enhancement

**File**: `src/ingestion/ttrpg/game_detector.rs` [EXTEND]

Add genre detection alongside game system detection:

```rust
#[derive(Debug, Clone, Copy)]
pub enum TTRPGGenre {
    Fantasy,
    SciFi,
    Cyberpunk,
    CosmicHorror,
    PostApocalyptic,
    Steampunk,
    UrbanFantasy,
    SpaceOpera,
    Superhero,
    Western,
    Noir,
    Horror,
    Military,
    Historical,
    Mixed,
}

pub fn detect_genre(text: &str) -> (TTRPGGenre, f32) {
    // Use keyword analysis from MDMAI's genre_classifier.py
}
```

### Task X.5: Example of Play Detection

**File**: `src/ingestion/ttrpg/classifier.rs` [EXTEND]

```rust
fn detect_example_of_play(text: &str) -> bool {
    let dialogue_pattern = Regex::new(
        r"(?i)(?:gm|dm|player|referee)\s*:\s*[\"']"
    ).unwrap();

    let example_markers = [
        "example of play",
        "for example",
        "example:",
    ];

    dialogue_pattern.find_iter(text).count() >= 2
        || example_markers.iter().any(|m| text.to_lowercase().contains(m))
}
```

## Summary: Gaps Identified

| Gap | Priority | Source Document |
|-----|----------|-----------------|
| Fluff/Crunch classification | P1 | Unique Features PDF |
| Two-page spread awareness | P2 | Design Language |
| Dice expression extraction | P1 | Unique Features PDF |
| Optional rules detection | P1 | Unique Features PDF |
| Genre classification | P2 | MDMAI codebase |
| Example of play detection | P2 | Unique Features PDF |
| Diegetic fiction detection | P3 | Unique Features PDF |
| GM-only content tagging | P2 | Design Language |

These additions would make the chunking pipeline more comprehensive for TTRPG materials.
