//! Mock implementations for testing
//!
//! This module provides mock traits and implementations using mockall
//! for testing the TTRPG Assistant components in isolation.

#![allow(dead_code)]

use async_trait::async_trait;
use mockall::automock;
use std::collections::HashMap;

// ============================================================================
// LLM Client Mock
// ============================================================================

/// Result type for LLM operations
pub type LlmResult<T> = Result<T, LlmError>;

/// Error type for LLM operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum LlmError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Rate limited")]
    RateLimited,
    #[error("Timeout")]
    Timeout,
    #[error("No providers available")]
    NoProviders,
}

/// Message role in a conversation
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MockMessageRole {
    System,
    User,
    Assistant,
}

/// A chat message for mock LLM
#[derive(Debug, Clone)]
pub struct MockChatMessage {
    pub role: MockMessageRole,
    pub content: String,
}

/// Response from mock LLM
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MockChatResponse {
    pub content: String,
    pub model: String,
    pub provider: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Trait for LLM client operations - mockable for testing
#[automock]
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Get the provider ID
    fn id(&self) -> String;

    /// Get the model name
    fn model(&self) -> String;

    /// Check if the LLM is healthy
    async fn health_check(&self) -> bool;

    /// Send a chat completion request
    async fn chat(&self, messages: Vec<MockChatMessage>, max_tokens: Option<u32>) -> LlmResult<MockChatResponse>;

    /// Check if streaming is supported
    fn supports_streaming(&self) -> bool;
}

// ============================================================================
// Voice Provider Mock
// ============================================================================

/// Result type for voice operations
pub type VoiceResult<T> = Result<T, VoiceError>;

/// Error type for voice operations
#[derive(Debug, Clone, thiserror::Error)]
#[allow(dead_code)]
pub enum VoiceError {
    #[error("Provider not configured: {0}")]
    NotConfigured(String),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Invalid voice ID: {0}")]
    InvalidVoiceId(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Quota exceeded")]
    QuotaExceeded,
}

/// Voice information for mock provider
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MockVoice {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub description: Option<String>,
}

/// Voice settings for synthesis
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct MockVoiceSettings {
    pub stability: f32,
    pub similarity_boost: f32,
}

/// Synthesis request for mock provider
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MockSynthesisRequest {
    pub text: String,
    pub voice_id: String,
    pub settings: Option<MockVoiceSettings>,
}

/// Usage info from voice provider
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct MockUsageInfo {
    pub characters_used: u64,
    pub characters_limit: u64,
}

/// Trait for voice provider operations - mockable for testing
#[automock]
#[async_trait]
pub trait VoiceProvider: Send + Sync {
    /// Get the provider ID
    fn id(&self) -> String;

    /// Synthesize speech from text, returns audio bytes
    async fn synthesize(&self, request: MockSynthesisRequest) -> VoiceResult<Vec<u8>>;

    /// List available voices from this provider
    async fn list_voices(&self) -> VoiceResult<Vec<MockVoice>>;

    /// Check usage quotas
    async fn check_usage(&self) -> VoiceResult<MockUsageInfo>;
}

// ============================================================================
// Search Client Mock
// ============================================================================

/// Result type for search operations
pub type SearchResult<T> = Result<T, SearchError>;

/// Error type for search operations
#[derive(Debug, Clone, thiserror::Error)]
#[allow(dead_code)]
pub enum SearchError {
    #[error("Search error: {0}")]
    SearchError(String),
    #[error("Index not found: {0}")]
    IndexNotFound(String),
    #[error("Document not found: {0}")]
    DocumentNotFound(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// A searchable document for mock client
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MockSearchDocument {
    pub id: String,
    pub content: String,
    pub source: String,
    pub source_type: String,
    pub metadata: HashMap<String, String>,
}

/// A search hit from mock client

#[derive(Debug, Clone)]
pub struct MockSearchHit {
    pub document: MockSearchDocument,
    pub score: f32,
    pub index: String,
}

/// Federated search results from mock client

#[derive(Debug, Clone)]
pub struct MockFederatedResults {
    pub results: Vec<MockSearchHit>,
    pub total_hits: usize,
    pub processing_time_ms: u64,
}

/// Trait for search client operations - mockable for testing
#[automock]
#[async_trait]
pub trait SearchClient: Send + Sync {
    /// Check if search is healthy
    async fn health_check(&self) -> bool;

    /// Wait for search to become healthy
    async fn wait_for_health(&self, timeout_secs: u64) -> bool;

    /// Add a document to an index
    async fn add_document(&self, index: String, document: MockSearchDocument) -> SearchResult<()>;

    /// Add multiple documents to an index
    async fn add_documents(&self, index: String, documents: Vec<MockSearchDocument>) -> SearchResult<()>;

    /// Search a single index
    async fn search(&self, index: String, query: String, limit: usize) -> SearchResult<Vec<MockSearchHit>>;

    /// Search across multiple indexes
    async fn federated_search(&self, query: String, indexes: Vec<String>, limit: usize) -> SearchResult<MockFederatedResults>;

    /// Delete a document by ID
    async fn delete_document(&self, index: String, document_id: String) -> SearchResult<()>;

    /// Clear all documents from an index
    async fn clear_index(&self, index: String) -> SearchResult<()>;
}

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a mock LLM client with default successful responses
pub fn create_mock_llm_client() -> MockLlmClient {
    let mut mock = MockLlmClient::new();

    mock.expect_id()
        .returning(|| "mock-llm".to_string());

    mock.expect_model()
        .returning(|| "mock-model-v1".to_string());

    mock.expect_health_check()
        .returning(|| true);

    mock.expect_supports_streaming()
        .returning(|| true);

    mock.expect_chat()
        .returning(|messages, _max_tokens| {
            let user_content = messages.iter()
                .filter(|m| m.role == MockMessageRole::User)
                .map(|m| m.content.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            Ok(MockChatResponse {
                content: format!("Mock response to: {}", user_content),
                model: "mock-model-v1".to_string(),
                provider: "mock-llm".to_string(),
                input_tokens: 10,
                output_tokens: 20,
            })
        });

    mock
}

/// Create a mock voice provider with default successful responses
pub fn create_mock_voice_provider() -> MockVoiceProvider {
    let mut mock = MockVoiceProvider::new();

    mock.expect_id()
        .returning(|| "mock-voice".to_string());

    mock.expect_synthesize()
        .returning(|_request| {
            // Return empty audio bytes for testing
            Ok(vec![0u8; 1024])
        });

    mock.expect_list_voices()
        .returning(|| {
            Ok(vec![
                MockVoice {
                    id: "voice-1".to_string(),
                    name: "Test Voice 1".to_string(),
                    provider: "mock-voice".to_string(),
                    description: Some("A test voice".to_string()),
                },
                MockVoice {
                    id: "voice-2".to_string(),
                    name: "Test Voice 2".to_string(),
                    provider: "mock-voice".to_string(),
                    description: None,
                },
            ])
        });

    mock.expect_check_usage()
        .returning(|| {
            Ok(MockUsageInfo {
                characters_used: 1000,
                characters_limit: 10000,
            })
        });

    mock
}

/// Create a mock search client with default successful responses
pub fn create_mock_search_client() -> MockSearchClient {
    let mut mock = MockSearchClient::new();

    mock.expect_health_check()
        .returning(|| true);

    mock.expect_wait_for_health()
        .returning(|_timeout| true);

    mock.expect_add_document()
        .returning(|_index, _doc| Ok(()));

    mock.expect_add_documents()
        .returning(|_index, _docs| Ok(()));

    mock.expect_search()
        .returning(|_index, _query, _limit| Ok(vec![]));

    mock.expect_federated_search()
        .returning(|_query, _indexes, _limit| {
            Ok(MockFederatedResults {
                results: vec![],
                total_hits: 0,
                processing_time_ms: 5,
            })
        });

    mock.expect_delete_document()
        .returning(|_index, _id| Ok(()));

    mock.expect_clear_index()
        .returning(|_index| Ok(()));

    mock
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_llm_client_creation() {
        let mock = create_mock_llm_client();

        assert_eq!(mock.id(), "mock-llm");
        assert_eq!(mock.model(), "mock-model-v1");
        assert!(mock.supports_streaming());
        assert!(mock.health_check().await);
    }

    #[tokio::test]
    async fn test_mock_llm_client_chat() {
        let mock = create_mock_llm_client();

        let messages = vec![
            MockChatMessage {
                role: MockMessageRole::User,
                content: "Hello".to_string(),
            },
        ];

        let response = mock.chat(messages, None).await.unwrap();
        assert!(response.content.contains("Hello"));
        assert_eq!(response.provider, "mock-llm");
    }

    #[tokio::test]
    async fn test_mock_voice_provider_creation() {
        let mock = create_mock_voice_provider();

        assert_eq!(mock.id(), "mock-voice");

        let voices = mock.list_voices().await.unwrap();
        assert_eq!(voices.len(), 2);
        assert_eq!(voices[0].id, "voice-1");
    }

    #[tokio::test]
    async fn test_mock_voice_provider_synthesize() {
        let mock = create_mock_voice_provider();

        let request = MockSynthesisRequest {
            text: "Hello world".to_string(),
            voice_id: "voice-1".to_string(),
            settings: None,
        };

        let audio = mock.synthesize(request).await.unwrap();
        assert!(!audio.is_empty());
    }

    #[tokio::test]
    async fn test_mock_search_client_creation() {
        let mock = create_mock_search_client();

        assert!(mock.health_check().await);
        assert!(mock.wait_for_health(10).await);
    }

    #[tokio::test]
    async fn test_mock_search_client_operations() {
        let mock = create_mock_search_client();

        let doc = MockSearchDocument {
            id: "doc-1".to_string(),
            content: "Test content".to_string(),
            source: "test.txt".to_string(),
            source_type: "document".to_string(),
            metadata: HashMap::new(),
        };

        mock.add_document("documents".to_string(), doc).await.unwrap();

        let results = mock.search("documents".to_string(), "test".to_string(), 10).await.unwrap();
        assert!(results.is_empty()); // Default mock returns empty
    }
}
