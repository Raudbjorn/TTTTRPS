//! Meilisearch Configuration Commands
//!
//! Commands for Meilisearch health checks, reindexing, and chat workspace configuration.
//! Uses the embedded MeilisearchLib for direct in-process operations.

use std::collections::HashMap;
use std::time::Duration;

use tauri::State;

use meilisearch_lib::{
    ChatConfig, ChatIndexConfig, ChatPrompts as MeiliChatPrompts, ChatSearchParams, ChatSource,
};

use crate::commands::AppState;
use crate::core::meilisearch_chat::{
    ChatLLMSource, ChatPrompts, ChatProviderConfig, ChatProviderInfo, ChatWorkspaceSettings,
    list_chat_providers as get_chat_providers, AZURE_DEFAULT_API_VERSION,
    AZURE_DEFAULT_DEPLOYMENT, COHERE_API_BASE_URL, COHERE_DEFAULT_MODEL, DEEPSEEK_API_BASE_URL,
    DEEPSEEK_DEFAULT_MODEL, DEFAULT_DM_SYSTEM_PROMPT, DEFAULT_SEARCH_DESCRIPTION,
    DEFAULT_SEARCH_INDEX_PARAM, DEFAULT_SEARCH_Q_PARAM, GOOGLE_API_BASE_URL,
    GOOGLE_DEFAULT_MODEL, GROK_API_BASE_URL, GROK_DEFAULT_MODEL, GROQ_API_BASE_URL,
    GROQ_DEFAULT_MODEL, OAUTH_PROXY_API_KEY_PLACEHOLDER, OLLAMA_API_KEY_PLACEHOLDER,
    OLLAMA_DEFAULT_HOST, OLLAMA_DEFAULT_MODEL, OPENROUTER_API_BASE_URL,
    TASK_COMPLETION_TIMEOUT_SECS, TOGETHER_API_BASE_URL,
};
use crate::core::llm::model_selector::model_selector;

use super::types::MeilisearchStatus;

// ============================================================================
// Helper Functions
// ============================================================================

/// Mask an API key for safe display. Shows first 4 and last 4 characters.
///
/// Uses character (not byte) indexing to avoid panics on multi-byte UTF-8 strings,
/// though API keys are typically ASCII-only.
///
/// # Returns
/// - `None` if the key is empty
/// - `Some("****")` if the key is 8 characters or fewer
/// - `Some("sk-t...xYzW")` for longer keys
fn mask_api_key(key: &str) -> Option<String> {
    if key.is_empty() {
        return None;
    }
    let char_count = key.chars().count();
    if char_count <= 8 {
        return Some("****".to_string());
    }
    let prefix: String = key.chars().take(4).collect();
    let suffix: String = key.chars().skip(char_count - 4).collect();
    Some(format!("{}...{}", prefix, suffix))
}

/// Build TTRPG-specific index configurations for chat context retrieval.
///
/// Configures the `chunks` index with:
/// - Semantic ratio 0.6 for hybrid search
/// - Liquid template for document rendering
/// - 400 byte max per document chunk
fn build_ttrpg_index_configs() -> HashMap<String, ChatIndexConfig> {
    let mut configs = HashMap::new();

    configs.insert(
        "chunks".to_string(),
        ChatIndexConfig {
            description: "Semantic chunks from TTRPG rulebooks, source materials, lore, and campaign notes. Contains rules, spells, creatures, items, and game mechanics.".to_string(),
            template: Some(
                "{{ content }}\n---\nSource: {{ source_name }} (p.{{ page_number }})".to_string(),
            ),
            max_bytes: Some(400),
            search_params: Some(ChatSearchParams {
                limit: Some(8),
                sort: None,
                matching_strategy: Some("last".to_string()),
                semantic_ratio: Some(0.6),
                embedder: Some("default".to_string()),
            }),
        },
    );

    configs.insert(
        "documents".to_string(),
        ChatIndexConfig {
            description: "Raw pages from uploaded PDFs, EPUBs, and documents. Use for full-text search when chunks index lacks detail.".to_string(),
            template: Some(
                "{{ content }}\n---\nSource: {{ source_name }} (p.{{ page_number }})".to_string(),
            ),
            max_bytes: Some(600),
            search_params: Some(ChatSearchParams {
                limit: Some(5),
                sort: None,
                matching_strategy: Some("last".to_string()),
                semantic_ratio: Some(0.5),
                embedder: Some("default".to_string()),
            }),
        },
    );

    configs
}

/// Build MeilisearchLib ChatPrompts by merging custom prompts with anti-filter defaults.
///
/// Custom prompts override defaults where provided. The anti-filter search parameter
/// prompts are always included to prevent LLM filter hallucination errors.
fn build_prompts(custom_prompts: Option<&ChatPrompts>) -> MeiliChatPrompts {
    match custom_prompts {
        Some(prompts) => MeiliChatPrompts {
            system: prompts
                .system
                .clone()
                .or_else(|| Some(DEFAULT_DM_SYSTEM_PROMPT.to_string())),
            search_description: prompts
                .search_description
                .clone()
                .or_else(|| Some(DEFAULT_SEARCH_DESCRIPTION.to_string())),
            search_q_param: prompts
                .search_q_param
                .clone()
                .or_else(|| Some(DEFAULT_SEARCH_Q_PARAM.to_string())),
            search_filter_param: prompts.search_filter_param.clone(),
            search_index_uid_param: prompts
                .search_index_uid_param
                .clone()
                .or_else(|| Some(DEFAULT_SEARCH_INDEX_PARAM.to_string())),
        },
        None => MeiliChatPrompts {
            system: Some(DEFAULT_DM_SYSTEM_PROMPT.to_string()),
            search_description: Some(DEFAULT_SEARCH_DESCRIPTION.to_string()),
            search_q_param: Some(DEFAULT_SEARCH_Q_PARAM.to_string()),
            search_filter_param: None,
            search_index_uid_param: Some(DEFAULT_SEARCH_INDEX_PARAM.to_string()),
        },
    }
}

/// Map a ChatProviderConfig to a meilisearch_lib::ChatConfig.
///
/// # Provider Mapping
///
/// | App Provider    | ChatSource   | Notes                                |
/// |-----------------|--------------|--------------------------------------|
/// | OpenAI          | OpenAi       | Native support                       |
/// | Claude          | Anthropic    | Native support with API key          |
/// | Mistral         | Mistral      | Native support                       |
/// | AzureOpenAI     | AzureOpenAi  | Requires base_url, deployment_id     |
/// | Grok            | VLlm         | OpenAI-compatible via xAI base URL   |
/// | Ollama          | VLlm         | Local model via /v1 endpoint         |
/// | Google          | VLlm         | Via proxy or direct with base_url    |
/// | OpenRouter      | VLlm         | Via proxy                            |
/// | Groq            | VLlm         | Via proxy                            |
/// | Together        | VLlm         | Via proxy                            |
/// | Cohere          | VLlm         | Via proxy                            |
/// | DeepSeek        | VLlm         | Via proxy                            |
/// | ClaudeOAuth     | VLlm         | Via proxy (OAuth, no API key)        |
/// | Gemini          | VLlm         | Via proxy (OAuth, no API key)        |
/// | Copilot         | VLlm         | Via proxy (OAuth, no API key)        |
fn map_provider_to_chat_config(
    provider: &ChatProviderConfig,
    custom_prompts: Option<&ChatPrompts>,
) -> Result<ChatConfig, String> {
    let prompts = build_prompts(custom_prompts);
    let index_configs = build_ttrpg_index_configs();

    let config = match provider {
        ChatProviderConfig::OpenAI {
            api_key,
            model,
            organization_id,
        } => ChatConfig {
            source: ChatSource::OpenAi,
            api_key: api_key.clone(),
            base_url: None,
            model: model.as_deref().unwrap_or("gpt-4o-mini").to_string(),
            org_id: organization_id.clone(),
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts,
            index_configs,
        },

        ChatProviderConfig::Claude {
            api_key,
            model,
            ..
        } => ChatConfig {
            source: ChatSource::Anthropic,
            api_key: api_key.clone(),
            base_url: None,
            model: model
                .clone()
                .unwrap_or_else(|| model_selector().select_model_sync()),
            org_id: None,
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts,
            index_configs,
        },

        ChatProviderConfig::Mistral { api_key, model } => ChatConfig {
            source: ChatSource::Mistral,
            api_key: api_key.clone(),
            base_url: None,
            model: model
                .as_deref()
                .unwrap_or("mistral-large-latest")
                .to_string(),
            org_id: None,
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts,
            index_configs,
        },

        ChatProviderConfig::AzureOpenAI {
            api_key,
            base_url,
            deployment_id,
            api_version,
        } => ChatConfig {
            source: ChatSource::AzureOpenAi,
            api_key: api_key.clone(),
            base_url: Some(base_url.clone()),
            model: deployment_id.clone(),
            org_id: None,
            project_id: None,
            api_version: Some(api_version.clone()),
            deployment_id: Some(deployment_id.clone()),
            prompts,
            index_configs,
        },

        ChatProviderConfig::Grok { api_key, model } => ChatConfig {
            source: ChatSource::VLlm,
            api_key: api_key.clone(),
            base_url: Some(GROK_API_BASE_URL.to_string()),
            model: model.as_deref().unwrap_or(GROK_DEFAULT_MODEL).to_string(),
            org_id: None,
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts,
            index_configs,
        },

        ChatProviderConfig::Ollama { host, model } => {
            let base_url = format!("{}/v1", host.trim_end_matches('/'));
            ChatConfig {
                source: ChatSource::VLlm,
                api_key: OLLAMA_API_KEY_PLACEHOLDER.to_string(),
                base_url: Some(base_url),
                model: model.clone(),
                org_id: None,
                project_id: None,
                api_version: None,
                deployment_id: None,
                prompts,
                index_configs,
            }
        }

        ChatProviderConfig::Google { api_key, model } => ChatConfig {
            source: ChatSource::VLlm,
            api_key: api_key.clone(),
            base_url: Some(GOOGLE_API_BASE_URL.to_string()),
            model: model
                .as_deref()
                .unwrap_or(GOOGLE_DEFAULT_MODEL)
                .to_string(),
            org_id: None,
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts,
            index_configs,
        },

        // Proxy-routed providers: use VLlm source with proxy base_url
        ChatProviderConfig::OpenRouter { api_key, model } => ChatConfig {
            source: ChatSource::VLlm,
            api_key: api_key.clone(),
            base_url: Some(OPENROUTER_API_BASE_URL.to_string()),
            model: model.clone(),
            org_id: None,
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts,
            index_configs,
        },

        ChatProviderConfig::Groq { api_key, model } => ChatConfig {
            source: ChatSource::VLlm,
            api_key: api_key.clone(),
            base_url: Some(GROQ_API_BASE_URL.to_string()),
            model: model.clone(),
            org_id: None,
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts,
            index_configs,
        },

        ChatProviderConfig::Together { api_key, model } => ChatConfig {
            source: ChatSource::VLlm,
            api_key: api_key.clone(),
            base_url: Some(TOGETHER_API_BASE_URL.to_string()),
            model: model.clone(),
            org_id: None,
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts,
            index_configs,
        },

        ChatProviderConfig::Cohere { api_key, model } => ChatConfig {
            source: ChatSource::VLlm,
            api_key: api_key.clone(),
            base_url: Some(COHERE_API_BASE_URL.to_string()),
            model: model.clone(),
            org_id: None,
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts,
            index_configs,
        },

        ChatProviderConfig::DeepSeek { api_key, model } => ChatConfig {
            source: ChatSource::VLlm,
            api_key: api_key.clone(),
            base_url: Some(DEEPSEEK_API_BASE_URL.to_string()),
            model: model.clone(),
            org_id: None,
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts,
            index_configs,
        },

        // OAuth-based providers: no API key, use placeholder via VLlm
        ChatProviderConfig::ClaudeOAuth { model, .. } => ChatConfig {
            source: ChatSource::VLlm,
            api_key: OAUTH_PROXY_API_KEY_PLACEHOLDER.to_string(),
            base_url: None, // Will use proxy URL if configured
            model: model.clone(),
            org_id: None,
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts,
            index_configs,
        },

        ChatProviderConfig::Gemini { model, .. } => ChatConfig {
            source: ChatSource::VLlm,
            api_key: OAUTH_PROXY_API_KEY_PLACEHOLDER.to_string(),
            base_url: None,
            model: model.clone(),
            org_id: None,
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts,
            index_configs,
        },

        ChatProviderConfig::Copilot { model, .. } => ChatConfig {
            source: ChatSource::VLlm,
            api_key: OAUTH_PROXY_API_KEY_PLACEHOLDER.to_string(),
            base_url: None,
            model: model.clone(),
            org_id: None,
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts,
            index_configs,
        },
    };

    Ok(config)
}

/// Map a meilisearch_lib::ChatConfig back to ChatWorkspaceSettings with masked API key.
fn map_config_to_settings(config: &ChatConfig) -> ChatWorkspaceSettings {
    let source = match config.source {
        ChatSource::OpenAi => ChatLLMSource::OpenAi,
        ChatSource::AzureOpenAi => ChatLLMSource::AzureOpenAi,
        ChatSource::Mistral => ChatLLMSource::Mistral,
        ChatSource::VLlm => ChatLLMSource::VLlm,
        ChatSource::Anthropic => ChatLLMSource::Anthropic,
    };

    let prompts = {
        let meili_prompts = &config.prompts;
        Some(ChatPrompts {
            system: meili_prompts.system.clone(),
            search_description: meili_prompts.search_description.clone(),
            search_q_param: meili_prompts.search_q_param.clone(),
            search_filter_param: meili_prompts.search_filter_param.clone(),
            search_index_uid_param: meili_prompts.search_index_uid_param.clone(),
        })
    };

    ChatWorkspaceSettings {
        source,
        api_key: mask_api_key(&config.api_key),
        deployment_id: config.deployment_id.clone(),
        api_version: config.api_version.clone(),
        org_id: config.org_id.clone(),
        project_id: config.project_id.clone(),
        prompts,
        base_url: config.base_url.clone(),
    }
}

/// Parse a provider string identifier and parameters into a ChatProviderConfig.
///
/// # Arguments
///
/// * `provider` - Provider type string (e.g., "openai", "claude", "ollama")
/// * `api_key` - API key (required for most providers, optional for ollama)
/// * `model` - Model name (optional, uses provider defaults)
/// * `host` - Host URL for ollama (optional, defaults to localhost:11434)
///
/// # Errors
///
/// Returns an error if:
/// - The provider string is unknown
/// - A required API key is missing
fn parse_provider_string(
    provider: &str,
    api_key: Option<String>,
    model: Option<String>,
    host: Option<String>,
) -> Result<ChatProviderConfig, String> {
    let normalized = provider.trim().to_lowercase();

    match normalized.as_str() {
        "openai" => {
            let key = api_key.ok_or_else(|| {
                "OpenAI requires an API key. Set it in Settings > API Keys.".to_string()
            })?;
            Ok(ChatProviderConfig::OpenAI {
                api_key: key,
                model,
                organization_id: None,
            })
        }

        "claude" | "anthropic" => {
            let key = api_key.ok_or_else(|| {
                "Anthropic Claude requires an API key. Set it in Settings > API Keys.".to_string()
            })?;
            Ok(ChatProviderConfig::Claude {
                api_key: key,
                model,
                max_tokens: None,
            })
        }

        "mistral" => {
            let key = api_key.ok_or_else(|| {
                "Mistral requires an API key. Set it in Settings > API Keys.".to_string()
            })?;
            Ok(ChatProviderConfig::Mistral {
                api_key: key,
                model,
            })
        }

        "ollama" => {
            let ollama_host = host.unwrap_or_else(|| OLLAMA_DEFAULT_HOST.to_string());
            let ollama_model = model.unwrap_or_else(|| OLLAMA_DEFAULT_MODEL.to_string());
            Ok(ChatProviderConfig::Ollama {
                host: ollama_host,
                model: ollama_model,
            })
        }

        "google" | "gemini" => {
            let key = api_key.ok_or_else(|| {
                "Google Gemini requires an API key. Set it in Settings > API Keys.".to_string()
            })?;
            Ok(ChatProviderConfig::Google {
                api_key: key,
                model,
            })
        }

        "openrouter" => {
            let key = api_key.ok_or_else(|| {
                "OpenRouter requires an API key. Set it in Settings > API Keys.".to_string()
            })?;
            let m = model.ok_or_else(|| {
                "OpenRouter requires a model name (e.g., 'openai/gpt-4o').".to_string()
            })?;
            Ok(ChatProviderConfig::OpenRouter {
                api_key: key,
                model: m,
            })
        }

        "azure" | "azure_openai" | "azureopenai" => {
            let key = api_key.ok_or_else(|| {
                "Azure OpenAI requires an API key. Set it in Settings > API Keys.".to_string()
            })?;
            let base = host.ok_or_else(|| {
                "Azure OpenAI requires a base URL (host). Provide the Azure endpoint URL."
                    .to_string()
            })?;
            Ok(ChatProviderConfig::AzureOpenAI {
                api_key: key,
                base_url: base,
                deployment_id: model.unwrap_or_else(|| AZURE_DEFAULT_DEPLOYMENT.to_string()),
                api_version: AZURE_DEFAULT_API_VERSION.to_string(),
            })
        }

        "groq" => {
            let key = api_key.ok_or_else(|| {
                "Groq requires an API key. Set it in Settings > API Keys.".to_string()
            })?;
            let m = model.unwrap_or_else(|| GROQ_DEFAULT_MODEL.to_string());
            Ok(ChatProviderConfig::Groq {
                api_key: key,
                model: m,
            })
        }

        "together" | "together.ai" | "togetherai" => {
            let key = api_key.ok_or_else(|| {
                "Together.ai requires an API key. Set it in Settings > API Keys.".to_string()
            })?;
            let m = model.ok_or_else(|| {
                "Together.ai requires a model name (e.g., 'meta-llama/Llama-3-70b-chat-hf')."
                    .to_string()
            })?;
            Ok(ChatProviderConfig::Together {
                api_key: key,
                model: m,
            })
        }

        "cohere" => {
            let key = api_key.ok_or_else(|| {
                "Cohere requires an API key. Set it in Settings > API Keys.".to_string()
            })?;
            let m = model.unwrap_or_else(|| COHERE_DEFAULT_MODEL.to_string());
            Ok(ChatProviderConfig::Cohere {
                api_key: key,
                model: m,
            })
        }

        "deepseek" => {
            let key = api_key.ok_or_else(|| {
                "DeepSeek requires an API key. Set it in Settings > API Keys.".to_string()
            })?;
            let m = model.unwrap_or_else(|| DEEPSEEK_DEFAULT_MODEL.to_string());
            Ok(ChatProviderConfig::DeepSeek {
                api_key: key,
                model: m,
            })
        }

        "grok" | "xai" => {
            let key = api_key.ok_or_else(|| {
                "Grok (xAI) requires an API key. Set it in Settings > API Keys.".to_string()
            })?;
            Ok(ChatProviderConfig::Grok {
                api_key: key,
                model,
            })
        }

        _ => Err(format!(
            "Unknown provider '{}'. Supported providers: openai, claude, mistral, ollama, \
             gemini, openrouter, azure, groq, together, cohere, deepseek, grok",
            provider
        )),
    }
}

// ============================================================================
// Meilisearch Health and Indexing Commands
// ============================================================================

/// Get Meilisearch health status with per-index document counts.
#[tauri::command]
pub async fn check_meilisearch_health(
    state: State<'_, AppState>,
) -> Result<MeilisearchStatus, String> {
    let meili = state.embedded_search.inner();
    let health = meili.health();
    let healthy = health.status == "available";

    // Gather per-index document counts via spawn_blocking (synchronous MeilisearchLib API)
    let meili_stats = state.embedded_search.clone_inner();
    let counts = tokio::task::spawn_blocking(move || -> Result<HashMap<String, u64>, String> {
        let mut counts = HashMap::new();
        let (_, indexes) = meili_stats
            .list_indexes(0, 200)
            .map_err(|e| format!("Failed to list indexes: {}", e))?;
        for index in indexes {
            match meili_stats.index_stats(&index.uid) {
                Ok(stats) => {
                    counts.insert(index.uid, stats.number_of_documents);
                }
                Err(e) => {
                    log::warn!("Failed to get stats for index '{}': {}", index.uid, e);
                }
            }
        }
        Ok(counts)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .unwrap_or_default();

    Ok(MeilisearchStatus {
        healthy,
        host: "embedded".to_string(),
        document_counts: if counts.is_empty() {
            None
        } else {
            Some(counts)
        },
    })
}

/// Indexes that may be cleared via reindex_library.
///
/// Uses an **allow-list** rather than a deny-list to prevent accidental clearing
/// of system-critical indexes (campaigns, archetypes, personality templates, etc.).
/// Only document library content indexes belong here.
const REINDEXABLE_INDEXES: &[&str] = &["rules", "fiction", "documents", "library_metadata"];

/// Reindex a library by clearing all documents from the specified index (or all indexes).
///
/// # Arguments
///
/// * `index_name` - If `Some`, clears only the named index. If `None`, clears all
///   non-protected indexes. Protected indexes (e.g., `chat`) are always skipped.
///
/// # Returns
///
/// A user-friendly message indicating what was cleared.
#[tauri::command]
pub async fn reindex_library(
    index_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let meili = state.embedded_search.clone_inner();

    match index_name {
        Some(ref name) => {
            // Only allow clearing known library content indexes
            if !REINDEXABLE_INDEXES.contains(&name.as_str()) {
                return Err(format!(
                    "Index '{}' is not a library content index and cannot be cleared via reindex. \
                     Allowed indexes: {}",
                    name,
                    REINDEXABLE_INDEXES.join(", ")
                ));
            }

            log::info!("reindex_library: clearing documents from index '{}'", name);

            let uid = name.clone();
            let task = tokio::task::spawn_blocking(move || {
                meili.delete_all_documents(&uid)
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))?
            .map_err(|e| format!("Failed to clear index '{}': {}", name, e))?;

            // Wait for task completion with 60s timeout
            let task_id = task.uid;
            let meili2 = state.embedded_search.clone_inner();
            let completed = tokio::task::spawn_blocking(move || {
                meili2.wait_for_task(task_id, Some(Duration::from_secs(TASK_COMPLETION_TIMEOUT_SECS)))
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))?
            .map_err(|e| format!("Timeout waiting for reindex of '{}': {}", name, e))?;

            log::info!(
                "reindex_library: cleared index '{}', task {} status: {:?}",
                name,
                completed.uid,
                completed.status
            );

            Ok(format!(
                "Cleared all documents from index '{}'. Re-ingest documents to rebuild.",
                name
            ))
        }

        None => {
            log::info!("reindex_library: clearing all indexes");

            // Paginate through all indexes (page size 200)
            const PAGE_SIZE: usize = 200;
            let mut all_indexes = Vec::new();
            let mut offset: usize = 0;

            loop {
                let meili_list = state.embedded_search.clone_inner();
                let current_offset = offset;
                let (total, page) = tokio::task::spawn_blocking(move || {
                    meili_list.list_indexes(current_offset, PAGE_SIZE)
                })
                .await
                .map_err(|e| format!("Task join error: {}", e))?
                .map_err(|e| format!("Failed to list indexes: {}", e))?;

                if total == 0 && all_indexes.is_empty() {
                    return Ok("No indexes found to clear.".to_string());
                }

                let page_len = page.len();
                all_indexes.extend(page);

                if all_indexes.len() >= total || page_len < PAGE_SIZE {
                    break;
                }
                offset += page_len;
            }

            // Only include known library content indexes (allow-list)
            let indexes: Vec<_> = all_indexes
                .into_iter()
                .filter(|idx| REINDEXABLE_INDEXES.contains(&idx.uid.as_str()))
                .collect();
            let total = indexes.len();

            if total == 0 {
                return Ok("No clearable library indexes found.".to_string());
            }

            let mut cleared = Vec::new();
            let mut errors = Vec::new();

            for index in &indexes {
                let uid = index.uid.clone();
                let meili_del = state.embedded_search.clone_inner();

                match tokio::task::spawn_blocking(move || meili_del.delete_all_documents(&uid))
                    .await
                {
                    Ok(Ok(task)) => {
                        let task_id = task.uid;
                        let meili_wait = state.embedded_search.clone_inner();

                        match tokio::task::spawn_blocking(move || {
                            meili_wait.wait_for_task(task_id, Some(Duration::from_secs(TASK_COMPLETION_TIMEOUT_SECS)))
                        })
                        .await
                        {
                            Ok(Ok(_)) => cleared.push(index.uid.clone()),
                            Ok(Err(e)) => {
                                errors.push(format!("{}: {}", index.uid, e));
                            }
                            Err(e) => {
                                errors.push(format!("{}: join error: {}", index.uid, e));
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        errors.push(format!("{}: {}", index.uid, e));
                    }
                    Err(e) => {
                        errors.push(format!("{}: join error: {}", index.uid, e));
                    }
                }
            }

            log::info!(
                "reindex_library: cleared {} of {} indexes. Errors: {}",
                cleared.len(),
                total,
                errors.len()
            );

            if errors.is_empty() {
                Ok(format!(
                    "Cleared all documents from {} indexes. Re-ingest documents to rebuild.",
                    cleared.len()
                ))
            } else {
                Err(format!(
                    "Partial failure: cleared {} of {} indexes. Errors: {}",
                    cleared.len(),
                    total,
                    errors.join("; ")
                ))
            }
        }
    }
}

// ============================================================================
// Meilisearch Chat Provider Commands
// ============================================================================

/// List available chat providers with their capabilities.
#[tauri::command]
pub fn list_chat_providers() -> Vec<ChatProviderInfo> {
    get_chat_providers()
}

/// Configure the global Meilisearch chat with a specific LLM provider.
///
/// Maps the provided `ChatProviderConfig` to a `meilisearch_lib::ChatConfig` and
/// sets it on the embedded Meilisearch instance. The configuration is global (not
/// per-workspace) and takes effect immediately.
#[tauri::command]
pub async fn configure_chat_workspace(
    provider: ChatProviderConfig,
    custom_prompts: Option<ChatPrompts>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!(
        "configure_chat_workspace: provider='{}'",
        provider.provider_id()
    );

    // map_provider_to_chat_config may call model_selector().select_model_sync() which
    // does blocking file I/O. Wrap in spawn_blocking to avoid stalling the Tokio runtime.
    let provider_clone = provider.clone();
    let config = tokio::task::spawn_blocking(move || {
        map_provider_to_chat_config(&provider_clone, custom_prompts.as_ref())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))??;

    let meili = state.embedded_search.inner();
    meili.set_chat_config(Some(config));

    log::info!(
        "configure_chat_workspace: configured provider '{}' (native: {})",
        provider.provider_id(),
        !provider.requires_proxy()
    );

    Ok(())
}

/// Get the current global Meilisearch chat settings.
///
/// Returns `None` if no chat configuration has been set. API keys in the response
/// are masked for security (e.g., "sk-t...xYzW"). The configuration is global
/// (not per-workspace).
#[tauri::command]
pub async fn get_chat_workspace_settings(
    state: State<'_, AppState>,
) -> Result<Option<ChatWorkspaceSettings>, String> {
    log::debug!("get_chat_workspace_settings: reading global config");

    let meili = state.embedded_search.inner();

    match meili.get_chat_config() {
        Some(config) => {
            let settings = map_config_to_settings(&config);
            Ok(Some(settings))
        }
        None => {
            log::debug!("get_chat_workspace_settings: no configuration set");
            Ok(None)
        }
    }
}

/// Configure Meilisearch chat workspace with individual parameters.
///
/// This is a convenience command that builds the `ChatProviderConfig` from
/// individual parameters, making it easier to call from the frontend.
///
/// # Arguments
///
/// * `provider` - Provider type: "openai", "claude", "mistral", "gemini", "ollama",
///                "openrouter", "groq", "together", "cohere", "deepseek", "grok"
/// * `api_key` - API key for the provider (optional for ollama)
/// * `model` - Model to use (optional, uses provider default if not specified)
/// * `custom_system_prompt` - Custom system prompt (optional)
/// * `host` - Host URL for ollama or Azure (optional)
#[tauri::command]
pub async fn configure_meilisearch_chat(
    provider: String,
    api_key: Option<String>,
    model: Option<String>,
    custom_system_prompt: Option<String>,
    host: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!("configure_meilisearch_chat: provider='{}'", provider);

    let provider_config = parse_provider_string(&provider, api_key, model, host)?;

    let custom_prompts = custom_system_prompt.map(|prompt| ChatPrompts::with_system_prompt(&prompt));

    // map_provider_to_chat_config may call model_selector().select_model_sync() which
    // does blocking file I/O. Wrap in spawn_blocking to avoid stalling the Tokio runtime.
    let config = tokio::task::spawn_blocking(move || {
        map_provider_to_chat_config(&provider_config, custom_prompts.as_ref())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))??;

    let meili = state.embedded_search.inner();
    meili.set_chat_config(Some(config));

    log::info!(
        "configure_meilisearch_chat: configured provider '{}'",
        provider,
    );

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Fake API key for tests. Clearly not a real credential.
    const TEST_FAKE_API_KEY: &str = "test_00000000_fake_key_00000000";

    // ========================================================================
    // mask_api_key tests
    // ========================================================================

    #[test]
    fn test_mask_api_key_empty() {
        assert_eq!(mask_api_key(""), None);
    }

    #[test]
    fn test_mask_api_key_short() {
        assert_eq!(mask_api_key("abc"), Some("****".to_string()));
        assert_eq!(mask_api_key("12345678"), Some("****".to_string()));
    }

    #[test]
    fn test_mask_api_key_normal() {
        let masked = mask_api_key("sk-test1234567890abcd").unwrap();
        assert!(masked.starts_with("sk-t"));
        assert!(masked.ends_with("abcd"));
        assert!(masked.contains("..."));
    }

    #[test]
    fn test_mask_api_key_exact_boundary() {
        // 9 chars: should show first 4 and last 4
        let masked = mask_api_key("123456789").unwrap();
        assert_eq!(masked, "1234...6789");
    }

    // ========================================================================
    // parse_provider_string tests
    // ========================================================================

    #[test]
    fn test_parse_openai() {
        let config = parse_provider_string("openai", Some("sk-test".into()), Some("gpt-4o".into()), None);
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.provider_id(), "openai");
    }

    #[test]
    fn test_parse_openai_missing_key() {
        let result = parse_provider_string("openai", None, None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("API key"));
    }

    #[test]
    fn test_parse_claude() {
        let config = parse_provider_string("claude", Some("sk-ant-test".into()), None, None);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().provider_id(), "claude");
    }

    #[test]
    fn test_parse_anthropic_alias() {
        let config = parse_provider_string("anthropic", Some("sk-ant-test".into()), None, None);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().provider_id(), "claude");
    }

    #[test]
    fn test_parse_mistral() {
        let config = parse_provider_string("mistral", Some("key".into()), None, None);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().provider_id(), "mistral");
    }

    #[test]
    fn test_parse_ollama_defaults() {
        let config = parse_provider_string("ollama", None, None, None);
        assert!(config.is_ok());
        if let ChatProviderConfig::Ollama { host, model } = config.unwrap() {
            assert_eq!(host, OLLAMA_DEFAULT_HOST);
            assert_eq!(model, OLLAMA_DEFAULT_MODEL);
        } else {
            panic!("Expected Ollama variant");
        }
    }

    #[test]
    fn test_parse_ollama_custom_host() {
        let config = parse_provider_string(
            "ollama",
            None,
            Some("mixtral".into()),
            Some("http://192.168.1.100:11434".into()),
        );
        assert!(config.is_ok());
        if let ChatProviderConfig::Ollama { host, model } = config.unwrap() {
            assert_eq!(host, "http://192.168.1.100:11434");
            assert_eq!(model, "mixtral");
        } else {
            panic!("Expected Ollama variant");
        }
    }

    #[test]
    fn test_parse_gemini_alias() {
        let config = parse_provider_string("gemini", Some("key".into()), None, None);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().provider_id(), "google");
    }

    #[test]
    fn test_parse_grok() {
        let config = parse_provider_string("grok", Some("key".into()), None, None);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().provider_id(), "grok");
    }

    #[test]
    fn test_parse_xai_alias() {
        let config = parse_provider_string("xai", Some("key".into()), None, None);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().provider_id(), "grok");
    }

    #[test]
    fn test_parse_unknown_provider() {
        let result = parse_provider_string("unknown_provider", Some("key".into()), None, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Unknown provider"));
        assert!(err.contains("unknown_provider"));
    }

    #[test]
    fn test_parse_case_insensitive() {
        let config = parse_provider_string("OpenAI", Some("key".into()), None, None);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().provider_id(), "openai");
    }

    #[test]
    fn test_parse_groq() {
        let config = parse_provider_string("groq", Some("key".into()), None, None);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().provider_id(), "groq");
    }

    #[test]
    fn test_parse_together() {
        let config = parse_provider_string(
            "together",
            Some("key".into()),
            Some("meta-llama/Llama-3-70b-chat-hf".into()),
            None,
        );
        assert!(config.is_ok());
        assert_eq!(config.unwrap().provider_id(), "together");
    }

    #[test]
    fn test_parse_deepseek() {
        let config = parse_provider_string("deepseek", Some("key".into()), None, None);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().provider_id(), "deepseek");
    }

    #[test]
    fn test_parse_cohere() {
        let config = parse_provider_string("cohere", Some("key".into()), None, None);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().provider_id(), "cohere");
    }

    // ========================================================================
    // map_provider_to_chat_config tests
    // ========================================================================

    #[test]
    fn test_map_openai_provider() {
        let provider = ChatProviderConfig::OpenAI {
            api_key: "sk-test-key".to_string(),
            model: Some("gpt-4o".to_string()),
            organization_id: Some("org-123".to_string()),
        };

        let config = map_provider_to_chat_config(&provider, None).unwrap();

        assert_eq!(config.source, ChatSource::OpenAi);
        assert_eq!(config.api_key, "sk-test-key");
        assert_eq!(config.model, "gpt-4o");
        assert_eq!(config.org_id, Some("org-123".to_string()));
        assert!(config.base_url.is_none());
        assert!(!config.index_configs.is_empty());
    }

    #[test]
    fn test_map_claude_provider() {
        let provider = ChatProviderConfig::Claude {
            api_key: "sk-ant-test".to_string(),
            model: Some("claude-sonnet-4-20250514".to_string()),
            max_tokens: Some(4096),
        };

        let config = map_provider_to_chat_config(&provider, None).unwrap();

        assert_eq!(config.source, ChatSource::Anthropic);
        assert_eq!(config.api_key, "sk-ant-test");
        assert_eq!(config.model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_map_ollama_provider() {
        let provider = ChatProviderConfig::Ollama {
            host: "http://localhost:11434".to_string(),
            model: "llama3.2".to_string(),
        };

        let config = map_provider_to_chat_config(&provider, None).unwrap();

        assert_eq!(config.source, ChatSource::VLlm);
        assert_eq!(config.api_key, "ollama");
        assert_eq!(config.model, "llama3.2");
        assert_eq!(
            config.base_url,
            Some("http://localhost:11434/v1".to_string())
        );
    }

    #[test]
    fn test_map_grok_provider() {
        let provider = ChatProviderConfig::Grok {
            api_key: "xai-test-key".to_string(),
            model: None,
        };

        let config = map_provider_to_chat_config(&provider, None).unwrap();

        assert_eq!(config.source, ChatSource::VLlm);
        assert_eq!(config.model, GROK_DEFAULT_MODEL);
        assert_eq!(config.base_url, Some(GROK_API_BASE_URL.to_string()));
    }

    #[test]
    fn test_map_azure_provider() {
        let provider = ChatProviderConfig::AzureOpenAI {
            api_key: "azure-key".to_string(),
            base_url: "https://myinstance.openai.azure.com".to_string(),
            deployment_id: "gpt-4".to_string(),
            api_version: "2024-06-01".to_string(),
        };

        let config = map_provider_to_chat_config(&provider, None).unwrap();

        assert_eq!(config.source, ChatSource::AzureOpenAi);
        assert_eq!(config.deployment_id, Some("gpt-4".to_string()));
        assert_eq!(config.api_version, Some("2024-06-01".to_string()));
        assert_eq!(
            config.base_url,
            Some("https://myinstance.openai.azure.com".to_string())
        );
    }

    #[test]
    fn test_map_proxy_providers() {
        // OpenRouter
        let provider = ChatProviderConfig::OpenRouter {
            api_key: "or-key".to_string(),
            model: "openai/gpt-4o".to_string(),
        };
        let config = map_provider_to_chat_config(&provider, None).unwrap();
        assert_eq!(config.source, ChatSource::VLlm);
        assert!(config.base_url.is_some());

        // Groq
        let provider = ChatProviderConfig::Groq {
            api_key: "groq-key".to_string(),
            model: "llama-3.3-70b-versatile".to_string(),
        };
        let config = map_provider_to_chat_config(&provider, None).unwrap();
        assert_eq!(config.source, ChatSource::VLlm);

        // DeepSeek
        let provider = ChatProviderConfig::DeepSeek {
            api_key: "ds-key".to_string(),
            model: "deepseek-chat".to_string(),
        };
        let config = map_provider_to_chat_config(&provider, None).unwrap();
        assert_eq!(config.source, ChatSource::VLlm);
    }

    // ========================================================================
    // build_prompts tests
    // ========================================================================

    #[test]
    fn test_build_prompts_defaults() {
        let prompts = build_prompts(None);

        assert!(prompts.system.is_some());
        assert!(prompts.search_description.is_some());
        assert!(prompts.search_q_param.is_some());
        assert!(prompts.search_index_uid_param.is_some());
        assert!(prompts.search_filter_param.is_none());

        // Verify anti-filter content
        let q_param = prompts.search_q_param.unwrap();
        assert!(q_param.contains("FORBIDDEN"));
    }

    #[test]
    fn test_build_prompts_custom_system() {
        let custom = ChatPrompts::with_system_prompt("You are a TTRPG expert.");
        let prompts = build_prompts(Some(&custom));

        assert_eq!(
            prompts.system,
            Some("You are a TTRPG expert.".to_string())
        );
        // Anti-filter defaults should still be present
        assert!(prompts.search_q_param.is_some());
        assert!(prompts.search_index_uid_param.is_some());
    }

    #[test]
    fn test_build_prompts_partial_override() {
        let custom = ChatPrompts {
            system: Some("Custom system".to_string()),
            search_description: Some("Custom search desc".to_string()),
            search_q_param: None, // Should fall back to default
            search_index_uid_param: None,
            search_filter_param: None,
        };
        let prompts = build_prompts(Some(&custom));

        assert_eq!(prompts.system, Some("Custom system".to_string()));
        assert_eq!(
            prompts.search_description,
            Some("Custom search desc".to_string())
        );
        // Falls back to default
        assert!(prompts.search_q_param.is_some());
        assert!(prompts
            .search_q_param
            .as_ref()
            .unwrap()
            .contains("FORBIDDEN"));
    }

    // ========================================================================
    // build_ttrpg_index_configs tests
    // ========================================================================

    #[test]
    fn test_ttrpg_index_configs() {
        let configs = build_ttrpg_index_configs();

        assert!(configs.contains_key("chunks"));
        assert!(configs.contains_key("documents"));

        let chunks = &configs["chunks"];
        assert!(chunks.description.contains("TTRPG"));
        assert_eq!(chunks.max_bytes, Some(400));

        let search_params = chunks.search_params.as_ref().unwrap();
        assert_eq!(search_params.semantic_ratio, Some(0.6));
        assert_eq!(search_params.limit, Some(8));
    }

    // ========================================================================
    // map_config_to_settings tests
    // ========================================================================

    #[test]
    fn test_map_config_to_settings_openai() {
        let config = ChatConfig {
            source: ChatSource::OpenAi,
            api_key: TEST_FAKE_API_KEY.to_string(),
            base_url: None,
            model: "gpt-4o".to_string(),
            org_id: Some("org-123".to_string()),
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts: MeiliChatPrompts::default(),
            index_configs: HashMap::new(),
        };

        let settings = map_config_to_settings(&config);

        assert_eq!(settings.source, ChatLLMSource::OpenAi);
        // API key should be masked
        let masked_key = settings.api_key.unwrap();
        assert!(masked_key.contains("..."));
        assert!(!masked_key.contains("real-key-for"));
        assert_eq!(settings.org_id, Some("org-123".to_string()));
    }

    #[test]
    fn test_map_config_to_settings_anthropic() {
        let config = ChatConfig {
            source: ChatSource::Anthropic,
            api_key: TEST_FAKE_API_KEY.to_string(),
            base_url: None,
            model: "claude-sonnet-4-20250514".to_string(),
            org_id: None,
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts: MeiliChatPrompts::default(),
            index_configs: HashMap::new(),
        };

        let settings = map_config_to_settings(&config);
        assert_eq!(settings.source, ChatLLMSource::Anthropic);
    }

    #[test]
    fn test_map_config_to_settings_vllm() {
        let config = ChatConfig {
            source: ChatSource::VLlm,
            api_key: "ollama".to_string(),
            base_url: Some("http://localhost:11434/v1".to_string()),
            model: "llama3.2".to_string(),
            org_id: None,
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts: MeiliChatPrompts::default(),
            index_configs: HashMap::new(),
        };

        let settings = map_config_to_settings(&config);

        assert_eq!(settings.source, ChatLLMSource::VLlm);
        assert_eq!(
            settings.base_url,
            Some("http://localhost:11434/v1".to_string())
        );
    }
}
