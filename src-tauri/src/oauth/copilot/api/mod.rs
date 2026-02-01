//! API request builders for the Copilot client.
//!
//! This module provides fluent builders for constructing and sending
//! requests to the various Copilot API endpoints:
//!
//! - [`chat`] - Chat completion requests (streaming and non-streaming)
//! - [`embeddings`] - Text embedding requests
//! - [`models`] - Available model listing
//! - [`usage`] - Usage and quota information

pub mod chat;
pub mod embeddings;
pub mod models;
pub mod usage;

// Re-export commonly used types
pub use chat::ChatRequestBuilder;
pub use embeddings::EmbeddingsRequestBuilder;
pub use usage::{QuotaInfo, QuotaSnapshots, UsageResponse};
