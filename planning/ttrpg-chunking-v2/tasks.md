# Tasks: TTRPG Chunking Pipeline v2

## Implementation Phases

This builds on existing infrastructure. Focus on gaps, not rewrites.

```
Phase 1: Classification (P0)     → TTRPG element detection
Phase 2: Hierarchy (P0)          → Section path tracking
Phase 3: Atomic Chunking (P0)    → Never split functional units
Phase 4: Context Injection (P1)  → Prepend metadata to chunks
Phase 5: Boundary Scoring (P1)   → Intelligent split points
Phase 6: Search Enhancement (P1) → Filterable attributes
Phase 7: Cross-References (P2)   → Parse "see page X"
Phase 8: LLM Fallback (P3)       → Ollama boundary detection
```

---

## Phase 1: TTRPG Element Classification

### Task 1.1: Create TTRPGElementType enum and ClassifiedElement struct

**File**: `src/ingestion/ttrpg/classifier.rs` [NEW]

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TTRPGElementType {
    StatBlock,
    RandomTable,
    SpellDescription,
    ItemDescription,
    ReadAloudText,
    Sidebar,
    SectionHeader,
    CrossReference,
    Narrative,
    Rules,
    GenericText,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedElement {
    pub content: String,
    pub element_type: TTRPGElementType,
    pub confidence: f32,
    pub page_number: u32,
    pub char_offset: usize,
    pub metadata: ElementMetadata,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ElementMetadata {
    pub detected_patterns: Vec<String>,  // Which regex matched
    pub header_level: Option<u8>,        // For SectionHeader
    pub dice_notation: Option<String>,   // For RandomTable
    pub cross_ref_target: Option<String>,// For CrossReference
}
```

**Test**: Construct each element type, verify serialization.

---

### Task 1.2: Implement stat block detection patterns

**File**: `src/ingestion/ttrpg/classifier.rs` [EXTEND]

**Patterns** (regex, any 3+ matches = StatBlock):
- `(?i)armor\s*class\s*\d+` — AC detection
- `(?i)hit\s*points?\s*\d+` — HP detection
- `(?i)\bHP\s*\d+` — Abbreviated HP
- `(?i)STR\s+\d+|DEX\s+\d+|CON\s+\d+|INT\s+\d+|WIS\s+\d+|CHA\s+\d+` — Ability scores
- `(?i)challenge\s*(?:rating)?\s*\d+` — CR detection
- `(?i)speed\s*\d+\s*ft` — Speed detection
- `(?i)saving\s*throws?:` — Saves header
- `(?i)skills?:` — Skills header
- `(?i)damage\s*(?:resistances?|immunities?|vulnerabilities?):` — Damage keywords
- `(?i)senses?:.*(?:darkvision|blindsight|tremorsense)` — Senses
- `(?i)languages?:` — Languages header
- `(?i)actions?:` — Actions section
- `(?i)legendary\s*actions?:` — Legendary actions

**Confidence scoring**:
- 3-4 matches: 0.6
- 5-6 matches: 0.8
- 7+ matches: 0.95

**Test**: Parse SRD Goblin, Zombie, Adult Red Dragon stat blocks.

---

### Task 1.3: Implement random table detection patterns

**File**: `src/ingestion/ttrpg/classifier.rs` [EXTEND]

**Patterns**:
- `(?i)\bd\d+\b` — Dice notation (d4, d6, d8, d10, d12, d20, d100)
- `(?i)\b\d+d\d+\b` — Multi-dice (2d6, 3d8)
- `(?i)\b(\d+)[-–—](\d+)\b` — Range patterns (1-3, 4-6, 01-50)
- `(?i)roll\s+(?:a\s+)?d\d+` — "Roll a d20"
- `(?i)table\s*\d*:?` — Table headers

**Additional heuristics**:
- Presence of `|` characters (markdown table)
- Multiple lines with aligned numbers at start
- Uniform line structure (number, dash, result)

**Extract dice notation** to `ElementMetadata::dice_notation`.

**Test**: Parse encounter tables, treasure tables, wild magic tables.

---

### Task 1.4: Implement read-aloud/boxed text detection

**File**: `src/ingestion/ttrpg/classifier.rs` [EXTEND]

**Patterns** (any match = ReadAloudText):
- `(?i)read\s*(?:the\s*)?(?:following\s*)?aloud` — Explicit instruction
- `(?i)boxed\s*text` — Layout reference
- Line starts with `>` (markdown blockquote)
- Entire block in italics (if typography detected)
- Block surrounded by horizontal rules

**Confidence**: 0.9 for explicit patterns, 0.7 for italics/blockquote.

**Test**: Parse adventure module read-aloud sections.

---

### Task 1.5: Implement section header detection with level

**File**: `src/ingestion/ttrpg/classifier.rs` [EXTEND]

**Patterns**:
- `^#{1,6}\s+(.+)$` — Markdown headers (level = # count)
- `^([A-Z][A-Z0-9\s]+)$` — All-caps line (level 1-2)
- `^(?:Chapter|Section|Part)\s+\d+` — Explicit chapter markers (level 1)
- Short line (<100 chars) followed by double newline
- Line in title case with no punctuation

**Level detection**:
- `# ` = 1, `## ` = 2, etc.
- All-caps = 1-2 (based on context)
- Numbered chapters = 1

Store in `ElementMetadata::header_level`.

**Test**: Parse PHB-style chapter structure.

---

### Task 1.6: Implement TTRPGClassifier with classify() method

**File**: `src/ingestion/ttrpg/classifier.rs` [EXTEND]

```rust
pub struct TTRPGClassifier {
    stat_block_patterns: Vec<Regex>,
    table_patterns: Vec<Regex>,
    read_aloud_patterns: Vec<Regex>,
    header_patterns: Vec<Regex>,
    cross_ref_patterns: Vec<Regex>,
}

impl TTRPGClassifier {
    pub fn new() -> Self;

    /// Classify a block of text
    pub fn classify(&self, text: &str, page_number: u32) -> ClassifiedElement;

    /// Classify all elements on a page
    pub fn classify_page(&self, page_text: &str, page_number: u32) -> Vec<ClassifiedElement>;

    /// Split page into classifiable blocks (by double newlines, etc.)
    fn split_into_blocks(&self, text: &str) -> Vec<(usize, &str)>;
}
```

**Test**: Classify mixed-content page with stat block, narrative, and table.

---

### Task 1.7: Update ingestion/ttrpg/mod.rs exports

**File**: `src/ingestion/ttrpg/mod.rs` [MODIFY]

```rust
pub mod classifier;

pub use classifier::{
    TTRPGClassifier, TTRPGElementType, ClassifiedElement, ElementMetadata,
};
```

---

## Phase 2: Section Hierarchy Tracking

### Task 2.1: Implement SectionHierarchy struct

**File**: `src/ingestion/chunker.rs` [EXTEND]

```rust
/// Tracks nested section structure as headers are encountered
#[derive(Debug, Clone, Default)]
pub struct SectionHierarchy {
    /// Stack of (level, title) pairs. Level 1 = top, 6 = deepest.
    stack: Vec<(u8, String)>,
}

impl SectionHierarchy {
    pub fn new() -> Self { Self::default() }

    /// Update hierarchy when a header is encountered.
    /// Pops sections of equal or greater level before pushing.
    pub fn update(&mut self, level: u8, title: &str) {
        // Pop all sections at this level or deeper
        while self.stack.last().map(|(l, _)| *l >= level).unwrap_or(false) {
            self.stack.pop();
        }
        self.stack.push((level, title.to_string()));
    }

    /// Get full path: "Chapter 3 > Combat > Grappling"
    pub fn path(&self) -> Option<String> {
        if self.stack.is_empty() {
            None
        } else {
            Some(self.stack.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>().join(" > "))
        }
    }

    /// Get current section depth (0 = no sections)
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Get parent sections (excluding current)
    pub fn parents(&self) -> Vec<String> {
        if self.stack.len() <= 1 {
            vec![]
        } else {
            self.stack[..self.stack.len()-1].iter().map(|(_, t)| t.clone()).collect()
        }
    }

    /// Get current (deepest) section title
    pub fn current(&self) -> Option<&str> {
        self.stack.last().map(|(_, t)| t.as_str())
    }
}
```

**Test**: Build hierarchy with Chapter > Section > Subsection, verify path.

---

### Task 2.2: Integrate SectionHierarchy into chunking

**File**: `src/ingestion/chunker.rs` [EXTEND]

Modify `SemanticChunker::chunk_with_pages` or add new `TTRPGChunker`:

```rust
pub struct TTRPGChunker {
    config: ChunkConfig,
    hierarchy: SectionHierarchy,
    classifier: TTRPGClassifier,
}

impl TTRPGChunker {
    pub fn new(config: ChunkConfig) -> Self {
        Self {
            config,
            hierarchy: SectionHierarchy::new(),
            classifier: TTRPGClassifier::new(),
        }
    }

    pub fn chunk_classified(
        &mut self,
        elements: &[ClassifiedElement],
        source_id: &str,
    ) -> Vec<ContentChunk> {
        let mut chunks = Vec::new();

        for element in elements {
            match element.element_type {
                TTRPGElementType::SectionHeader => {
                    if let Some(level) = element.metadata.header_level {
                        self.hierarchy.update(level, &element.content);
                    }
                    // Don't create chunk for headers alone
                }
                _ => {
                    let mut chunk = self.create_chunk(element, source_id);
                    // Attach hierarchy metadata
                    chunk.section = self.hierarchy.current().map(String::from);
                    chunk.metadata.insert(
                        "section_path".to_string(),
                        self.hierarchy.path().unwrap_or_default(),
                    );
                    chunks.push(chunk);
                }
            }
        }

        chunks
    }
}
```

**Test**: Chunk document with headers, verify section_path in each chunk.

---

## Phase 3: Atomic Element Preservation

### Task 3.1: Add TTRPGChunkConfig with atomic settings

**File**: `src/ingestion/chunker.rs` [EXTEND]

```rust
#[derive(Debug, Clone)]
pub struct TTRPGChunkConfig {
    pub base: ChunkConfig,
    /// Elements that should never be split
    pub atomic_elements: Vec<TTRPGElementType>,
    /// Multiplier for max_size when handling atomic elements (default 2.0)
    pub atomic_max_multiplier: f32,
    /// Whether to inject hierarchy context into chunk content
    pub inject_context: bool,
}

impl Default for TTRPGChunkConfig {
    fn default() -> Self {
        Self {
            base: ChunkConfig::default(),
            atomic_elements: vec![
                TTRPGElementType::StatBlock,
                TTRPGElementType::RandomTable,
                TTRPGElementType::SpellDescription,
                TTRPGElementType::ItemDescription,
            ],
            atomic_max_multiplier: 2.0,
            inject_context: true,
        }
    }
}
```

---

### Task 3.2: Implement atomic element handling in chunker

**File**: `src/ingestion/chunker.rs` [EXTEND]

```rust
impl TTRPGChunker {
    fn is_atomic(&self, element: &ClassifiedElement) -> bool {
        self.config.atomic_elements.contains(&element.element_type)
    }

    fn max_size_for(&self, element: &ClassifiedElement) -> usize {
        if self.is_atomic(element) {
            (self.config.base.max_size as f32 * self.config.atomic_max_multiplier) as usize
        } else {
            self.config.base.max_size
        }
    }

    fn chunk_element(&mut self, element: &ClassifiedElement, source_id: &str) -> Vec<ContentChunk> {
        let max_size = self.max_size_for(element);

        if element.content.len() <= max_size {
            // Emit as single chunk
            vec![self.create_chunk(element, source_id)]
        } else if self.is_atomic(element) {
            // Atomic but too large: split reluctantly, log warning
            tracing::warn!(
                "Splitting oversized atomic element: {} chars (type: {:?})",
                element.content.len(),
                element.element_type
            );
            self.split_oversized(element, source_id, max_size)
        } else {
            // Non-atomic: split at best boundaries
            self.split_at_boundaries(element, source_id)
        }
    }
}
```

**Test**: Large stat block (>2x max) splits with warning; normal stat block stays intact.

---

### Task 3.3: Implement buffer flushing for atomic elements

**File**: `src/ingestion/chunker.rs` [EXTEND]

When an atomic element is encountered, any buffered content must be flushed first:

```rust
struct ChunkBuffer {
    content: String,
    start_page: u32,
    elements: Vec<(u32, usize)>,  // (page, offset) for provenance
}

impl TTRPGChunker {
    fn process_elements(&mut self, elements: &[ClassifiedElement], source_id: &str) -> Vec<ContentChunk> {
        let mut chunks = Vec::new();
        let mut buffer = ChunkBuffer::default();

        for element in elements {
            if element.element_type == TTRPGElementType::SectionHeader {
                self.flush_buffer(&mut buffer, source_id, &mut chunks);
                self.update_hierarchy(element);
                continue;
            }

            if self.is_atomic(element) {
                // Flush buffer before atomic
                self.flush_buffer(&mut buffer, source_id, &mut chunks);
                // Emit atomic as its own chunk(s)
                chunks.extend(self.chunk_element(element, source_id));
            } else {
                // Add to buffer
                buffer.push(element);
                if buffer.len() >= self.config.base.target_size {
                    self.flush_buffer(&mut buffer, source_id, &mut chunks);
                }
            }
        }

        // Final flush
        self.flush_buffer(&mut buffer, source_id, &mut chunks);
        chunks
    }
}
```

**Test**: Narrative → StatBlock → Narrative results in 3 chunks with correct ordering.

---

## Phase 4: Context Injection

### Task 4.1: Implement context header generation

**File**: `src/core/meilisearch_pipeline.rs` [EXTEND]

```rust
/// Generate context header string to prepend to chunk content
pub fn generate_context_header(
    element_type: Option<&TTRPGElementType>,
    section_path: Option<&str>,
    game_system: Option<&str>,
) -> String {
    let mut parts = Vec::new();

    if let Some(path) = section_path {
        if !path.is_empty() {
            parts.push(format!("[Section: {}]", path));
        }
    }

    if let Some(etype) = element_type {
        parts.push(format!("[Type: {}]", etype.display_name()));
    }

    if let Some(system) = game_system {
        parts.push(format!("[System: {}]", system));
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!("{} ", parts.join(" "))
    }
}

impl TTRPGElementType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::StatBlock => "Stat Block",
            Self::RandomTable => "Random Table",
            Self::SpellDescription => "Spell",
            Self::ItemDescription => "Item",
            Self::ReadAloudText => "Read Aloud",
            Self::Sidebar => "Sidebar",
            Self::SectionHeader => "Header",
            Self::CrossReference => "Reference",
            Self::Narrative => "Narrative",
            Self::Rules => "Rules",
            Self::GenericText => "Text",
        }
    }
}
```

**Test**: Verify header format for various combinations.

---

### Task 4.2: Inject context during chunking

**File**: `src/ingestion/chunker.rs` [EXTEND]

```rust
impl TTRPGChunker {
    fn create_chunk(&self, element: &ClassifiedElement, source_id: &str) -> ContentChunk {
        let context_header = if self.config.inject_context {
            generate_context_header(
                Some(&element.element_type),
                self.hierarchy.path().as_deref(),
                None,  // Game system added later in pipeline
            )
        } else {
            String::new()
        };

        ContentChunk {
            id: Uuid::new_v4().to_string(),
            source_id: source_id.to_string(),
            content: format!("{}{}", context_header, element.content),
            page_number: Some(element.page_number),
            section: self.hierarchy.current().map(String::from),
            chunk_type: element.element_type.as_str().to_string(),
            // ... other fields
        }
    }
}
```

---

## Phase 5: Boundary Scoring

### Task 5.1: Define BoundaryType enum and scoring

**File**: `src/ingestion/chunker.rs` [EXTEND]

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoundaryType {
    SectionHeader,      // 0.95 - Markdown headers, all-caps titles
    DoubleNewline,      // 0.85 - Paragraph break
    AllCapsLine,        // 0.80 - OSR-style headers
    BulletStart,        // 0.70 - List item
    SentenceCapital,    // 0.60 - Sentence end + capital letter
    TransitionWord,     // 0.50 - "However", "Therefore"
    SentenceEnd,        // 0.40 - Period + space
    ClauseBoundary,     // 0.20 - Comma, semicolon
    Fallback,           // 0.10 - Character limit
}

impl BoundaryType {
    pub fn default_weight(&self) -> f32 {
        match self {
            Self::SectionHeader => 0.95,
            Self::DoubleNewline => 0.85,
            Self::AllCapsLine => 0.80,
            Self::BulletStart => 0.70,
            Self::SentenceCapital => 0.60,
            Self::TransitionWord => 0.50,
            Self::SentenceEnd => 0.40,
            Self::ClauseBoundary => 0.20,
            Self::Fallback => 0.10,
        }
    }
}

pub struct BoundaryScorer {
    weights: HashMap<BoundaryType, f32>,
    transition_words: Vec<&'static str>,
}

impl Default for BoundaryScorer {
    fn default() -> Self {
        Self {
            weights: BoundaryType::iter().map(|b| (b, b.default_weight())).collect(),
            transition_words: vec![
                "however", "therefore", "additionally", "furthermore",
                "nevertheless", "consequently", "meanwhile", "alternatively",
            ],
        }
    }
}
```

---

### Task 5.2: Implement boundary detection and scoring

**File**: `src/ingestion/chunker.rs` [EXTEND]

```rust
impl BoundaryScorer {
    /// Find all boundaries in text range and score them
    pub fn find_boundaries(&self, text: &str) -> Vec<(usize, BoundaryType, f32)> {
        let mut boundaries = Vec::new();

        // Double newlines
        for (idx, _) in text.match_indices("\n\n") {
            boundaries.push((idx, BoundaryType::DoubleNewline, self.weights[&BoundaryType::DoubleNewline]));
        }

        // Sentence boundaries (. followed by space and capital)
        let sentence_re = Regex::new(r"\.\s+[A-Z]").unwrap();
        for m in sentence_re.find_iter(text) {
            boundaries.push((m.start() + 1, BoundaryType::SentenceCapital, self.weights[&BoundaryType::SentenceCapital]));
        }

        // Bullet points
        let bullet_re = Regex::new(r"\n\s*[-*•]\s").unwrap();
        for m in bullet_re.find_iter(text) {
            boundaries.push((m.start(), BoundaryType::BulletStart, self.weights[&BoundaryType::BulletStart]));
        }

        // Transition words at sentence start
        for word in &self.transition_words {
            let pattern = format!(r"(?i)\.\s+{}\s", regex::escape(word));
            if let Ok(re) = Regex::new(&pattern) {
                for m in re.find_iter(text) {
                    boundaries.push((m.start() + 1, BoundaryType::TransitionWord, self.weights[&BoundaryType::TransitionWord]));
                }
            }
        }

        boundaries.sort_by_key(|(pos, _, _)| *pos);
        boundaries
    }

    /// Find best split point near target position
    pub fn find_best_split(&self, text: &str, target: usize, window: usize) -> usize {
        let boundaries = self.find_boundaries(text);

        let start = target.saturating_sub(window);
        let end = (target + window).min(text.len());

        boundaries
            .into_iter()
            .filter(|(pos, _, _)| *pos >= start && *pos <= end)
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
            .map(|(pos, _, _)| pos)
            .unwrap_or(target)
    }
}
```

**Test**: Find boundaries in sample text, verify scoring order.

---

### Task 5.3: Use boundary scoring in split logic

**File**: `src/ingestion/chunker.rs` [EXTEND]

```rust
impl TTRPGChunker {
    fn split_at_boundaries(&self, element: &ClassifiedElement, source_id: &str) -> Vec<ContentChunk> {
        let scorer = BoundaryScorer::default();
        let text = &element.content;
        let target_size = self.config.base.target_size;
        let window = target_size / 4;  // Search 25% around target

        let mut chunks = Vec::new();
        let mut start = 0;

        while start < text.len() {
            let target_end = start + target_size;

            if target_end >= text.len() {
                // Last chunk
                chunks.push(self.create_chunk_from_range(element, source_id, start, text.len()));
                break;
            }

            // Find best split point
            let split_at = scorer.find_best_split(text, target_end, window);
            chunks.push(self.create_chunk_from_range(element, source_id, start, split_at));

            // Move start with overlap
            start = split_at.saturating_sub(self.config.base.overlap_size);
        }

        chunks
    }
}
```

---

## Phase 6: Search Enhancement

### Task 6.1: Add TTRPG filterable attributes to ChunkedDocument

**File**: `src/core/meilisearch_pipeline.rs` [EXTEND]

```rust
impl ChunkedDocument {
    // Existing fields...

    // Add new fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_path: Option<String>,
    #[serde(default)]
    pub section_depth: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parent_sections: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cross_refs: Vec<CrossReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossReference {
    pub ref_type: String,   // "page", "chapter", "section"
    pub ref_target: String, // "47", "Combat", "Appendix A"
    pub ref_text: String,   // Original text: "See page 47"
}
```

---

### Task 6.2: Configure Meilisearch index with new attributes

**File**: `src/core/search_client.rs` [EXTEND]

```rust
impl SearchClient {
    pub async fn configure_ttrpg_index(&self, index_name: &str) -> Result<(), SearchError> {
        let index = self.client.index(index_name);

        let settings = Settings::new()
            .with_filterable_attributes([
                // Existing
                "source_slug",
                "page_start",
                "page_end",
                "game_system",
                "content_category",
                // New TTRPG attributes
                "element_type",
                "section_path",
                "section_depth",
                "parent_sections",
                // From vocabulary extraction
                "damage_types",
                "creature_types",
                "conditions",
                "challenge_rating",
                "spell_level",
            ])
            .with_sortable_attributes([
                "page_start",
                "chunk_index",
                "section_depth",
                "challenge_rating",
            ])
            .with_searchable_attributes([
                "content",
                "section_path",
                "book_title",
                "semantic_keywords",
            ]);

        index.set_settings(&settings).await?;
        Ok(())
    }
}
```

---

### Task 6.3: Add element_type filter to search queries

**File**: `src/core/search_client.rs` [EXTEND]

```rust
#[derive(Debug, Clone, Default)]
pub struct TTRPGSearchFilters {
    pub element_types: Vec<String>,
    pub game_systems: Vec<String>,
    pub section_path_contains: Option<String>,
    pub min_section_depth: Option<u32>,
    pub max_section_depth: Option<u32>,
}

impl SearchClient {
    pub fn build_ttrpg_filter(&self, filters: &TTRPGSearchFilters) -> Option<String> {
        let mut conditions = Vec::new();

        if !filters.element_types.is_empty() {
            let types: Vec<_> = filters.element_types.iter()
                .map(|t| format!("element_type = '{}'", t))
                .collect();
            conditions.push(format!("({})", types.join(" OR ")));
        }

        if !filters.game_systems.is_empty() {
            let systems: Vec<_> = filters.game_systems.iter()
                .map(|s| format!("game_system = '{}'", s))
                .collect();
            conditions.push(format!("({})", systems.join(" OR ")));
        }

        if let Some(path) = &filters.section_path_contains {
            conditions.push(format!("section_path CONTAINS '{}'", path));
        }

        if conditions.is_empty() {
            None
        } else {
            Some(conditions.join(" AND "))
        }
    }
}
```

---

## Phase 7: Cross-Reference Parsing

### Task 7.1: Implement cross-reference detection patterns

**File**: `src/ingestion/ttrpg/classifier.rs` [EXTEND]

```rust
impl TTRPGClassifier {
    /// Extract cross-references from text
    pub fn extract_cross_refs(&self, text: &str) -> Vec<CrossReference> {
        let mut refs = Vec::new();

        // Page references
        let page_re = Regex::new(r"(?i)(?:see|refer to|check|consult)\s+(?:page|p\.?)\s*(\d+)").unwrap();
        for cap in page_re.captures_iter(text) {
            refs.push(CrossReference {
                ref_type: "page".to_string(),
                ref_target: cap[1].to_string(),
                ref_text: cap[0].to_string(),
            });
        }

        // Chapter references
        let chapter_re = Regex::new(r"(?i)(?:see|refer to)\s+chapter\s+(\d+|[IVXLC]+)").unwrap();
        for cap in chapter_re.captures_iter(text) {
            refs.push(CrossReference {
                ref_type: "chapter".to_string(),
                ref_target: cap[1].to_string(),
                ref_text: cap[0].to_string(),
            });
        }

        // Section references (quoted)
        let section_re = Regex::new(r#"(?i)(?:see|refer to)\s+(?:the\s+)?["']([^"']+)["']\s+section"#).unwrap();
        for cap in section_re.captures_iter(text) {
            refs.push(CrossReference {
                ref_type: "section".to_string(),
                ref_target: cap[1].to_string(),
                ref_text: cap[0].to_string(),
            });
        }

        refs
    }
}
```

**Test**: Parse "See page 47", "refer to Chapter 3", "see the 'Combat' section".

---

### Task 7.2: Store cross-references in chunk metadata

**File**: `src/ingestion/chunker.rs` [EXTEND]

```rust
impl TTRPGChunker {
    fn create_chunk(&self, element: &ClassifiedElement, source_id: &str) -> ContentChunk {
        let cross_refs = self.classifier.extract_cross_refs(&element.content);

        ContentChunk {
            // ... existing fields
            metadata: {
                let mut meta = HashMap::new();
                if !cross_refs.is_empty() {
                    meta.insert(
                        "cross_refs".to_string(),
                        serde_json::to_string(&cross_refs).unwrap_or_default(),
                    );
                }
                meta
            },
        }
    }
}
```

---

## Phase 8: LLM Fallback (Optional)

### Task 8.1: Add Ollama boundary detection fallback

**File**: `src/ingestion/chunker.rs` [EXTEND]

```rust
#[cfg(feature = "llm-boundaries")]
pub async fn detect_boundaries_with_llm(
    text: &str,
    ollama_client: &OllamaClient,
) -> Result<Vec<(usize, f32)>> {
    let truncated = &text[..text.len().min(4000)];

    let prompt = format!(
        "You are analyzing TTRPG rulebook text. Identify major topic/section boundaries.\n\
         Return line numbers where new topics begin, with confidence 0.0-1.0.\n\
         Format: one 'line_number,confidence' per line.\n\n\
         Text:\n{}\n\n\
         Boundaries:",
        truncated
    );

    let response = ollama_client.generate("qwen2.5:7b", &prompt).await?;

    // Parse response
    let boundaries: Vec<(usize, f32)> = response
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() == 2 {
                let line_num: usize = parts[0].trim().parse().ok()?;
                let confidence: f32 = parts[1].trim().parse().ok()?;
                Some((line_num, confidence))
            } else {
                None
            }
        })
        .collect();

    Ok(boundaries)
}
```

This task is optional and gated behind a feature flag.

---

## Implementation Order

```
Week 1: Classification Foundation
├── Task 1.1: TTRPGElementType enum
├── Task 1.2: Stat block patterns
├── Task 1.3: Random table patterns
├── Task 1.4: Read-aloud patterns
├── Task 1.5: Header detection
├── Task 1.6: TTRPGClassifier
└── Task 1.7: Update mod.rs

Week 2: Hierarchy & Atomics
├── Task 2.1: SectionHierarchy struct
├── Task 2.2: Integrate into chunking
├── Task 3.1: TTRPGChunkConfig
├── Task 3.2: Atomic handling
└── Task 3.3: Buffer flushing

Week 3: Context & Boundaries
├── Task 4.1: Context header generation
├── Task 4.2: Inject during chunking
├── Task 5.1: BoundaryType enum
├── Task 5.2: Boundary detection
└── Task 5.3: Use in split logic

Week 4: Search & Polish
├── Task 6.1: ChunkedDocument fields
├── Task 6.2: Meilisearch config
├── Task 6.3: TTRPG filters
├── Task 7.1: Cross-ref patterns
└── Task 7.2: Store cross-refs
```

## MVP Scope

For minimum viable:
- Phase 1 (Classification) - Tasks 1.1-1.7
- Phase 2 (Hierarchy) - Tasks 2.1-2.2
- Phase 3 (Atomics) - Tasks 3.1-3.3

This gives you:
- Stat blocks, tables, and read-aloud detection
- Section path tracking in every chunk
- Atomic preservation (never split stat blocks)

Phases 4-8 are enhancements for improved retrieval quality.
