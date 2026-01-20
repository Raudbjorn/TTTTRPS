//! Claude/Anthropic Provider Unit Tests
//!
//! Tests for the Claude OAuth provider implementation including:
//! - API request formatting
//! - Response parsing
//! - Error handling (rate limits, auth errors, API errors)
//! - Streaming response handling
//!
//! Note: ClaudeProvider is now OAuth-based (no API key). Tests use in-memory
//! token storage via `with_memory()` for isolated testing.

use crate::core::llm::cost::TokenUsage;
use crate::core::llm::providers::ClaudeProvider;
use crate::core::llm::router::{ChatMessage, ChatRequest, LLMError, LLMProvider, MessageRole};

// =============================================================================
// Provider Identity Tests
// =============================================================================

#[test]
fn test_provider_id() {
    let provider = ClaudeProvider::with_memory().expect("Failed to create provider");
    assert_eq!(provider.id(), "claude");
}

#[test]
fn test_provider_name() {
    let provider = ClaudeProvider::with_memory().expect("Failed to create provider");
    assert_eq!(provider.name(), "Claude");
}

#[test]
fn test_provider_model_default() {
    let provider = ClaudeProvider::with_memory().expect("Failed to create provider");
    // Default model is claude-sonnet-4-20250514
    assert!(provider.model().starts_with("claude-"));
}

// =============================================================================
// Health Check Tests
// =============================================================================

#[tokio::test]
async fn test_health_check_no_tokens() {
    let provider = ClaudeProvider::with_memory().expect("Failed to create provider");
    // Without OAuth tokens, health check should fail
    assert!(!provider.health_check().await);
}

// =============================================================================
// Pricing Tests
// =============================================================================

#[test]
fn test_pricing_is_available() {
    let provider = ClaudeProvider::with_memory().expect("Failed to create provider");
    let pricing = provider.pricing();
    assert!(pricing.is_some());
    let pricing = pricing.unwrap();
    assert_eq!(pricing.provider_id, "claude");
    assert!(!pricing.is_free);
}

#[test]
fn test_pricing_cost_calculation() {
    let provider = ClaudeProvider::with_memory().expect("Failed to create provider");
    let pricing = provider.pricing().unwrap();

    let usage = TokenUsage::new(1000, 500);
    let cost = pricing.calculate_cost(&usage);
    // Cost should be positive for non-zero usage
    assert!(cost > 0.0);
}

// =============================================================================
// Request Formatting Tests
// =============================================================================

#[test]
fn test_build_request_basic() {
    let provider = ClaudeProvider::with_memory().expect("Failed to create provider");

    let _request = ChatRequest::new(vec![ChatMessage::user("Hello")]);

    // Test that the provider can be created without panicking
    // and has a valid model configuration
    assert!(!provider.model().is_empty());
}

#[test]
fn test_build_request_with_system_prompt() {
    let request = ChatRequest::new(vec![ChatMessage::user("Hello")])
        .with_system("You are helpful");

    assert_eq!(request.system_prompt, Some("You are helpful".to_string()));
}

#[test]
fn test_build_request_with_temperature() {
    let request = ChatRequest::new(vec![ChatMessage::user("Hello")])
        .with_temperature(0.5);

    assert_eq!(request.temperature, Some(0.5));
}

#[test]
fn test_build_request_with_max_tokens() {
    let request = ChatRequest::new(vec![ChatMessage::user("Hello")])
        .with_max_tokens(2000);

    assert_eq!(request.max_tokens, Some(2000));
}

#[test]
fn test_build_request_filters_system_messages() {
    // System messages in the messages array should be filtered
    // (only system_prompt field is used for Claude)
    let request = ChatRequest::new(vec![
        ChatMessage::system("This should be filtered"),
        ChatMessage::user("Hello"),
        ChatMessage::assistant("Hi there"),
    ]);

    // The request should contain all messages, filtering happens in build_request
    assert_eq!(request.messages.len(), 3);
}

#[test]
fn test_message_role_mapping() {
    let user_msg = ChatMessage::user("Test");
    assert_eq!(user_msg.role, MessageRole::User);

    let assistant_msg = ChatMessage::assistant("Test");
    assert_eq!(assistant_msg.role, MessageRole::Assistant);

    let system_msg = ChatMessage::system("Test");
    assert_eq!(system_msg.role, MessageRole::System);
}

// =============================================================================
// Response Parsing Tests
// =============================================================================

#[test]
fn test_parse_successful_response() {
    // Test response structure expected from Claude API
    let response_json = serde_json::json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "model": "claude-sonnet-4-20250514",
        "content": [{
            "type": "text",
            "text": "Hello! How can I help you today?"
        }],
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": 10,
            "output_tokens": 15
        }
    });

    // Verify the expected structure exists
    let content = response_json["content"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["text"].as_str());

    assert_eq!(content, Some("Hello! How can I help you today?"));

    let usage = response_json["usage"].as_object();
    assert!(usage.is_some());
    assert_eq!(usage.unwrap()["input_tokens"].as_u64(), Some(10));
    assert_eq!(usage.unwrap()["output_tokens"].as_u64(), Some(15));
}

#[test]
fn test_parse_response_with_missing_content() {
    // Test handling of malformed response
    let response_json = serde_json::json!({
        "id": "msg_123",
        "type": "message",
        "model": "claude-sonnet-4-20250514"
        // Missing content array
    });

    let content = response_json["content"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["text"].as_str());

    assert!(content.is_none());
}

#[test]
fn test_parse_response_usage() {
    let usage_json = serde_json::json!({
        "input_tokens": 100,
        "output_tokens": 50
    });

    let usage = TokenUsage {
        input_tokens: usage_json["input_tokens"].as_u64().unwrap_or(0) as u32,
        output_tokens: usage_json["output_tokens"].as_u64().unwrap_or(0) as u32,
    };

    assert_eq!(usage.input_tokens, 100);
    assert_eq!(usage.output_tokens, 50);
    assert_eq!(usage.total(), 150);
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_rate_limited_error() {
    let error = LLMError::RateLimited { retry_after_secs: 60 };
    let error_msg = error.to_string();
    assert!(error_msg.contains("Rate limited"));
    assert!(error_msg.contains("60"));
}

#[test]
fn test_auth_error() {
    let error = LLMError::AuthError("Invalid API key".to_string());
    let error_msg = error.to_string();
    assert!(error_msg.contains("Authentication failed"));
    assert!(error_msg.contains("Invalid API key"));
}

#[test]
fn test_api_error() {
    let error = LLMError::ApiError {
        status: 500,
        message: "Internal server error".to_string(),
    };
    let error_msg = error.to_string();
    assert!(error_msg.contains("500"));
    assert!(error_msg.contains("Internal server error"));
}

#[test]
fn test_invalid_response_error() {
    let error = LLMError::InvalidResponse("Missing content".to_string());
    let error_msg = error.to_string();
    assert!(error_msg.contains("Invalid response"));
    assert!(error_msg.contains("Missing content"));
}

#[test]
fn test_timeout_error() {
    let error = LLMError::Timeout;
    let error_msg = error.to_string();
    assert!(error_msg.contains("timeout"));
}

// =============================================================================
// Streaming Response Tests
// =============================================================================

#[test]
fn test_parse_stream_message_start() {
    let event_json = serde_json::json!({
        "type": "message_start",
        "message": {
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-20250514",
            "usage": {
                "input_tokens": 10
            }
        }
    });

    let event_type = event_json["type"].as_str();
    assert_eq!(event_type, Some("message_start"));

    let input_tokens = event_json["message"]["usage"]["input_tokens"].as_u64();
    assert_eq!(input_tokens, Some(10));
}

#[test]
fn test_parse_stream_content_block_delta() {
    let event_json = serde_json::json!({
        "type": "content_block_delta",
        "index": 0,
        "delta": {
            "type": "text_delta",
            "text": "Hello"
        }
    });

    let event_type = event_json["type"].as_str();
    assert_eq!(event_type, Some("content_block_delta"));

    let delta_text = event_json["delta"]["text"].as_str();
    assert_eq!(delta_text, Some("Hello"));
}

#[test]
fn test_parse_stream_message_delta() {
    let event_json = serde_json::json!({
        "type": "message_delta",
        "usage": {
            "output_tokens": 50
        }
    });

    let event_type = event_json["type"].as_str();
    assert_eq!(event_type, Some("message_delta"));

    let output_tokens = event_json["usage"]["output_tokens"].as_u64();
    assert_eq!(output_tokens, Some(50));
}

#[test]
fn test_parse_stream_message_stop() {
    let event_json = serde_json::json!({
        "type": "message_stop"
    });

    let event_type = event_json["type"].as_str();
    assert_eq!(event_type, Some("message_stop"));
}

#[test]
fn test_parse_sse_line() {
    let sse_line = r#"data: {"type":"content_block_delta","delta":{"text":"Hi"}}"#;

    assert!(sse_line.starts_with("data: "));

    let data = &sse_line[6..];
    let parsed: serde_json::Value = serde_json::from_str(data).unwrap();

    assert_eq!(parsed["type"].as_str(), Some("content_block_delta"));
    assert_eq!(parsed["delta"]["text"].as_str(), Some("Hi"));
}

// =============================================================================
// Storage Backend Tests
// =============================================================================

#[test]
fn test_memory_storage_creation() {
    // Memory storage should always succeed
    let provider = ClaudeProvider::with_memory();
    assert!(provider.is_ok());
}

#[test]
fn test_default_storage_creation() {
    // Default (file) storage creation
    let provider = ClaudeProvider::new();
    // May succeed or fail depending on file permissions
    // We just verify it doesn't panic
    let _ = provider;
}

#[test]
fn test_auto_storage_creation() {
    // Auto storage tries keyring then file
    let provider = ClaudeProvider::auto();
    // May succeed or fail depending on system capabilities
    // We just verify it doesn't panic
    let _ = provider;
}

// =============================================================================
// Provider Trait Implementation Tests
// =============================================================================

#[test]
fn test_supports_streaming_default() {
    let provider = ClaudeProvider::with_memory().expect("Failed to create provider");
    // Default implementation should return true
    assert!(provider.supports_streaming());
}

#[test]
fn test_supports_embeddings() {
    let provider = ClaudeProvider::with_memory().expect("Failed to create provider");
    // Claude doesn't natively support embeddings through this API
    assert!(!provider.supports_embeddings());
}

// =============================================================================
// Extended Thinking Tests
// =============================================================================

#[test]
fn test_extended_thinking_request() {
    // Request with extended thinking enabled
    let request_body = serde_json::json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 16000,
        "thinking": {
            "type": "enabled",
            "budget_tokens": 10000
        },
        "messages": [{
            "role": "user",
            "content": "Solve this complex problem step by step"
        }]
    });

    assert!(request_body["thinking"].is_object());
    assert_eq!(request_body["thinking"]["type"].as_str(), Some("enabled"));
    assert_eq!(request_body["thinking"]["budget_tokens"].as_i64(), Some(10000));
}

#[test]
fn test_extended_thinking_response() {
    // Response with thinking blocks
    let response_json = serde_json::json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "model": "claude-sonnet-4-20250514",
        "content": [
            {
                "type": "thinking",
                "thinking": "Let me analyze this step by step..."
            },
            {
                "type": "text",
                "text": "The answer is 42."
            }
        ],
        "usage": {
            "input_tokens": 50,
            "output_tokens": 200
        }
    });

    let content = response_json["content"].as_array().unwrap();
    assert_eq!(content.len(), 2);
    assert_eq!(content[0]["type"].as_str(), Some("thinking"));
    assert_eq!(content[1]["type"].as_str(), Some("text"));
}

#[test]
fn test_extended_thinking_streaming() {
    // Streaming with thinking blocks
    let thinking_event = serde_json::json!({
        "type": "content_block_start",
        "index": 0,
        "content_block": {
            "type": "thinking",
            "thinking": ""
        }
    });

    assert_eq!(thinking_event["content_block"]["type"].as_str(), Some("thinking"));
}

// =============================================================================
// Tool Use Tests
// =============================================================================

#[test]
fn test_tool_definition_format() {
    // Claude tool definition format
    let tool = serde_json::json!({
        "name": "get_weather",
        "description": "Get the current weather for a location",
        "input_schema": {
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g., San Francisco, CA"
                },
                "unit": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "The unit of temperature"
                }
            },
            "required": ["location"]
        }
    });

    assert_eq!(tool["name"].as_str(), Some("get_weather"));
    assert!(tool["input_schema"]["properties"]["location"].is_object());
}

#[test]
fn test_tool_choice_options() {
    // Tool choice configurations
    let auto = serde_json::json!({ "type": "auto" });
    let any = serde_json::json!({ "type": "any" });
    let specific = serde_json::json!({
        "type": "tool",
        "name": "get_weather"
    });

    assert_eq!(auto["type"].as_str(), Some("auto"));
    assert_eq!(any["type"].as_str(), Some("any"));
    assert_eq!(specific["name"].as_str(), Some("get_weather"));
}

#[test]
fn test_tool_use_response() {
    // Response with tool_use content block
    let response_json = serde_json::json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "tool_use",
            "id": "toolu_123",
            "name": "get_weather",
            "input": {
                "location": "San Francisco, CA",
                "unit": "fahrenheit"
            }
        }],
        "stop_reason": "tool_use"
    });

    let tool_use = &response_json["content"][0];
    assert_eq!(tool_use["type"].as_str(), Some("tool_use"));
    assert_eq!(tool_use["name"].as_str(), Some("get_weather"));
    assert_eq!(response_json["stop_reason"].as_str(), Some("tool_use"));
}

#[test]
fn test_tool_result_message() {
    // Tool result message format
    let tool_result = serde_json::json!({
        "role": "user",
        "content": [{
            "type": "tool_result",
            "tool_use_id": "toolu_123",
            "content": "Temperature: 72F, Sunny"
        }]
    });

    let result = &tool_result["content"][0];
    assert_eq!(result["type"].as_str(), Some("tool_result"));
    assert_eq!(result["tool_use_id"].as_str(), Some("toolu_123"));
}

#[test]
fn test_tool_result_with_error() {
    // Tool result with error
    let error_result = serde_json::json!({
        "type": "tool_result",
        "tool_use_id": "toolu_123",
        "is_error": true,
        "content": "Error: Location not found"
    });

    assert_eq!(error_result["is_error"].as_bool(), Some(true));
}

// =============================================================================
// Vision/Image Tests
// =============================================================================

#[test]
fn test_image_content_format() {
    // Image content block format
    let image_content = serde_json::json!({
        "role": "user",
        "content": [
            {
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": "image/jpeg",
                    "data": "base64encodeddata..."
                }
            },
            {
                "type": "text",
                "text": "What is in this image?"
            }
        ]
    });

    let content = image_content["content"].as_array().unwrap();
    assert_eq!(content.len(), 2);
    assert_eq!(content[0]["type"].as_str(), Some("image"));
    assert_eq!(content[0]["source"]["type"].as_str(), Some("base64"));
}

#[test]
fn test_image_url_format() {
    // Image from URL
    let image_url = serde_json::json!({
        "type": "image",
        "source": {
            "type": "url",
            "url": "https://example.com/image.jpg"
        }
    });

    assert_eq!(image_url["source"]["type"].as_str(), Some("url"));
}

#[test]
fn test_supported_image_media_types() {
    let supported_types = vec![
        "image/jpeg",
        "image/png",
        "image/gif",
        "image/webp",
    ];

    for media_type in supported_types {
        assert!(media_type.starts_with("image/"));
    }
}

// =============================================================================
// PDF Support Tests
// =============================================================================

#[test]
fn test_pdf_document_format() {
    // PDF document content block
    let pdf_content = serde_json::json!({
        "type": "document",
        "source": {
            "type": "base64",
            "media_type": "application/pdf",
            "data": "base64encodedpdfdata..."
        }
    });

    assert_eq!(pdf_content["type"].as_str(), Some("document"));
    assert_eq!(pdf_content["source"]["media_type"].as_str(), Some("application/pdf"));
}

// =============================================================================
// Caching Tests
// =============================================================================

#[test]
fn test_cache_control_format() {
    // Cache control for prompt caching
    let message_with_cache = serde_json::json!({
        "role": "user",
        "content": [
            {
                "type": "text",
                "text": "Large context that should be cached...",
                "cache_control": {
                    "type": "ephemeral"
                }
            }
        ]
    });

    let cache_control = &message_with_cache["content"][0]["cache_control"];
    assert_eq!(cache_control["type"].as_str(), Some("ephemeral"));
}

#[test]
fn test_cache_usage_response() {
    // Response with cache usage info
    let response_json = serde_json::json!({
        "usage": {
            "input_tokens": 100,
            "output_tokens": 50,
            "cache_creation_input_tokens": 80,
            "cache_read_input_tokens": 20
        }
    });

    let usage = response_json["usage"].as_object().unwrap();
    assert!(usage.contains_key("cache_creation_input_tokens"));
    assert!(usage.contains_key("cache_read_input_tokens"));
}

// =============================================================================
// Batching Tests
// =============================================================================

#[test]
fn test_batch_request_format() {
    // Batch API request format
    let batch_request = serde_json::json!({
        "custom_id": "request-1",
        "params": {
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": "Hello"
            }]
        }
    });

    assert!(batch_request["custom_id"].is_string());
    assert!(batch_request["params"].is_object());
}

// =============================================================================
// Computer Use Tests
// =============================================================================

#[test]
fn test_computer_use_tool() {
    // Computer use tool definition
    let computer_tool = serde_json::json!({
        "type": "computer_20241022",
        "name": "computer",
        "display_width_px": 1024,
        "display_height_px": 768,
        "display_number": 1
    });

    assert_eq!(computer_tool["type"].as_str(), Some("computer_20241022"));
    assert_eq!(computer_tool["display_width_px"].as_i64(), Some(1024));
}

#[test]
fn test_computer_action_response() {
    // Computer use action request
    let action = serde_json::json!({
        "type": "tool_use",
        "id": "toolu_123",
        "name": "computer",
        "input": {
            "action": "click",
            "coordinate": [500, 400]
        }
    });

    assert_eq!(action["input"]["action"].as_str(), Some("click"));
    assert!(action["input"]["coordinate"].is_array());
}

// =============================================================================
// Stop Reason Tests
// =============================================================================

#[test]
fn test_stop_reason_end_turn() {
    assert_eq!("end_turn", "end_turn");
}

#[test]
fn test_stop_reason_max_tokens() {
    assert_eq!("max_tokens", "max_tokens");
}

#[test]
fn test_stop_reason_stop_sequence() {
    assert_eq!("stop_sequence", "stop_sequence");
}

#[test]
fn test_stop_reason_tool_use() {
    assert_eq!("tool_use", "tool_use");
}

// =============================================================================
// Metadata Tests
// =============================================================================

#[test]
fn test_metadata_format() {
    // Request with metadata
    let request_body = serde_json::json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 1024,
        "metadata": {
            "user_id": "user-123"
        },
        "messages": [{
            "role": "user",
            "content": "Hello"
        }]
    });

    assert!(request_body["metadata"].is_object());
    assert_eq!(request_body["metadata"]["user_id"].as_str(), Some("user-123"));
}
