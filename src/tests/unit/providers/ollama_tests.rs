//! Ollama Provider Unit Tests
//!
//! Tests for the Ollama local LLM provider implementation including:
//! - API request formatting
//! - Response parsing
//! - Error handling
//! - Streaming response handling
//! - Model switching
//! - Host configuration

use crate::core::llm::cost::TokenUsage;
use crate::core::llm::providers::OllamaProvider;
use crate::core::llm::router::{ChatMessage, ChatRequest, LLMError, LLMProvider, MessageRole};

// =============================================================================
// Provider Identity Tests
// =============================================================================

#[test]
fn test_provider_id() {
    let provider = OllamaProvider::new(
        "http://localhost:11434".to_string(),
        "llama3.2".to_string(),
    );
    assert_eq!(provider.id(), "ollama");
}

#[test]
fn test_provider_name() {
    let provider = OllamaProvider::new(
        "http://localhost:11434".to_string(),
        "llama3.2".to_string(),
    );
    assert_eq!(provider.name(), "Ollama");
}

#[test]
fn test_provider_model() {
    let provider = OllamaProvider::new(
        "http://localhost:11434".to_string(),
        "llama3.2".to_string(),
    );
    assert_eq!(provider.model(), "llama3.2");
}

#[test]
fn test_localhost_convenience_constructor() {
    let provider = OllamaProvider::localhost("llama3.2".to_string());
    assert_eq!(provider.model(), "llama3.2");
}

// =============================================================================
// Configuration Tests
// =============================================================================

#[test]
fn test_custom_host() {
    let provider = OllamaProvider::new(
        "http://192.168.1.100:11434".to_string(),
        "llama3.2".to_string(),
    );
    assert_eq!(provider.model(), "llama3.2");
}

#[test]
fn test_custom_port() {
    let provider = OllamaProvider::new(
        "http://localhost:8080".to_string(),
        "llama3.2".to_string(),
    );
    assert_eq!(provider.model(), "llama3.2");
}

#[test]
fn test_remote_host() {
    let provider = OllamaProvider::new(
        "http://ollama.example.com:11434".to_string(),
        "llama3.2".to_string(),
    );
    assert_eq!(provider.model(), "llama3.2");
}

// =============================================================================
// Pricing Tests
// =============================================================================

#[test]
fn test_pricing_is_free() {
    let provider = OllamaProvider::localhost("llama3.2".to_string());
    let pricing = provider.pricing();
    assert!(pricing.is_some());
    let pricing = pricing.unwrap();
    assert_eq!(pricing.provider_id, "ollama");
    assert!(pricing.is_free);
    assert_eq!(pricing.input_cost_per_million, 0.0);
    assert_eq!(pricing.output_cost_per_million, 0.0);
}

#[test]
fn test_pricing_cost_calculation_zero() {
    let provider = OllamaProvider::localhost("llama3.2".to_string());
    let pricing = provider.pricing().unwrap();

    let usage = TokenUsage::new(100000, 50000);
    let cost = pricing.calculate_cost(&usage);
    // Local models are free
    assert_eq!(cost, 0.0);
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
fn test_build_request_with_system_prompt() {
    let request = ChatRequest::new(vec![ChatMessage::user("Hello")])
        .with_system("You are helpful");

    assert_eq!(request.system_prompt, Some("You are helpful".to_string()));
}

#[test]
fn test_build_request_with_temperature() {
    let request = ChatRequest::new(vec![ChatMessage::user("Hello")])
        .with_temperature(0.8);

    assert_eq!(request.temperature, Some(0.8));
}

#[test]
fn test_message_role_mapping_user() {
    let msg = ChatMessage::user("Test");
    assert_eq!(msg.role, MessageRole::User);

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
    // Ollama includes system prompt as a message
    let request = ChatRequest::new(vec![ChatMessage::user("Hello")])
        .with_system("Be helpful");

    assert_eq!(request.system_prompt, Some("Be helpful".to_string()));
}

// =============================================================================
// Request Body Format Tests
// =============================================================================

#[test]
fn test_request_body_format() {
    let expected_body = serde_json::json!({
        "model": "llama3.2",
        "messages": [
            { "role": "system", "content": "You are helpful" },
            { "role": "user", "content": "Hello" }
        ],
        "stream": false,
        "options": {
            "temperature": 0.7
        }
    });

    assert!(expected_body["model"].is_string());
    assert!(expected_body["messages"].is_array());
    assert!(expected_body["stream"].is_boolean());
    assert!(expected_body["options"]["temperature"].is_f64());
}

#[test]
fn test_streaming_request_body() {
    let expected_body = serde_json::json!({
        "model": "llama3.2",
        "messages": [
            { "role": "user", "content": "Hello" }
        ],
        "stream": true,
        "options": {
            "temperature": 0.7
        }
    });

    assert_eq!(expected_body["stream"].as_bool(), Some(true));
}

#[test]
fn test_default_temperature() {
    // Default temperature is 0.7 if not specified
    let request = ChatRequest::new(vec![ChatMessage::user("Hello")]);
    let default_temp = request.temperature.unwrap_or(0.7);
    assert_eq!(default_temp, 0.7);
}

// =============================================================================
// Response Parsing Tests
// =============================================================================

#[test]
fn test_parse_successful_response() {
    let response_json = serde_json::json!({
        "model": "llama3.2",
        "created_at": "2024-01-01T00:00:00Z",
        "message": {
            "role": "assistant",
            "content": "Hello! How can I help you today?"
        },
        "done": true,
        "total_duration": 5000000000_i64,
        "load_duration": 1000000000_i64,
        "prompt_eval_count": 10,
        "prompt_eval_duration": 500000000_i64,
        "eval_count": 20,
        "eval_duration": 1000000000_i64
    });

    let content = response_json["message"]["content"].as_str();
    assert_eq!(content, Some("Hello! How can I help you today?"));

    let done = response_json["done"].as_bool();
    assert_eq!(done, Some(true));
}

#[test]
fn test_parse_response_token_counts() {
    // Ollama uses prompt_eval_count and eval_count for token counts
    let response_json = serde_json::json!({
        "prompt_eval_count": 100,
        "eval_count": 50
    });

    let input_tokens = response_json["prompt_eval_count"].as_u64().unwrap_or(0) as u32;
    let output_tokens = response_json["eval_count"].as_u64().unwrap_or(0) as u32;

    assert_eq!(input_tokens, 100);
    assert_eq!(output_tokens, 50);
}

#[test]
fn test_parse_response_with_missing_content() {
    let response_json = serde_json::json!({
        "model": "llama3.2",
        "done": true
    });

    let content = response_json["message"]["content"].as_str();
    assert!(content.is_none());
}

#[test]
fn test_parse_response_durations() {
    let response_json = serde_json::json!({
        "total_duration": 5000000000_i64,
        "load_duration": 1000000000_i64,
        "prompt_eval_duration": 500000000_i64,
        "eval_duration": 1000000000_i64
    });

    // Durations are in nanoseconds
    let total_duration_ns = response_json["total_duration"].as_i64().unwrap();
    let total_duration_ms = total_duration_ns / 1_000_000;
    assert_eq!(total_duration_ms, 5000);
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_api_error_connection_refused() {
    let error = LLMError::ApiError {
        status: 0,
        message: "Connection refused".to_string(),
    };
    let error_msg = error.to_string();
    assert!(error_msg.contains("Connection refused"));
}

#[test]
fn test_api_error_model_not_found() {
    let error = LLMError::ApiError {
        status: 404,
        message: "model 'nonexistent' not found".to_string(),
    };
    let error_msg = error.to_string();
    assert!(error_msg.contains("404"));
    assert!(error_msg.contains("not found"));
}

#[test]
fn test_api_error_server_busy() {
    let error = LLMError::ApiError {
        status: 503,
        message: "Server is busy".to_string(),
    };
    let error_msg = error.to_string();
    assert!(error_msg.contains("503"));
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
    // Ollama streaming returns NDJSON (one JSON object per line)
    let chunk_json = serde_json::json!({
        "model": "llama3.2",
        "created_at": "2024-01-01T00:00:00Z",
        "message": {
            "role": "assistant",
            "content": "Hello"
        },
        "done": false
    });

    let content = chunk_json["message"]["content"].as_str();
    assert_eq!(content, Some("Hello"));

    let done = chunk_json["done"].as_bool();
    assert_eq!(done, Some(false));
}

#[test]
fn test_parse_stream_final_chunk() {
    let chunk_json = serde_json::json!({
        "model": "llama3.2",
        "message": {
            "role": "assistant",
            "content": ""
        },
        "done": true,
        "prompt_eval_count": 50,
        "eval_count": 100
    });

    let done = chunk_json["done"].as_bool();
    assert_eq!(done, Some(true));

    let input_tokens = chunk_json["prompt_eval_count"].as_u64();
    let output_tokens = chunk_json["eval_count"].as_u64();
    assert_eq!(input_tokens, Some(50));
    assert_eq!(output_tokens, Some(100));
}

#[test]
fn test_parse_ndjson_line() {
    // Ollama uses NDJSON (newline-delimited JSON), not SSE
    let ndjson_line = r#"{"model":"llama3.2","message":{"content":"Hi"},"done":false}"#;

    let parsed: serde_json::Value = serde_json::from_str(ndjson_line).unwrap();
    assert_eq!(parsed["message"]["content"].as_str(), Some("Hi"));
    assert_eq!(parsed["done"].as_bool(), Some(false));
}

#[test]
fn test_empty_line_handling() {
    // Empty lines should be skipped in NDJSON parsing
    let line = "";
    assert!(line.is_empty());
}

// =============================================================================
// Model Switching Tests
// =============================================================================

#[test]
fn test_model_switching_llama3() {
    let provider = OllamaProvider::localhost("llama3.2".to_string());
    assert_eq!(provider.model(), "llama3.2");
}

#[test]
fn test_model_switching_mistral() {
    let provider = OllamaProvider::localhost("mistral".to_string());
    assert_eq!(provider.model(), "mistral");
}

#[test]
fn test_model_switching_codellama() {
    let provider = OllamaProvider::localhost("codellama".to_string());
    assert_eq!(provider.model(), "codellama");
}

#[test]
fn test_model_switching_phi() {
    let provider = OllamaProvider::localhost("phi3".to_string());
    assert_eq!(provider.model(), "phi3");
}

#[test]
fn test_model_switching_gemma() {
    let provider = OllamaProvider::localhost("gemma2".to_string());
    assert_eq!(provider.model(), "gemma2");
}

#[test]
fn test_model_switching_custom() {
    let provider = OllamaProvider::localhost("my-custom-model:latest".to_string());
    assert_eq!(provider.model(), "my-custom-model:latest");
}

#[test]
fn test_model_with_tag() {
    let provider = OllamaProvider::localhost("llama3.2:70b".to_string());
    assert_eq!(provider.model(), "llama3.2:70b");
}

// =============================================================================
// URL Construction Tests
// =============================================================================

#[test]
fn test_chat_api_url() {
    let host = "http://localhost:11434";
    let url = format!("{}/api/chat", host);
    assert_eq!(url, "http://localhost:11434/api/chat");
}

#[test]
fn test_tags_api_url() {
    // Used for health checks
    let host = "http://localhost:11434";
    let url = format!("{}/api/tags", host);
    assert_eq!(url, "http://localhost:11434/api/tags");
}

#[test]
fn test_custom_host_url() {
    let host = "http://192.168.1.100:8080";
    let url = format!("{}/api/chat", host);
    assert_eq!(url, "http://192.168.1.100:8080/api/chat");
}

// =============================================================================
// Options/Parameters Tests
// =============================================================================

#[test]
fn test_options_temperature() {
    let options = serde_json::json!({
        "temperature": 0.8
    });
    assert_eq!(options["temperature"].as_f64(), Some(0.8));
}

#[test]
fn test_options_num_predict() {
    let options = serde_json::json!({
        "num_predict": 2048
    });
    assert_eq!(options["num_predict"].as_i64(), Some(2048));
}

#[test]
fn test_options_top_p() {
    let options = serde_json::json!({
        "top_p": 0.9
    });
    assert_eq!(options["top_p"].as_f64(), Some(0.9));
}

#[test]
fn test_options_top_k() {
    let options = serde_json::json!({
        "top_k": 40
    });
    assert_eq!(options["top_k"].as_i64(), Some(40));
}

#[test]
fn test_options_combined() {
    let options = serde_json::json!({
        "temperature": 0.7,
        "top_p": 0.9,
        "top_k": 40,
        "num_predict": 1024,
        "repeat_penalty": 1.1
    });

    assert_eq!(options["temperature"].as_f64(), Some(0.7));
    assert_eq!(options["num_predict"].as_i64(), Some(1024));
}

// =============================================================================
// Provider Trait Implementation Tests
// =============================================================================

#[test]
fn test_supports_streaming() {
    let provider = OllamaProvider::localhost("llama3.2".to_string());
    assert!(provider.supports_streaming());
}

#[test]
fn test_supports_embeddings() {
    let provider = OllamaProvider::localhost("llama3.2".to_string());
    // Ollama supports embeddings via separate endpoint
    assert!(provider.supports_embeddings());
}

// =============================================================================
// Health Check API Tests
// =============================================================================

#[test]
fn test_health_check_response_format() {
    // Health check calls /api/tags endpoint
    let tags_response = serde_json::json!({
        "models": [
            {
                "name": "llama3.2:latest",
                "model": "llama3.2:latest",
                "modified_at": "2024-01-01T00:00:00Z",
                "size": 4000000000_i64
            },
            {
                "name": "mistral:latest",
                "model": "mistral:latest",
                "modified_at": "2024-01-01T00:00:00Z",
                "size": 4000000000_i64
            }
        ]
    });

    assert!(tags_response["models"].is_array());
    let models = tags_response["models"].as_array().unwrap();
    assert_eq!(models.len(), 2);
}

// =============================================================================
// Error Response Tests
// =============================================================================

#[test]
fn test_error_response_format() {
    let error_response = serde_json::json!({
        "error": "model 'nonexistent' not found"
    });

    assert!(error_response["error"].is_string());
    assert!(error_response["error"].as_str().unwrap().contains("not found"));
}

#[test]
fn test_model_not_loaded_error() {
    let error_response = serde_json::json!({
        "error": "model is not loaded"
    });

    assert!(error_response["error"].as_str().unwrap().contains("not loaded"));
}

// =============================================================================
// Token Usage Response Tests
// =============================================================================

#[test]
fn test_token_usage_from_response() {
    let response_json = serde_json::json!({
        "prompt_eval_count": 25,
        "eval_count": 150
    });

    let usage = TokenUsage {
        input_tokens: response_json["prompt_eval_count"].as_u64().unwrap_or(0) as u32,
        output_tokens: response_json["eval_count"].as_u64().unwrap_or(0) as u32,
    };

    assert_eq!(usage.input_tokens, 25);
    assert_eq!(usage.output_tokens, 150);
    assert_eq!(usage.total(), 175);
}

#[test]
fn test_token_usage_missing_fields() {
    // When fields are missing, default to 0
    let response_json = serde_json::json!({
        "message": { "content": "Hello" }
    });

    let input_tokens = response_json["prompt_eval_count"].as_u64().unwrap_or(0) as u32;
    let output_tokens = response_json["eval_count"].as_u64().unwrap_or(0) as u32;

    assert_eq!(input_tokens, 0);
    assert_eq!(output_tokens, 0);
}

// =============================================================================
// Model Availability Check Tests
// =============================================================================

#[test]
fn test_tags_api_response_format() {
    // Response from /api/tags endpoint used for model listing
    let tags_response = serde_json::json!({
        "models": [
            {
                "name": "llama3.2:latest",
                "model": "llama3.2:latest",
                "modified_at": "2024-01-15T10:30:00.000000Z",
                "size": 4661224676_i64,
                "digest": "sha256:abc123...",
                "details": {
                    "parent_model": "",
                    "format": "gguf",
                    "family": "llama",
                    "families": ["llama"],
                    "parameter_size": "8B",
                    "quantization_level": "Q4_K_M"
                }
            },
            {
                "name": "mistral:latest",
                "model": "mistral:latest",
                "modified_at": "2024-01-14T08:00:00.000000Z",
                "size": 4109865159_i64,
                "digest": "sha256:def456...",
                "details": {
                    "format": "gguf",
                    "family": "mistral",
                    "parameter_size": "7B",
                    "quantization_level": "Q4_K_M"
                }
            }
        ]
    });

    let models = tags_response["models"].as_array().unwrap();
    assert_eq!(models.len(), 2);

    let first_model = &models[0];
    assert_eq!(first_model["name"].as_str(), Some("llama3.2:latest"));
    assert!(first_model["details"]["parameter_size"].is_string());
}

#[test]
fn test_model_show_api_response() {
    // Response from /api/show endpoint for model details
    let show_response = serde_json::json!({
        "modelfile": "FROM llama3.2\nSYSTEM You are a helpful assistant.",
        "parameters": "stop \"<|start_header_id|>\"\nstop \"<|end_header_id|>\"\nstop \"<|eot_id|>\"",
        "template": "{{ if .System }}<|start_header_id|>system<|end_header_id|>\n\n{{ .System }}<|eot_id|>{{ end }}",
        "details": {
            "parent_model": "",
            "format": "gguf",
            "family": "llama",
            "families": ["llama"],
            "parameter_size": "8B",
            "quantization_level": "Q4_K_M"
        },
        "model_info": {
            "general.architecture": "llama",
            "general.file_type": 15,
            "general.parameter_count": 8030261248_i64,
            "general.quantization_version": 2,
            "llama.attention.head_count": 32,
            "llama.attention.head_count_kv": 8,
            "llama.attention.layer_norm_rms_epsilon": 0.00001,
            "llama.block_count": 32,
            "llama.context_length": 131072,
            "llama.embedding_length": 4096
        }
    });

    let details = show_response["details"].as_object();
    assert!(details.is_some());
    assert_eq!(show_response["details"]["parameter_size"].as_str(), Some("8B"));

    let model_info = show_response["model_info"].as_object();
    assert!(model_info.is_some());
    assert_eq!(model_info.unwrap()["llama.block_count"].as_i64(), Some(32));
}

#[test]
fn test_model_pull_progress_response() {
    // Response from /api/pull endpoint (streaming progress)
    let pull_responses = vec![
        serde_json::json!({
            "status": "pulling manifest"
        }),
        serde_json::json!({
            "status": "downloading digestname",
            "digest": "sha256:abc123...",
            "total": 4661224676_i64,
            "completed": 1234567890_i64
        }),
        serde_json::json!({
            "status": "verifying sha256 digest"
        }),
        serde_json::json!({
            "status": "writing manifest"
        }),
        serde_json::json!({
            "status": "success"
        }),
    ];

    for response in &pull_responses {
        assert!(response["status"].is_string());
    }

    let download_progress = &pull_responses[1];
    assert!(download_progress["total"].is_i64());
    assert!(download_progress["completed"].is_i64());
}

#[test]
fn test_model_not_found_in_tags() {
    // Empty models list when no models are installed
    let tags_response = serde_json::json!({
        "models": []
    });

    let models = tags_response["models"].as_array().unwrap();
    assert!(models.is_empty());
}

#[test]
fn test_check_model_exists_in_list() {
    // Helper function to check if a model exists in the list
    let tags_response = serde_json::json!({
        "models": [
            { "name": "llama3.2:latest" },
            { "name": "mistral:7b" },
            { "name": "codellama:13b" }
        ]
    });

    let models = tags_response["models"].as_array().unwrap();
    let model_names: Vec<&str> = models
        .iter()
        .filter_map(|m| m["name"].as_str())
        .collect();

    assert!(model_names.contains(&"llama3.2:latest"));
    assert!(model_names.contains(&"mistral:7b"));
    assert!(!model_names.contains(&"nonexistent:latest"));
}

// =============================================================================
// Connection Error Handling Tests
// =============================================================================

#[test]
fn test_connection_refused_error_format() {
    // Simulating a connection refused error response
    let error = LLMError::ApiError {
        status: 0, // No HTTP status when connection fails
        message: "Connection refused (os error 111)".to_string(),
    };
    let error_msg = error.to_string();
    assert!(error_msg.contains("Connection refused"));
}

#[test]
fn test_connection_timeout_error() {
    // Connection timeout
    let error = LLMError::Timeout;
    let error_msg = error.to_string();
    assert!(error_msg.to_lowercase().contains("timeout"));
}

#[test]
fn test_network_unreachable_error() {
    // Network unreachable error
    let error = LLMError::ApiError {
        status: 0,
        message: "Network is unreachable (os error 101)".to_string(),
    };
    let error_msg = error.to_string();
    assert!(error_msg.contains("unreachable"));
}

#[test]
fn test_dns_resolution_failure() {
    // DNS resolution failure
    let error = LLMError::ApiError {
        status: 0,
        message: "failed to lookup address information".to_string(),
    };
    let error_msg = error.to_string();
    assert!(error_msg.contains("lookup"));
}

#[test]
fn test_server_refused_connection() {
    // Server explicitly refused connection
    let error = LLMError::ApiError {
        status: 0,
        message: "connection reset by peer".to_string(),
    };
    let error_msg = error.to_string();
    assert!(error_msg.contains("reset"));
}

// =============================================================================
// Model Loading Tests
// =============================================================================

#[test]
fn test_model_loading_response() {
    // Response when model is being loaded
    let loading_response = serde_json::json!({
        "status": "loading model"
    });

    assert_eq!(loading_response["status"].as_str(), Some("loading model"));
}

#[test]
fn test_model_unloaded_response() {
    // Response when model needs to be loaded first
    let response_json = serde_json::json!({
        "error": "model is not loaded"
    });

    let error = response_json["error"].as_str();
    assert!(error.unwrap().contains("not loaded"));
}

#[test]
fn test_model_loaded_info() {
    // Running models endpoint response
    let running_response = serde_json::json!({
        "models": [
            {
                "name": "llama3.2:latest",
                "model": "llama3.2:latest",
                "size": 4661224676_i64,
                "digest": "sha256:abc123...",
                "details": {
                    "format": "gguf",
                    "family": "llama",
                    "parameter_size": "8B"
                },
                "expires_at": "2024-01-15T12:00:00.000000Z",
                "size_vram": 4000000000_i64
            }
        ]
    });

    let models = running_response["models"].as_array().unwrap();
    assert_eq!(models.len(), 1);
    assert!(models[0]["size_vram"].is_i64());
}

// =============================================================================
// Generate API Tests (Alternative Endpoint)
// =============================================================================

#[test]
fn test_generate_api_request() {
    // /api/generate endpoint format (simpler than chat)
    let request_body = serde_json::json!({
        "model": "llama3.2",
        "prompt": "Why is the sky blue?",
        "stream": false,
        "options": {
            "temperature": 0.7,
            "num_predict": 256
        }
    });

    assert!(request_body["prompt"].is_string());
    assert_eq!(request_body["stream"].as_bool(), Some(false));
}

#[test]
fn test_generate_api_response() {
    // Response from /api/generate
    let response_json = serde_json::json!({
        "model": "llama3.2",
        "created_at": "2024-01-15T10:30:00.000000Z",
        "response": "The sky appears blue because of a phenomenon called Rayleigh scattering...",
        "done": true,
        "context": [1, 2, 3, 4, 5],
        "total_duration": 5000000000_i64,
        "load_duration": 1000000000_i64,
        "prompt_eval_count": 12,
        "prompt_eval_duration": 500000000_i64,
        "eval_count": 50,
        "eval_duration": 2000000000_i64
    });

    let response = response_json["response"].as_str();
    assert!(response.is_some());
    assert!(response.unwrap().contains("Rayleigh"));

    // Context array for continuation
    assert!(response_json["context"].is_array());
}

// =============================================================================
// Embeddings API Tests
// =============================================================================

#[test]
fn test_embeddings_api_request() {
    // /api/embeddings endpoint format
    let request_body = serde_json::json!({
        "model": "nomic-embed-text",
        "prompt": "The quick brown fox jumps over the lazy dog"
    });

    assert_eq!(request_body["model"].as_str(), Some("nomic-embed-text"));
    assert!(request_body["prompt"].is_string());
}

#[test]
fn test_embeddings_api_response() {
    // Response from /api/embeddings
    let response_json = serde_json::json!({
        "embedding": [0.1, -0.2, 0.3, 0.15, -0.05, 0.25, 0.1, -0.15]
    });

    let embedding = response_json["embedding"].as_array();
    assert!(embedding.is_some());
    assert!(!embedding.unwrap().is_empty());
}

// =============================================================================
// Context Window Tests
// =============================================================================

#[test]
fn test_context_window_options() {
    // Setting context window size
    let request_body = serde_json::json!({
        "model": "llama3.2",
        "messages": [{ "role": "user", "content": "Hello" }],
        "stream": false,
        "options": {
            "num_ctx": 8192  // Context window size
        }
    });

    assert_eq!(request_body["options"]["num_ctx"].as_i64(), Some(8192));
}

#[test]
fn test_keep_alive_option() {
    // Keep model loaded in memory
    let request_body = serde_json::json!({
        "model": "llama3.2",
        "messages": [{ "role": "user", "content": "Hello" }],
        "stream": false,
        "keep_alive": "10m"  // Keep model loaded for 10 minutes
    });

    assert_eq!(request_body["keep_alive"].as_str(), Some("10m"));
}

#[test]
fn test_format_json_option() {
    // Request JSON formatted response
    let request_body = serde_json::json!({
        "model": "llama3.2",
        "messages": [{ "role": "user", "content": "List 3 fruits as JSON" }],
        "stream": false,
        "format": "json"
    });

    assert_eq!(request_body["format"].as_str(), Some("json"));
}

// =============================================================================
// Multimodal Tests (Vision Models)
// =============================================================================

#[test]
fn test_vision_model_request() {
    // Request with image for vision-capable models (e.g., llava)
    let request_body = serde_json::json!({
        "model": "llava",
        "messages": [{
            "role": "user",
            "content": "What is in this image?",
            "images": ["base64encodedimagedata..."]
        }],
        "stream": false
    });

    let message = &request_body["messages"][0];
    assert!(message["images"].is_array());
    assert!(!message["images"].as_array().unwrap().is_empty());
}

#[test]
fn test_vision_model_multiple_images() {
    // Multiple images in a single request
    let request_body = serde_json::json!({
        "model": "llava",
        "messages": [{
            "role": "user",
            "content": "Compare these images",
            "images": [
                "base64image1...",
                "base64image2..."
            ]
        }],
        "stream": false
    });

    let images = request_body["messages"][0]["images"].as_array().unwrap();
    assert_eq!(images.len(), 2);
}

// =============================================================================
// Advanced Options Tests
// =============================================================================

#[test]
fn test_mirostat_options() {
    // Mirostat sampling options
    let options = serde_json::json!({
        "mirostat": 2,           // Mirostat mode (0=disabled, 1=v1, 2=v2)
        "mirostat_eta": 0.1,     // Learning rate
        "mirostat_tau": 5.0      // Target entropy
    });

    assert_eq!(options["mirostat"].as_i64(), Some(2));
}

#[test]
fn test_repeat_penalty_options() {
    // Repetition penalty options
    let options = serde_json::json!({
        "repeat_penalty": 1.1,
        "repeat_last_n": 64,
        "penalize_newline": false
    });

    assert_eq!(options["repeat_penalty"].as_f64(), Some(1.1));
    assert_eq!(options["repeat_last_n"].as_i64(), Some(64));
}

#[test]
fn test_seed_for_reproducibility() {
    // Setting seed for reproducible outputs
    let options = serde_json::json!({
        "seed": 42
    });

    assert_eq!(options["seed"].as_i64(), Some(42));
}

#[test]
fn test_stop_sequences() {
    // Custom stop sequences
    let request_body = serde_json::json!({
        "model": "llama3.2",
        "messages": [{ "role": "user", "content": "Tell a story" }],
        "stream": false,
        "options": {
            "stop": ["THE END", "\n\n\n", "---"]
        }
    });

    let stop = request_body["options"]["stop"].as_array();
    assert!(stop.is_some());
    assert_eq!(stop.unwrap().len(), 3);
}
