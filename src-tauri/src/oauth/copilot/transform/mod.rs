//! Format transformation modules.
//!
//! This module provides bidirectional transformations between various
//! LLM API formats and Copilot's native format.
//!
//! - [`openai`] - OpenAI Chat Completions API format
//! - [`anthropic`] - Anthropic Messages API format

pub mod anthropic;
pub mod openai;

// Re-export commonly used transformation functions
pub use anthropic::{
    request_to_copilot as anthropic_to_copilot, response_from_copilot as copilot_to_anthropic,
    stream_events_from_copilot as stream_to_anthropic, AnthropicMessagesRequest,
    AnthropicMessagesResponse, AnthropicStreamEvent, AnthropicStreamState,
};
pub use openai::{
    request_to_copilot as openai_to_copilot, response_from_copilot as copilot_to_openai,
    stream_chunk_from_copilot as stream_to_openai, stream_data_to_openai, OpenAIChatRequest,
    OpenAIChatResponse, OpenAIStreamChunk,
};
