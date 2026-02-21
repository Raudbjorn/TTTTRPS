//! RAG (Retrieval-Augmented Generation) Commands Module
//!
//! Provides Tauri commands for configuring and executing RAG queries using
//! embedded Meilisearch's chat completion capabilities.
//!
//! # Overview
//!
//! This module exposes the meilisearch_lib chat API to the frontend, enabling:
//! - Configuration of LLM providers (OpenAI, Anthropic, Azure, Mistral, vLLM)
//! - Non-streaming RAG queries with source citations
//! - Streaming RAG queries with real-time chunk emission
//!
//! # Architecture
//!
//! ```text
//! Frontend                     Backend                      MeilisearchLib
//! --------                     -------                      --------------
//!    |                            |                              |
//!    |-- configure_rag() -------->|                              |
//!    |                            |-- set_chat_config() -------->|
//!    |                            |                              |
//!    |-- rag_query() ------------>|                              |
//!    |                            |-- chat_completion() -------->|
//!    |<-- RagResponsePayload -----|<-- ChatResponse -------------|
//!    |                            |                              |
//!    |-- rag_query_stream() ----->|                              |
//!    |                            |-- chat_completion_stream() ->|
//!    |<-- "rag-chunk" events -----|<-- Stream<ChatChunk> --------|
//! ```

pub mod commands;
pub mod types;

pub use commands::*;
pub use types::*;
