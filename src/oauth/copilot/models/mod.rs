//! Data models for the Copilot API.
//!
//! This module contains all the data structures used for communicating
//! with the Copilot API, including:
//!
//! - [`auth`] - Authentication tokens and responses
//! - [`chat`] - Chat completion requests and responses
//! - [`embeddings`] - Text embedding types
//! - [`models`] - Available model information
//! - [`streaming`] - SSE streaming types

pub mod auth;
pub mod chat;
pub mod embeddings;
pub mod models;
pub mod streaming;

// Re-export commonly used types
pub use auth::{CopilotTokenResponse, DeviceCodeResponse, GitHubTokenResponse, TokenInfo};
pub use chat::{
    ChatRequest, ChatResponse, Choice, Content, ContentPart, ImageDetail, ImageUrl, Message, Role,
    Usage,
};
pub use embeddings::{
    EmbeddingData, EmbeddingInput, EmbeddingRequest, EmbeddingResponse, EmbeddingUsage,
    EncodingFormat,
};
pub use models::{ModelCapabilities, ModelInfo, ModelLimits, ModelSupports, ModelsResponse};
pub use streaming::{SseParser, StreamChunk, StreamChoice, StreamData, StreamDelta};
