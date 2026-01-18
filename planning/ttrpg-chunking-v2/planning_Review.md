TTRPG Semantic Chunking Review
Overview
This review analyzes the docling-core codebase (specifically transforms/chunker) against the requirements outlined in "The Design Language of Tabletop Roleplaying Game Books". The goal is to identify existing features that are highly applicable to TTRPG materials and propose novel extensions.

Key Findings from Design Language
Hierarchy is Critical: TTRPG books are deeply nested (e.g., Chapter 5: Combat > Actions > Attack).
The "Spread" is the Unit: Content is often designed to fit on a visible 2-page spread.
Self-Contained Stat Blocks: Monsters/Items are discrete, structured units that must not be split.
Complex Tables: Random tables and probability matrices need precise representation.
Role-Based Content: Distinction between Player-facing and GM-facing (boxed) text.
Applicable Codebase Features
1. Hierarchical Context (
HierarchicalChunker
)
Relevance: High Why: TTRPG rules are context-dependent. A line saying "On a roll of 1-2, the weapon breaks" is useless without knowing it belongs to the "Obsidian Dagger" item description. Code Match: docling_core.transforms.chunker.hierarchical_chunker.HierarchicalChunker maintains a stack of headings in DocMeta.headings.

# From hierarchical_chunker.py
meta=DocMeta(
    doc_items=doc_items,
    headings=headings, # Captures the full path (Book > Chapter > Section)
    origin=dl_doc.origin,
),
Application: This natively solves the context problem by attaching the "ancestry" of every rule to its chunk.

2. Hybrid Merging (
HybridChunker
)
Relevance: High (for Stat Blocks) Why: Stat blocks composed of many short lines (e.g., "STR: 18", "DEX: 12") risk being shattered by naive chunkers. Code Match: docling_core.transforms.chunker.hybrid_chunker.HybridChunker uses 
_merge_chunks_with_matching_metadata
. Application: If a stat block falls under a single header (e.g., "Goblin"), the 
HybridChunker
 will attempt to merge the small attribute lines back into a single meaningful chunk, provided they fit within the token limit.

3. Triplet Table Serialization (
TripletTableSerializer
)
Relevance: Novel / High Why: TTRPG tables are often dense lookups (Roll d6 -> Result). Code Match: docling_core.transforms.chunker.hierarchical_chunker.TripletTableSerializer

# Serializes to: "Row Header, Column Header = Cell Value"
table_text_parts = [
    f"{rows[i]}, {cols[j]} = {str(table_df.iloc[i, j]).strip()}"
    ...
]
Application: This is a novel way to handle mechanics tables. Instead of a messy grid text, it creates explicit logic statements (e.g., "Encounter Table, Die Roll 6 = Ancient Red Dragon"), which is ideal for RAG retrieval.

Novel Extensions for TTRPGs
1. Spread-Aware Chunking (Extension of 
PageChunker
)
Gap: 
PageChunker
 processes single pages. Proposal: Create a SpreadChunker that iterates pages in pairs (or uses layout analysis to detect spreads). Feature: This aligns with the "control panel" design philosophy where all rules for a topic (e.g., a specific Class) are visible on one spread.

2. Semantic Region "Fencing" (Stat Block Protection)
Gap: While 
HybridChunker
 helps, it doesn't guarantee a stat block isn't split if it exceeds token limits or spans pages. Proposal: Implement a StatBlockStrategy (similar to CodeChunkingStrategy). Feature: Detect stat block boundaries (perhaps via specific styles or keywords like "STR/DEX/CON") and treat the entire block as an atomic unit, refusing to split it internally.

3. Role-Based Metadata Tagging
Gap: No native distinction for "Boxed Text". Proposal: specific detection of "boxed" or "shaded" regions during the parse phase, tagging them in 
DocMeta
. Feature: Allow filtering chunks by target audience (e.g., role="GM_ONLY" vs role="PLAYER"), crucial for preventing spoiler leakage in player-facing bots.

Findings from Codebase Search
1. Robust Heading Support (Shadowed Headings)
Source: 
test/test_hybrid_chunker.py
 Finding: The 
HybridChunker
 explicitly handles "shadowed headings" (nested sections with no immediate text content). Relevance: TTRPG books often have structure like Chapter 6: Combat > Actions > Attack. "Actions" acts as a container. docling-core already correctly preserves this full path in DocMeta.headings, ensuring the context is never lost even for deeply nested rules.

2. Flexible Metadata (
MiscAnnotation
)
Source: 
docling_core/types/doc/document.py
 Finding: The 
MiscAnnotation
 type allows attaching arbitrary dictionary 
content
 to DocItems. Relevance: This is the ideal mechanism for implementing TTRPG-specific tags without forking the core Schema. We can inject a 
MiscAnnotation(kind="ttrpg_meta", content={"role": "GM_ONLY", "rule_type": "flavor_text"})
 during the parsing stage, which the chunker can then respect.

docling-ibm-models
 Analysis
1. Layout Labels (
labels.py
)
Finding: The layout model detects a fixed set of 16 classes, including "Section-header", "Table", "Key-Value Region", and "text". TTRPG Implication: "Stat Blocks" are likely to be detected as "Key-Value Regions" or generic "Text". Since we cannot easily add "StatBlock" or "BoxedText" labels without retraining the vision model, we should rely on post-processing heuristics (e.g., looking for "Key-Value Region" clusters that contain specific keywords like STR/DEX or "Armor Class") to upgrade these regions to 
MiscAnnotation(kind="stat_block")
.

2. Complex Table Handling (TableFormer / OTSL)
Finding: The TableFormer model uses a customized sequence-tagging token set (OTSL - Open Table Structure Language) to represent tables. It explicitly handles rowspan and colspan (ucel, lcel, xcel tags in 
otsl.py
). TTRPG Implication: This is excellent for TTRPG "weapons tables" or "encounter tables" which often feature merged headers or sub-category rows. The existing logic in 
otsl_to_html
 ensures these complex grids are preserved accurately, which is a pre-requisite for the 
TripletTableSerializer
 (from docling-core) to generate meaningful semantic logical statements.
