//! Chat Provider Integration Tests
//!
//! Tests for Meilisearch chat provider configuration and operation.
//! These tests verify that OpenAI, Claude, and Grok (xAI) providers
//! can be properly configured with Meilisearch.
//!
//! # Environment Variables Required
//!
//! - `OPENAI_API_KEY`: OpenAI API key
//! - `ANTHROPIC_API_KEY`: Anthropic/Claude API key
//! - `GROK_API_KEY`: xAI/Grok API key
//!
//! # Running Tests
//!
//! ```bash
//! # Set environment variables first
//! export OPENAI_API_KEY="sk-..."
//! export ANTHROPIC_API_KEY="sk-ant-..."
//! export GROK_API_KEY="xai-..."
//!
//! # Run chat provider tests
//! cargo test chat_provider_integration -- --ignored --nocapture
//! ```

use crate::core::meilisearch_chat::{
    ChatProviderConfig, list_chat_providers, GROK_DEFAULT_MODEL, GROK_API_BASE_URL,
};
use crate::core::llm::providers::ProviderConfig;

// =============================================================================
// Test Configuration
// =============================================================================

const MEILISEARCH_HOST: &str = "http://127.0.0.1:7700";

/// Get API key from environment, returning None if not set
fn get_env_key(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.is_empty())
}

// =============================================================================
// ChatProviderConfig Unit Tests
// =============================================================================

#[test]
fn test_openai_provider_config() {
    let config = ChatProviderConfig::OpenAI {
        api_key: "test-key".to_string(),
        model: Some("gpt-4o".to_string()),
        organization_id: None,
    };

    assert_eq!(config.provider_id(), "openai");
    assert!(!config.requires_proxy(), "OpenAI should be native (no proxy)");
    assert_eq!(config.proxy_model_id(), "openai:gpt-4o");

    // Check to_meilisearch_settings
    let settings = config.to_meilisearch_settings("http://localhost:18787");
    assert!(settings.api_key.is_some());
    assert_eq!(settings.api_key.unwrap(), "test-key");
    assert!(settings.base_url.is_none(), "OpenAI native should not use base_url");
}

#[test]
fn test_claude_provider_config() {
    let config = ChatProviderConfig::Claude {
        api_key: "test-key".to_string(),
        model: Some("claude-sonnet-4-20250514".to_string()),
        max_tokens: Some(4096),
    };

    assert_eq!(config.provider_id(), "claude");
    assert!(config.requires_proxy(), "Claude should require proxy");
    assert_eq!(config.proxy_model_id(), "claude:claude-sonnet-4-20250514");

    // Check to_meilisearch_settings
    let settings = config.to_meilisearch_settings("http://localhost:18787");
    assert!(settings.api_key.is_none(), "Claude proxied should not pass API key directly");
    assert!(settings.base_url.is_some(), "Claude should use proxy base_url");
}

#[test]
fn test_grok_provider_config() {
    let config = ChatProviderConfig::Grok {
        api_key: "test-key".to_string(),
        model: Some("grok-3".to_string()),
    };

    assert_eq!(config.provider_id(), "grok");
    assert!(!config.requires_proxy(), "Grok should be native (OpenAI-compatible)");
    assert_eq!(config.proxy_model_id(), "grok:grok-3");

    // Check to_meilisearch_settings
    let settings = config.to_meilisearch_settings("http://localhost:18787");
    assert!(settings.api_key.is_some());
    assert_eq!(settings.api_key.unwrap(), "test-key");
    assert!(settings.base_url.is_some(), "Grok should use xAI base_url");
    assert_eq!(settings.base_url.unwrap(), GROK_API_BASE_URL);
}

#[test]
fn test_grok_default_model() {
    let config = ChatProviderConfig::Grok {
        api_key: "test-key".to_string(),
        model: None,
    };

    // Should use GROK_DEFAULT_MODEL as default in proxy_model_id
    assert_eq!(config.proxy_model_id(), format!("grok:{}", GROK_DEFAULT_MODEL));
}

#[test]
fn test_grok_default_model_in_provider_config() {
    let config = ChatProviderConfig::Grok {
        api_key: "test-key".to_string(),
        model: None,
    };

    // Should use GROK_DEFAULT_MODEL as default in to_provider_config
    match config.to_provider_config() {
        ProviderConfig::OpenAI { model, .. } => {
            assert_eq!(model, GROK_DEFAULT_MODEL);
        }
        _ => panic!("Grok should map to OpenAI ProviderConfig"),
    }
}

#[test]
fn test_grok_to_provider_config() {
    let config = ChatProviderConfig::Grok {
        api_key: "test-key".to_string(),
        model: Some("grok-3".to_string()),
    };

    let provider_config = config.to_provider_config();

    // Grok maps to OpenAI provider with xAI base_url
    match provider_config {
        ProviderConfig::OpenAI { api_key, model, base_url, .. } => {
            assert_eq!(api_key, "test-key");
            assert_eq!(model, "grok-3");
            assert_eq!(base_url, Some(GROK_API_BASE_URL.to_string()));
        }
        _ => panic!("Grok should map to OpenAI ProviderConfig"),
    }
}

#[test]
fn test_list_chat_providers_includes_grok() {
    let providers = list_chat_providers();

    let grok = providers.iter().find(|p| p.id == "grok");
    assert!(grok.is_some(), "list_chat_providers should include grok");

    let grok = grok.unwrap();
    assert_eq!(grok.name, "Grok (xAI)");
    assert!(grok.requires_api_key);
    assert!(grok.is_native, "Grok should be marked as native");
}

#[test]
fn test_core_providers_present() {
    let providers = list_chat_providers();

    // Test only core stable providers to avoid brittleness
    // as new providers are added/removed
    let core_providers = vec!["openai", "claude", "grok"];

    for id in core_providers {
        assert!(
            providers.iter().any(|p| p.id == id),
            "Missing core provider: {}",
            id
        );
    }

    // Grok should be marked as native (no proxy)
    let grok = providers.iter().find(|p| p.id == "grok").unwrap();
    assert!(grok.is_native, "Grok should be native provider");
}

// =============================================================================
// Configuration Smoke Tests (require API keys from env vars)
// =============================================================================
//
// These tests validate provider configuration wiring, not actual chat completions.
// They verify that ChatProviderConfig correctly maps to Meilisearch settings.

#[tokio::test]
#[ignore = "Requires OPENAI_API_KEY environment variable"]
async fn test_openai_configuration_smoke_test() {
    let api_key = get_env_key("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set for this test");

    let config = ChatProviderConfig::OpenAI {
        api_key,
        model: Some("gpt-4o-mini".to_string()),
        organization_id: None,
    };

    assert!(!config.requires_proxy());
    let settings = config.to_meilisearch_settings(MEILISEARCH_HOST);
    assert!(settings.api_key.is_some());

    println!("OpenAI provider configured successfully");
    println!("Settings: {:?}", settings);
}

#[tokio::test]
#[ignore = "Requires ANTHROPIC_API_KEY environment variable"]
async fn test_claude_configuration_smoke_test() {
    let api_key = get_env_key("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY must be set for this test");

    let config = ChatProviderConfig::Claude {
        api_key,
        model: Some("claude-sonnet-4-20250514".to_string()),
        max_tokens: Some(4096),
    };

    assert!(config.requires_proxy());
    let provider_config = config.to_provider_config();

    match provider_config {
        ProviderConfig::Claude { model, max_tokens, .. } => {
            assert_eq!(model, "claude-sonnet-4-20250514");
            assert_eq!(max_tokens, 4096);
        }
        _ => panic!("Expected Claude ProviderConfig"),
    }

    println!("Claude provider configured successfully");
}

#[tokio::test]
#[ignore = "Requires GROK_API_KEY environment variable"]
async fn test_grok_configuration_smoke_test() {
    let api_key = get_env_key("GROK_API_KEY")
        .expect("GROK_API_KEY must be set for this test");

    let config = ChatProviderConfig::Grok {
        api_key,
        model: Some(GROK_DEFAULT_MODEL.to_string()),
    };

    assert!(!config.requires_proxy(), "Grok should not require proxy");

    let settings = config.to_meilisearch_settings(MEILISEARCH_HOST);
    assert!(settings.api_key.is_some());
    assert_eq!(settings.base_url, Some(GROK_API_BASE_URL.to_string()));

    println!("Grok provider configured successfully");
    println!("Settings: {:?}", settings);
}

#[tokio::test]
#[ignore = "Requires all API keys"]
async fn test_all_providers_configuration_smoke_test() {
    let openai_key = get_env_key("OPENAI_API_KEY");
    let anthropic_key = get_env_key("ANTHROPIC_API_KEY");
    let grok_key = get_env_key("GROK_API_KEY");

    let mut configured = 0;

    if let Some(key) = openai_key {
        let config = ChatProviderConfig::OpenAI {
            api_key: key,
            model: Some("gpt-4o-mini".to_string()),
            organization_id: None,
        };
        let settings = config.to_meilisearch_settings(MEILISEARCH_HOST);
        println!("OpenAI configured: source={:?}", settings.source);
        configured += 1;
    }

    if let Some(key) = anthropic_key {
        let config = ChatProviderConfig::Claude {
            api_key: key,
            model: Some("claude-sonnet-4-20250514".to_string()),
            max_tokens: Some(4096),
        };
        let settings = config.to_meilisearch_settings("http://localhost:18787");
        println!("Claude configured: source={:?}, base_url={:?}", settings.source, settings.base_url);
        configured += 1;
    }

    if let Some(key) = grok_key {
        let config = ChatProviderConfig::Grok {
            api_key: key,
            model: Some(GROK_DEFAULT_MODEL.to_string()),
        };
        let settings = config.to_meilisearch_settings(MEILISEARCH_HOST);
        println!("Grok configured: source={:?}, base_url={:?}", settings.source, settings.base_url);
        configured += 1;
    }

    assert!(configured > 0, "At least one API key must be set");
    println!("Successfully configured {} providers", configured);
}
