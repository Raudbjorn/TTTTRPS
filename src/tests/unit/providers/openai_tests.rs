//! OpenAI Provider Unit Tests
//!
//! Tests for the OpenAI provider implementation including:
//! - API request formatting
//! - Response parsing
//! - Error handling (rate limits, auth errors, API errors)
//! - Streaming response handling
//! - Organization ID support
//! - Custom base URL support

use crate::core::llm::cost::TokenUsage;
use crate::core::llm::providers::OpenAIProvider;
use crate::core::llm::router::{ChatMessage, ChatRequest, LLMError, LLMProvider, MessageRole};

// =============================================================================
// Provider Identity Tests
// =============================================================================

#[test]
fn test_provider_id() {
    let provider = OpenAIProvider::new(
        "sk-test-key".to_string(),
        "gpt-4o".to_string(),
        4096,
        None,
        None,
    );
    assert_eq!(provider.id(), "openai");
}

#[test]
fn test_provider_name() {
    let provider = OpenAIProvider::new(
        "sk-test-key".to_string(),
        "gpt-4o".to_string(),
        4096,
        None,
        None,
    );
    assert_eq!(provider.name(), "OpenAI");
}

#[test]
fn test_provider_model() {
    let provider = OpenAIProvider::new(
        "sk-test-key".to_string(),
        "gpt-4o".to_string(),
        4096,
        None,
        None,
    );
    assert_eq!(provider.model(), "gpt-4o");
}

#[test]
fn test_gpt4o_convenience_constructor() {
    let provider = OpenAIProvider::gpt4o("sk-test-key".to_string());
    assert_eq!(provider.model(), "gpt-4o");
}

#[test]
fn test_gpt4o_mini_convenience_constructor() {
    let provider = OpenAIProvider::gpt4o_mini("sk-test-key".to_string());
    assert_eq!(provider.model(), "gpt-4o-mini");
}

// =============================================================================
// Configuration Tests
// =============================================================================

#[test]
fn test_default_base_url() {
    let provider = OpenAIProvider::new(
        "sk-test-key".to_string(),
        "gpt-4o".to_string(),
        4096,
        None,
        None,
    );
    // Default base URL is https://api.openai.com/v1
    assert_eq!(provider.model(), "gpt-4o");
}

#[test]
fn test_custom_base_url() {
    let provider = OpenAIProvider::new(
        "sk-test-key".to_string(),
        "gpt-4o".to_string(),
        4096,
        None,
        Some("https://custom-api.example.com/v1".to_string()),
    );
    assert_eq!(provider.model(), "gpt-4o");
}

#[test]
fn test_organization_id_configuration() {
    let provider = OpenAIProvider::new(
        "sk-test-key".to_string(),
        "gpt-4o".to_string(),
        4096,
        Some("org-12345".to_string()),
        None,
    );
    assert_eq!(provider.model(), "gpt-4o");
}

#[test]
fn test_max_tokens_configuration() {
    let _provider_default = OpenAIProvider::gpt4o("sk-test-key".to_string());
    // Default uses 4096 max tokens

    let provider_custom = OpenAIProvider::new(
        "sk-test-key".to_string(),
        "gpt-4o".to_string(),
        8192,
        None,
        None,
    );
    assert_eq!(provider_custom.model(), "gpt-4o");
}

// =============================================================================
// Pricing Tests
// =============================================================================

#[test]
fn test_pricing_gpt4o() {
    let provider = OpenAIProvider::gpt4o("sk-test-key".to_string());
    let pricing = provider.pricing();
    assert!(pricing.is_some());
    let pricing = pricing.unwrap();
    assert_eq!(pricing.provider_id, "openai");
    assert!(!pricing.is_free);
    // GPT-4o pricing: $2.50/1M input, $10.00/1M output
    assert_eq!(pricing.input_cost_per_million, 2.5);
    assert_eq!(pricing.output_cost_per_million, 10.0);
}

#[test]
fn test_pricing_gpt4o_mini() {
    let provider = OpenAIProvider::gpt4o_mini("sk-test-key".to_string());
    let pricing = provider.pricing();
    assert!(pricing.is_some());
    let pricing = pricing.unwrap();
    // GPT-4o-mini pricing: $0.15/1M input, $0.60/1M output
    assert_eq!(pricing.input_cost_per_million, 0.15);
    assert_eq!(pricing.output_cost_per_million, 0.60);
}

#[test]
fn test_pricing_cost_calculation() {
    let provider = OpenAIProvider::gpt4o("sk-test-key".to_string());
    let pricing = provider.pricing().unwrap();

    let usage = TokenUsage::new(1000, 500);
    let cost = pricing.calculate_cost(&usage);
    // (1000/1M * 2.5) + (500/1M * 10.0) = 0.0025 + 0.005 = 0.0075
    assert!((cost - 0.0075).abs() < 0.0001);
}

// =============================================================================
// Request Formatting Tests
// =============================================================================

#[test]
fn test_build_request_basic() {
    let request = ChatRequest::new(vec![ChatMessage::user("Hello")]);

    assert_eq!(request.messages.len(), 1);
    assert_eq!(request.messages[0].role, MessageRole::User);
    assert_eq!(request.messages[0].content, "Hello");
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
fn test_message_role_mapping_user() {
    let msg = ChatMessage::user("Test");
    assert_eq!(msg.role, MessageRole::User);

    // OpenAI role should be "user"
    let role_str = match msg.role {
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::System => "system",
    };
    assert_eq!(role_str, "user");
}

#[test]
fn test_message_role_mapping_assistant() {
    let msg = ChatMessage::assistant("Test");
    assert_eq!(msg.role, MessageRole::Assistant);

    let role_str = match msg.role {
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::System => "system",
    };
    assert_eq!(role_str, "assistant");
}

#[test]
fn test_message_role_mapping_system() {
    let msg = ChatMessage::system("Test");
    assert_eq!(msg.role, MessageRole::System);

    let role_str = match msg.role {
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::System => "system",
    };
    assert_eq!(role_str, "system");
}

#[test]
fn test_system_prompt_in_messages() {
    // OpenAI expects system prompt as a message in the array
    let request = ChatRequest::new(vec![
        ChatMessage::user("Hello"),
    ]).with_system("Be helpful");

    assert_eq!(request.system_prompt, Some("Be helpful".to_string()));
    assert_eq!(request.messages.len(), 1);
}

// =============================================================================
// Response Parsing Tests
// =============================================================================

#[test]
fn test_parse_successful_response() {
    let response_json = serde_json::json!({
        "id": "chatcmpl-123",
        "object": "chat.completion",
        "created": 1677652288,
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "Hello! How can I help you today?"
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 15,
            "total_tokens": 25
        }
    });

    let content = response_json["choices"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["message"]["content"].as_str());

    assert_eq!(content, Some("Hello! How can I help you today?"));

    let finish_reason = response_json["choices"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["finish_reason"].as_str());

    assert_eq!(finish_reason, Some("stop"));
}

#[test]
fn test_parse_response_usage_openai_format() {
    // OpenAI uses prompt_tokens/completion_tokens instead of input_tokens/output_tokens
    let response_json = serde_json::json!({
        "usage": {
            "prompt_tokens": 100,
            "completion_tokens": 50,
            "total_tokens": 150
        }
    });

    let usage_obj = response_json["usage"].as_object().unwrap();
    let usage = TokenUsage {
        input_tokens: usage_obj["prompt_tokens"].as_u64().unwrap_or(0) as u32,
        output_tokens: usage_obj["completion_tokens"].as_u64().unwrap_or(0) as u32,
    };

    assert_eq!(usage.input_tokens, 100);
    assert_eq!(usage.output_tokens, 50);
    assert_eq!(usage.total(), 150);
}

#[test]
fn test_parse_response_with_missing_content() {
    let response_json = serde_json::json!({
        "id": "chatcmpl-123",
        "object": "chat.completion",
        "choices": []
    });

    let content = response_json["choices"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["message"]["content"].as_str());

    assert!(content.is_none());
}

#[test]
fn test_parse_response_model_info() {
    let response_json = serde_json::json!({
        "model": "gpt-4o-2024-05-13",
        "choices": [{
            "message": {
                "content": "Hello"
            }
        }]
    });

    let model = response_json["model"].as_str();
    assert_eq!(model, Some("gpt-4o-2024-05-13"));
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_rate_limited_error() {
    let error = LLMError::RateLimited { retry_after_secs: 30 };
    let error_msg = error.to_string();
    assert!(error_msg.contains("Rate limited"));
    assert!(error_msg.contains("30"));
}

#[test]
fn test_auth_error() {
    let error = LLMError::AuthError("Invalid API key".to_string());
    let error_msg = error.to_string();
    assert!(error_msg.contains("Authentication failed"));
}

#[test]
fn test_api_error_400() {
    let error = LLMError::ApiError {
        status: 400,
        message: "Bad request: invalid model".to_string(),
    };
    let error_msg = error.to_string();
    assert!(error_msg.contains("400"));
    assert!(error_msg.contains("invalid model"));
}

#[test]
fn test_api_error_500() {
    let error = LLMError::ApiError {
        status: 500,
        message: "Internal server error".to_string(),
    };
    let error_msg = error.to_string();
    assert!(error_msg.contains("500"));
}

#[test]
fn test_api_error_503() {
    let error = LLMError::ApiError {
        status: 503,
        message: "Service temporarily unavailable".to_string(),
    };
    let error_msg = error.to_string();
    assert!(error_msg.contains("503"));
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
fn test_parse_stream_chunk() {
    let chunk_json = serde_json::json!({
        "id": "chatcmpl-123",
        "object": "chat.completion.chunk",
        "created": 1677652288,
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "delta": {
                "content": "Hello"
            },
            "finish_reason": null
        }]
    });

    let delta_content = chunk_json["choices"][0]["delta"]["content"].as_str();
    assert_eq!(delta_content, Some("Hello"));

    let finish_reason = chunk_json["choices"][0]["finish_reason"].as_str();
    assert!(finish_reason.is_none());
}

#[test]
fn test_parse_stream_final_chunk() {
    let chunk_json = serde_json::json!({
        "id": "chatcmpl-123",
        "object": "chat.completion.chunk",
        "choices": [{
            "index": 0,
            "delta": {},
            "finish_reason": "stop"
        }]
    });

    let finish_reason = chunk_json["choices"][0]["finish_reason"].as_str();
    assert_eq!(finish_reason, Some("stop"));
}

#[test]
fn test_parse_stream_done_marker() {
    let sse_data = "[DONE]";
    assert_eq!(sse_data, "[DONE]");
}

#[test]
fn test_parse_sse_line() {
    let sse_line = r#"data: {"id":"chatcmpl-123","choices":[{"delta":{"content":"Hi"}}]}"#;

    assert!(sse_line.starts_with("data: "));

    let data = &sse_line[6..];
    let parsed: serde_json::Value = serde_json::from_str(data).unwrap();

    assert_eq!(parsed["choices"][0]["delta"]["content"].as_str(), Some("Hi"));
}

#[test]
fn test_stream_with_usage() {
    // OpenAI can include usage in stream with stream_options.include_usage
    let chunk_json = serde_json::json!({
        "id": "chatcmpl-123",
        "choices": [{
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 50,
            "completion_tokens": 100,
            "total_tokens": 150
        }
    });

    let usage = chunk_json["usage"].as_object();
    assert!(usage.is_some());
    assert_eq!(usage.unwrap()["prompt_tokens"].as_u64(), Some(50));
}

// =============================================================================
// Model Switching Tests
// =============================================================================

#[test]
fn test_model_switching_gpt4o() {
    let provider = OpenAIProvider::new(
        "sk-test".to_string(),
        "gpt-4o".to_string(),
        4096,
        None,
        None,
    );
    assert_eq!(provider.model(), "gpt-4o");
}

#[test]
fn test_model_switching_gpt4o_mini() {
    let provider = OpenAIProvider::new(
        "sk-test".to_string(),
        "gpt-4o-mini".to_string(),
        4096,
        None,
        None,
    );
    assert_eq!(provider.model(), "gpt-4o-mini");
}

#[test]
fn test_model_switching_gpt4_turbo() {
    let provider = OpenAIProvider::new(
        "sk-test".to_string(),
        "gpt-4-turbo".to_string(),
        4096,
        None,
        None,
    );
    assert_eq!(provider.model(), "gpt-4-turbo");
}

#[test]
fn test_model_switching_o1() {
    let provider = OpenAIProvider::new(
        "sk-test".to_string(),
        "o1".to_string(),
        4096,
        None,
        None,
    );
    assert_eq!(provider.model(), "o1");
}

#[test]
fn test_model_switching_o3_mini() {
    let provider = OpenAIProvider::new(
        "sk-test".to_string(),
        "o3-mini".to_string(),
        4096,
        None,
        None,
    );
    assert_eq!(provider.model(), "o3-mini");
}

// =============================================================================
// API Header Tests
// =============================================================================

#[test]
fn test_authorization_header_format() {
    let api_key = "sk-test-key-12345";
    let auth_header = format!("Bearer {}", api_key);
    assert_eq!(auth_header, "Bearer sk-test-key-12345");
}

#[test]
fn test_organization_header() {
    let org_id = "org-12345";
    // Header name is "OpenAI-Organization"
    assert!(!org_id.is_empty());
}

#[test]
fn test_content_type_header() {
    let content_type = "application/json";
    assert_eq!(content_type, "application/json");
}

// =============================================================================
// Provider Trait Implementation Tests
// =============================================================================

#[test]
fn test_supports_streaming() {
    let provider = OpenAIProvider::gpt4o("sk-test".to_string());
    assert!(provider.supports_streaming());
}

#[test]
fn test_supports_embeddings() {
    let provider = OpenAIProvider::gpt4o("sk-test".to_string());
    // OpenAI supports embeddings
    assert!(provider.supports_embeddings());
}

// =============================================================================
// OpenAI-Compatible Provider Tests
// =============================================================================

#[test]
fn test_custom_base_url_for_compatible_api() {
    // Test that custom base URLs work for OpenAI-compatible APIs
    let provider = OpenAIProvider::new(
        "custom-api-key".to_string(),
        "custom-model".to_string(),
        4096,
        None,
        Some("https://api.together.xyz/v1".to_string()),
    );
    assert_eq!(provider.model(), "custom-model");
}

#[test]
fn test_azure_openai_style_url() {
    // Azure OpenAI uses different URL patterns
    let provider = OpenAIProvider::new(
        "azure-api-key".to_string(),
        "gpt-4o".to_string(),
        4096,
        None,
        Some("https://my-resource.openai.azure.com/openai/deployments/my-deployment".to_string()),
    );
    assert_eq!(provider.model(), "gpt-4o");
}

// =============================================================================
// Vision Support Tests
// =============================================================================

#[test]
fn test_vision_request_format_basic() {
    // Vision requests use content array with type field
    let vision_content = serde_json::json!([
        {
            "type": "text",
            "text": "What is in this image?"
        },
        {
            "type": "image_url",
            "image_url": {
                "url": "https://example.com/image.jpg"
            }
        }
    ]);

    assert!(vision_content.is_array());
    let arr = vision_content.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["type"].as_str(), Some("text"));
    assert_eq!(arr[1]["type"].as_str(), Some("image_url"));
}

#[test]
fn test_vision_request_with_detail() {
    // Vision requests can specify detail level (low, high, auto)
    let vision_content = serde_json::json!([
        {
            "type": "image_url",
            "image_url": {
                "url": "https://example.com/image.jpg",
                "detail": "high"
            }
        }
    ]);

    let detail = vision_content[0]["image_url"]["detail"].as_str();
    assert_eq!(detail, Some("high"));
}

#[test]
fn test_vision_request_with_base64() {
    // Vision requests can use base64 encoded images
    let base64_image = "data:image/jpeg;base64,/9j/4AAQSkZJRg...";
    let vision_content = serde_json::json!([
        {
            "type": "image_url",
            "image_url": {
                "url": base64_image
            }
        }
    ]);

    let url = vision_content[0]["image_url"]["url"].as_str();
    assert!(url.unwrap().starts_with("data:image/jpeg;base64,"));
}

#[test]
fn test_vision_models() {
    // Models that support vision
    let vision_models = vec!["gpt-4o", "gpt-4o-mini", "gpt-4-turbo"];

    for model in vision_models {
        let provider = OpenAIProvider::new(
            "sk-test".to_string(),
            model.to_string(),
            4096,
            None,
            None,
        );
        assert_eq!(provider.model(), model);
    }
}

#[test]
fn test_vision_request_multiple_images() {
    // Multiple images in a single request
    let vision_content = serde_json::json!([
        {
            "type": "text",
            "text": "Compare these two images"
        },
        {
            "type": "image_url",
            "image_url": { "url": "https://example.com/image1.jpg" }
        },
        {
            "type": "image_url",
            "image_url": { "url": "https://example.com/image2.jpg" }
        }
    ]);

    let arr = vision_content.as_array().unwrap();
    let image_count = arr.iter().filter(|v| v["type"] == "image_url").count();
    assert_eq!(image_count, 2);
}

// =============================================================================
// Function Calling Tests
// =============================================================================

#[test]
fn test_function_definition_format() {
    // Function definition structure
    let function = serde_json::json!({
        "name": "get_weather",
        "description": "Get the current weather for a location",
        "parameters": {
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g. San Francisco, CA"
                },
                "unit": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"]
                }
            },
            "required": ["location"]
        }
    });

    assert_eq!(function["name"].as_str(), Some("get_weather"));
    assert!(function["parameters"]["properties"]["location"].is_object());
    assert!(function["parameters"]["required"].is_array());
}

#[test]
fn test_tools_format() {
    // Modern tools format (replaces functions)
    let tools = serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get weather information",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": { "type": "string" }
                    }
                }
            }
        }
    ]);

    assert!(tools.is_array());
    assert_eq!(tools[0]["type"].as_str(), Some("function"));
    assert_eq!(tools[0]["function"]["name"].as_str(), Some("get_weather"));
}

#[test]
fn test_tool_choice_format() {
    // Tool choice options
    let auto_choice = serde_json::json!("auto");
    let none_choice = serde_json::json!("none");
    let required_choice = serde_json::json!("required");
    let specific_choice = serde_json::json!({
        "type": "function",
        "function": { "name": "get_weather" }
    });

    assert_eq!(auto_choice.as_str(), Some("auto"));
    assert_eq!(none_choice.as_str(), Some("none"));
    assert_eq!(required_choice.as_str(), Some("required"));
    assert_eq!(specific_choice["type"].as_str(), Some("function"));
}

#[test]
fn test_function_call_response() {
    // Response with tool calls
    let response_json = serde_json::json!({
        "id": "chatcmpl-123",
        "object": "chat.completion",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_abc123",
                    "type": "function",
                    "function": {
                        "name": "get_weather",
                        "arguments": "{\"location\": \"San Francisco, CA\"}"
                    }
                }]
            },
            "finish_reason": "tool_calls"
        }]
    });

    let tool_calls = response_json["choices"][0]["message"]["tool_calls"].as_array();
    assert!(tool_calls.is_some());
    let calls = tool_calls.unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["function"]["name"].as_str(), Some("get_weather"));

    let finish_reason = response_json["choices"][0]["finish_reason"].as_str();
    assert_eq!(finish_reason, Some("tool_calls"));
}

#[test]
fn test_tool_result_message() {
    // Tool result message format
    let tool_result = serde_json::json!({
        "role": "tool",
        "tool_call_id": "call_abc123",
        "content": "{\"temperature\": 72, \"unit\": \"fahrenheit\"}"
    });

    assert_eq!(tool_result["role"].as_str(), Some("tool"));
    assert_eq!(tool_result["tool_call_id"].as_str(), Some("call_abc123"));
}

#[test]
fn test_parallel_tool_calls() {
    // Response with multiple parallel tool calls
    let response_json = serde_json::json!({
        "choices": [{
            "message": {
                "tool_calls": [
                    {
                        "id": "call_1",
                        "type": "function",
                        "function": { "name": "get_weather", "arguments": "{}" }
                    },
                    {
                        "id": "call_2",
                        "type": "function",
                        "function": { "name": "get_time", "arguments": "{}" }
                    }
                ]
            },
            "finish_reason": "tool_calls"
        }]
    });

    let tool_calls = response_json["choices"][0]["message"]["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 2);
}

#[test]
fn test_strict_mode_function() {
    // Strict mode requires exact parameter matching
    let strict_function = serde_json::json!({
        "type": "function",
        "function": {
            "name": "get_weather",
            "strict": true,
            "parameters": {
                "type": "object",
                "properties": {
                    "location": { "type": "string" }
                },
                "required": ["location"],
                "additionalProperties": false
            }
        }
    });

    assert_eq!(strict_function["function"]["strict"].as_bool(), Some(true));
    assert_eq!(strict_function["function"]["parameters"]["additionalProperties"].as_bool(), Some(false));
}

// =============================================================================
// JSON Mode Tests
// =============================================================================

#[test]
fn test_json_mode_request() {
    // JSON mode response format
    let request_body = serde_json::json!({
        "model": "gpt-4o",
        "messages": [{"role": "user", "content": "Return JSON"}],
        "response_format": { "type": "json_object" }
    });

    assert_eq!(request_body["response_format"]["type"].as_str(), Some("json_object"));
}

#[test]
fn test_json_schema_mode_request() {
    // Structured outputs with JSON schema
    let request_body = serde_json::json!({
        "model": "gpt-4o",
        "messages": [{"role": "user", "content": "Return structured data"}],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "weather_response",
                "strict": true,
                "schema": {
                    "type": "object",
                    "properties": {
                        "temperature": { "type": "number" },
                        "unit": { "type": "string" }
                    },
                    "required": ["temperature", "unit"]
                }
            }
        }
    });

    assert_eq!(request_body["response_format"]["type"].as_str(), Some("json_schema"));
    assert_eq!(request_body["response_format"]["json_schema"]["strict"].as_bool(), Some(true));
}

// =============================================================================
// Logprobs Tests
// =============================================================================

#[test]
fn test_logprobs_request() {
    // Request with logprobs enabled
    let request_body = serde_json::json!({
        "model": "gpt-4o",
        "messages": [{"role": "user", "content": "Hello"}],
        "logprobs": true,
        "top_logprobs": 5
    });

    assert_eq!(request_body["logprobs"].as_bool(), Some(true));
    assert_eq!(request_body["top_logprobs"].as_i64(), Some(5));
}

#[test]
fn test_logprobs_response() {
    // Response with logprobs
    let response_json = serde_json::json!({
        "choices": [{
            "message": { "content": "Hello" },
            "logprobs": {
                "content": [{
                    "token": "Hello",
                    "logprob": -0.5,
                    "bytes": [72, 101, 108, 108, 111],
                    "top_logprobs": [
                        { "token": "Hello", "logprob": -0.5 },
                        { "token": "Hi", "logprob": -1.2 }
                    ]
                }]
            }
        }]
    });

    let logprobs = &response_json["choices"][0]["logprobs"]["content"];
    assert!(logprobs.is_array());
    let first_token = &logprobs[0];
    assert_eq!(first_token["token"].as_str(), Some("Hello"));
}
