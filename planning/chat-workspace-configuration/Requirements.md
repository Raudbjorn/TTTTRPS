# Requirements: Chat Workspace Configuration Migration

## Overview

Complete the migration of chat workspace configuration commands from HTTP-based `MeilisearchChatClient` to the embedded `meilisearch-lib` API. This is Phase 4 of the broader Meilisearch integration migration.

---

## Context

### Current State

Four Tauri commands are currently stubs returning errors:

| Command | Current Behavior |
|---------|------------------|
| `configure_chat_workspace` | Returns error: "migration in progress" |
| `get_chat_workspace_settings` | Returns `None` |
| `configure_meilisearch_chat` | Returns error: "migration in progress" |
| `reindex_library` | Returns error: "migration in progress" |

### Target State

These commands should use the embedded `MeilisearchLib` via `state.embedded_search.inner()` to:
- Configure LLM providers for RAG-powered chat
- Retrieve current workspace settings
- Support all 14 LLM providers (native + proxy)

---

## User Stories

### US-1: Configure LLM Provider
**As a** Game Master setting up the application,
**I want** to configure my preferred LLM provider (Claude, GPT-4, Mistral, etc.),
**So that** I can use AI-powered RAG queries on my indexed rulebooks.

### US-2: View Current Configuration
**As a** user troubleshooting connection issues,
**I want** to view the current chat workspace configuration,
**So that** I can verify the correct provider and settings are active.

### US-3: Switch Providers
**As a** user with multiple LLM subscriptions,
**I want** to switch between providers without restarting the application,
**So that** I can use Claude for some queries and GPT-4 for others.

### US-4: Local LLM Support
**As a** user in an offline environment,
**I want** to configure Ollama as my LLM provider,
**So that** I can use RAG queries without internet access.

### US-5: Custom System Prompts
**As a** power user,
**I want** to provide custom system prompts for RAG responses,
**So that** the AI responses match my preferred style and focus.

---

## Functional Requirements

### FR-1: Configure Chat Workspace

#### FR-1.1: Provider Configuration
- WHEN user calls `configure_chat_workspace(workspace_id, provider, prompts)` THEN system SHALL:
  1. Map `ChatProviderConfig` to `meilisearch_lib::ChatConfig`
  2. Call `meili.set_chat_config(Some(config))`
  3. Return success on completion

#### FR-1.2: Provider Mapping
- WHEN provider is `OpenAI` THEN system SHALL map to `ChatSource::OpenAi`
- WHEN provider is `Claude` THEN system SHALL map to `ChatSource::Anthropic`
- WHEN provider is `Mistral` THEN system SHALL map to `ChatSource::Mistral`
- WHEN provider is `AzureOpenAI` THEN system SHALL map to `ChatSource::AzureOpenAi`
- WHEN provider is `Ollama`, `OpenRouter`, `Groq`, `Together`, `Cohere`, `DeepSeek`, or `Gemini` THEN system SHALL map to `ChatSource::VLlm` with appropriate `base_url`
- WHEN provider is `Grok` THEN system SHALL map to `ChatSource::OpenAi` with Grok's API base URL

#### FR-1.3: Prompt Configuration
- WHEN custom prompts are provided THEN system SHALL include them in `ChatConfig.prompts`
- WHEN custom prompts are NOT provided THEN system SHALL use defaults from `core/meilisearch_chat/prompts.rs`
- WHEN prompts contain anti-filter instructions THEN system SHALL preserve them

#### FR-1.4: Index Configuration
- WHEN configuring workspace THEN system SHALL include TTRPG-specific index configurations:
  - Chunks index with semantic_ratio 0.6
  - Rules-focused indexes with semantic_ratio 0.7
  - Fiction-focused indexes with semantic_ratio 0.8

### FR-2: Get Chat Workspace Settings

#### FR-2.1: Retrieve Configuration
- WHEN user calls `get_chat_workspace_settings(workspace_id)` THEN system SHALL:
  1. Call `meili.get_chat_config()`
  2. Map `ChatConfig` to `ChatWorkspaceSettings`
  3. Mask API key in response (show only last 4 characters)
  4. Return the mapped settings

#### FR-2.2: No Configuration
- WHEN no configuration exists THEN system SHALL return `Ok(None)`

### FR-3: Convenience Configuration Command

#### FR-3.1: Simple Configuration
- WHEN user calls `configure_meilisearch_chat(provider, api_key, model, system_prompt, host)` THEN system SHALL:
  1. Parse provider string to `ChatProviderConfig`
  2. Build complete configuration with defaults
  3. Call `configure_chat_workspace` internally
  4. Return success or specific error

#### FR-3.2: Provider String Parsing
- WHEN provider is "openai" THEN system SHALL create `ChatProviderConfig::OpenAI`
- WHEN provider is "claude" THEN system SHALL create `ChatProviderConfig::Claude`
- WHEN provider is "ollama" THEN system SHALL create `ChatProviderConfig::Ollama` with provided host
- WHEN provider is unrecognized THEN system SHALL return descriptive error

### FR-4: Reindex Library

#### FR-4.1: Full Reindex
- WHEN user calls `reindex_library(None)` THEN system SHALL:
  1. List all document indexes
  2. Call `meili.delete_all_documents(uid)` for each
  3. Return list of cleared indexes

#### FR-4.2: Single Index Reindex
- WHEN user calls `reindex_library(Some(index_name))` THEN system SHALL:
  1. Call `meili.delete_all_documents(index_name)`
  2. Wait for task completion
  3. Return success message

---

## Non-Functional Requirements

### NFR-1: Performance

#### NFR-1.1: Configuration Speed
- Configuration operations SHALL complete in < 100ms (no network I/O for embedded)

#### NFR-1.2: Settings Retrieval
- `get_chat_workspace_settings` SHALL return in < 10ms

### NFR-2: Error Handling

#### NFR-2.1: Clear Error Messages
- All errors SHALL include:
  - What operation failed
  - Why it failed (if determinable)
  - Suggested remediation

#### NFR-2.2: No Panics
- No configuration operation SHALL panic
- All errors SHALL be propagated as `Result<T, String>`

### NFR-3: Backward Compatibility

#### NFR-3.1: API Signature Preservation
- Command signatures SHALL remain unchanged
- Return types SHALL remain unchanged
- Frontend bindings SHALL work without modification

### NFR-4: Security

#### NFR-4.1: API Key Handling
- API keys SHALL be accepted but NOT logged
- Retrieved settings SHALL mask API keys (show last 4 chars only)
- API keys in `ChatConfig` are stored in memory only (not persisted to disk by meilisearch-lib)

---

## Constraints

### C-1: No HTTP Communication
- All operations SHALL use direct `MeilisearchLib` method calls
- No HTTP requests to external Meilisearch instances

### C-2: Type Compatibility
- `ChatProviderConfig` (project type) must map cleanly to `meilisearch_lib::ChatConfig`
- `ChatWorkspaceSettings` (project type) must be constructible from `meilisearch_lib::ChatConfig`

### C-3: Proxy Provider Support
- Providers not natively supported by meilisearch-lib (Claude, Gemini, etc.) SHALL continue working via `ChatSource::VLlm` with appropriate proxy URLs
- This maintains feature parity with the HTTP-based implementation

---

## Assumptions

### A-1: MeilisearchLib Stability
- `meili.set_chat_config()` and `meili.get_chat_config()` APIs are stable
- `ChatConfig` structure matches design expectations

### A-2: Thread Safety
- `EmbeddedSearch.inner()` returns a reference that can be called from async contexts
- `set_chat_config` uses internal locking (RwLock) for thread safety

### A-3: Provider Proxy
- For non-native providers, an LLM proxy service is assumed to be available
- Proxy URL is passed in configuration

---

## Glossary

| Term | Definition |
|------|------------|
| **Workspace** | Named configuration context for chat (e.g., "dm-assistant") |
| **ChatConfig** | meilisearch-lib's configuration type for LLM integration |
| **ChatProviderConfig** | Project's provider enum with credentials |
| **ChatWorkspaceSettings** | Project's settings DTO returned to frontend |
| **ChatSource** | meilisearch-lib's provider enum (OpenAi, Anthropic, etc.) |
| **Native Provider** | Provider directly supported by meilisearch-lib |
| **Proxy Provider** | Provider routed through LLM proxy as VLlm endpoint |
