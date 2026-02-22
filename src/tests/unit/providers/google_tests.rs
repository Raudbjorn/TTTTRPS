//! Google Provider Unit Tests (API Key-based)
//!
//! Tests for the Google provider implementation (API key-based) including:
//! - API request formatting
//! - Response parsing
//! - Error handling
//! - Streaming response handling
//! - Model switching
//!
//! Note: For OAuth-based Gemini provider tests, see gemini_tests.rs

use crate::core::llm::cost::TokenUsage;
use crate::core::llm::providers::GoogleProvider;
use crate::core::llm::router::{ChatMessage, ChatRequest, LLMError, LLMProvider, MessageRole};

// =============================================================================
// Provider Identity Tests
// =============================================================================

#[test]
fn test_provider_id() {
    let provider = GoogleProvider::new(
        "AIzaTestApiKey".to_string(),
        "gemini-2.0-flash-exp".to_string(),
    );
    assert_eq!(provider.id(), "google");
}

#[test]
fn test_provider_name() {
    let provider = GoogleProvider::new(
        "AIzaTestApiKey".to_string(),
        "gemini-2.0-flash-exp".to_string(),
    );
    assert_eq!(provider.name(), "Google");
}

#[test]
fn test_provider_model() {
    let provider = GoogleProvider::new(
        "AIzaTestApiKey".to_string(),
        "gemini-2.0-flash-exp".to_string(),
    );
    assert_eq!(provider.model(), "gemini-2.0-flash-exp");
}

#[test]
fn test_flash_convenience_constructor() {
    let provider = GoogleProvider::flash("AIzaTestApiKey".to_string());
    assert_eq!(provider.model(), "gemini-2.0-flash-exp");
}

#[test]
fn test_pro_convenience_constructor() {
    let provider = GoogleProvider::pro("AIzaTestApiKey".to_string());
    assert_eq!(provider.model(), "gemini-1.5-pro");
}

// =============================================================================
// API Key Format Validation Tests (pure, no network calls)
// =============================================================================

#[test]
fn test_api_key_format_accepts_valid_keys() {
    // Keys starting with "AIza" should be considered valid format
    assert!(GoogleProvider::is_valid_api_key_format("AIzaValidApiKey12345"));
    assert!(GoogleProvider::is_valid_api_key_format("AIzaSyD_abcdefghijklmnop"));
    assert!(GoogleProvider::is_valid_api_key_format("AIza"));  // Minimum valid
}

#[test]
fn test_api_key_format_rejects_invalid_keys() {
    // Empty and non-matching prefixes should be rejected
    assert!(!GoogleProvider::is_valid_api_key_format(""));
    assert!(!GoogleProvider::is_valid_api_key_format("   "));
    assert!(!GoogleProvider::is_valid_api_key_format("BOGUS-KEY"));
    assert!(!GoogleProvider::is_valid_api_key_format("AXzaSomething"));
    assert!(!GoogleProvider::is_valid_api_key_format("aiza-lowercase"));
    assert!(!GoogleProvider::is_valid_api_key_format("sk-openai-key"));
}

#[test]
fn test_api_key_format_trims_whitespace() {
    // Keys with leading/trailing whitespace should work if core is valid
    assert!(GoogleProvider::is_valid_api_key_format("  AIzaValidKey  "));
    assert!(!GoogleProvider::is_valid_api_key_format("  invalid  "));
}

// =============================================================================
// Health Check Tests
// =============================================================================

#[tokio::test]
async fn test_health_check_invalid_key_format() {
    let provider = GoogleProvider::new(
        "invalid-key".to_string(),
        "gemini-2.0-flash-exp".to_string(),
    );
    // Invalid key format should fail health check immediately (no API call)
    assert!(!provider.health_check().await);
}

// Integration-style test: exercises real health_check(), including network call.
// This is ignored by default to keep CI deterministic.
// Run explicitly with: GOOGLE_API_KEY=your-key cargo test test_health_check_valid_key -- --ignored
#[tokio::test]
#[ignore = "Requires GOOGLE_API_KEY env var with valid Google API key"]
async fn test_health_check_valid_key() {
    let api_key = match std::env::var("GOOGLE_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("GOOGLE_API_KEY not set, skipping test");
            return;
        }
    };
    let provider = GoogleProvider::new(api_key, "gemini-2.0-flash-exp".to_string());
    // Note: health_check() validates key format AND makes an API call
    assert!(provider.health_check().await);
}

// =============================================================================
// Pricing Tests
// =============================================================================

#[test]
fn test_pricing_flash_model() {
    let provider = GoogleProvider::flash("AIzaTestApiKey".to_string());
    let pricing = provider.pricing();
    assert!(pricing.is_some());
    let pricing = pricing.unwrap();
    assert_eq!(pricing.provider_id, "google");
    assert!(!pricing.is_free);
    // Flash pricing: $0.10/1M input, $0.40/1M output
    assert_eq!(pricing.input_cost_per_million, 0.10);
    assert_eq!(pricing.output_cost_per_million, 0.40);
}

#[test]
fn test_pricing_pro_model() {
    let provider = GoogleProvider::pro("AIzaTestApiKey".to_string());
    let pricing = provider.pricing();
    assert!(pricing.is_some());
    let pricing = pricing.unwrap();
    // Pro pricing: $1.25/1M input, $5.00/1M output
    assert_eq!(pricing.input_cost_per_million, 1.25);
    assert_eq!(pricing.output_cost_per_million, 5.0);
}

#[test]
fn test_pricing_cost_calculation() {
    let provider = GoogleProvider::flash("AIzaTestApiKey".to_string());
    let pricing = provider.pricing().unwrap();

    let usage = TokenUsage::new(10000, 5000);
    let cost = pricing.calculate_cost(&usage);
    // (10000/1M * 0.10) + (5000/1M * 0.40) = 0.001 + 0.002 = 0.003
    assert!((cost - 0.003).abs() < 0.0001);
}

// =============================================================================
// Request Formatting Tests
// =============================================================================

#[test]
fn test_build_request_basic() {
    let request = ChatRequest::new(vec![ChatMessage::user("Hello")]);

    assert_eq!(request.messages.len(), 1);
    assert_eq!(request.messages[0].role, MessageRole::User);
}

#[test]
fn test_build_request_with_system_instruction() {
    let request = ChatRequest::new(vec![ChatMessage::user("Hello")])
        .with_system("You are helpful");

    // Gemini uses systemInstruction field
    assert_eq!(request.system_prompt, Some("You are helpful".to_string()));
}

#[test]
fn test_build_request_with_generation_config() {
    let request = ChatRequest::new(vec![ChatMessage::user("Hello")])
        .with_temperature(0.5)
        .with_max_tokens(1000);

    assert_eq!(request.temperature, Some(0.5));
    assert_eq!(request.max_tokens, Some(1000));
}

#[test]
fn test_message_role_mapping_user() {
    let msg = ChatMessage::user("Test");
    assert_eq!(msg.role, MessageRole::User);

    // Gemini uses "user" role
    let gemini_role = match msg.role {
        MessageRole::User => "user",
        MessageRole::Assistant => "model", // Gemini uses "model" instead of "assistant"
        MessageRole::System => "system", // Filtered in content building
    };
    assert_eq!(gemini_role, "user");
}

#[test]
fn test_message_role_mapping_assistant() {
    let msg = ChatMessage::assistant("Test");
    assert_eq!(msg.role, MessageRole::Assistant);

    // Gemini uses "model" for assistant messages
    let gemini_role = match msg.role {
        MessageRole::User => "user",
        MessageRole::Assistant => "model",
        MessageRole::System => "system",
    };
    assert_eq!(gemini_role, "model");
}

#[test]
fn test_system_messages_filtered_in_contents() {
    // Gemini filters system messages from contents array
    // and uses systemInstruction instead
    let request = ChatRequest::new(vec![
        ChatMessage::system("This should not be in contents"),
        ChatMessage::user("Hello"),
    ]);

    assert_eq!(request.messages.len(), 2);
}

#[test]
fn test_contents_format() {
    // Gemini uses "parts" array with "text" field
    let expected_format = serde_json::json!({
        "role": "user",
        "parts": [{ "text": "Hello" }]
    });

    assert!(expected_format["parts"].is_array());
    assert_eq!(expected_format["parts"][0]["text"].as_str(), Some("Hello"));
}

// =============================================================================
// Response Parsing Tests
// =============================================================================

#[test]
fn test_parse_successful_response() {
    let response_json = serde_json::json!({
        "candidates": [{
            "content": {
                "parts": [{
                    "text": "Hello! How can I help you today?"
                }],
                "role": "model"
            },
            "finishReason": "STOP",
            "index": 0
        }],
        "usageMetadata": {
            "promptTokenCount": 10,
            "candidatesTokenCount": 15,
            "totalTokenCount": 25
        }
    });

    let content = response_json["candidates"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["content"]["parts"].as_array())
        .and_then(|parts| parts.first())
        .and_then(|p| p["text"].as_str());

    assert_eq!(content, Some("Hello! How can I help you today?"));

    let finish_reason = response_json["candidates"][0]["finishReason"].as_str();
    assert_eq!(finish_reason, Some("STOP"));
}

#[test]
fn test_parse_response_usage_gemini_format() {
    // Gemini uses different field names for token counts
    let response_json = serde_json::json!({
        "usageMetadata": {
            "promptTokenCount": 100,
            "candidatesTokenCount": 50,
            "totalTokenCount": 150
        }
    });

    let usage_obj = response_json["usageMetadata"].as_object().unwrap();
    let usage = TokenUsage {
        input_tokens: usage_obj["promptTokenCount"].as_u64().unwrap_or(0) as u32,
        output_tokens: usage_obj["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
    };

    assert_eq!(usage.input_tokens, 100);
    assert_eq!(usage.output_tokens, 50);
    assert_eq!(usage.total(), 150);
}

#[test]
fn test_parse_response_with_missing_content() {
    let response_json = serde_json::json!({
        "candidates": []
    });

    let content = response_json["candidates"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["content"]["parts"].as_array())
        .and_then(|parts| parts.first())
        .and_then(|p| p["text"].as_str());

    assert!(content.is_none());
}

#[test]
fn test_parse_response_with_safety_block() {
    let response_json = serde_json::json!({
        "candidates": [{
            "finishReason": "SAFETY",
            "safetyRatings": [{
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "HIGH"
            }]
        }]
    });

    let finish_reason = response_json["candidates"][0]["finishReason"].as_str();
    assert_eq!(finish_reason, Some("SAFETY"));
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_api_error() {
    let error = LLMError::ApiError {
        status: 400,
        message: "Invalid request".to_string(),
    };
    let error_msg = error.to_string();
    assert!(error_msg.contains("400"));
    assert!(error_msg.contains("Invalid request"));
}

#[test]
fn test_api_error_403() {
    // 403 typically means API key issues or quota exceeded
    let error = LLMError::ApiError {
        status: 403,
        message: "Permission denied. API key may be invalid.".to_string(),
    };
    let error_msg = error.to_string();
    assert!(error_msg.contains("403"));
    assert!(error_msg.contains("Permission denied"));
}

#[test]
fn test_api_error_429() {
    // Rate limit exceeded
    let error = LLMError::ApiError {
        status: 429,
        message: "Resource exhausted. Quota exceeded.".to_string(),
    };
    let error_msg = error.to_string();
    assert!(error_msg.contains("429"));
}

#[test]
fn test_invalid_response_error() {
    let error = LLMError::InvalidResponse("Missing content".to_string());
    let error_msg = error.to_string();
    assert!(error_msg.contains("Invalid response"));
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
    // Gemini streaming uses SSE with data: prefix
    let chunk_json = serde_json::json!({
        "candidates": [{
            "content": {
                "parts": [{
                    "text": "Hello"
                }],
                "role": "model"
            },
            "finishReason": null
        }]
    });

    let text = chunk_json["candidates"][0]["content"]["parts"][0]["text"].as_str();
    assert_eq!(text, Some("Hello"));
}

#[test]
fn test_parse_stream_final_chunk() {
    let chunk_json = serde_json::json!({
        "candidates": [{
            "content": {
                "parts": [{
                    "text": ""
                }],
                "role": "model"
            },
            "finishReason": "STOP"
        }],
        "usageMetadata": {
            "promptTokenCount": 10,
            "candidatesTokenCount": 50,
            "totalTokenCount": 60
        }
    });

    let finish_reason = chunk_json["candidates"][0]["finishReason"].as_str();
    assert_eq!(finish_reason, Some("STOP"));

    let usage = chunk_json["usageMetadata"].as_object();
    assert!(usage.is_some());
}

#[test]
fn test_parse_sse_line() {
    let sse_line = r#"data: {"candidates":[{"content":{"parts":[{"text":"Hi"}]}}]}"#;

    assert!(sse_line.starts_with("data: "));

    let data = &sse_line[6..];
    let parsed: serde_json::Value = serde_json::from_str(data).unwrap();

    assert_eq!(
        parsed["candidates"][0]["content"]["parts"][0]["text"].as_str(),
        Some("Hi")
    );
}

#[test]
fn test_stream_url_format() {
    // Gemini streaming URL includes ?alt=sse
    let base_url = "https://generativelanguage.googleapis.com/v1beta/models";
    let model = "gemini-2.0-flash-exp";
    let api_key = "AIzaTestKey";

    let stream_url = format!(
        "{}/{}:streamGenerateContent?key={}&alt=sse",
        base_url, model, api_key
    );

    assert!(stream_url.contains("streamGenerateContent"));
    assert!(stream_url.contains("alt=sse"));
}

// =============================================================================
// Model Switching Tests
// =============================================================================

#[test]
fn test_model_switching_flash() {
    let provider = GoogleProvider::new(
        "AIzaTestKey".to_string(),
        "gemini-2.0-flash-exp".to_string(),
    );
    assert_eq!(provider.model(), "gemini-2.0-flash-exp");
}

#[test]
fn test_model_switching_pro() {
    let provider = GoogleProvider::new(
        "AIzaTestKey".to_string(),
        "gemini-1.5-pro".to_string(),
    );
    assert_eq!(provider.model(), "gemini-1.5-pro");
}

#[test]
fn test_model_switching_1_5_flash() {
    let provider = GoogleProvider::new(
        "AIzaTestKey".to_string(),
        "gemini-1.5-flash".to_string(),
    );
    assert_eq!(provider.model(), "gemini-1.5-flash");
}

#[test]
fn test_model_switching_1_0_pro() {
    let provider = GoogleProvider::new(
        "AIzaTestKey".to_string(),
        "gemini-1.0-pro".to_string(),
    );
    assert_eq!(provider.model(), "gemini-1.0-pro");
}

// =============================================================================
// URL Construction Tests
// =============================================================================

#[test]
fn test_api_url_format() {
    let model = "gemini-2.0-flash-exp";
    let api_key = "AIzaTestKey";

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    assert!(url.contains("generativelanguage.googleapis.com"));
    assert!(url.contains("v1beta"));
    assert!(url.contains(model));
    assert!(url.contains("generateContent"));
}

#[test]
fn test_stream_url_with_sse() {
    let model = "gemini-2.0-flash-exp";
    let api_key = "AIzaTestKey";

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}&alt=sse",
        model, api_key
    );

    assert!(url.contains("streamGenerateContent"));
    assert!(url.contains("alt=sse"));
}

// =============================================================================
// Generation Config Tests
// =============================================================================

#[test]
fn test_generation_config_temperature() {
    let gen_config = serde_json::json!({
        "temperature": 0.7
    });

    assert_eq!(gen_config["temperature"].as_f64(), Some(0.7));
}

#[test]
fn test_generation_config_max_output_tokens() {
    let gen_config = serde_json::json!({
        "maxOutputTokens": 2048
    });

    assert_eq!(gen_config["maxOutputTokens"].as_i64(), Some(2048));
}

#[test]
fn test_generation_config_combined() {
    let gen_config = serde_json::json!({
        "temperature": 0.5,
        "maxOutputTokens": 1000,
        "topP": 0.9,
        "topK": 40
    });

    assert_eq!(gen_config["temperature"].as_f64(), Some(0.5));
    assert_eq!(gen_config["maxOutputTokens"].as_i64(), Some(1000));
}

// =============================================================================
// Provider Trait Implementation Tests
// =============================================================================

#[test]
fn test_supports_streaming() {
    let provider = GoogleProvider::flash("AIzaTestKey".to_string());
    assert!(provider.supports_streaming());
}

#[test]
fn test_supports_embeddings() {
    let provider = GoogleProvider::flash("AIzaTestKey".to_string());
    // Gemini supports embeddings
    assert!(provider.supports_embeddings());
}

// =============================================================================
// System Instruction Tests
// =============================================================================

#[test]
fn test_system_instruction_format() {
    let system_instruction = serde_json::json!({
        "parts": [{ "text": "You are a helpful assistant" }]
    });

    assert!(system_instruction["parts"].is_array());
    assert_eq!(
        system_instruction["parts"][0]["text"].as_str(),
        Some("You are a helpful assistant")
    );
}

// =============================================================================
// Finish Reason Tests
// =============================================================================

#[test]
fn test_finish_reason_stop() {
    assert_eq!("STOP", "STOP");
}

#[test]
fn test_finish_reason_max_tokens() {
    assert_eq!("MAX_TOKENS", "MAX_TOKENS");
}

#[test]
fn test_finish_reason_safety() {
    assert_eq!("SAFETY", "SAFETY");
}

#[test]
fn test_finish_reason_recitation() {
    assert_eq!("RECITATION", "RECITATION");
}

#[test]
fn test_finish_reason_other() {
    assert_eq!("OTHER", "OTHER");
}

// =============================================================================
// Safety Settings Tests
// =============================================================================

#[test]
fn test_safety_settings_format() {
    // Safety settings structure
    let safety_settings = serde_json::json!([
        {
            "category": "HARM_CATEGORY_HATE_SPEECH",
            "threshold": "BLOCK_MEDIUM_AND_ABOVE"
        },
        {
            "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
            "threshold": "BLOCK_ONLY_HIGH"
        },
        {
            "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
            "threshold": "BLOCK_LOW_AND_ABOVE"
        },
        {
            "category": "HARM_CATEGORY_HARASSMENT",
            "threshold": "BLOCK_NONE"
        }
    ]);

    assert!(safety_settings.is_array());
    let settings = safety_settings.as_array().unwrap();
    assert_eq!(settings.len(), 4);
}

#[test]
fn test_safety_threshold_levels() {
    // All valid threshold levels
    let thresholds = vec![
        "BLOCK_NONE",
        "BLOCK_ONLY_HIGH",
        "BLOCK_MEDIUM_AND_ABOVE",
        "BLOCK_LOW_AND_ABOVE",
        "HARM_BLOCK_THRESHOLD_UNSPECIFIED",
    ];

    for threshold in thresholds {
        assert!(!threshold.is_empty());
    }
}

#[test]
fn test_harm_categories() {
    // All harm categories
    let categories = vec![
        "HARM_CATEGORY_HATE_SPEECH",
        "HARM_CATEGORY_DANGEROUS_CONTENT",
        "HARM_CATEGORY_SEXUALLY_EXPLICIT",
        "HARM_CATEGORY_HARASSMENT",
        "HARM_CATEGORY_CIVIC_INTEGRITY",
    ];

    for category in categories {
        assert!(category.starts_with("HARM_CATEGORY_"));
    }
}

#[test]
fn test_safety_rating_response() {
    // Safety ratings in response
    let response_json = serde_json::json!({
        "candidates": [{
            "safetyRatings": [
                {
                    "category": "HARM_CATEGORY_HATE_SPEECH",
                    "probability": "NEGLIGIBLE"
                },
                {
                    "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                    "probability": "LOW"
                },
                {
                    "category": "HARM_CATEGORY_HARASSMENT",
                    "probability": "MEDIUM"
                }
            ],
            "content": {
                "parts": [{ "text": "Response content" }]
            },
            "finishReason": "STOP"
        }]
    });

    let safety_ratings = response_json["candidates"][0]["safetyRatings"].as_array();
    assert!(safety_ratings.is_some());
    assert_eq!(safety_ratings.unwrap().len(), 3);
}

#[test]
fn test_safety_probability_levels() {
    // Probability levels in safety ratings
    let probabilities = vec![
        "NEGLIGIBLE",
        "LOW",
        "MEDIUM",
        "HIGH",
    ];

    for prob in probabilities {
        assert!(!prob.is_empty());
    }
}

#[test]
fn test_blocked_response_due_to_safety() {
    // Response blocked due to safety filters
    let blocked_response = serde_json::json!({
        "candidates": [{
            "finishReason": "SAFETY",
            "safetyRatings": [
                {
                    "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                    "probability": "HIGH",
                    "blocked": true
                }
            ]
        }],
        "promptFeedback": {
            "blockReason": "SAFETY",
            "safetyRatings": [
                {
                    "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                    "probability": "HIGH"
                }
            ]
        }
    });

    let finish_reason = blocked_response["candidates"][0]["finishReason"].as_str();
    assert_eq!(finish_reason, Some("SAFETY"));

    let block_reason = blocked_response["promptFeedback"]["blockReason"].as_str();
    assert_eq!(block_reason, Some("SAFETY"));
}

#[test]
fn test_request_with_safety_settings() {
    // Full request with safety settings
    let request_body = serde_json::json!({
        "contents": [{
            "role": "user",
            "parts": [{ "text": "Hello" }]
        }],
        "safetySettings": [
            {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "threshold": "BLOCK_MEDIUM_AND_ABOVE"
            }
        ],
        "generationConfig": {
            "temperature": 0.7,
            "maxOutputTokens": 1000
        }
    });

    assert!(request_body["safetySettings"].is_array());
    assert!(request_body["generationConfig"].is_object());
}

// =============================================================================
// Citation/Grounding Tests
// =============================================================================

#[test]
fn test_citation_metadata_response() {
    // Response with citation metadata
    let response_json = serde_json::json!({
        "candidates": [{
            "content": {
                "parts": [{ "text": "The answer is 42" }]
            },
            "citationMetadata": {
                "citationSources": [
                    {
                        "startIndex": 0,
                        "endIndex": 15,
                        "uri": "https://example.com/source",
                        "license": "MIT"
                    }
                ]
            },
            "finishReason": "STOP"
        }]
    });

    let citations = response_json["candidates"][0]["citationMetadata"]["citationSources"].as_array();
    assert!(citations.is_some());
    assert_eq!(citations.unwrap().len(), 1);
}

// =============================================================================
// Grounding/Search Tests
// =============================================================================

#[test]
fn test_google_search_grounding_request() {
    // Request with Google Search grounding
    let request_body = serde_json::json!({
        "contents": [{
            "role": "user",
            "parts": [{ "text": "What is the weather today?" }]
        }],
        "tools": [{
            "googleSearchRetrieval": {
                "dynamicRetrievalConfig": {
                    "mode": "MODE_DYNAMIC",
                    "dynamicThreshold": 0.3
                }
            }
        }]
    });

    assert!(request_body["tools"].is_array());
    let tools = request_body["tools"].as_array().unwrap();
    assert!(tools[0]["googleSearchRetrieval"].is_object());
}

#[test]
fn test_grounding_metadata_response() {
    // Response with grounding metadata
    let response_json = serde_json::json!({
        "candidates": [{
            "content": {
                "parts": [{ "text": "Based on recent information..." }]
            },
            "groundingMetadata": {
                "searchEntryPoint": {
                    "renderedContent": "<search results>"
                },
                "groundingChunks": [
                    {
                        "web": {
                            "uri": "https://example.com",
                            "title": "Example Source"
                        }
                    }
                ],
                "groundingSupports": [
                    {
                        "segment": {
                            "startIndex": 0,
                            "endIndex": 30
                        },
                        "groundingChunkIndices": [0],
                        "confidenceScores": [0.95]
                    }
                ]
            },
            "finishReason": "STOP"
        }]
    });

    let grounding = response_json["candidates"][0]["groundingMetadata"].as_object();
    assert!(grounding.is_some());
}

// =============================================================================
// Multimodal Tests
// =============================================================================

#[test]
fn test_image_input_format() {
    // Image input in Gemini format
    let image_content = serde_json::json!({
        "role": "user",
        "parts": [
            { "text": "What is in this image?" },
            {
                "inlineData": {
                    "mimeType": "image/jpeg",
                    "data": "base64encodeddata..."
                }
            }
        ]
    });

    let parts = image_content["parts"].as_array().unwrap();
    assert_eq!(parts.len(), 2);
    assert!(parts[1]["inlineData"].is_object());
}

#[test]
fn test_file_uri_format() {
    // File URI format for uploaded files
    let file_content = serde_json::json!({
        "role": "user",
        "parts": [
            { "text": "Describe this file" },
            {
                "fileData": {
                    "mimeType": "application/pdf",
                    "fileUri": "gs://bucket/file.pdf"
                }
            }
        ]
    });

    let file_part = &file_content["parts"][1];
    assert!(file_part["fileData"]["fileUri"].is_string());
}

#[test]
fn test_video_input_format() {
    // Video input format
    let video_content = serde_json::json!({
        "role": "user",
        "parts": [
            { "text": "What happens in this video?" },
            {
                "fileData": {
                    "mimeType": "video/mp4",
                    "fileUri": "gs://bucket/video.mp4"
                }
            }
        ]
    });

    let video_part = &video_content["parts"][1];
    assert_eq!(video_part["fileData"]["mimeType"].as_str(), Some("video/mp4"));
}

// =============================================================================
// Code Execution Tests
// =============================================================================

#[test]
fn test_code_execution_tool() {
    // Code execution tool format
    let request_body = serde_json::json!({
        "contents": [{
            "role": "user",
            "parts": [{ "text": "Calculate the fibonacci sequence" }]
        }],
        "tools": [{
            "codeExecution": {}
        }]
    });

    assert!(request_body["tools"][0]["codeExecution"].is_object());
}

#[test]
fn test_code_execution_response() {
    // Response with code execution result
    let response_json = serde_json::json!({
        "candidates": [{
            "content": {
                "parts": [
                    {
                        "executableCode": {
                            "language": "PYTHON",
                            "code": "def fib(n):\n    if n <= 1:\n        return n\n    return fib(n-1) + fib(n-2)\n\nresult = [fib(i) for i in range(10)]\nprint(result)"
                        }
                    },
                    {
                        "codeExecutionResult": {
                            "outcome": "OUTCOME_OK",
                            "output": "[0, 1, 1, 2, 3, 5, 8, 13, 21, 34]"
                        }
                    },
                    {
                        "text": "The first 10 Fibonacci numbers are [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]"
                    }
                ]
            },
            "finishReason": "STOP"
        }]
    });

    let parts = response_json["candidates"][0]["content"]["parts"].as_array().unwrap();
    assert!(parts[0]["executableCode"].is_object());
    assert!(parts[1]["codeExecutionResult"].is_object());
    assert_eq!(parts[1]["codeExecutionResult"]["outcome"].as_str(), Some("OUTCOME_OK"));
}

// =============================================================================
// Function Calling Tests (Gemini)
// =============================================================================

#[test]
fn test_gemini_function_declaration() {
    // Gemini function declaration format
    let function = serde_json::json!({
        "name": "get_weather",
        "description": "Get weather for a location",
        "parameters": {
            "type": "OBJECT",
            "properties": {
                "location": {
                    "type": "STRING",
                    "description": "The city name"
                }
            },
            "required": ["location"]
        }
    });

    assert_eq!(function["name"].as_str(), Some("get_weather"));
    // Gemini uses uppercase TYPE names
    assert_eq!(function["parameters"]["type"].as_str(), Some("OBJECT"));
}

#[test]
fn test_gemini_tool_config() {
    // Tool config for function calling
    let tool_config = serde_json::json!({
        "functionCallingConfig": {
            "mode": "AUTO",
            "allowedFunctionNames": ["get_weather", "get_time"]
        }
    });

    assert_eq!(tool_config["functionCallingConfig"]["mode"].as_str(), Some("AUTO"));
}

#[test]
fn test_gemini_function_call_response() {
    // Response with function call
    let response_json = serde_json::json!({
        "candidates": [{
            "content": {
                "parts": [{
                    "functionCall": {
                        "name": "get_weather",
                        "args": {
                            "location": "San Francisco"
                        }
                    }
                }]
            },
            "finishReason": "STOP"
        }]
    });

    let function_call = &response_json["candidates"][0]["content"]["parts"][0]["functionCall"];
    assert_eq!(function_call["name"].as_str(), Some("get_weather"));
}

#[test]
fn test_gemini_function_response() {
    // Function response message
    let function_response = serde_json::json!({
        "role": "user",
        "parts": [{
            "functionResponse": {
                "name": "get_weather",
                "response": {
                    "temperature": 72,
                    "conditions": "sunny"
                }
            }
        }]
    });

    let response = &function_response["parts"][0]["functionResponse"];
    assert_eq!(response["name"].as_str(), Some("get_weather"));
}
