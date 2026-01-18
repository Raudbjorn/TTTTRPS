 Now I have a comprehensive view. Let me analyze the docling-core capabilities against TTRPG-specific challenges.

  Analysis: Docling-Core for TTRPG Material Processing

  🎯 Key TTRPG Challenges (from your documents)
  ┌─────────────────────┬──────────────────────────────────────────────────────────────────┐
  │      Challenge      │                           Description                            │
  ├─────────────────────┼──────────────────────────────────────────────────────────────────┤
  │ Triple Mandate      │ Simultaneously tutorial, reference, and inspiration              │
  ├─────────────────────┼──────────────────────────────────────────────────────────────────┤
  │ Mixed Content Types │ Rules (crunch) vs. narrative (fluff) vs. tables vs. stat blocks  │
  ├─────────────────────┼──────────────────────────────────────────────────────────────────┤
  │ Stat Blocks         │ Self-contained formatted data for monsters/items/NPCs            │
  ├─────────────────────┼──────────────────────────────────────────────────────────────────┤
  │ Dice Notation       │ Embedded expressions like 2d6+3, DC 15 Wisdom save               │
  ├─────────────────────┼──────────────────────────────────────────────────────────────────┤
  │ Sidebars/Boxed Text │ Visually separated asides, optional rules, GM tips, read-aloud   │
  ├─────────────────────┼──────────────────────────────────────────────────────────────────┤
  │ Random Tables       │ d6, d20, d66, d100 with probability-aware formatting             │
  ├─────────────────────┼──────────────────────────────────────────────────────────────────┤
  │ Two-Page Spread     │ Content confined to visible spreads for scanning                 │
  ├─────────────────────┼──────────────────────────────────────────────────────────────────┤
  │ Hierarchical Depth  │ Deep section nesting (chapters → sections → subsections → rules) │
  └─────────────────────┴──────────────────────────────────────────────────────────────────┘
  ---
  ✅ Directly Applicable Docling-Core Features

  1. Hierarchical Chunking with Section Context (docling_core/transforms/chunker/hierarchical_chunker.py:44-147)

  This is the most directly useful feature. The HierarchicalChunker tracks heading hierarchy and emits chunks with full section context:

  # Key algorithm: maintains heading_by_level dict
  # On new heading → removes shadowed headings at higher levels
  # On content → serializes with active heading hierarchy

  TTRPG Application: Perfect for preserving context like "Chapter 5: Combat → Initiative → Surprise Rounds" when chunking rules. A GM asking "how does surprise work?" gets the full hierarchy.

  2. Rich Table Model (docling_core/types/doc/document.py:350-580)

  The TableData and TableCell models support:
  - Row/column spanning
  - Header detection (column_header, row_header flags)
  - Rich cells containing references to other document items
  - Export to DataFrame, Markdown, HTML

  TTRPG Application: Ideal for equipment lists, spell tables, random encounter tables. The span support handles merged cells common in weapon stats tables (e.g., "Martial Weapons" header spanning multiple rows).

  3. TripletTableSerializer (docling_core/transforms/chunker/hierarchical_chunker.py:25-40)

  Converts tables to text triplets for LLM compatibility:
  "Longsword, Damage = 1d8 slashing"
  "Longsword, Weight = 3 lb"
  "Longsword, Properties = Versatile (1d10)"

  TTRPG Application: Excellent for stat block data and equipment tables. Makes tabular game data queryable by LLMs.

  4. Semantic Labels (docling_core/types/doc/labels.py:14-65)

  30+ DocItemLabel values including:
  - TITLE, SECTION_HEADER, PARAGRAPH
  - TABLE, CAPTION, FOOTNOTE
  - KEY_VALUE_REGION (could map to stat block fields)

  TTRPG Application: Foundation for classifying crunch vs. fluff content. KEY_VALUE_REGION is particularly relevant for stat blocks.

  5. HybridChunker with Token Budgets (docling_core/transforms/chunker/hybrid_chunker.py:50-180)

  Token-aware chunking that:
  - Respects max_tokens limit
  - Merges undersized peer chunks
  - Preserves metadata through splits

  TTRPG Application: Ensures stat blocks and rule sections fit within LLM context windows. The merge_peers option keeps related rules together.

  6. Pluggable Serializer Architecture (docling_core/transforms/serializer/)

  Strategy pattern allowing custom serializers per item type:
  BaseDocSerializer
  ├── table_serializer: BaseTableSerializer
  ├── text_serializer: BaseTextSerializer
  ├── picture_serializer: BasePictureSerializer
  └── ...

  TTRPG Application: Could create StatBlockSerializer, ReadAloudTextSerializer, SidebarSerializer for TTRPG-specific formatting.

  7. Code Chunking Strategy Pattern (docling_core/transforms/chunker/code_chunking/)

  The AST-aware code chunker architecture provides a template:
  class StandardCodeChunkingStrategy:
      # Maps language → specific chunker
      # Uses Tree-sitter for AST parsing
      # Extracts functions, classes, methods with line ranges

  TTRPG Application: Inspiring pattern for stat block chunking. Stat blocks have consistent structure that could be parsed similarly (name, type, stats, abilities, actions).

  ---
  🔧 Novel Techniques Worth Adapting

  1. Metadata Exclusion for Embeddings vs. LLM (docling_core/transforms/chunker/base.py:36-58)

  class DocMeta(BaseMeta):
      excluded_embed: list[str] = ["schema_name", "version", "doc_items", "origin"]
      excluded_llm: list[str] = ["schema_name", "version", "doc_items", "origin"]

  TTRPG Adaptation: Separate what goes into vector embeddings vs. LLM prompts:
  - Embeddings: Strip flavor text, keep mechanical keywords
  - LLM context: Include flavor for narrative questions

  2. Provenance Tracking via SerializationResult (docling_core/transforms/serializer/base.py:80-120)

  class SerializationResult:
      text: str
      spans: list[Span]  # Maps output → source items

  TTRPG Adaptation: Track which page/spread a rule came from for citation. Essential for "where is this rule?" queries.

  3. Rich Table Cell References (docling_core/types/doc/document.py:400-450)

  Tables can contain references to other document items, enabling nested content resolution.

  TTRPG Adaptation: Spell tables referencing full spell descriptions, monster tables linking to stat blocks.

  ---
  🚧 Gaps for TTRPG Processing
  ┌─────────────────────────────────┬──────────────────────────────────────────────────┬──────────────────────────────────────────────────────────────────────┐
  │               Gap               │                  What's Missing                  │                         Suggested Extension                          │
  ├─────────────────────────────────┼──────────────────────────────────────────────────┼──────────────────────────────────────────────────────────────────────┤
  │ Stat Block Detection            │ No native recognition of TTRPG stat block format │ Create StatBlockItem type with fields for AC, HP, abilities, actions │
  ├─────────────────────────────────┼──────────────────────────────────────────────────┼──────────────────────────────────────────────────────────────────────┤
  │ Dice Notation Parsing           │ No regex/parser for 2d6+3, DC 15, etc.           │ Add DiceExpression type, extract during serialization                │
  ├─────────────────────────────────┼──────────────────────────────────────────────────┼──────────────────────────────────────────────────────────────────────┤
  │ Sidebar/Boxed Text              │ No label for visually separated asides           │ Add SIDEBAR, BOXED_TEXT, READ_ALOUD to DocItemLabel                  │
  ├─────────────────────────────────┼──────────────────────────────────────────────────┼──────────────────────────────────────────────────────────────────────┤
  │ Fluff vs. Crunch Classification │ No semantic distinction                          │ Add ContentPurpose enum: MECHANICAL, NARRATIVE, EXAMPLE, OPTIONAL    │
  ├─────────────────────────────────┼──────────────────────────────────────────────────┼──────────────────────────────────────────────────────────────────────┤
  │ Two-Page Spread Awareness       │ Page-based but not spread-based                  │ Add SpreadItem grouping facing pages                                 │
  ├─────────────────────────────────┼──────────────────────────────────────────────────┼──────────────────────────────────────────────────────────────────────┤
  │ Random Table Semantics          │ Tables are generic, no dice-roll awareness       │ Add RandomTableItem with probability distribution metadata           │
  └─────────────────────────────────┴──────────────────────────────────────────────────┴──────────────────────────────────────────────────────────────────────┘
  ---
  💡 Recommended Implementation Approach

  Phase 1: Extend Labels (minimal code change)

  Add TTRPG-specific labels to DocItemLabel:
  # In docling_core/types/doc/labels.py
  class DocItemLabel(str, Enum):
      # Existing...
      STAT_BLOCK = "stat_block"
      SIDEBAR = "sidebar"
      READ_ALOUD = "read_aloud"
      EXAMPLE_OF_PLAY = "example_of_play"
      OPTIONAL_RULE = "optional_rule"
      RANDOM_TABLE = "random_table"

  Phase 2: Custom Chunking Strategy

  Create TTRPGChunkingStrategy following the code chunking pattern:
  class StatBlockChunkingStrategy:
      def chunk_stat_block(self, item: DocItem, doc: DoclingDocument) -> Iterator[StatBlockChunk]:
          # Parse stat block structure
          # Extract: name, type, size, AC, HP, stats, abilities, actions
          # Yield self-contained chunks

  Phase 3: Custom Serializer Provider

  class TTRPGSerializerProvider(BaseSerializerProvider):
      def get_serializer(self, doc):
          return TTRPGDocSerializer(
              table_serializer=RandomTableSerializer(),  # Dice-aware
              stat_block_serializer=StatBlockSerializer(),
              sidebar_serializer=SidebarSerializer(),
          )

  Phase 4: Dice Notation Extraction

  DICE_PATTERN = re.compile(r'(\d+)?d(\d+)([+-]\d+)?')
  DC_PATTERN = re.compile(r'DC\s*(\d+)\s*(\w+)?')

  class DiceExpression(BaseModel):
      raw: str
      num_dice: int
      die_size: int
      modifier: int
      average: float  # Computed

  ---
  📊 Feature Mapping Summary
  ┌────────────────────────┬───────────────────────────────────────────┬───────────────────────┐
  │       TTRPG Need       │           Docling-Core Feature            │       Readiness       │
  ├────────────────────────┼───────────────────────────────────────────┼───────────────────────┤
  │ Section hierarchy      │ HierarchicalChunker                       │ ✅ Ready              │
  ├────────────────────────┼───────────────────────────────────────────┼───────────────────────┤
  │ Token limits           │ HybridChunker                             │ ✅ Ready              │
  ├────────────────────────┼───────────────────────────────────────────┼───────────────────────┤
  │ Equipment/spell tables │ TableItem + TripletTableSerializer        │ ✅ Ready              │
  ├────────────────────────┼───────────────────────────────────────────┼───────────────────────┤
  │ Multi-format output    │ Markdown/HTML/LaTeX serializers           │ ✅ Ready              │
  ├────────────────────────┼───────────────────────────────────────────┼───────────────────────┤
  │ Custom processing      │ Pluggable serializer/chunker architecture │ ✅ Ready              │
  ├────────────────────────┼───────────────────────────────────────────┼───────────────────────┤
  │ Stat blocks            │ KeyValueItem (partial)                    │ ⚠️ Needs extension    │
  ├────────────────────────┼───────────────────────────────────────────┼───────────────────────┤
  │ Sidebars               │ No native support                         │ ❌ Needs new label    │
  ├────────────────────────┼───────────────────────────────────────────┼───────────────────────┤
  │ Dice notation          │ No support                                │ ❌ Needs new type     │
  ├────────────────────────┼───────────────────────────────────────────┼───────────────────────┤
  │ Fluff/crunch split     │ No classification                         │ ❌ Needs new metadata │
  ├────────────────────────┼───────────────────────────────────────────┼───────────────────────┤
  │ Read-aloud text        │ No support                                │ ❌ Needs new label    │
  └────────────────────────┴───────────────────────────────────────────┴───────────────────────┘
  ---
  🎲 Bottom Line

  Docling-core provides ~70% of what you need for TTRPG processing out of the box:
  - Excellent hierarchical chunking with section context
  - Solid table handling with triplet serialization
  - Flexible extension points via strategy patterns
  - Token-aware chunking for LLM compatibility

  The remaining 30% requires domain-specific extensions:
  - New DocItemLabel values for TTRPG content types
  - Custom chunking strategy for stat blocks
  - Dice notation parsing/extraction
  - Semantic classification (rules vs. narrative)


  The architecture is well-suited for these extensions—the pluggable serializer/chunker patterns make it straightforward to add TTRPG-specific processing without modifying core code.


 Analysis: Docling-IBM-Models for TTRPG Material Processing

  🏗️ Architecture Overview

  This repository provides AI/ML models for document understanding, complementing docling-core's data structures:
  ┌───────────────────┬───────────────────────────┬──────────────────────────────────────────┐
  │     Component     │        Model Type         │                 Purpose                  │
  ├───────────────────┼───────────────────────────┼──────────────────────────────────────────┤
  │ LayoutModel       │ RTDetr (Object Detection) │ Detect document regions (17 classes)     │
  ├───────────────────┼───────────────────────────┼──────────────────────────────────────────┤
  │ TableFormer       │ Dual-Decoder Transformer  │ Extract table structure with cell bboxes │
  ├───────────────────┼───────────────────────────┼──────────────────────────────────────────┤
  │ Figure Classifier │ EfficientNetB0            │ Classify images into 16 categories       │
  ├───────────────────┼───────────────────────────┼──────────────────────────────────────────┤
  │ Code/Formula      │ SamOPT (Vision-Language)  │ Extract code/equations from images       │
  ├───────────────────┼───────────────────────────┼──────────────────────────────────────────┤
  │ Reading Order     │ Rule-based spatial        │ Determine logical reading sequence       │
  ├───────────────────┼───────────────────────────┼──────────────────────────────────────────┤
  │ List Normalizer   │ Regex + heuristics        │ Detect and normalize list markers        │
  └───────────────────┴───────────────────────────┴──────────────────────────────────────────┘
  ---
  ✅ Directly Applicable Features for TTRPG

  1. Layout Detection with Form/Key-Value Labels (layoutmodel/labels.py:1-54)

  The 17-class layout model includes labels directly useful for TTRPG:

  # Highly relevant for TTRPG stat blocks and sidebars:
  15: Form              # Structured data regions (stat blocks!)
  16: Key-Value Region  # Label-value pairs (ability scores, stats)
  13: Checkbox-Selected # Interactive elements (character sheets)
  14: Checkbox-Unselected
  12: Code              # Could detect dice notation blocks

  TTRPG Application:
  - Form + Key-Value Region → Stat block detection
  - Checkbox-* → Character sheet fillable fields
  - Can differentiate stat blocks from narrative prose

  2. TableFormer with OTSL Tags (tableformer/otsl.py:30-245)

  The OTSL (One-Table Segmentation Language) tags map perfectly to TTRPG table structures:
  ┌──────────┬──────────────────────────────┬────────────────────────────────────────┐
  │ OTSL Tag │           Meaning            │           TTRPG Application            │
  ├──────────┼──────────────────────────────┼────────────────────────────────────────┤
  │ ched     │ Column header                │ "Ability", "Score", "Modifier" headers │
  ├──────────┼──────────────────────────────┼────────────────────────────────────────┤
  │ rhed     │ Row header                   │ STR, DEX, CON labels in stat blocks    │
  ├──────────┼──────────────────────────────┼────────────────────────────────────────┤
  │ srow     │ Section row                  │ "Melee Attacks", "Spells" dividers     │
  ├──────────┼──────────────────────────────┼────────────────────────────────────────┤
  │ fcel     │ First/regular cell           │ Actual stat values                     │
  ├──────────┼──────────────────────────────┼────────────────────────────────────────┤
  │ ucel     │ Underspan (vertical merge)   │ Multi-row abilities                    │
  ├──────────┼──────────────────────────────┼────────────────────────────────────────┤
  │ lcel     │ Left span (horizontal merge) │ Feature descriptions spanning columns  │
  ├──────────┼──────────────────────────────┼────────────────────────────────────────┤
  │ xcel     │ Cross span (2D merge)        │ Large feature blocks                   │
  └──────────┴──────────────────────────────┴────────────────────────────────────────┘
  Example stat block parsing:
  ┌────────┬───────┬──────────┐
  │ Ability│ Score │ Modifier │  ← ched, ched, ched
  ├────────┼───────┼──────────┤
  │ STR    │  16   │   +3     │  ← rhed, fcel, fcel
  │ DEX    │  14   │   +2     │  ← rhed, fcel, fcel
  ├────────┴───────┴──────────┤
  │ Actions                   │  ← srow (section divider)
  ├───────────────────────────┤
  │ Multiattack: ...          │  ← lcel (spans all columns)
  └───────────────────────────┘

  3. Two-Column Layout Handling (reading_order/reading_order_rb.py:411-472)

  The reading order predictor has explicit multi-column support:

  # Horizontal dilation threshold: 15% of page width
  # Detects column boundaries and preserves reading order
  def _do_horizontal_dilation(self, elements, page_width):
      dilation = page_width * 0.15  # Two-column detection threshold

  TTRPG Application: Critical for rulebooks where:
  - Stat blocks appear in one column, narrative in another
  - Rules continue across column breaks
  - Sidebars interrupt main text flow

  4. Figure Classification (document_figure_classifier_model/...predictor.py:21-183)

  16 figure classes with several TTRPG-relevant categories:
  ┌────────────┬──────────────────────────────────────────────────────┐
  │   Class    │                   TTRPG Relevance                    │
  ├────────────┼──────────────────────────────────────────────────────┤
  │ flow_chart │ Spell progression, decision trees, combat flowcharts │
  ├────────────┼──────────────────────────────────────────────────────┤
  │ icon       │ Attribute symbols, class icons, condition markers    │
  ├────────────┼──────────────────────────────────────────────────────┤
  │ map        │ Dungeon maps, world maps, encounter areas            │
  ├────────────┼──────────────────────────────────────────────────────┤
  │ other      │ Character art, monster illustrations                 │
  ├────────────┼──────────────────────────────────────────────────────┤
  │ logo       │ Publisher branding, game system logos                │
  └────────────┴──────────────────────────────────────────────────────┘
  5. List Marker Detection (list_item_normalizer/list_marker_processor.py:25-362)

  Comprehensive bullet and numbering detection:

  # Bullet types recognized:
  BULLETS = "• ◦ ◉ ○ ▸ ► ✓ ✔ ✗ ✘ - * +"

  # Numbered formats:
  "1. 2. 3."      # Decimal
  "i. ii. iii."   # Roman lower
  "a) b) c)"      # Alpha with paren
  "(1) (2) (3)"   # Parenthesized

  TTRPG Application:
  - Class feature lists ("At 3rd level, you gain...")
  - Spell component lists
  - Equipment bullet points
  - Step-by-step procedure rules

  ---
  🔧 Novel Techniques Worth Adapting

  1. Dual-Decoder Architecture for Tables (tableformer/models/table04_rs/tablemodel04_rs.py:276-328)

  Simultaneous prediction of:
  1. Structure (tag sequence) - What cells exist and their relationships
  2. Geometry (bounding boxes) - Where cells are located

  TTRPG Adaptation: Could train a similar dual-decoder for stat blocks:
  - Structure decoder: Predicts stat block schema (name, type, AC, HP, stats, abilities, actions)
  - Bbox decoder: Locates each field spatially

  2. OTSL ↔ HTML Bidirectional Conversion (tableformer/otsl.py:125-554)

  def otsl_to_html(otsl_sequence, cell_texts):
      """Convert OTSL tags to HTML with colspan/rowspan"""

  def html_to_otsl(html_table):
      """Reverse: extract OTSL from HTML structure"""

  TTRPG Adaptation: Create stat_block_to_structured() and structured_to_stat_block() for round-trip conversion.

  3. Post-Processing Pipeline (tableformer/data_management/tf_predictor.py:812-843)

  # Cell matching: align detected cells with text content
  # Overlap correction: fix overlapping bbox predictions
  # Sync validation: ensure structure matches geometry

  TTRPG Adaptation: Post-process stat block predictions to:
  - Validate required fields present (AC, HP, etc.)
  - Fix common OCR errors in dice notation
  - Normalize stat formatting

  4. Spatial R-Tree Indexing (reading_order/reading_order_rb.py:335)

  Efficient spatial queries for element relationships:
  from rtree import index
  # Fast lookup: "what's above/below/left/right of this element?"

  TTRPG Adaptation: Use for:
  - Finding captions near stat blocks
  - Associating sidebars with adjacent content
  - Linking footnotes to rules text

  ---
  🎯 TTRPG-Specific Gaps
  ┌──────────────────────────┬─────────────────────────────────┬───────────────────────────────────────────┐
  │           Gap            │          Current State          │             Needed Extension              │
  ├──────────────────────────┼─────────────────────────────────┼───────────────────────────────────────────┤
  │ Stat Block Detection     │ Generic Form + Key-Value labels │ Train specialized stat block classifier   │
  ├──────────────────────────┼─────────────────────────────────┼───────────────────────────────────────────┤
  │ Dice Notation OCR        │ Code/Formula model (general)    │ Fine-tune for 2d6+3, DC 15, etc.          │
  ├──────────────────────────┼─────────────────────────────────┼───────────────────────────────────────────┤
  │ Sidebar Classification   │ No distinct label               │ Add SIDEBAR, CALLOUT_BOX, READ_ALOUD      │
  ├──────────────────────────┼─────────────────────────────────┼───────────────────────────────────────────┤
  │ Crunch vs Fluff          │ No semantic distinction         │ Train text classifier for content purpose │
  ├──────────────────────────┼─────────────────────────────────┼───────────────────────────────────────────┤
  │ Random Table Recognition │ Generic table detection         │ Add probability-aware table parsing       │
  ├──────────────────────────┼─────────────────────────────────┼───────────────────────────────────────────┤
  │ Game System Detection    │ None                            │ Classify D&D 5e vs PF2e vs OSR formats    │
  └──────────────────────────┴─────────────────────────────────┴───────────────────────────────────────────┘
  ---
  💡 Recommended Extensions for TTRPG

  Phase 1: Fine-Tune Layout Model Labels

  Add TTRPG-specific classes to the 17-class layout model:

  # Extended labels for TTRPG:
  17: Stat-Block         # Monster/NPC stat blocks
  18: Sidebar            # Boxed asides, tips, optional rules
  19: Read-Aloud         # Boxed text for GM to read
  20: Random-Table       # Tables with dice roll columns
  21: Spell-Block        # Formatted spell descriptions
  22: Class-Feature      # Formatted class/feat descriptions

  Training data: Annotate ~1000 pages from D&D, Pathfinder, OSR books.

  Phase 2: Stat Block Structure Model

  Create StatBlockFormer following TableFormer architecture:

  class StatBlockFormer:
      """Dual-decoder for stat block structure extraction"""

      STAT_TAGS = [
          'name', 'type', 'size', 'alignment',
          'ac', 'hp', 'speed',
          'str', 'dex', 'con', 'int', 'wis', 'cha',
          'saves', 'skills', 'senses', 'languages', 'cr',
          'trait', 'action', 'reaction', 'legendary'
      ]

  Phase 3: Dice Notation Extractor

  Extend Code/Formula model for game notation:

  class DiceNotationPredictor:
      """Extract and parse TTRPG dice expressions"""

      PATTERNS = {
          'dice_roll': r'(\d+)?d(\d+)([+-]\d+)?',
          'dc_check': r'DC\s*(\d+)\s*(\w+)?',
          'modifier': r'[+-]\d+',
          'damage_type': r'(slashing|piercing|bludgeoning|fire|...)',
      }

  Phase 4: Content Purpose Classifier

  Train text classifier for crunch vs fluff:

  class ContentPurposeClassifier:
      """Classify text segments by purpose"""

      LABELS = [
          'mechanical_rule',      # "Roll 2d6 and add..."
          'flavor_text',          # "The ancient dragon..."
          'example_of_play',      # "DM: You see a door..."
          'designer_note',        # "Optional: Some groups..."
          'reference_entry',      # Spell/item descriptions
      ]

  ---
  📊 Feature Mapping Summary
  ┌────────────────────────┬──────────────────────────────┬────────────────────────────────┐
  │       TTRPG Need       │  Docling-IBM-Models Feature  │           Readiness            │
  ├────────────────────────┼──────────────────────────────┼────────────────────────────────┤
  │ Table extraction       │ TableFormer + OTSL           │ ✅ Excellent                   │
  ├────────────────────────┼──────────────────────────────┼────────────────────────────────┤
  │ Two-column layout      │ Reading Order Predictor      │ ✅ Ready                       │
  ├────────────────────────┼──────────────────────────────┼────────────────────────────────┤
  │ List detection         │ List Marker Processor        │ ✅ Ready                       │
  ├────────────────────────┼──────────────────────────────┼────────────────────────────────┤
  │ Form/Key-Value regions │ Layout Model (classes 15-16) │ ✅ Ready                       │
  ├────────────────────────┼──────────────────────────────┼────────────────────────────────┤
  │ Image classification   │ Figure Classifier            │ ⚠️ Partial (16 classes)        │
  ├────────────────────────┼──────────────────────────────┼────────────────────────────────┤
  │ Checkbox detection     │ Layout Model (classes 13-14) │ ✅ Ready                       │
  ├────────────────────────┼──────────────────────────────┼────────────────────────────────┤
  │ Code/formula OCR       │ Code Formula Predictor       │ ⚠️ Partial (not dice-specific) │
  ├────────────────────────┼──────────────────────────────┼────────────────────────────────┤
  │ Stat block detection   │ Form + Key-Value (indirect)  │ ⚠️ Needs fine-tuning           │
  ├────────────────────────┼──────────────────────────────┼────────────────────────────────┤
  │ Sidebar detection      │ No specific label            │ ❌ Needs new class             │
  ├────────────────────────┼──────────────────────────────┼────────────────────────────────┤
  │ Dice notation          │ No support                   │ ❌ Needs new model             │
  ├────────────────────────┼──────────────────────────────┼────────────────────────────────┤
  │ Crunch/fluff split     │ No classification            │ ❌ Needs new model             │
  └────────────────────────┴──────────────────────────────┴────────────────────────────────┘
  ---
  🔬 Technical Deep-Dive: Key Algorithms

  TableFormer Inference Loop (tf_predictor.py:705-843)

  Input: Cropped table image
    ↓
  Resize (max 1024px height, preserve aspect)
    ↓
  Normalize (ImageNet mean/std)
    ↓
  ResNet-18 Encoder → Feature map (256 × 28 × 28)
    ↓
  Tag Decoder (beam search, max_steps iterations)
    → Predict OTSL sequence: [fcel, fcel, nl, ched, ched, ...]
    ↓
  BBox Decoder (parallel with tags)
    → Predict cell boxes: [(cx, cy, w, h), ...]
    ↓
  Post-process:
    1. Validate tag/bbox sync
    2. Match cells to OCR text
    3. Correct overlapping boxes
    4. Generate HTML with spans
    ↓
  Output: Structured table with cells, spans, text

  Reading Order Algorithm (reading_order_rb.py:555-584)

  Input: List of detected elements with bboxes
    ↓
  Build R-tree spatial index
    ↓
  Compute up/down/left/right relationships
    ↓
  Detect column boundaries (horizontal dilation)
    ↓
  Find "head" elements (nothing above them)
    ↓
  DFS traversal:
    - Follow down edges preferentially
    - Cross to next column when current exhausted
    - Handle interruptions (sidebars, figures)
    ↓
  Output: Ordered list of elements in reading sequence

  ---
  🎲 Bottom Line

  Docling-IBM-Models provides ~60% of the ML infrastructure needed for TTRPG processing:

  Excellent out-of-box:
  - Table structure extraction (TableFormer is production-grade)
  - Two-column layout handling
  - List/bullet detection
  - Form and key-value region detection

  Needs domain adaptation:
  - Stat block recognition (fine-tune layout model)
  - Sidebar/callout detection (add new class)
  - Dice notation OCR (extend code/formula model)
  - Content purpose classification (train new model)

  The architecture is extensible - all models use standard PyTorch with clear training pipelines, making fine-tuning for TTRPG domains feasible with ~1000-2000 annotated pages.

  ---
  🔄 Combined Pipeline: docling-core + docling-ibm-models

  TTRPG PDF Input
         ↓
  ┌──────────────────────────────────────────┐
  │ docling-ibm-models (Detection Layer)     │
  ├──────────────────────────────────────────┤
  │ 1. LayoutModel → Detect regions          │
  │    - Stat blocks (Form + Key-Value)      │
  │    - Tables, Pictures, Headers           │
  │ 2. TableFormer → Extract table structure │
  │ 3. ReadingOrder → Sequence elements      │
  │ 4. ListNormalizer → Process lists        │
  │ 5. FigureClassifier → Categorize art     │
  └──────────────────────────────────────────┘
         ↓
  ┌──────────────────────────────────────────┐
  │ docling-core (Processing Layer)          │
  ├──────────────────────────────────────────┤
  │ 1. DoclingDocument → Unified model       │
  │ 2. HierarchicalChunker → Section context │
  │ 3. HybridChunker → Token-aware splits    │
  │ 4. Serializers → MD/HTML/LaTeX output    │
  │ 5. Provenance tracking → Citations       │
  └──────────────────────────────────────────┘
         ↓
  Chunked TTRPG content with:
  - Stat blocks as structured data
  - Tables with semantic headers
  - Proper reading order
  - Section hierarchy preserved
  - Embedded images handled
