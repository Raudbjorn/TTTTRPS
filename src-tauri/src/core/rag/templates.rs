//! TTRPG-specific Liquid templates for RAG document formatting

/// System prompt for TTRPG Game Master assistant
pub const TTRPG_SYSTEM_PROMPT: &str = r#"
You are an expert TTRPG Game Master assistant with deep knowledge of tabletop roleplaying games.

When answering questions:
1. Use ONLY the provided context from the rulebooks
2. Always cite your sources with page numbers when available
3. If the context doesn't contain enough information, say so clearly
4. Format rules and mechanics clearly for quick reference
5. Distinguish between core rules and optional/variant rules

If asked about something not in the provided context, clearly state that you don't have that information in your indexed sources.
"#;

/// Liquid template for rules documents
///
/// Expected document fields:
/// - source: The rulebook name (e.g., "DMG", "PHB")
/// - page_number: Optional page number
/// - section_path: Optional section hierarchy
/// - content: The document content
pub const RULES_TEMPLATE: &str = r#"
[{{ doc.source }}{% if doc.page_number %} (p.{{ doc.page_number }}){% endif %}]
{% if doc.section_path %}Section: {{ doc.section_path }}{% endif %}
{{ doc.content }}
"#;

/// Liquid template for fiction/lore documents
///
/// Expected document fields:
/// - source: The sourcebook name
/// - page_number: Optional page number
/// - content: The document content
pub const FICTION_TEMPLATE: &str = r#"
[{{ doc.source }}{% if doc.page_number %} (p.{{ doc.page_number }}){% endif %}]
{{ doc.content }}
"#;

/// Liquid template for semantic chunks from ingested documents
///
/// Expected document fields:
/// - book_title: The full book title
/// - source_slug: Short identifier for the source
/// - page_start: Optional starting page number
/// - section_path: Optional section hierarchy
/// - content: The chunk content
pub const CHUNK_TEMPLATE: &str = r#"
[{{ doc.book_title }} - {{ doc.source_slug }}{% if doc.page_start %} (p.{{ doc.page_start }}){% endif %}]
{% if doc.section_path %}{{ doc.section_path }}{% endif %}
{{ doc.content }}
"#;

/// Template for campaign-specific context
///
/// Used when querying with campaign data
pub const CAMPAIGN_CONTEXT_TEMPLATE: &str = r#"
### CAMPAIGN CONTEXT ###
Campaign: {{ campaign.name }}
Setting: {{ campaign.setting }}
{% if campaign.current_session %}
Current Session: {{ campaign.current_session.name }}
{% endif %}
### END CAMPAIGN CONTEXT ###

{{ doc.content }}
"#;

/// Template for NPC reference cards
pub const NPC_TEMPLATE: &str = r#"
[NPC: {{ doc.name }}]
Role: {{ doc.role }}
{% if doc.faction %}Faction: {{ doc.faction }}{% endif %}
{{ doc.description }}
"#;
