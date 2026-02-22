//! Meilisearch Chat HTTP Client
//!
//! Handles low-level HTTP communication with Meilisearch's experimental Chat API,
//! including SSE stream processing and error handling.

use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::core::llm::providers::ProviderConfig;

use super::config::{ChatPrompts, ChatProviderConfig, ChatWorkspaceSettings};
use super::prompts::DEFAULT_DM_SYSTEM_PROMPT;
use super::types::{
    ChatCompletionRequest, ChatMessage, MeilisearchErrorResponse, ParsedToolCall, StreamChunk,
    get_meilisearch_chat_tools,
};

// ============================================================================
// Chat Client
// ============================================================================

/// Meilisearch Chat Client
pub struct MeilisearchChatClient {
    host: String,
    api_key: Option<String>,
    http_client: reqwest::Client,
}

impl MeilisearchChatClient {
    pub fn new(host: &str, api_key: Option<&str>) -> Self {
        Self {
            host: host.to_string(),
            api_key: api_key.map(|s| s.to_string()),
            http_client: reqwest::Client::new(),
        }
    }

    /// Get the host URL
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Enable experimental chat features
    pub async fn enable_chat_feature(&self) -> Result<(), String> {
        let url = format!("{}/experimental-features", self.host);

        let mut request = self.http_client
            .patch(&url)
            .json(&serde_json::json!({
                "chatCompletions": true
            }));

        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await
            .map_err(|e| format!("Failed to enable chat feature: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Failed to enable chat feature: {}", error));
        }

        log::info!("Meilisearch chat feature enabled");
        Ok(())
    }

    /// Configure a chat workspace
    pub async fn configure_workspace(
        &self,
        workspace_id: &str,
        settings: &ChatWorkspaceSettings,
    ) -> Result<(), String> {
        let url = format!("{}/chats/{}/settings", self.host, workspace_id);

        let mut request = self.http_client
            .patch(&url)
            .json(settings);

        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await
            .map_err(|e| format!("Failed to configure workspace: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Failed to configure workspace: {}", error));
        }

        log::info!("Configured chat workspace: {}", workspace_id);
        Ok(())
    }

    /// Get workspace settings
    pub async fn get_workspace_settings(
        &self,
        workspace_id: &str,
    ) -> Result<Option<ChatWorkspaceSettings>, String> {
        let url = format!("{}/chats/{}/settings", self.host, workspace_id);

        let mut request = self.http_client.get(&url);

        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await
            .map_err(|e| format!("Failed to get workspace settings: {}", e))?;

        if response.status().is_success() {
            let settings = response.json().await
                .map_err(|e| format!("Failed to parse settings: {}", e))?;
            Ok(Some(settings))
        } else if response.status().as_u16() == 404 {
            Ok(None)
        } else {
            let error = response.text().await.unwrap_or_default();
            Err(format!("Failed to get workspace settings: {}", error))
        }
    }

    /// Configure a workspace with a specific chat provider
    ///
    /// This is a convenience method that converts a ChatProviderConfig to
    /// Meilisearch settings and configures the workspace.
    ///
    /// # Arguments
    /// * `workspace_id` - The workspace identifier
    /// * `provider` - The chat provider configuration
    /// * `proxy_url` - URL of the LLM proxy (for non-native providers)
    /// * `custom_prompts` - Optional custom prompts to override defaults
    pub async fn configure_workspace_with_provider(
        &self,
        workspace_id: &str,
        provider: &ChatProviderConfig,
        proxy_url: &str,
        custom_prompts: Option<ChatPrompts>,
    ) -> Result<(), String> {
        // First ensure chat feature is enabled
        self.enable_chat_feature().await?;

        // Convert provider config to Meilisearch settings
        let mut settings = provider.to_meilisearch_settings(proxy_url);

        // Apply custom prompts if provided
        if let Some(prompts) = custom_prompts {
            settings.prompts = Some(prompts);
        }

        // Configure the workspace
        self.configure_workspace(workspace_id, &settings).await?;

        log::info!(
            "Configured workspace '{}' with provider: {} (native: {})",
            workspace_id,
            provider.provider_id(),
            !provider.requires_proxy()
        );

        Ok(())
    }

    /// Create a chat completion with streaming
    pub async fn chat_completion_stream(
        &self,
        workspace_id: &str,
        request: ChatCompletionRequest,
    ) -> Result<mpsc::Receiver<Result<String, String>>, String> {
        let url = format!("{}/chats/{}/chat/completions", self.host, workspace_id);

        let mut http_request = self.http_client
            .post(&url)
            .json(&request);

        if let Some(key) = &self.api_key {
            http_request = http_request.header("Authorization", format!("Bearer {}", key));
        }

        let response = http_request.send().await
            .map_err(|e| format!("Chat request failed: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Chat completion failed: {}", error));
        }

        // Create channel for streaming responses
        let (tx, rx) = mpsc::channel(100);

        // Spawn task to process SSE stream
        tokio::spawn(async move {
            process_sse_stream(response, tx).await;
        });

        Ok(rx)
    }

    /// Non-streaming chat completion (collects full response)
    pub async fn chat_completion(
        &self,
        workspace_id: &str,
        messages: Vec<ChatMessage>,
        model: &str,
    ) -> Result<String, String> {
        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages,
            stream: true, // Meilisearch only supports streaming
            temperature: Some(0.7),
            max_tokens: Some(2048),
            tools: Some(get_meilisearch_chat_tools()),
        };

        let mut rx = self.chat_completion_stream(workspace_id, request).await?;
        let mut full_response = String::new();

        while let Some(result) = rx.recv().await {
            match result {
                Ok(content) => {
                    if content == "[DONE]" {
                        break;
                    }
                    full_response.push_str(&content);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(full_response)
    }

    /// Delete a workspace
    pub async fn delete_workspace(&self, workspace_id: &str) -> Result<(), String> {
        let url = format!("{}/chats/{}/settings", self.host, workspace_id);

        let mut request = self.http_client.delete(&url);

        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await
            .map_err(|e| format!("Failed to delete workspace: {}", e))?;

        if !response.status().is_success() && response.status().as_u16() != 404 {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Failed to delete workspace: {}", error));
        }

        Ok(())
    }

    /// Configure from LLM ProviderConfig (convenience method for default workspace)
    pub async fn configure_from_provider_config(
        &self,
        config: &ProviderConfig,
        proxy_url: &str,
        custom_system_prompt: Option<&str>,
    ) -> Result<(), String> {
        const DEFAULT_WORKSPACE: &str = "dm-assistant";

        let chat_config = ChatProviderConfig::try_from(config)
            .map_err(|e| e.to_string())?;

        let prompts = Some(ChatPrompts {
            system: Some(
                custom_system_prompt
                    .unwrap_or(DEFAULT_DM_SYSTEM_PROMPT)
                    .to_string()
            ),
            ..Default::default()
        });

        self.configure_workspace_with_provider(
            DEFAULT_WORKSPACE,
            &chat_config,
            proxy_url,
            prompts
        ).await
    }
}

// ============================================================================
// SSE Stream Processing
// ============================================================================

/// Process SSE (Server-Sent Events) stream from Meilisearch Chat API
///
/// This function handles:
/// - Parsing SSE events from the HTTP response stream
/// - Extracting content deltas from streaming chunks
/// - Processing tool calls (_meiliAppendConversationMessage, _meiliSearchProgress, _meiliSearchSources)
/// - Filtering out tool call JSON that models incorrectly output as text
/// - Detecting and suppressing LLM filter hallucination errors (Anti-Filter Hallucination)
async fn process_sse_stream(
    response: reqwest::Response,
    tx: mpsc::Sender<Result<String, String>>,
) {
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    log::info!("Starting SSE stream processing");

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(bytes) => {
                let chunk_str = String::from_utf8_lossy(&bytes);
                log::debug!("Received chunk: {}", chunk_str);
                buffer.push_str(&chunk_str);

                // Process complete SSE events
                while let Some(pos) = buffer.find("\n\n") {
                    let event = buffer[..pos].to_string();
                    buffer = buffer[pos + 2..].to_string();

                    log::debug!("Processing event: {}", event);

                    // Parse SSE event
                    for line in event.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                log::info!("Stream finished with [DONE]");
                                let _ = tx.send(Ok("[DONE]".to_string())).await;
                                return;
                            }

                            // Process the data line
                            if !process_sse_data(data, &tx).await {
                                return; // Fatal error, stop processing
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Stream error: {}", e);
                let _ = tx.send(Err(e.to_string())).await;
                return;
            }
        }
    }

    // Handle any remaining buffer content (e.g. if stream ended without double newline)
    if !buffer.is_empty() {
        log::debug!("Stream ended with data in buffer: {}", buffer);
        if buffer.trim() == "data: [DONE]" {
            let _ = tx.send(Ok("[DONE]".to_string())).await;
        }
    }
    log::info!("SSE stream ended");
}

/// Process a single SSE data line
///
/// Returns `true` to continue processing, `false` to stop (fatal error)
async fn process_sse_data(data: &str, tx: &mpsc::Sender<Result<String, String>>) -> bool {
    // Parse JSON chunk
    match serde_json::from_str::<StreamChunk>(data) {
        Ok(chunk) => {
            for choice in chunk.choices {
                if let Some(content) = choice.delta.content {
                    // Filter out tool call JSON that models output as text
                    // when they don't support structured tool calling.
                    // Use JSON parsing for accurate detection instead of brittle string matching.
                    let trimmed = content.trim();
                    let is_tool_call_json = if trimmed.starts_with('{') {
                        // Attempt to parse as JSON and check structure
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
                            if let Some(obj) = parsed.as_object() {
                                // Check for "name" field with tool-like naming pattern
                                if let Some(serde_json::Value::String(name)) = obj.get("name") {
                                    name.ends_with("_meili")
                                        || name.contains("_search")
                                        || name.starts_with("_meili")
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if is_tool_call_json {
                        log::debug!("Filtering tool call JSON from content: {}", content);
                    } else {
                        log::debug!("Emitting content: {}", content);
                        let _ = tx.send(Ok(content)).await;
                    }
                } else if let Some(tool_calls) = choice.delta.tool_calls {
                    process_tool_calls(&tool_calls, tx).await;
                }
            }
            true
        }
        Err(e) => {
            // Try parsing as error
            if let Ok(error_response) = serde_json::from_str::<MeilisearchErrorResponse>(data) {
                return handle_meilisearch_error(&error_response, tx).await;
            }

            log::warn!("Failed to parse chunk: {} Data: {}", e, data);
            true // Continue processing despite parse failure
        }
    }
}

/// Process tool calls from the streaming response
async fn process_tool_calls(
    tool_calls: &serde_json::Value,
    tx: &mpsc::Sender<Result<String, String>>,
) {
    // These include _meiliAppendConversationMessage for context management
    if let Some(tool_array) = tool_calls.as_array() {
        for tool_call in tool_array {
            if let Some(func) = tool_call.get("function") {
                let func_name = func.get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("");
                let func_args = func.get("arguments")
                    .and_then(|a| a.as_str())
                    .unwrap_or("{}");

                // Parse and handle the tool call
                match ParsedToolCall::parse(func_name, func_args) {
                    Some(ParsedToolCall::AppendMessage(args)) => {
                        // _meiliAppendConversationMessage: track conversation context
                        // Emit as structured JSON for frontend to maintain history
                        log::debug!("Conversation append: role={}, has_tool_calls={}, tool_call_id={:?}",
                            args.role,
                            args.tool_calls.is_some(),
                            args.tool_call_id);

                        // Emit as special event for frontend conversation management
                        if let Ok(json) = serde_json::to_string(&serde_json::json!({
                            "_type": "conversation_append",
                            "role": args.role,
                            "content": args.content,
                            "tool_calls": args.tool_calls,
                            "tool_call_id": args.tool_call_id
                        })) {
                            let _ = tx.send(Ok(format!("[MEILI_CONTEXT:{}]", json))).await;
                        }
                    }
                    Some(ParsedToolCall::SearchProgress(args)) => {
                        // _meiliSearchProgress: emit for UI progress display
                        log::debug!("Search progress: {} - {:?}", args.function_name, args.function_parameters);

                        // Parse function_parameters to extract search query
                        if let Ok(params) = serde_json::from_str::<serde_json::Value>(&args.function_parameters) {
                            if let Some(q) = params.get("q").and_then(|q| q.as_str()) {
                                let _ = tx.send(Ok(format!("[MEILI_SEARCH:{}]", q))).await;
                            }
                        }
                    }
                    Some(ParsedToolCall::SearchSources(args)) => {
                        // _meiliSearchSources: emit for citation display
                        log::debug!("Search sources: call_id={}", args.call_id);

                        if let Ok(json) = serde_json::to_string(&serde_json::json!({
                            "_type": "sources",
                            "call_id": args.call_id,
                            "documents": args.documents
                        })) {
                            let _ = tx.send(Ok(format!("[MEILI_SOURCES:{}]", json))).await;
                        }
                    }
                    Some(ParsedToolCall::Unknown { name, .. }) => {
                        log::debug!("Unknown tool call: {}", name);
                    }
                    None => {
                        log::warn!("Failed to parse tool call: {} - {}", func_name, func_args);
                    }
                }
            }
        }
    } else {
        log::debug!("Received non-array tool_calls: {:?}", tool_calls);
    }
}

/// Handle Meilisearch error responses
///
/// Returns `true` to continue processing, `false` to stop (fatal error)
///
/// # Anti-Filter Hallucination Mitigation
///
/// The LLM sometimes generates SQL-like syntax or invalid filter operators
/// that Meilisearch doesn't understand. Instead of terminating the stream,
/// we detect these hallucination errors and suppress them, allowing the LLM
/// to retry the search or respond without RAG context.
async fn handle_meilisearch_error(
    error_response: &MeilisearchErrorResponse,
    tx: &mpsc::Sender<Result<String, String>>,
) -> bool {
    let msg = &error_response.error.message;
    let error_code = error_response.error.code.as_deref().unwrap_or("");

    // MITIGATION: Detect and suppress LLM filter hallucination errors
    // The LLM sometimes generates SQL-like syntax or invalid operators
    // that Meilisearch doesn't understand. Instead of terminating,
    // we continue processing to let the LLM retry or respond without search.
    let is_filter_hallucination =
        // Error code check
        error_code == "invalid_search_filter" ||
        // Pattern: Generic filter parse errors
        msg.contains("Was expecting a value") ||
        msg.contains("Was expecting an operation") ||
        msg.contains("unexpected characters at the end of the filter") ||
        // Pattern: Regex-like operators (=~, !~)
        msg.contains("=~") ||
        msg.contains("!~") ||
        // Pattern: SQL-like operators the LLM hallucinates
        msg.contains("'NULL'") ||
        msg.contains("'IS NOT NULL'") ||
        msg.contains("'IS EMPTY'") ||
        msg.contains("'CONTAINS'") ||
        msg.contains("'NOT CONTAINS'") ||
        msg.contains("'STARTS WITH'") ||
        msg.contains("'NOT STARTS WITH'") ||
        msg.contains("'LIKE'") ||
        msg.contains("'ILIKE'") ||
        // Pattern: Geo filter errors
        msg.contains("'_geoRadius'") ||
        msg.contains("'_geoBoundingBox'") ||
        msg.contains("'_geoPolygon'") ||
        // Pattern: Attribute not filterable (when filterable_attributes disabled)
        msg.contains("is not filterable") ||
        // Pattern: Invalid filter syntax variations
        (msg.contains("filter") && msg.contains("invalid"));

    if is_filter_hallucination {
        log::warn!("Suppressed LLM filter hallucination error (continuing stream): {}", msg);
        // Continue processing - don't terminate the stream
        // The LLM will either retry the search or respond without RAG context
        return true;
    }

    log::error!("Meilisearch API error: {}", msg);
    let _ = tx.send(Err(msg.clone())).await;
    false // Stop processing
}
