# Design: Chat Workspace Configuration Migration

## Overview

This document describes the technical design for completing the chat workspace configuration migration from HTTP-based `MeilisearchChatClient` to embedded `meilisearch-lib` API calls.

---

## 1. Architecture

### 1.1 Current Flow (Stubs)

```
Frontend                    Backend (Stubs)
   │                            │
   │ configure_meilisearch_chat │
   ├───────────────────────────►│
   │                            │ Returns error:
   │◄───────────────────────────┤ "migration in progress"
   │                            │
```

### 1.2 Target Flow

```
Frontend                    Backend                     EmbeddedSearch
   │                            │                            │
   │ configure_meilisearch_chat │                            │
   ├───────────────────────────►│                            │
   │                            │ parse_provider_config()    │
   │                            ├────────────────────────────┤
   │                            │ build_chat_config()        │
   │                            ├────────────────────────────┤
   │                            │ meili.set_chat_config()    │
   │                            ├───────────────────────────►│
   │                            │◄───────────────────────────┤
   │◄───────────────────────────┤ Ok(())                     │
   │                            │                            │
```

---

## 2. Component Design

### 2.1 Provider Mapping Function

**Location**: `src-tauri/src/commands/search/meilisearch.rs`

Maps project's `ChatProviderConfig` to `meilisearch_lib::ChatConfig`:

```rust
use meilisearch_lib::{ChatConfig, ChatSource, ChatPrompts as LibChatPrompts};

fn map_provider_to_chat_config(
    provider: &ChatProviderConfig,
    custom_prompts: Option<&ChatPrompts>,
    proxy_url: Option<&str>,
) -> Result<ChatConfig, String> {
    let (source, api_key, model, base_url) = match provider {
        ChatProviderConfig::OpenAI { api_key, model, .. } => {
            (ChatSource::OpenAi, api_key.clone(), model.clone(), None)
        }
        ChatProviderConfig::Claude { api_key, model, .. } => {
            // Claude via Anthropic native support
            (ChatSource::Anthropic, api_key.clone(), model.clone(), None)
        }
        ChatProviderConfig::Mistral { api_key, model, .. } => {
            (ChatSource::Mistral, api_key.clone(), model.clone(), None)
        }
        ChatProviderConfig::AzureOpenAI { api_key, base_url, deployment_id, api_version } => {
            (ChatSource::AzureOpenAi, api_key.clone(), None, Some(base_url.clone()))
        }
        ChatProviderConfig::Grok { api_key, model } => {
            // Grok is OpenAI-compatible
            (ChatSource::OpenAi, api_key.clone(), model.clone(), Some(GROK_API_BASE_URL.to_string()))
        }
        ChatProviderConfig::Ollama { host, model } => {
            // Ollama via VLlm endpoint
            (ChatSource::VLlm, String::new(), Some(model.clone()), Some(host.clone()))
        }
        // Proxy providers
        ChatProviderConfig::Gemini { .. } |
        ChatProviderConfig::OpenRouter { .. } |
        ChatProviderConfig::Groq { .. } |
        ChatProviderConfig::Together { .. } |
        ChatProviderConfig::Cohere { .. } |
        ChatProviderConfig::DeepSeek { .. } => {
            // Route through LLM proxy
            let url = proxy_url.ok_or("Proxy URL required for this provider")?;
            (ChatSource::VLlm, provider.api_key().unwrap_or_default(), provider.model(), Some(url.to_string()))
        }
    };

    let prompts = build_prompts(custom_prompts);
    let index_configs = build_ttrpg_index_configs();

    Ok(ChatConfig {
        source,
        api_key,
        base_url,
        model: model.unwrap_or_default(),
        org_id: provider.org_id(),
        project_id: provider.project_id(),
        api_version: provider.api_version(),
        deployment_id: provider.deployment_id(),
        prompts,
        index_configs,
    })
}
```

### 2.2 Prompt Builder

**Location**: `src-tauri/src/commands/search/meilisearch.rs`

Builds `meilisearch_lib::ChatPrompts` with anti-filter defaults:

```rust
use crate::core::meilisearch_chat::{
    DEFAULT_DM_SYSTEM_PROMPT, DEFAULT_SEARCH_DESCRIPTION,
    DEFAULT_SEARCH_Q_PARAM, DEFAULT_SEARCH_INDEX_PARAM,
};

fn build_prompts(custom: Option<&ChatPrompts>) -> meilisearch_lib::ChatPrompts {
    let custom = custom.cloned().unwrap_or_default();

    meilisearch_lib::ChatPrompts {
        system: custom.system.or_else(|| Some(DEFAULT_DM_SYSTEM_PROMPT.to_string())),
        search_description: custom.search_description.or_else(|| Some(DEFAULT_SEARCH_DESCRIPTION.to_string())),
        search_q_param: custom.search_q_param.or_else(|| Some(DEFAULT_SEARCH_Q_PARAM.to_string())),
        search_filter_param: custom.search_filter_param,  // None by default
        search_index_uid_param: custom.search_index_uid_param.or_else(|| Some(DEFAULT_SEARCH_INDEX_PARAM.to_string())),
    }
}
```

### 2.3 TTRPG Index Configuration

**Location**: `src-tauri/src/commands/search/meilisearch.rs`

Pre-configured index settings for TTRPG content:

```rust
use meilisearch_lib::{ChatIndexConfig, ChatSearchParams};
use std::collections::HashMap;

fn build_ttrpg_index_configs() -> HashMap<String, ChatIndexConfig> {
    let mut configs = HashMap::new();

    // Chunks index (general document chunks)
    configs.insert("chunks".to_string(), ChatIndexConfig {
        description: "Semantic chunks from ingested TTRPG documents including rules, lore, and mechanics".to_string(),
        template: Some(CHUNK_TEMPLATE.to_string()),
        max_bytes: Some(600),
        search_params: Some(ChatSearchParams {
            limit: Some(10),
            semantic_ratio: Some(0.6),
            embedder: Some("default".to_string()),
            sort: None,
            matching_strategy: None,
        }),
    });

    // Per-document indexes follow the pattern: {slug}
    // These are dynamically added based on ingested documents
    // The default config covers unknown indexes

    configs
}

const CHUNK_TEMPLATE: &str = r#"[{{ doc.book_title }} - {{ doc.source_slug }}{% if doc.page_start %} (p.{{ doc.page_start }}){% endif %}]
{% if doc.section_path %}{{ doc.section_path }}{% endif %}
{{ doc.content }}"#;
```

### 2.4 Settings Mapper

**Location**: `src-tauri/src/commands/search/meilisearch.rs`

Maps `meilisearch_lib::ChatConfig` back to `ChatWorkspaceSettings`:

```rust
fn map_config_to_settings(config: &meilisearch_lib::ChatConfig) -> ChatWorkspaceSettings {
    let source = match config.source {
        meilisearch_lib::ChatSource::OpenAi => ChatLLMSource::OpenAi,
        meilisearch_lib::ChatSource::Anthropic => ChatLLMSource::Anthropic,
        meilisearch_lib::ChatSource::Mistral => ChatLLMSource::Mistral,
        meilisearch_lib::ChatSource::AzureOpenAi => ChatLLMSource::AzureOpenAi,
        meilisearch_lib::ChatSource::VLlm => ChatLLMSource::VLlm,
    };

    ChatWorkspaceSettings {
        source,
        api_key: mask_api_key(&config.api_key),
        deployment_id: config.deployment_id.clone(),
        api_version: config.api_version.clone(),
        org_id: config.org_id.clone(),
        project_id: config.project_id.clone(),
        prompts: Some(map_lib_prompts(&config.prompts)),
        base_url: config.base_url.clone(),
    }
}

fn mask_api_key(key: &str) -> Option<String> {
    if key.is_empty() {
        None
    } else if key.len() <= 8 {
        Some("****".to_string())
    } else {
        Some(format!("{}...{}", &key[..4], &key[key.len()-4..]))
    }
}

fn map_lib_prompts(prompts: &meilisearch_lib::ChatPrompts) -> ChatPrompts {
    ChatPrompts {
        system: prompts.system.clone(),
        search_description: prompts.search_description.clone(),
        search_q_param: prompts.search_q_param.clone(),
        search_filter_param: prompts.search_filter_param.clone(),
        search_index_uid_param: prompts.search_index_uid_param.clone(),
    }
}
```

---

## 3. Command Implementations

### 3.1 configure_chat_workspace

```rust
#[tauri::command]
pub async fn configure_chat_workspace(
    workspace_id: String,
    provider: ChatProviderConfig,
    custom_prompts: Option<ChatPrompts>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let meili = state.embedded_search.inner();

    // Get proxy URL from state if needed for non-native providers
    let proxy_url = state.llm_proxy_url.as_deref();

    // Map provider to meilisearch-lib ChatConfig
    let config = map_provider_to_chat_config(&provider, custom_prompts.as_ref(), proxy_url)?;

    // Set the configuration
    meili.set_chat_config(Some(config));

    log::info!(
        "Chat workspace '{}' configured with provider: {}",
        workspace_id,
        provider.provider_id()
    );

    Ok(())
}
```

### 3.2 get_chat_workspace_settings

```rust
#[tauri::command]
pub async fn get_chat_workspace_settings(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<Option<ChatWorkspaceSettings>, String> {
    let meili = state.embedded_search.inner();

    match meili.get_chat_config() {
        Some(config) => {
            let settings = map_config_to_settings(&config);
            log::debug!("Retrieved chat workspace settings for '{}'", workspace_id);
            Ok(Some(settings))
        }
        None => {
            log::debug!("No chat configuration found for workspace '{}'", workspace_id);
            Ok(None)
        }
    }
}
```

### 3.3 configure_meilisearch_chat

```rust
#[tauri::command]
pub async fn configure_meilisearch_chat(
    provider: String,
    api_key: Option<String>,
    model: Option<String>,
    custom_system_prompt: Option<String>,
    host: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Parse provider string to enum
    let provider_config = parse_provider_string(&provider, api_key, model, host)?;

    // Build custom prompts if system prompt provided
    let custom_prompts = custom_system_prompt.map(|prompt| {
        ChatPrompts::with_system_prompt(&prompt)
    });

    // Delegate to configure_chat_workspace
    let meili = state.embedded_search.inner();
    let proxy_url = state.llm_proxy_url.as_deref();
    let config = map_provider_to_chat_config(&provider_config, custom_prompts.as_ref(), proxy_url)?;

    meili.set_chat_config(Some(config));

    log::info!("Meilisearch chat configured with provider: {}", provider);

    Ok(())
}

fn parse_provider_string(
    provider: &str,
    api_key: Option<String>,
    model: Option<String>,
    host: Option<String>,
) -> Result<ChatProviderConfig, String> {
    match provider.to_lowercase().as_str() {
        "openai" => {
            let key = api_key.ok_or("API key required for OpenAI")?;
            Ok(ChatProviderConfig::OpenAI {
                api_key: key,
                model,
                organization_id: None,
            })
        }
        "claude" | "anthropic" => {
            let key = api_key.ok_or("API key required for Claude")?;
            Ok(ChatProviderConfig::Claude {
                api_key: key,
                model,
                max_tokens: Some(4096),
            })
        }
        "mistral" => {
            let key = api_key.ok_or("API key required for Mistral")?;
            Ok(ChatProviderConfig::Mistral {
                api_key: key,
                model,
            })
        }
        "ollama" => {
            let host = host.unwrap_or_else(|| "http://localhost:11434".to_string());
            let model = model.unwrap_or_else(|| "llama3".to_string());
            Ok(ChatProviderConfig::Ollama { host, model })
        }
        "grok" => {
            let key = api_key.ok_or("API key required for Grok")?;
            Ok(ChatProviderConfig::Grok {
                api_key: key,
                model,
            })
        }
        "gemini" | "google" => {
            let key = api_key.ok_or("API key required for Gemini")?;
            Ok(ChatProviderConfig::Gemini {
                api_key: key,
                model,
            })
        }
        "openrouter" => {
            let key = api_key.ok_or("API key required for OpenRouter")?;
            Ok(ChatProviderConfig::OpenRouter {
                api_key: key,
                model,
            })
        }
        "groq" => {
            let key = api_key.ok_or("API key required for Groq")?;
            Ok(ChatProviderConfig::Groq {
                api_key: key,
                model,
            })
        }
        "together" => {
            let key = api_key.ok_or("API key required for Together")?;
            Ok(ChatProviderConfig::Together {
                api_key: key,
                model,
            })
        }
        "cohere" => {
            let key = api_key.ok_or("API key required for Cohere")?;
            Ok(ChatProviderConfig::Cohere {
                api_key: key,
                model,
            })
        }
        "deepseek" => {
            let key = api_key.ok_or("API key required for DeepSeek")?;
            Ok(ChatProviderConfig::DeepSeek {
                api_key: key,
                model,
            })
        }
        "azure" => {
            Err("Azure OpenAI requires base_url, deployment_id, and api_version. Use configure_chat_workspace() for Azure.".to_string())
        }
        _ => Err(format!("Unknown provider: '{}'. Supported: openai, claude, mistral, ollama, grok, gemini, openrouter, groq, together, cohere, deepseek", provider))
    }
}
```

### 3.4 reindex_library

```rust
#[tauri::command]
pub async fn reindex_library(
    index_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let meili = state.embedded_search.inner();

    if let Some(name) = index_name {
        // Single index reindex
        let task = meili.delete_all_documents(&name)
            .map_err(|e| format!("Failed to clear index '{}': {}", name, e))?;

        // Wait for task completion
        meili.wait_for_task(task.uid, Some(std::time::Duration::from_secs(60)))
            .map_err(|e| format!("Task failed: {}", e))?;

        log::info!("Cleared index '{}' for reindexing", name);
        Ok(format!("Index '{}' cleared. Re-ingest documents to rebuild.", name))
    } else {
        // All indexes - list and clear each
        let indexes = meili.list_indexes(None, None)
            .map_err(|e| format!("Failed to list indexes: {}", e))?;

        let mut cleared = Vec::new();
        for index in indexes.results {
            if let Ok(task) = meili.delete_all_documents(&index.uid) {
                let _ = meili.wait_for_task(task.uid, Some(std::time::Duration::from_secs(60)));
                cleared.push(index.uid);
            }
        }

        log::info!("Cleared {} indexes for reindexing", cleared.len());
        Ok(format!("Cleared indexes: {}. Re-ingest documents to rebuild.", cleared.join(", ")))
    }
}
```

---

## 4. Data Flow

### 4.1 Configuration Flow

```
User selects provider in Settings UI
                │
                ▼
Frontend: invoke("configure_meilisearch_chat", {
    provider: "claude",
    api_key: "sk-ant-...",
    model: "claude-sonnet-4-20250514",
    custom_system_prompt: null
})
                │
                ▼
Backend: parse_provider_string("claude", ...)
                │
                ▼
Backend: map_provider_to_chat_config()
    ├── source: Anthropic
    ├── api_key: "sk-ant-..."
    ├── model: "claude-sonnet-4-20250514"
    └── prompts: {anti-filter defaults}
                │
                ▼
MeilisearchLib: set_chat_config(Some(config))
    └── Stores in Arc<RwLock<Option<ChatConfig>>>
                │
                ▼
Response: Ok(())
```

### 4.2 Settings Retrieval Flow

```
User opens Settings panel
                │
                ▼
Frontend: invoke("get_chat_workspace_settings", {
    workspace_id: "dm-assistant"
})
                │
                ▼
Backend: meili.get_chat_config()
                │
                ▼
Backend: map_config_to_settings()
    ├── Masks API key
    └── Maps ChatSource → ChatLLMSource
                │
                ▼
Response: Some({
    source: "anthropic",
    api_key: "sk-a...3xyz",
    model: "claude-sonnet-4-20250514",
    ...
})
```

---

## 5. Error Handling

### 5.1 Error Types

| Error | Cause | User Message |
|-------|-------|--------------|
| Missing API Key | Provider requires key but none provided | "API key required for {provider}" |
| Unknown Provider | Provider string not recognized | "Unknown provider: '{name}'. Supported: openai, claude, ..." |
| Proxy Required | Non-native provider but no proxy URL | "Proxy URL required for this provider" |
| Index Not Found | Reindex on nonexistent index | "Index '{name}' not found" |
| Task Timeout | delete_all_documents task exceeded 60s | "Task timed out" |

### 5.2 Error Propagation

All errors are converted to `String` for Tauri command returns. Internal errors from meilisearch-lib are wrapped with context:

```rust
meili.delete_all_documents(&name)
    .map_err(|e| format!("Failed to clear index '{}': {}", name, e))?;
```

---

## 6. Thread Safety

The `MeilisearchLib` instance uses internal locking:

```rust
// From meili-dev/crates/meilisearch-lib/src/client.rs
chat_config: Arc<RwLock<Option<ChatConfig>>>,
```

- `get_chat_config()`: Acquires read lock, clones config
- `set_chat_config()`: Acquires write lock, replaces config

This is safe for concurrent access from multiple Tauri command handlers.

---

## 7. Testing Strategy

### 7.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_provider_string_openai() {
        let result = parse_provider_string("openai", Some("sk-test".into()), None, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().provider_id(), "openai");
    }

    #[test]
    fn test_parse_provider_string_missing_key() {
        let result = parse_provider_string("claude", None, None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("API key required"));
    }

    #[test]
    fn test_mask_api_key() {
        assert_eq!(mask_api_key(""), None);
        assert_eq!(mask_api_key("short"), Some("****".into()));
        assert_eq!(mask_api_key("sk-ant-api-key-here"), Some("sk-a...here".into()));
    }

    #[test]
    fn test_map_provider_to_chat_config() {
        let provider = ChatProviderConfig::OpenAI {
            api_key: "test".into(),
            model: Some("gpt-4".into()),
            organization_id: None,
        };
        let config = map_provider_to_chat_config(&provider, None, None).unwrap();
        assert!(matches!(config.source, meilisearch_lib::ChatSource::OpenAi));
    }
}
```

### 7.2 Integration Tests

```rust
#[tokio::test]
async fn test_configure_and_retrieve_chat_settings() {
    let meili = setup_test_meilisearch();

    // Configure
    let config = ChatConfig {
        source: ChatSource::OpenAi,
        api_key: "sk-test".into(),
        model: "gpt-4".into(),
        // ...
    };
    meili.set_chat_config(Some(config));

    // Retrieve
    let retrieved = meili.get_chat_config();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().model, "gpt-4");
}
```

---

## 8. Decisions Log

### Decision 1: Workspace ID Parameter Unused

**Context**: `workspace_id` is passed to commands but meilisearch-lib has a single global `ChatConfig`.

**Decision**: Accept `workspace_id` for API compatibility but ignore it internally.

**Rationale**:
- Maintains frontend API compatibility
- Embedded meilisearch doesn't support multiple workspaces
- Future HTTP mode could use workspace IDs

### Decision 2: Proxy Providers via VLlm

**Context**: Claude, Gemini, etc. aren't natively supported by meilisearch-lib.

**Decision**: Route through `ChatSource::VLlm` with LLM proxy URL as `base_url`.

**Rationale**:
- Maintains feature parity with HTTP implementation
- Leverages existing proxy infrastructure
- VLlm endpoint is OpenAI-compatible

### Decision 3: In-Memory Configuration Only

**Context**: meilisearch-lib stores `ChatConfig` in `Arc<RwLock>`, not persisted to disk.

**Decision**: Accept this limitation; app should restore config on startup.

**Rationale**:
- Matches embedded architecture (no persistent workspace LMDB)
- App already stores provider config in keyring/settings
- Configuration is restored during app initialization
