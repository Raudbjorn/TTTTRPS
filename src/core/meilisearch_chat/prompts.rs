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

// ============================================================================
// Provider Constants
// ============================================================================

/// Default model for Grok/xAI provider
pub const GROK_DEFAULT_MODEL: &str = "grok-3-mini";

/// Base URL for Grok/xAI API (OpenAI-compatible)
pub const GROK_API_BASE_URL: &str = "https://api.x.ai/v1";

/// Base URL for Google Gemini (OpenAI-compatible endpoint)
pub const GOOGLE_API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/openai";

/// Base URL for OpenRouter
pub const OPENROUTER_API_BASE_URL: &str = "https://openrouter.ai/api/v1";

/// Base URL for Groq (OpenAI-compatible)
pub const GROQ_API_BASE_URL: &str = "https://api.groq.com/openai/v1";

/// Base URL for Together.ai
pub const TOGETHER_API_BASE_URL: &str = "https://api.together.xyz/v1";

/// Base URL for Cohere
pub const COHERE_API_BASE_URL: &str = "https://api.cohere.ai/v1";

/// Base URL for DeepSeek
pub const DEEPSEEK_API_BASE_URL: &str = "https://api.deepseek.com/v1";

// ============================================================================
// Default Models
// ============================================================================

/// Default model for Ollama
pub const OLLAMA_DEFAULT_MODEL: &str = "llama3.2";

/// Default host for Ollama
pub const OLLAMA_DEFAULT_HOST: &str = "http://localhost:11434";

/// Default model for Azure OpenAI
pub const AZURE_DEFAULT_DEPLOYMENT: &str = "gpt-4";

/// Default API version for Azure OpenAI
pub const AZURE_DEFAULT_API_VERSION: &str = "2024-06-01";

/// Default model for Groq
pub const GROQ_DEFAULT_MODEL: &str = "llama-3.3-70b-versatile";

/// Default model for Cohere
pub const COHERE_DEFAULT_MODEL: &str = "command-r-plus";

/// Default model for DeepSeek
pub const DEEPSEEK_DEFAULT_MODEL: &str = "deepseek-chat";

/// Default model for Google Gemini
pub const GOOGLE_DEFAULT_MODEL: &str = "gemini-2.0-flash";

// ============================================================================
// Placeholder API Keys
// ============================================================================
// Meilisearch requires a non-empty API key even for providers that don't need one.

/// Placeholder API key for Ollama (no auth required)
pub const OLLAMA_API_KEY_PLACEHOLDER: &str = "ollama";

/// Placeholder API key for OAuth-proxy providers (ClaudeOAuth, Gemini, Copilot)
pub const OAUTH_PROXY_API_KEY_PLACEHOLDER: &str = "oauth-proxy";

// ============================================================================
// Timeouts
// ============================================================================

/// Timeout in seconds for waiting on Meilisearch task completion
pub const TASK_COMPLETION_TIMEOUT_SECS: u64 = 60;
