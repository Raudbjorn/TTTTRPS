//! Embeddings-related data models.
//!
//! This module contains data structures for text embedding requests
//! and responses.

use serde::{Deserialize, Serialize};

// =============================================================================
// Embedding Request Types
// =============================================================================

/// An embedding request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    /// The model to use for embeddings.
    pub model: String,

    /// The input text(s) to embed.
    pub input: EmbeddingInput,

    /// The encoding format for the embeddings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<EncodingFormat>,

    /// Number of dimensions for the embedding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,

    /// User identifier for abuse monitoring.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// Input text for embedding - single string or array.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    /// Single text input.
    Single(String),
    /// Multiple text inputs.
    Multiple(Vec<String>),
}

impl EmbeddingInput {
    /// Returns the number of inputs.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Single(_) => 1,
            Self::Multiple(v) => v.len(),
        }
    }

    /// Returns true if there are no inputs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Single(s) => s.is_empty(),
            Self::Multiple(v) => v.is_empty(),
        }
    }
}

/// Encoding format for embeddings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EncodingFormat {
    /// Float array (default).
    Float,
    /// Base64-encoded array.
    Base64,
}

// =============================================================================
// Embedding Response Types
// =============================================================================

/// An embedding response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// Object type (always "list").
    pub object: String,

    /// The model used.
    pub model: String,

    /// The embedding data.
    pub data: Vec<EmbeddingData>,

    /// Token usage.
    pub usage: EmbeddingUsage,
}

impl EmbeddingResponse {
    /// Returns the first embedding vector, if available.
    #[must_use]
    pub fn first_embedding(&self) -> Option<&[f32]> {
        self.data.first().map(|d| d.embedding.as_slice())
    }

    /// Returns all embeddings.
    #[must_use]
    pub fn embeddings(&self) -> Vec<&[f32]> {
        self.data.iter().map(|d| d.embedding.as_slice()).collect()
    }
}

/// An individual embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingData {
    /// Object type (always "embedding").
    pub object: String,

    /// The index of this embedding.
    pub index: u32,

    /// The embedding vector.
    pub embedding: Vec<f32>,
}

/// Token usage for embeddings.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    /// Tokens in the prompt.
    pub prompt_tokens: u32,

    /// Total tokens used.
    pub total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_input_single() {
        let input = EmbeddingInput::Single("hello".to_string());
        assert_eq!(input.len(), 1);
        assert!(!input.is_empty());
    }

    #[test]
    fn test_embedding_input_multiple() {
        let input = EmbeddingInput::Multiple(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(input.len(), 2);
        assert!(!input.is_empty());
    }

    #[test]
    fn test_embedding_input_empty() {
        let single_empty = EmbeddingInput::Single(String::new());
        assert!(single_empty.is_empty());

        let multiple_empty = EmbeddingInput::Multiple(vec![]);
        assert!(multiple_empty.is_empty());
    }

    #[test]
    fn test_embedding_request_serialization() {
        let request = EmbeddingRequest {
            model: "text-embedding-3-small".to_string(),
            input: EmbeddingInput::Single("Hello world".to_string()),
            encoding_format: Some(EncodingFormat::Float),
            dimensions: Some(256),
            user: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("text-embedding-3-small"));
        assert!(json.contains("Hello world"));
        assert!(json.contains("\"dimensions\":256"));
    }

    #[test]
    fn test_embedding_response_deserialization() {
        let json = r#"{
            "object": "list",
            "model": "text-embedding-3-small",
            "data": [
                {
                    "object": "embedding",
                    "index": 0,
                    "embedding": [0.1, 0.2, 0.3]
                }
            ],
            "usage": {
                "prompt_tokens": 5,
                "total_tokens": 5
            }
        }"#;

        let response: EmbeddingResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.model, "text-embedding-3-small");
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.first_embedding().unwrap().len(), 3);
        assert_eq!(response.usage.prompt_tokens, 5);
    }

    #[test]
    fn test_embedding_response_multiple() {
        let json = r#"{
            "object": "list",
            "model": "text-embedding-3-small",
            "data": [
                {"object": "embedding", "index": 0, "embedding": [0.1, 0.2]},
                {"object": "embedding", "index": 1, "embedding": [0.3, 0.4]}
            ],
            "usage": {"prompt_tokens": 10, "total_tokens": 10}
        }"#;

        let response: EmbeddingResponse = serde_json::from_str(json).unwrap();
        let embeddings = response.embeddings();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0], &[0.1, 0.2]);
        assert_eq!(embeddings[1], &[0.3, 0.4]);
    }

    #[test]
    fn test_encoding_format_serialization() {
        assert_eq!(
            serde_json::to_string(&EncodingFormat::Float).unwrap(),
            "\"float\""
        );
        assert_eq!(
            serde_json::to_string(&EncodingFormat::Base64).unwrap(),
            "\"base64\""
        );
    }
}
