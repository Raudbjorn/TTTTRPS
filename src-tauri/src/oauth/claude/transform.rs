//! Request and response transformation utilities.
//!
//! This module handles:
//! - Header injection for OAuth authentication
//! - System prompt injection (identifies as Claude Code)
//! - Model alias resolution
//! - Request/response format conversion

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use serde_json::Value;
use tracing::debug;

use super::models::model_aliases;

/// Anthropic API version header value.
pub const ANTHROPIC_VERSION: &str = "2023-06-01";

/// OAuth beta header value.
pub const ANTHROPIC_BETA: &str = "oauth-2025-04-20";

/// User agent to identify as Claude Code CLI.
pub const CLAUDE_CODE_USER_AGENT: &str = "claude-code/1.0.0";

/// System prompt prefix injected to identify as Claude Code.
pub const CLAUDE_CODE_SYSTEM_PREFIX: &str =
    "You are Claude Code, Anthropic's official CLI for Claude.";

/// Create headers for Anthropic API requests.
///
/// Injects headers to identify as Claude Code CLI:
/// - Authorization: Bearer {token}
/// - anthropic-version: 2023-06-01
/// - anthropic-beta: oauth-2025-04-20
/// - Content-Type: application/json
/// - User-Agent: claude-code/1.0.0
/// - Accept: */*
#[must_use]
pub fn create_headers(access_token: &str) -> HeaderMap {
    create_headers_with_options(access_token, false)
}

/// Create headers for streaming Anthropic API requests.
///
/// Adds streaming-specific headers:
/// - Connection: close
/// - Cache-Control: no-cache
#[must_use]
pub fn create_streaming_headers(access_token: &str) -> HeaderMap {
    create_headers_with_options(access_token, true)
}

/// Create headers with optional streaming settings.
fn create_headers_with_options(access_token: &str, streaming: bool) -> HeaderMap {
    let mut headers = HeaderMap::new();

    // Authorization - Bearer token format for OAuth
    let auth_value = format!("Bearer {}", access_token);
    if let Ok(value) = HeaderValue::from_str(&auth_value) {
        headers.insert(AUTHORIZATION, value);
    }

    // Anthropic headers - required for OAuth authentication
    if let Ok(value) = HeaderValue::from_str(ANTHROPIC_VERSION) {
        headers.insert("anthropic-version", value);
    }
    if let Ok(value) = HeaderValue::from_str(ANTHROPIC_BETA) {
        headers.insert("anthropic-beta", value);
    }

    // User-Agent to identify as Claude Code CLI
    headers.insert(USER_AGENT, HeaderValue::from_static(CLAUDE_CODE_USER_AGENT));

    // Content type
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(ACCEPT, HeaderValue::from_static("*/*"));

    // Streaming-specific headers
    if streaming {
        headers.insert("Connection", HeaderValue::from_static("close"));
        headers.insert("Cache-Control", HeaderValue::from_static("no-cache"));
    }

    headers
}

/// Transform a request body before sending to the API.
///
/// This applies:
/// - Model alias resolution
/// - System prompt injection
#[must_use]
pub fn transform_request(mut body: Value) -> Value {
    // Resolve model alias
    if let Some(model) = body.get("model").and_then(Value::as_str) {
        let resolved = model_aliases::resolve(model);
        if resolved != model {
            debug!(from = model, to = resolved, "Resolved model alias");
            body["model"] = Value::String(resolved.to_string());
        }
    }

    // Inject system prompt
    body = inject_system_prompt(body);

    body
}

/// Check if the Claude Code system prompt is already present.
fn has_claude_code_prompt(system: &Value) -> bool {
    match system {
        Value::String(s) => s.contains("Claude Code"),
        Value::Array(arr) => arr.iter().any(|item| {
            item.get("text")
                .and_then(Value::as_str)
                .map(|t| t.contains("Claude Code"))
                .unwrap_or(false)
        }),
        _ => false,
    }
}

/// Inject the Claude Code system prompt prefix.
///
/// Matches Go behavior:
/// - String system prompts are converted to array format
/// - Array system prompts get Claude Code prepended as first element
/// - Checks for existing Claude Code prompt to avoid duplication
fn inject_system_prompt(mut body: Value) -> Value {
    // Check if already has Claude Code prompt to avoid duplication
    if let Some(system) = body.get("system") {
        if has_claude_code_prompt(system) {
            debug!("Claude Code prompt already present, skipping injection");
            return body;
        }
    }

    let claude_code_block = serde_json::json!({
        "type": "text",
        "text": CLAUDE_CODE_SYSTEM_PREFIX
    });

    match body.get("system") {
        // String system prompt - convert to array with Claude Code first (matches Go)
        Some(Value::String(existing)) => {
            let existing_block = serde_json::json!({
                "type": "text",
                "text": existing
            });
            body["system"] = Value::Array(vec![claude_code_block, existing_block]);
        }
        // Array system prompt - prepend Claude Code as first element
        Some(Value::Array(existing)) => {
            let mut new_array = vec![claude_code_block];
            new_array.extend(existing.clone());
            body["system"] = Value::Array(new_array);
        }
        // No system prompt - add Claude Code as array (for consistency)
        None => {
            body["system"] = Value::Array(vec![claude_code_block]);
        }
        // Other types - leave as is
        _ => {}
    }

    body
}

/// Check if a request is for streaming.
#[must_use]
pub fn is_streaming_request(body: &Value) -> bool {
    body.get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// OpenAI-compatible format conversion utilities.
pub mod openai {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    use crate::oauth::claude::models::{Message, MessagesResponse, StopReason};

    /// Get model-aware max_tokens default.
    ///
    /// Returns appropriate default based on model family:
    /// - Claude 4.5 (opus/sonnet/haiku): 32768
    /// - Claude 4 (opus/sonnet): 16384
    /// - Claude 3.7/3.5: 8192
    /// - Claude 3: 4096
    /// - Unknown: 4096
    #[must_use]
    pub fn default_max_tokens(model: &str) -> u32 {
        let model_lower = model.to_lowercase();

        if model_lower.contains("4-5") || model_lower.contains("4.5") {
            32768
        } else if model_lower.contains("opus-4") || model_lower.contains("sonnet-4") {
            16384
        } else if model_lower.contains("3-7") || model_lower.contains("3.7")
            || model_lower.contains("3-5") || model_lower.contains("3.5")
        {
            8192
        } else {
            4096
        }
    }

    /// OpenAI chat completion request format.
    #[derive(Debug, Deserialize)]
    pub struct ChatCompletionRequest {
        /// Model to use
        pub model: String,
        /// Messages in the conversation
        pub messages: Vec<OpenAIMessage>,
        /// Maximum tokens to generate
        pub max_tokens: Option<u32>,
        /// Temperature for sampling
        pub temperature: Option<f32>,
        /// Whether to stream
        pub stream: Option<bool>,
        /// Stop sequences
        pub stop: Option<Vec<String>>,
    }

    /// OpenAI message format.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct OpenAIMessage {
        /// Role (system, user, assistant)
        pub role: String,
        /// Content
        pub content: String,
    }

    /// OpenAI chat completion response format.
    #[derive(Debug, Serialize)]
    pub struct ChatCompletionResponse {
        /// Unique ID
        pub id: String,
        /// Object type (always "chat.completion")
        pub object: String,
        /// Creation timestamp
        pub created: i64,
        /// Model used
        pub model: String,
        /// Choices array
        pub choices: Vec<ChatCompletionChoice>,
        /// Token usage
        pub usage: ChatCompletionUsage,
    }

    /// OpenAI choice format.
    #[derive(Debug, Serialize)]
    pub struct ChatCompletionChoice {
        /// Choice index
        pub index: usize,
        /// Message
        pub message: OpenAIMessage,
        /// Finish reason
        pub finish_reason: Option<String>,
    }

    /// OpenAI usage format.
    #[derive(Debug, Serialize)]
    pub struct ChatCompletionUsage {
        /// Input tokens
        pub prompt_tokens: u32,
        /// Output tokens
        pub completion_tokens: u32,
        /// Total tokens
        pub total_tokens: u32,
    }

    /// Convert OpenAI chat completion request to Anthropic format.
    #[must_use]
    pub fn to_anthropic_request(request: ChatCompletionRequest) -> Value {
        let mut messages: Vec<Message> = Vec::new();
        let mut system_prompt: Option<String> = None;

        for msg in request.messages {
            match msg.role.as_str() {
                "system" => {
                    system_prompt = Some(msg.content);
                }
                "user" => {
                    messages.push(Message::user(msg.content));
                }
                "assistant" => {
                    messages.push(Message::assistant(msg.content));
                }
                _ => {
                    // Treat unknown roles as user
                    messages.push(Message::user(msg.content));
                }
            }
        }

        // Use model-aware max_tokens default if not specified
        let max_tokens = request.max_tokens.unwrap_or_else(|| default_max_tokens(&request.model));

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": max_tokens,
        });

        if let Some(system) = system_prompt {
            body["system"] = Value::String(system);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = Value::from(temp);
        }

        if let Some(stream) = request.stream {
            body["stream"] = Value::Bool(stream);
        }

        if let Some(stop) = request.stop {
            body["stop_sequences"] = Value::Array(stop.into_iter().map(Value::String).collect());
        }

        body
    }

    /// Convert Anthropic response to OpenAI format.
    #[must_use]
    pub fn from_anthropic_response(response: &MessagesResponse) -> ChatCompletionResponse {
        let content = response.text();

        let finish_reason = response.stop_reason.map(|r| {
            match r {
                StopReason::EndTurn => "stop",
                StopReason::MaxTokens => "length",
                StopReason::StopSequence => "stop",
                StopReason::ToolUse => "tool_calls",
            }
            .to_string()
        });

        ChatCompletionResponse {
            id: format!("chatcmpl-{}", response.id),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: response.model.clone(),
            choices: vec![ChatCompletionChoice {
                index: 0,
                message: OpenAIMessage {
                    role: "assistant".to_string(),
                    content,
                },
                finish_reason,
            }],
            usage: ChatCompletionUsage {
                prompt_tokens: response.usage.input_tokens,
                completion_tokens: response.usage.output_tokens,
                total_tokens: response.usage.input_tokens + response.usage.output_tokens,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_headers() {
        let headers = create_headers("test_token");

        assert!(headers.contains_key(AUTHORIZATION));
        assert!(headers.contains_key("anthropic-version"));
        assert!(headers.contains_key("anthropic-beta"));
    }

    #[test]
    fn test_transform_request_model_alias() {
        let body = json!({
            "model": "claude-opus-4-5",
            "messages": []
        });

        let transformed = transform_request(body);
        assert_eq!(
            transformed["model"].as_str().unwrap(),
            "claude-opus-4-5-20251101"
        );
    }

    #[test]
    fn test_inject_system_prompt_none() {
        let body = json!({
            "messages": []
        });

        let transformed = inject_system_prompt(body);
        // Should be array format with Claude Code prompt
        let system = transformed["system"].as_array().unwrap();
        assert_eq!(system.len(), 1);
        assert_eq!(system[0]["type"], "text");
        assert_eq!(system[0]["text"], CLAUDE_CODE_SYSTEM_PREFIX);
    }

    #[test]
    fn test_inject_system_prompt_existing_string() {
        let body = json!({
            "system": "Be helpful.",
            "messages": []
        });

        let transformed = inject_system_prompt(body);
        // String should be converted to array with Claude Code first
        let system = transformed["system"].as_array().unwrap();
        assert_eq!(system.len(), 2);
        assert_eq!(system[0]["text"], CLAUDE_CODE_SYSTEM_PREFIX);
        assert_eq!(system[1]["text"], "Be helpful.");
    }

    #[test]
    fn test_inject_system_prompt_existing_array() {
        let body = json!({
            "system": [{"type": "text", "text": "Be helpful."}],
            "messages": []
        });

        let transformed = inject_system_prompt(body);
        let system = transformed["system"].as_array().unwrap();
        assert_eq!(system.len(), 2);
        assert_eq!(system[0]["text"], CLAUDE_CODE_SYSTEM_PREFIX);
        assert_eq!(system[1]["text"], "Be helpful.");
    }

    #[test]
    fn test_inject_system_prompt_already_has_claude_code() {
        let body = json!({
            "system": "You are Claude Code, Anthropic's CLI.",
            "messages": []
        });

        let transformed = inject_system_prompt(body);
        // Should not duplicate - keeps original string
        assert!(transformed["system"].is_string());
        assert_eq!(
            transformed["system"].as_str().unwrap(),
            "You are Claude Code, Anthropic's CLI."
        );
    }

    #[test]
    fn test_is_streaming_request() {
        assert!(is_streaming_request(&json!({"stream": true})));
        assert!(!is_streaming_request(&json!({"stream": false})));
        assert!(!is_streaming_request(&json!({})));
    }
}
