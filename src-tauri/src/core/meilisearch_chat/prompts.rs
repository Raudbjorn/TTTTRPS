//! Static prompt constants for Meilisearch Chat
//!
//! These prompts are carefully designed to prevent LLMs from generating filter syntax
//! that Meilisearch cannot process, which causes invalid_search_filter errors.

// ============================================================================
// Anti-Filter Hallucination Prompts
// ============================================================================
// These prompts explicitly instruct the LLM to use only keyword searches
// and forbid filter syntax that causes Meilisearch errors.

/// Default search description - tells LLM when/how to use search
pub const DEFAULT_SEARCH_DESCRIPTION: &str = r#"Search the TTRPG knowledge base for rules, lore, creatures, spells, and game content.

WHEN TO SEARCH:
- User asks about game mechanics, rules, or stats
- User asks about specific creatures, spells, items, or characters
- User needs information from rulebooks or source materials
- You need to cite sources or verify information

DO NOT SEARCH FOR:
- Greetings or casual conversation
- Questions you can answer from conversation context
- Creative content generation (unless researching source material first)"#;

/// Default search query parameter prompt - CRITICAL for preventing filter errors
pub const DEFAULT_SEARCH_Q_PARAM: &str = r#"Generate a simple keyword search query using 2-6 relevant terms.

RULES:
1. Use ONLY plain keywords separated by spaces
2. Include specific names, terms, and concepts from the question
3. Prioritize unique/specific terms over generic ones

FORBIDDEN - NEVER USE:
- Filter operators: = != > < >= <= AND OR NOT IN TO
- Field syntax: field:value, field=value, category:X
- SQL syntax: WHERE, SELECT, LIKE, IS NULL, IS NOT NULL
- Regex operators: =~ !~ * ? [ ]
- Quotes for exact matching: "exact phrase"
- Boolean operators: && || !

EXAMPLES:
✓ "goblin stat block challenge rating"
✓ "fireball spell damage evocation"
✓ "Delta Green agent character creation"
✗ "type = monster AND cr > 5"
✗ "category:spell school:evocation"
✗ "name =~ 'dragon.*'"#;

/// Default index selection prompt
pub const DEFAULT_SEARCH_INDEX_PARAM: &str = r#"Select which index to search.

AVAILABLE INDEXES:
- 'documents': Primary index containing all uploaded PDFs, rulebooks, and source materials. USE THIS FOR MOST QUERIES.
- 'rules': Game mechanics and rulebooks
- 'fiction': Lore and narrative content
- 'chat': Conversation history

RULES:
- ALWAYS use 'documents' for rules, lore, creatures, spells, items
- Use 'rules' specifically for game mechanics queries
- Use 'fiction' for lore, story, and narrative content
- Use 'chat' to search conversation history
- NEVER invent index names or use the topic as an index name
- When in doubt, use 'documents'"#;

/// Default system prompt for the DM persona
pub const DEFAULT_DM_SYSTEM_PROMPT: &str = r#"You are an expert Dungeon Master assistant for tabletop role-playing games.

Your role is to:
- Help Game Masters run engaging sessions
- Provide rules clarifications citing specific sources
- Generate creative content (NPCs, locations, plot hooks)
- Answer questions about game mechanics
- Suggest narrative ideas that fit the campaign's tone

When answering questions:
- Search the available indexes for relevant rules and lore
- Cite your sources when providing rules information
- Be concise but thorough
- Maintain the tone appropriate to the game being played

You have access to the player's rulebooks, campaign notes, and lore documents.
Use the search tool to find relevant information before answering.
VALID INDEXES:
- `documents`: User uploaded files (PDFs, etc.)
- `rules`: Game mechanics and rulebooks
- `fiction`: Lore and narrative content
- `chat`: Conversation history

Do NOT invent index names. Only use the ones listed above."#;

/// Default model for Grok/xAI provider
pub const GROK_DEFAULT_MODEL: &str = "grok-3-mini";

/// Base URL for Grok/xAI API (OpenAI-compatible)
pub const GROK_API_BASE_URL: &str = "https://api.x.ai/v1";
