//! GitHub Copilot API client.
//!
//! This module provides a comprehensive client for interacting with the GitHub Copilot API,
//! including OAuth device code flow authentication, chat completions, embeddings, and more.
//!
//! # Features
//!
//! - **Device Code OAuth**: Authenticate without browser redirects
//! - **Automatic Token Refresh**: Proactive token management
//! - **Chat Completions**: Streaming and non-streaming
//! - **Embeddings**: Text embedding generation
//! - **Model Listing**: Query available models
//! - **Format Conversion**: Transform between OpenAI/Anthropic formats
//!
//! # Authentication Flow
//!
//! ```no_run
//! use crate::oauth::copilot::{CopilotClient, PollResult};
//! use tokio::time::{sleep, Duration};
//!
//! # async fn example() -> crate::oauth::copilot::Result<()> {
//! let client = CopilotClient::builder().build()?;
//!
//! // 1. Start device flow
//! let pending = client.start_device_flow().await?;
//! println!("Visit: {}", pending.verification_uri);
//! println!("Enter code: {}", pending.user_code);
//!
//! // 2. Poll for completion
//! loop {
//!     sleep(Duration::from_secs(pending.interval)).await;
//!
//!     match client.poll_for_token(&pending).await? {
//!         PollResult::Pending => continue,
//!         PollResult::SlowDown => sleep(Duration::from_secs(5)).await,
//!         PollResult::Complete(github_token) => {
//!             // 3. Complete authentication
//!             client.complete_auth(github_token).await?;
//!             break;
//!         }
//!     }
//! }
//!
//! // 4. Make API calls
//! let response = client
//!     .chat()
//!     .user("Hello!")
//!     .send()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Chat Completions
//!
//! ```no_run
//! use crate::oauth::copilot::CopilotClient;
//!
//! # async fn example() -> crate::oauth::copilot::Result<()> {
//! let client = CopilotClient::builder().build()?;
//!
//! // Non-streaming
//! let response = client
//!     .chat()
//!     .model("gpt-4o")
//!     .system("You are a helpful assistant")
//!     .user("What is 2+2?")
//!     .max_tokens(100)
//!     .send()
//!     .await?;
//!
//! println!("{}", response.first_content().unwrap_or_default());
//! # Ok(())
//! # }
//! ```
//!
//! # Streaming
//!
//! ```no_run
//! use crate::oauth::copilot::{CopilotClient, StreamChunk};
//! use futures_util::StreamExt;
//!
//! # async fn example() -> crate::oauth::copilot::Result<()> {
//! let client = CopilotClient::builder().build()?;
//!
//! let mut stream = client
//!     .chat()
//!     .user("Tell me a story")
//!     .send_stream()
//!     .await?;
//!
//! while let Some(chunk) = stream.next().await {
//!     match chunk? {
//!         StreamChunk::Delta { content, .. } => print!("{content}"),
//!         StreamChunk::FinishReason { reason, .. } => println!("\n[{reason}]"),
//!         StreamChunk::Done => break,
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```

// Submodules
pub mod api;
pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod models;
pub mod storage;
pub mod transform;

// Re-export commonly used types at module root

// Client
pub use client::{CopilotClient, CopilotClientBuilder};

// Config
pub use config::CopilotConfig;

// Error
pub use error::{Error, Result};

// Auth
pub use auth::{
    mask_token, DeviceFlowPending, PollResult, TokenExchangeConfig,
};

// Models
pub use models::{
    // Auth
    CopilotTokenResponse, TokenInfo,
    // Chat
    ChatRequest, ChatResponse, Choice, Content, ContentPart, ImageDetail, ImageUrl, Message, Role,
    Usage,
    // Embeddings
    EmbeddingInput, EmbeddingRequest, EmbeddingResponse, EncodingFormat,
    // Models
    ModelCapabilities, ModelInfo, ModelLimits, ModelsResponse,
    // Streaming
    SseParser, StreamChunk, StreamData,
};

// Storage
pub use storage::{CopilotTokenStorage, GateStorageAdapter, MemoryTokenStorage, COPILOT_PROVIDER_ID};

// API
pub use api::{ChatRequestBuilder, EmbeddingsRequestBuilder, QuotaInfo, UsageResponse};

// Transform
pub use transform::{
    anthropic_to_copilot, copilot_to_anthropic, copilot_to_openai, openai_to_copilot,
    stream_to_anthropic, stream_to_openai, AnthropicMessagesRequest, AnthropicMessagesResponse,
    AnthropicStreamEvent, AnthropicStreamState, OpenAIChatRequest, OpenAIChatResponse,
    OpenAIStreamChunk,
};
