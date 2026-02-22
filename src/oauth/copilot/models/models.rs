//! Model listing data types.
//!
//! This module contains data structures for querying available models
//! from the Copilot API.

use serde::{Deserialize, Serialize};

/// Response containing available models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    /// Object type (always "list").
    pub object: String,

    /// Available models.
    pub data: Vec<ModelInfo>,
}

impl ModelsResponse {
    /// Returns the number of models.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if there are no models.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Finds a model by ID.
    #[must_use]
    pub fn find(&self, id: &str) -> Option<&ModelInfo> {
        self.data.iter().find(|m| m.id == id)
    }

    /// Returns models that support chat.
    #[must_use]
    pub fn chat_models(&self) -> Vec<&ModelInfo> {
        self.data
            .iter()
            .filter(|m| {
                m.capabilities
                    .as_ref()
                    .map(|c| c.supports_chat())
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Returns models that support embeddings.
    #[must_use]
    pub fn embedding_models(&self) -> Vec<&ModelInfo> {
        self.data
            .iter()
            .filter(|m| m.id.contains("embedding"))
            .collect()
    }

    /// Returns all model IDs.
    #[must_use]
    pub fn model_ids(&self) -> Vec<&str> {
        self.data.iter().map(|m| m.id.as_str()).collect()
    }
}

/// Information about a single model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// The model ID.
    pub id: String,

    /// Object type (always "model").
    pub object: String,

    /// Unix timestamp when created.
    #[serde(default)]
    pub created: i64,

    /// Owner organization.
    #[serde(default)]
    pub owned_by: String,

    /// Model capabilities.
    #[serde(default)]
    pub capabilities: Option<ModelCapabilities>,

    /// Model policy information.
    #[serde(default)]
    pub model_picker_enabled: Option<bool>,

    /// Preview status.
    #[serde(default)]
    pub preview: Option<bool>,
}

impl ModelInfo {
    /// Returns the maximum context window size in tokens.
    #[must_use]
    pub fn max_context_tokens(&self) -> u32 {
        self.capabilities
            .as_ref()
            .and_then(|c| c.limits.as_ref())
            .map(|l| l.max_context_window_tokens)
            .unwrap_or(4096)
    }

    /// Returns the maximum output tokens.
    #[must_use]
    pub fn max_output_tokens(&self) -> Option<u32> {
        self.capabilities
            .as_ref()
            .and_then(|c| c.limits.as_ref())
            .map(|l| l.max_output_tokens)
    }

    /// Returns true if this model supports vision.
    #[must_use]
    pub fn supports_vision(&self) -> bool {
        self.capabilities
            .as_ref()
            .map(|c| c.supports.as_ref().map(|s| s.vision).unwrap_or(false))
            .unwrap_or(false)
    }

    /// Returns true if this model supports function calling.
    #[must_use]
    pub fn supports_tool_calls(&self) -> bool {
        self.capabilities
            .as_ref()
            .map(|c| c.supports.as_ref().map(|s| s.tool_calls).unwrap_or(false))
            .unwrap_or(false)
    }
}

/// Model capability information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Model family (e.g., "gpt-4o").
    #[serde(default)]
    pub family: String,

    /// Model type ("chat", "embeddings", etc.).
    #[serde(default, rename = "type")]
    pub model_type: String,

    /// Feature support flags.
    #[serde(default)]
    pub supports: Option<ModelSupports>,

    /// Token and context limits.
    #[serde(default)]
    pub limits: Option<ModelLimits>,
}

impl ModelCapabilities {
    /// Returns true if this is a chat model.
    #[must_use]
    pub fn supports_chat(&self) -> bool {
        self.model_type == "chat"
            || self
                .supports
                .as_ref()
                .map(|s| s.chat_completions)
                .unwrap_or(false)
    }
}

/// Feature support flags for a model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelSupports {
    /// Supports chat completions.
    #[serde(default)]
    pub chat_completions: bool,

    /// Supports tool/function calling.
    #[serde(default)]
    pub tool_calls: bool,

    /// Supports parallel tool calls.
    #[serde(default)]
    pub parallel_tool_calls: bool,

    /// Supports vision (images).
    #[serde(default)]
    pub vision: bool,
}

/// Token and context limits for a model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelLimits {
    /// Maximum context window in tokens.
    #[serde(default)]
    pub max_context_window_tokens: u32,

    /// Maximum output tokens.
    #[serde(default)]
    pub max_output_tokens: u32,

    /// Maximum prompt tokens.
    #[serde(default)]
    pub max_prompt_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_models_response_find() {
        let response = ModelsResponse {
            object: "list".to_string(),
            data: vec![
                ModelInfo {
                    id: "gpt-4o".to_string(),
                    object: "model".to_string(),
                    created: 0,
                    owned_by: "openai".to_string(),
                    capabilities: None,
                    model_picker_enabled: None,
                    preview: None,
                },
                ModelInfo {
                    id: "gpt-4".to_string(),
                    object: "model".to_string(),
                    created: 0,
                    owned_by: "openai".to_string(),
                    capabilities: None,
                    model_picker_enabled: None,
                    preview: None,
                },
            ],
        };

        assert!(response.find("gpt-4o").is_some());
        assert!(response.find("nonexistent").is_none());
        assert_eq!(response.len(), 2);
    }

    #[test]
    fn test_models_response_chat_models() {
        let response = ModelsResponse {
            object: "list".to_string(),
            data: vec![
                ModelInfo {
                    id: "gpt-4o".to_string(),
                    object: "model".to_string(),
                    created: 0,
                    owned_by: "openai".to_string(),
                    capabilities: Some(ModelCapabilities {
                        family: "gpt-4o".to_string(),
                        model_type: "chat".to_string(),
                        supports: None,
                        limits: None,
                    }),
                    model_picker_enabled: None,
                    preview: None,
                },
                ModelInfo {
                    id: "text-embedding-3-small".to_string(),
                    object: "model".to_string(),
                    created: 0,
                    owned_by: "openai".to_string(),
                    capabilities: Some(ModelCapabilities {
                        family: "".to_string(),
                        model_type: "embeddings".to_string(),
                        supports: None,
                        limits: None,
                    }),
                    model_picker_enabled: None,
                    preview: None,
                },
            ],
        };

        let chat_models = response.chat_models();
        assert_eq!(chat_models.len(), 1);
        assert_eq!(chat_models[0].id, "gpt-4o");
    }

    #[test]
    fn test_model_info_limits() {
        let model = ModelInfo {
            id: "gpt-4o".to_string(),
            object: "model".to_string(),
            created: 0,
            owned_by: "openai".to_string(),
            capabilities: Some(ModelCapabilities {
                family: "gpt-4o".to_string(),
                model_type: "chat".to_string(),
                supports: Some(ModelSupports {
                    chat_completions: true,
                    tool_calls: true,
                    parallel_tool_calls: true,
                    vision: true,
                }),
                limits: Some(ModelLimits {
                    max_context_window_tokens: 128000,
                    max_output_tokens: 4096,
                    max_prompt_tokens: 123000,
                }),
            }),
            model_picker_enabled: None,
            preview: None,
        };

        assert_eq!(model.max_context_tokens(), 128000);
        assert_eq!(model.max_output_tokens(), Some(4096));
        assert!(model.supports_vision());
        assert!(model.supports_tool_calls());
    }

    #[test]
    fn test_model_info_deserialization() {
        let json = r#"{
            "id": "gpt-4o",
            "object": "model",
            "created": 1700000000,
            "owned_by": "openai",
            "capabilities": {
                "family": "gpt-4o",
                "type": "chat",
                "supports": {
                    "chat_completions": true,
                    "tool_calls": true,
                    "vision": true
                },
                "limits": {
                    "max_context_window_tokens": 128000,
                    "max_output_tokens": 4096
                }
            }
        }"#;

        let model: ModelInfo = serde_json::from_str(json).unwrap();
        assert_eq!(model.id, "gpt-4o");
        assert_eq!(model.max_context_tokens(), 128000);
        assert!(model.supports_vision());
    }

    #[test]
    fn test_model_info_defaults() {
        let model = ModelInfo {
            id: "test".to_string(),
            object: "model".to_string(),
            created: 0,
            owned_by: String::new(),
            capabilities: None,
            model_picker_enabled: None,
            preview: None,
        };

        // Should return defaults when capabilities are None
        assert_eq!(model.max_context_tokens(), 4096);
        assert!(!model.supports_vision());
        assert!(!model.supports_tool_calls());
    }

    #[test]
    fn test_models_response_model_ids() {
        let response = ModelsResponse {
            object: "list".to_string(),
            data: vec![
                ModelInfo {
                    id: "gpt-4o".to_string(),
                    object: "model".to_string(),
                    created: 0,
                    owned_by: String::new(),
                    capabilities: None,
                    model_picker_enabled: None,
                    preview: None,
                },
                ModelInfo {
                    id: "gpt-4".to_string(),
                    object: "model".to_string(),
                    created: 0,
                    owned_by: String::new(),
                    capabilities: None,
                    model_picker_enabled: None,
                    preview: None,
                },
            ],
        };

        let ids = response.model_ids();
        assert_eq!(ids, vec!["gpt-4o", "gpt-4"]);
    }
}
