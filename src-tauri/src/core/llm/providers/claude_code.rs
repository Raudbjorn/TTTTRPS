//! Claude Code Provider via CLI
//!
//! Provides LLM access through Claude Code CLI (claude -p).
//! This enables programmatic interaction with Claude Code, useful for
//! development/testing without API costs, using your existing authentication.
//!
//! ## Features
//!
//! - Full Claude Code capabilities (file access, tool use, etc.)
//! - Conversation management via session IDs
//! - JSON and streaming output parsing
//! - Session resumption (`--continue`, `--resume`)
//! - Conversation compaction (`/compact`)
//! - Configurable timeout and model selection
//! - Automatic rate limit fallback (try primary model, fallback on 429)
//!
//! ## Session Management
//!
//! Sessions are tracked via the `session_id` returned in responses.
//! - `--continue`: Resume the most recent session
//! - `--resume <id>`: Resume a specific session
//! - `--fork-session`: Create a new branch when resuming
//!
//! ## Usage
//!
//! ```rust,no_run
//! use crate::core::llm::providers::ClaudeCodeProvider;
//!
//! // Using builder pattern (recommended)
//! let provider = ClaudeCodeProvider::builder()
//!     .model("claude-sonnet-4-20250514")
//!     .timeout_secs(300)
//!     .fallback_model("claude-haiku-4-20250514")
//!     .build();
//!
//! // Or with simple constructors
//! let provider = ClaudeCodeProvider::new();
//! let provider = ClaudeCodeProvider::with_config(300, None, None);
//! ```

use crate::core::llm::cost::ProviderPricing;
use crate::core::llm::model_selector::model_selector;
use crate::core::llm::router::{
    ChatChunk, ChatRequest, ChatResponse, LLMError, LLMProvider, Result,
};
use crate::core::llm::session::{
    ClaudeStreamEvent, ContentBlock, ProviderSession, SessionError, SessionId, SessionInfo,
    SessionResult, SessionStore,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

// ============================================================================
// Constants
// ============================================================================

/// Default timeout in seconds for CLI operations.
const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Default model (Claude Code auto-selects based on task complexity).
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

/// Fallback model when rate limited (429) - faster, cheaper.
const RATE_LIMIT_FALLBACK_MODEL: &str = "claude-sonnet-4-20250514";

// ============================================================================
// Response Types
// ============================================================================

/// Response structure from Claude Code CLI JSON output
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaudeCodeResponse {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    result: String,
    #[serde(default)]
    usage: Option<ClaudeCodeUsage>,
    #[serde(default)]
    cost: Option<ClaudeCodeCost>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    is_error: Option<bool>,
    #[serde(default)]
    duration_ms: Option<u64>,
    #[serde(default)]
    num_turns: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaudeCodeUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaudeCodeCost {
    #[serde(default)]
    usd: f64,
}

// ============================================================================
// Session Configuration
// ============================================================================

/// Session mode for requests
#[derive(Debug, Clone, Default)]
pub enum SessionMode {
    /// Start a new session
    #[default]
    New,
    /// Continue the most recent session
    Continue,
    /// Resume a specific session by ID
    Resume(SessionId),
    /// Fork an existing session (create new branch)
    Fork(SessionId),
}

// ============================================================================
// Claude Code Provider
// ============================================================================

/// Claude Code provider using CLI.
pub struct ClaudeCodeProvider {
    timeout_secs: u64,
    model: Option<String>,
    fallback_model: Option<String>,
    working_dir: Option<String>,
    /// Session store for tracking conversations
    session_store: Arc<SessionStore>,
    /// Current active session ID
    current_session: RwLock<Option<SessionId>>,
    /// Whether to persist sessions to disk
    persist_sessions: bool,
    /// Whether to automatically fallback on rate limit errors.
    auto_fallback: bool,
}

// ============================================================================
// Builder Pattern
// ============================================================================

/// Builder for ClaudeCodeProvider.
#[derive(Debug)]
pub struct ClaudeCodeProviderBuilder {
    model: Option<String>,
    fallback_model: Option<String>,
    timeout_secs: Option<u64>,
    working_dir: Option<String>,
    persist_sessions: bool,
    auto_fallback: bool,
}

impl Default for ClaudeCodeProviderBuilder {
    fn default() -> Self {
        Self {
            model: None,
            fallback_model: Some(RATE_LIMIT_FALLBACK_MODEL.to_string()),
            timeout_secs: None,
            working_dir: None,
            persist_sessions: true,
            auto_fallback: true,
        }
    }
}

impl ClaudeCodeProviderBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the model to use.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the fallback model (used when rate limited).
    pub fn fallback_model(mut self, model: impl Into<String>) -> Self {
        self.fallback_model = Some(model.into());
        self
    }

    /// Disable fallback model (don't retry on rate limit).
    pub fn no_fallback(mut self) -> Self {
        self.fallback_model = None;
        self.auto_fallback = false;
        self
    }

    /// Enable/disable automatic fallback on rate limit errors.
    pub fn auto_fallback(mut self, enabled: bool) -> Self {
        self.auto_fallback = enabled;
        self
    }

    /// Set the timeout in seconds.
    pub fn timeout_secs(mut self, timeout: u64) -> Self {
        self.timeout_secs = Some(timeout);
        self
    }

    /// Set the working directory for the CLI.
    pub fn working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Enable/disable session persistence to disk.
    pub fn persist_sessions(mut self, enabled: bool) -> Self {
        self.persist_sessions = enabled;
        self
    }

    /// Build the provider.
    pub fn build(self) -> ClaudeCodeProvider {
        let store_path = if self.persist_sessions {
            dirs::data_local_dir()
                .map(|d| d.join("ttrpg-assistant").join("claude-sessions.json"))
        } else {
            None
        };

        let session_store = match store_path {
            Some(path) => Arc::new(SessionStore::with_persistence(path)),
            None => Arc::new(SessionStore::new()),
        };

        ClaudeCodeProvider {
            timeout_secs: self.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS),
            model: self.model,
            fallback_model: self.fallback_model,
            working_dir: self.working_dir,
            session_store,
            current_session: RwLock::new(None),
            persist_sessions: self.persist_sessions,
            auto_fallback: self.auto_fallback,
        }
    }
}

impl ClaudeCodeProvider {
    /// Create a new provider with default configuration.
    pub fn new() -> Self {
        Self::builder().build()
    }

    /// Create a new builder.
    pub fn builder() -> ClaudeCodeProviderBuilder {
        ClaudeCodeProviderBuilder::new()
    }

    /// Create a new provider with custom timeout.
    pub fn with_timeout(timeout_secs: u64) -> Self {
        Self::builder().timeout_secs(timeout_secs).build()
    }

    /// Create a new provider with custom configuration.
    pub fn with_config(
        timeout_secs: u64,
        model: Option<String>,
        working_dir: Option<String>,
    ) -> Self {
        let mut builder = Self::builder().timeout_secs(timeout_secs);
        if let Some(m) = model {
            builder = builder.model(m);
        }
        if let Some(dir) = working_dir {
            builder = builder.working_dir(dir);
        }
        builder.build()
    }

    /// Create provider with custom session store
    pub fn with_session_store(
        timeout_secs: u64,
        model: Option<String>,
        working_dir: Option<String>,
        session_store: Arc<SessionStore>,
    ) -> Self {
        Self {
            timeout_secs,
            model,
            fallback_model: Some(RATE_LIMIT_FALLBACK_MODEL.to_string()),
            working_dir,
            session_store,
            current_session: RwLock::new(None),
            persist_sessions: true,
            auto_fallback: true,
        }
    }

    /// Check if an error indicates a rate limit / quota exhaustion.
    /// Uses confirmed Anthropic rate limit indicators.
    fn is_rate_limit_error(output: &str) -> bool {
        let lower = output.to_lowercase();
        lower.contains("429")
            || lower.contains("rate limit")
            || lower.contains("too many requests")
            || lower.contains("quota")
            || lower.contains("overloaded")
            || lower.contains("capacity")
    }

    /// Build the message content from the request.
    fn build_message(&self, request: &ChatRequest) -> String {
        let mut parts = Vec::new();

        // Add system prompt if present
        if let Some(ref system) = request.system_prompt {
            parts.push(format!("[System Instructions: {}]", system));
        }

        // Add conversation context (messages)
        for msg in request.messages.iter() {
            match msg.role {
                crate::core::llm::router::MessageRole::User => {
                    parts.push(format!("User: {}", msg.content));
                }
                crate::core::llm::router::MessageRole::Assistant => {
                    parts.push(format!("Assistant: {}", msg.content));
                }
                crate::core::llm::router::MessageRole::System => {
                    parts.push(format!("[System: {}]", msg.content));
                }
            }
        }

        if parts.is_empty() {
            String::new()
        } else {
            parts.join("\n\n")
        }
    }

    /// Build CLI arguments for a request with optional model override.
    fn build_args_with_model(
        &self,
        prompt: &str,
        session_mode: &SessionMode,
        streaming: bool,
        model_override: Option<&str>,
    ) -> Vec<String> {
        let mut args = vec!["-p".to_string(), prompt.to_string()];

        // Output format
        if streaming {
            args.extend(["--output-format".to_string(), "stream-json".to_string()]);
        } else {
            args.extend(["--output-format".to_string(), "json".to_string()]);
        }

        // Model selection - use override, explicit config, or dynamic selection
        let model = model_override
            .map(String::from)
            .or_else(|| self.model.clone())
            .unwrap_or_else(|| model_selector().select_model_sync());
        args.extend(["--model".to_string(), model]);

        // Session handling
        match session_mode {
            SessionMode::New => {}
            SessionMode::Continue => {
                args.push("--continue".to_string());
            }
            SessionMode::Resume(session_id) => {
                args.extend(["--resume".to_string(), session_id.clone()]);
            }
            SessionMode::Fork(session_id) => {
                args.extend([
                    "--resume".to_string(),
                    session_id.clone(),
                    "--fork-session".to_string(),
                ]);
            }
        }

        args
    }

    /// Build CLI arguments for a request
    fn build_args(&self, prompt: &str, session_mode: &SessionMode, streaming: bool) -> Vec<String> {
        self.build_args_with_model(prompt, session_mode, streaming, None)
    }

    /// Execute a prompt via Claude Code CLI (non-streaming).
    ///
    /// If a rate limit (429) error is encountered and no explicit model was configured,
    /// automatically retries with Sonnet as a fallback.
    async fn execute_prompt(&self, prompt: &str, session_mode: SessionMode) -> Result<ClaudeCodeResponse> {
        self.execute_prompt_impl(prompt, session_mode, None).await
    }

    /// Execute a prompt with a specific model override (for fallback scenarios).
    async fn execute_prompt_with_model(
        &self,
        prompt: &str,
        session_mode: SessionMode,
        model: &str,
    ) -> Result<ClaudeCodeResponse> {
        self.execute_prompt_impl(prompt, session_mode, Some(model)).await
    }

    /// Core implementation for executing prompts via Claude Code CLI.
    /// Includes rate limit fallback logic.
    async fn execute_prompt_impl(
        &self,
        prompt: &str,
        session_mode: SessionMode,
        model_override: Option<&str>,
    ) -> Result<ClaudeCodeResponse> {
        let binary = which::which("claude").map_err(|_| {
            LLMError::NotConfigured(
                "Claude Code CLI not found. Install with: npm install -g @anthropic-ai/claude-code"
                    .to_string(),
            )
        })?;

        let args = self.build_args_with_model(prompt, &session_mode, false, model_override);

        let mut cmd = Command::new(binary);
        cmd.args(&args);

        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(model) = model_override {
            debug!(model = model, "executing Claude Code CLI with model override");
        } else {
            debug!(command = ?cmd, "executing Claude Code CLI");
        }

        let timeout = tokio::time::Duration::from_secs(self.timeout_secs);
        let mut child = cmd.spawn().map_err(|e| LLMError::ApiError {
            status: 0,
            message: format!("Failed to spawn Claude Code: {}", e),
        })?;

        let result = tokio::time::timeout(timeout, async {
            let mut stdout = String::new();
            let mut stderr = String::new();

            if let Some(mut stdout_handle) = child.stdout.take() {
                stdout_handle.read_to_string(&mut stdout).await?;
            }

            if let Some(mut stderr_handle) = child.stderr.take() {
                stderr_handle.read_to_string(&mut stderr).await?;
            }

            let status = child.wait().await?;
            Ok::<_, std::io::Error>((status, stdout, stderr))
        })
        .await;

        match result {
            Ok(Ok((status, stdout, stderr))) => {
                debug!(
                    status = %status,
                    stdout_len = stdout.len(),
                    stderr_len = stderr.len(),
                    "Claude Code process completed"
                );

                if !status.success() {
                    let error_msg = if !stderr.is_empty() {
                        stderr.clone()
                    } else if !stdout.is_empty() {
                        stdout.clone()
                    } else {
                        "unknown error".to_string()
                    };

                    // Check for rate limit and retry with Sonnet if no explicit model was set
                    // and we haven't already tried the fallback
                    if Self::is_rate_limit_error(&error_msg)
                        && model_override.is_none()
                        && self.model.is_none()
                    {
                        warn!(
                            error = %error_msg,
                            fallback_model = RATE_LIMIT_FALLBACK_MODEL,
                            "Rate limit detected, retrying with fallback model"
                        );
                        // Retry with Sonnet - use Box::pin for recursive async call
                        return Box::pin(self.execute_prompt_impl(
                            prompt,
                            session_mode,
                            Some(RATE_LIMIT_FALLBACK_MODEL),
                        ))
                        .await;
                    }

                    error!(status = %status, error = %error_msg, "Claude Code failed");
                    return Err(LLMError::ApiError {
                        status: status.code().unwrap_or(1) as u16,
                        message: error_msg,
                    });
                }

                // Parse JSON response
                let response: ClaudeCodeResponse =
                    serde_json::from_str(&stdout).unwrap_or_else(|_| ClaudeCodeResponse {
                        session_id: None,
                        result: stdout.trim().to_string(),
                        usage: None,
                        cost: None,
                        error: None,
                        is_error: None,
                        duration_ms: None,
                        num_turns: None,
                    });

                // Store session ID if returned
                if let Some(ref session_id) = response.session_id {
                    let mut current = self.current_session.write().await;
                    *current = Some(session_id.clone());

                    // Store session info
                    let mut info = SessionInfo::new(session_id.clone(), "claude-code");
                    info.working_dir = self.working_dir.clone();
                    let _ = self.session_store.store(info).await;
                }

                Ok(response)
            }
            Ok(Err(io_err)) => {
                error!(error = %io_err, "I/O error during Claude Code execution");
                Err(LLMError::ApiError {
                    status: 0,
                    message: format!("I/O error: {}", io_err),
                })
            }
            Err(_) => {
                warn!(timeout_secs = self.timeout_secs, "Claude Code request timed out");
                let _ = child.kill().await;
                Err(LLMError::Timeout)
            }
        }
    }

    /// Execute a prompt with streaming output
    async fn execute_streaming(
        &self,
        prompt: &str,
        session_mode: SessionMode,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        let binary = which::which("claude").map_err(|_| {
            LLMError::NotConfigured(
                "Claude Code CLI not found. Install with: npm install -g @anthropic-ai/claude-code"
                    .to_string(),
            )
        })?;

        let args = self.build_args(prompt, &session_mode, true);
        let (tx, rx) = mpsc::channel::<Result<ChatChunk>>(100);

        let mut cmd = Command::new(binary);
        cmd.args(&args);

        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| LLMError::ApiError {
            status: 0,
            message: format!("Failed to spawn Claude Code: {}", e),
        })?;

        let stdout = child.stdout.take().ok_or_else(|| LLMError::ApiError {
            status: 0,
            message: "Failed to capture stdout".to_string(),
        })?;

        let model = self.model.clone().unwrap_or_else(|| model_selector().select_model_sync());
        let timeout_secs = self.timeout_secs;
        let session_store = self.session_store.clone();
        let working_dir = self.working_dir.clone();

        // Spawn task to read streaming output
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let stream_id = uuid::Uuid::new_v4().to_string();
            let mut chunk_index: u32 = 0;
            let mut session_id: Option<String> = None;
            let mut total_content = String::new();

            let timeout = tokio::time::Duration::from_secs(timeout_secs);
            let start = Instant::now();

            loop {
                if start.elapsed() > timeout {
                    let _ = tx
                        .send(Err(LLMError::Timeout))
                        .await;
                    let _ = child.kill().await;
                    break;
                }

                let line_result = tokio::time::timeout(
                    tokio::time::Duration::from_secs(30),
                    lines.next_line(),
                )
                .await;

                match line_result {
                    Ok(Ok(Some(line))) => {
                        if line.is_empty() {
                            continue;
                        }

                        // Parse stream-json event
                        match serde_json::from_str::<ClaudeStreamEvent>(&line) {
                            Ok(event) => match event {
                                ClaudeStreamEvent::System { session_id: sid, .. } => {
                                    session_id = sid;
                                }
                                ClaudeStreamEvent::Assistant { message, .. } => {
                                    if let Some(msg) = message {
                                        if let Some(content_blocks) = msg.content {
                                            for block in content_blocks {
                                                if let ContentBlock::Text { text } = block {
                                                    total_content.push_str(&text);

                                                    let chunk = ChatChunk {
                                                        stream_id: stream_id.clone(),
                                                        content: text,
                                                        provider: "claude-code".to_string(),
                                                        model: model.clone(),
                                                        is_final: false,
                                                        finish_reason: None,
                                                        usage: None,
                                                        index: chunk_index,
                                                    };
                                                    chunk_index += 1;

                                                    if tx.send(Ok(chunk)).await.is_err() {
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                ClaudeStreamEvent::Result {
                                    session_id: sid,
                                    is_error,
                                    cost_usd,
                                    ..
                                } => {
                                    if sid.is_some() {
                                        session_id = sid;
                                    }

                                    // Store session
                                    if let Some(ref s_id) = session_id {
                                        let mut info = SessionInfo::new(s_id.clone(), "claude-code");
                                        info.working_dir = working_dir.clone();
                                        let _ = session_store.store(info).await;
                                    }

                                    // Send final chunk
                                    let final_chunk = ChatChunk {
                                        stream_id: stream_id.clone(),
                                        content: String::new(),
                                        provider: "claude-code".to_string(),
                                        model: model.clone(),
                                        is_final: true,
                                        finish_reason: if is_error.unwrap_or(false) {
                                            Some("error".to_string())
                                        } else {
                                            Some("stop".to_string())
                                        },
                                        usage: None,
                                        index: chunk_index,
                                    };
                                    let _ = tx.send(Ok(final_chunk)).await;
                                    break;
                                }
                                ClaudeStreamEvent::User { .. } => {
                                    // User message acknowledgment, ignore
                                }
                            },
                            Err(e) => {
                                debug!(line = %line, error = %e, "Failed to parse stream event");
                            }
                        }
                    }
                    Ok(Ok(None)) => {
                        // EOF - send final chunk if we haven't already
                        let final_chunk = ChatChunk {
                            stream_id: stream_id.clone(),
                            content: String::new(),
                            provider: "claude-code".to_string(),
                            model: model.clone(),
                            is_final: true,
                            finish_reason: Some("stop".to_string()),
                            usage: None,
                            index: chunk_index,
                        };
                        let _ = tx.send(Ok(final_chunk)).await;
                        break;
                    }
                    Ok(Err(e)) => {
                        let _ = tx
                            .send(Err(LLMError::ApiError {
                                status: 0,
                                message: format!("Read error: {}", e),
                            }))
                            .await;
                        break;
                    }
                    Err(_) => {
                        // Line read timeout - continue waiting
                        continue;
                    }
                }
            }

            // Wait for process to finish
            let _ = child.wait().await;
        });

        Ok(rx)
    }

    /// Check if Claude Code CLI is available
    pub fn is_available() -> bool {
        which::which("claude").is_ok()
    }

    /// Get Claude Code version
    pub async fn version() -> std::result::Result<String, String> {
        let binary = which::which("claude").map_err(|_| "Claude Code CLI not found".to_string())?;

        let output = Command::new(binary)
            .arg("--version")
            .output()
            .await
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err("Failed to get Claude Code version".to_string())
        }
    }

    /// Get full status of Claude Code CLI (installed, logged in, version)
    pub async fn get_status() -> ClaudeCodeStatus {
        // Check if skill is installed via plugin system
        let skill_installed = Self::check_plugin_installed("claude-code-bridge").await;

        // Check if binary exists
        let binary = match which::which("claude") {
            Ok(b) => b,
            Err(_) => {
                return ClaudeCodeStatus {
                    installed: false,
                    logged_in: false,
                    skill_installed,
                    version: None,
                    user_email: None,
                    error: Some("Claude Code CLI not installed".to_string()),
                };
            }
        };

        // Get version
        let version = match Command::new(&binary).arg("--version").output().await {
            Ok(output) if output.status.success() => {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
            _ => None,
        };

        // Check auth by attempting a minimal prompt
        let auth_result = tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            Command::new(&binary)
                .args(["-p", "hi", "--output-format", "json"])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await;

        match auth_result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() {
                    ClaudeCodeStatus {
                        installed: true,
                        logged_in: true,
                        skill_installed,
                        version,
                        user_email: None,
                        error: None,
                    }
                } else {
                    let combined = format!("{} {}", stdout, stderr);
                    let is_auth_error = combined.contains("login")
                        || combined.contains("authenticate")
                        || combined.contains("unauthorized")
                        || combined.contains("session")
                        || combined.contains("token");

                    ClaudeCodeStatus {
                        installed: true,
                        logged_in: false,
                        skill_installed,
                        version,
                        user_email: None,
                        error: if is_auth_error {
                            Some("Not logged in - run 'claude' to authenticate".to_string())
                        } else {
                            Some(stderr.trim().to_string())
                        },
                    }
                }
            }
            Ok(Err(e)) => ClaudeCodeStatus {
                installed: true,
                logged_in: false,
                skill_installed,
                version,
                user_email: None,
                error: Some(format!("Failed to check auth: {}", e)),
            },
            Err(_) => ClaudeCodeStatus {
                installed: true,
                logged_in: false,
                skill_installed,
                version,
                user_email: None,
                error: Some("Auth check timed out".to_string()),
            },
        }
    }

    /// Check if a plugin is installed using `claude plugin` command
    async fn check_plugin_installed(plugin_name: &str) -> bool {
        // For now, check the legacy skill file path
        // TODO: Use `claude plugin list` when available
        Self::skill_path().map(|p| p.exists()).unwrap_or(false)
    }

    /// Get the path to the legacy skill file
    fn skill_path() -> Option<std::path::PathBuf> {
        dirs::home_dir().map(|h| h.join(".claude").join("commands").join("claude-code-bridge.md"))
    }

    /// Spawn the Claude Code login flow
    pub async fn login() -> std::result::Result<(), String> {
        let binary = which::which("claude").map_err(|_| "Claude Code CLI not installed")?;

        #[cfg(target_os = "linux")]
        {
            let terminals = [
                ("kitty", vec!["-e"]),
                ("gnome-terminal", vec!["--"]),
                ("konsole", vec!["-e"]),
                ("xterm", vec!["-e"]),
                ("alacritty", vec!["-e"]),
            ];

            for (term, args) in terminals {
                if which::which(term).is_ok() {
                    let mut cmd = Command::new(term);
                    for arg in &args {
                        cmd.arg(arg);
                    }
                    cmd.arg(&binary);

                    if cmd.spawn().is_ok() {
                        return Ok(());
                    }
                }
            }
            Err("Could not open terminal. Please run 'claude' manually to login.".to_string())
        }

        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .args(["-a", "Terminal", binary.to_str().unwrap_or("claude")])
                .spawn()
                .map_err(|e| format!("Failed to open Terminal: {}", e))?;
            Ok(())
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .args(["/c", "start", "cmd", "/k", binary.to_str().unwrap_or("claude")])
                .spawn()
                .map_err(|e| format!("Failed to open terminal: {}", e))?;
            Ok(())
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        Err("Unsupported platform for automatic login. Please run 'claude' manually.".to_string())
    }

    /// Logout from Claude Code
    pub async fn logout() -> std::result::Result<(), String> {
        if let Some(home) = dirs::home_dir() {
            let creds_path = home.join(".claude").join("credentials.json");
            if creds_path.exists() {
                tokio::fs::remove_file(&creds_path)
                    .await
                    .map_err(|e| format!("Failed to remove credentials: {}", e))?;
                info!("Removed Claude Code credentials");
                return Ok(());
            }
        }
        Ok(())
    }

    /// Install the claude-code-bridge plugin using `claude plugin install`
    pub async fn install_skill() -> std::result::Result<(), String> {
        // First, try using the plugin system
        let binary = which::which("claude").map_err(|_| "Claude Code CLI not installed")?;

        // For now, fall back to creating the skill file manually
        // since plugin marketplace may not have the bridge skill yet
        let skill_path = Self::skill_path().ok_or("Could not determine home directory")?;

        if let Some(parent) = skill_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create commands directory: {}", e))?;
        }

        tokio::fs::write(&skill_path, CLAUDE_CODE_BRIDGE_SKILL)
            .await
            .map_err(|e| format!("Failed to write skill file: {}", e))?;

        info!("Installed claude-code-bridge skill to {:?}", skill_path);
        Ok(())
    }

    /// Install Claude Code CLI via npm
    pub async fn install_cli() -> std::result::Result<(), String> {
        let npm = which::which("npm")
            .or_else(|_| which::which("pnpm"))
            .or_else(|_| which::which("bun"))
            .map_err(|_| "No package manager found. Please install npm, pnpm, or bun.")?;

        let pkg_manager = npm
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("npm");

        let install_cmd = format!("{} install -g @anthropic-ai/claude-code", pkg_manager);

        #[cfg(target_os = "linux")]
        {
            let terminals = [
                ("kitty", vec!["-e", "bash", "-c"]),
                ("gnome-terminal", vec!["--", "bash", "-c"]),
                ("konsole", vec!["-e", "bash", "-c"]),
                ("xterm", vec!["-e", "bash", "-c"]),
                ("alacritty", vec!["-e", "bash", "-c"]),
            ];

            for (term, args) in terminals {
                if which::which(term).is_ok() {
                    let mut cmd = Command::new(term);
                    for arg in &args {
                        cmd.arg(arg);
                    }
                    cmd.arg(format!(
                        "{}; echo ''; echo 'Press Enter to close...'; read",
                        install_cmd
                    ));

                    if cmd.spawn().is_ok() {
                        return Ok(());
                    }
                }
            }
            Err(format!(
                "Could not open terminal. Please run manually: {}",
                install_cmd
            ))
        }

        #[cfg(target_os = "macos")]
        {
            Command::new("osascript")
                .args([
                    "-e",
                    &format!(r#"tell application "Terminal" to do script "{}""#, install_cmd),
                ])
                .spawn()
                .map_err(|e| format!("Failed to open Terminal: {}", e))?;
            Ok(())
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .args(["/c", "start", "cmd", "/k", &install_cmd])
                .spawn()
                .map_err(|e| format!("Failed to open terminal: {}", e))?;
            Ok(())
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        Err(format!(
            "Unsupported platform. Please run manually: {}",
            install_cmd
        ))
    }

    /// Request conversation compaction via /compact command
    pub async fn compact(&self) -> std::result::Result<(), String> {
        let binary = which::which("claude").map_err(|_| "Claude Code CLI not installed")?;

        // Get current session
        let session_id = {
            let current = self.current_session.read().await;
            current.clone()
        };

        let mut cmd = Command::new(binary);
        cmd.args(["-p", "/compact", "--output-format", "json"]);

        // Resume session if we have one
        if let Some(ref sid) = session_id {
            cmd.args(["--resume", sid]);
        } else {
            cmd.arg("--continue");
        }

        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = tokio::time::timeout(tokio::time::Duration::from_secs(120), cmd.output())
            .await
            .map_err(|_| "Compaction timed out")?
            .map_err(|e| format!("Failed to run compact: {}", e))?;

        if output.status.success() {
            // Update session info to mark as compacted
            if let Some(sid) = session_id {
                if let Some(mut info) = self.session_store.get(&sid).await {
                    info.is_compacted = true;
                    let _ = self.session_store.store(info).await;
                }
            }
            info!("Session compacted successfully");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Compaction failed: {}", stderr))
        }
    }
}

/// The claude-code-bridge skill content
const CLAUDE_CODE_BRIDGE_SKILL: &str = r#"---
name: claude-code-bridge
description: Integrate with Claude Code CLI for delegating complex coding tasks. Use when you need to spawn a separate Claude Code instance to work on files, run tests, or perform multi-step coding operations in a specific directory. Enables "Claude calling Claude" patterns for parallel work or specialized contexts.
---

# Claude Code Bridge

Delegate coding tasks to Claude Code CLI from any context.

## When to Use

- **Parallel Work**: Spawn Claude Code to work on a subtask while you continue
- **Different Context**: Need Claude Code's file access in a specific directory
- **Specialized Tasks**: Let Claude Code handle complex refactoring, testing, or debugging
- **Isolation**: Keep risky operations in a separate session

## Usage Patterns

### Via MCP Server (Recommended)

If the `claude-code-mcp` server is configured, use the MCP tools:

```
Use claude_prompt to ask Claude Code: "Refactor the authentication module to use JWT"
```

Tools available:
- `claude_prompt` - Send a prompt to Claude Code
- `claude_continue` - Continue most recent conversation
- `claude_resume` - Resume a specific session by ID
- `claude_version` - Get version info

### Via CLI (Direct)

Execute Claude Code directly:

```bash
# Single prompt
claude -p "List all TODO comments in this project" --output-format json

# Continue conversation
claude -p "Now fix the first TODO" --continue

# Resume specific session
claude -p "What files did we modify?" --resume <session-id>
```

## Best Practices

1. **Specify Working Directory**: Always set `working_dir` to give Claude Code proper context
2. **Use Descriptive Prompts**: Be specific about what you want done
3. **Capture Session IDs**: Save session IDs from responses to resume conversations
4. **Set Timeouts**: Complex tasks may need longer timeouts (default: 5 min)
5. **Check Results**: Verify Claude Code's output before proceeding

## Error Handling

- **Not Found**: Ensure Claude Code is installed (`npm install -g @anthropic-ai/claude-code`)
- **Timeout**: Increase `timeout_secs` for complex tasks
- **Permission Denied**: Claude Code may need approval for file writes
"#;

/// Status of Claude Code CLI installation and authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeCodeStatus {
    /// Whether the CLI binary is installed
    pub installed: bool,
    /// Whether the user is logged in
    pub logged_in: bool,
    /// Whether the claude-code-bridge skill is installed
    pub skill_installed: bool,
    /// CLI version if available
    pub version: Option<String>,
    /// User email if logged in
    pub user_email: Option<String>,
    /// Error message if any
    pub error: Option<String>,
}

impl Default for ClaudeCodeStatus {
    fn default() -> Self {
        Self {
            installed: false,
            logged_in: false,
            skill_installed: false,
            version: None,
            user_email: None,
            error: None,
        }
    }
}

impl Default for ClaudeCodeProvider {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// LLMProvider Implementation
// ============================================================================

#[async_trait]
impl LLMProvider for ClaudeCodeProvider {
    fn id(&self) -> &str {
        "claude-code"
    }

    fn name(&self) -> &str {
        "Claude Code (CLI)"
    }

    fn model(&self) -> &str {
        self.model.as_deref().unwrap_or("claude-code")
    }

    async fn health_check(&self) -> bool {
        Self::version().await.is_ok()
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        None
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let message = self.build_message(&request);
        debug!(message_len = message.len(), "sending message to Claude Code");

        let start = Instant::now();

        // Check if we should continue a session
        let session_mode = {
            let current = self.current_session.read().await;
            if current.is_some() {
                SessionMode::Continue
            } else {
                SessionMode::New
            }
        };

        // Try primary model first
        let result = self.execute_prompt(&message, session_mode.clone()).await;

        let (response, used_model) = match result {
            Ok(resp) => (resp, self.model.clone().unwrap_or_else(|| DEFAULT_MODEL.to_string())),
            Err(LLMError::RateLimited { .. }) if self.auto_fallback => {
                // Rate limited - try fallback model if available
                if let Some(ref fallback) = self.fallback_model {
                    warn!(
                        primary = ?self.model,
                        fallback = fallback,
                        "Rate limited on primary model, switching to fallback"
                    );

                    let fallback_response = self
                        .execute_prompt_with_model(&message, session_mode, fallback)
                        .await?;

                    (fallback_response, fallback.clone())
                } else {
                    // No fallback configured
                    return Err(LLMError::RateLimited { retry_after_secs: 60 });
                }
            }
            Err(e) => return Err(e),
        };

        let latency_ms = start.elapsed().as_millis() as u64;

        if let Some(error) = response.error {
            return Err(LLMError::ApiError {
                status: 0,
                message: error,
            });
        }

        info!(
            response_len = response.result.len(),
            latency_ms,
            model = used_model,
            session_id = ?response.session_id,
            "received response from Claude Code"
        );

        Ok(ChatResponse {
            content: response.result,
            model: used_model,
            provider: "claude-code".to_string(),
            usage: response.usage.map(|u| crate::core::llm::cost::TokenUsage {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
            }),
            finish_reason: Some("stop".to_string()),
            latency_ms,
            cost_usd: response.cost.map(|c| c.usd),
            tool_calls: None,
        })
    }

    async fn stream_chat(&self, request: ChatRequest) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        let message = self.build_message(&request);
        debug!(message_len = message.len(), "streaming message to Claude Code");

        // Check if we should continue a session
        let session_mode = {
            let current = self.current_session.read().await;
            if current.is_some() {
                SessionMode::Continue
            } else {
                SessionMode::New
            }
        };

        self.execute_streaming(&message, session_mode).await
    }

    fn supports_streaming(&self) -> bool {
        true // Now supports streaming via --output-format stream-json
    }

    fn supports_embeddings(&self) -> bool {
        false
    }
}

// ============================================================================
// ProviderSession Implementation
// ============================================================================

#[async_trait]
impl ProviderSession for ClaudeCodeProvider {
    fn supports_sessions(&self) -> bool {
        true
    }

    async fn current_session(&self) -> Option<SessionId> {
        self.current_session.read().await.clone()
    }

    async fn resume_session(&self, session_id: &SessionId) -> SessionResult<SessionInfo> {
        // Verify session exists
        let info = self
            .session_store
            .get(session_id)
            .await
            .ok_or_else(|| SessionError::NotFound(session_id.clone()))?;

        // Set as current session
        {
            let mut current = self.current_session.write().await;
            *current = Some(session_id.clone());
        }

        // Touch to update last used time
        let _ = self.session_store.touch(session_id).await;

        Ok(info)
    }

    async fn continue_session(&self) -> SessionResult<SessionInfo> {
        let recent = self
            .session_store
            .most_recent("claude-code")
            .await
            .ok_or_else(|| SessionError::NotFound("no recent session".to_string()))?;

        self.resume_session(&recent.id).await
    }

    async fn fork_session(&self, session_id: &SessionId) -> SessionResult<SessionId> {
        // Verify source session exists
        let source = self
            .session_store
            .get(session_id)
            .await
            .ok_or_else(|| SessionError::NotFound(session_id.clone()))?;

        // Execute a minimal prompt with fork to create new session
        let binary = which::which("claude")
            .map_err(|_| SessionError::OperationFailed("Claude Code CLI not found".to_string()))?;

        let mut cmd = Command::new(binary);
        cmd.args([
            "-p",
            "Continue from here.",
            "--output-format",
            "json",
            "--resume",
            session_id,
            "--fork-session",
        ]);

        if let Some(ref dir) = source.working_dir {
            cmd.current_dir(dir);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd.output().await.map_err(|e| {
            SessionError::OperationFailed(format!("Failed to fork session: {}", e))
        })?;

        if !output.status.success() {
            return Err(SessionError::OperationFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        // Parse response to get new session ID
        let response: ClaudeCodeResponse = serde_json::from_slice(&output.stdout)
            .map_err(|e| SessionError::OperationFailed(format!("Failed to parse response: {}", e)))?;

        let new_session_id = response
            .session_id
            .ok_or_else(|| SessionError::OperationFailed("No session ID in response".to_string()))?;

        // Store new session
        let mut new_info = SessionInfo::new(new_session_id.clone(), "claude-code");
        new_info.working_dir = source.working_dir;
        self.session_store.store(new_info).await?;

        // Set as current
        {
            let mut current = self.current_session.write().await;
            *current = Some(new_session_id.clone());
        }

        Ok(new_session_id)
    }

    async fn compact_session(&self) -> SessionResult<()> {
        self.compact()
            .await
            .map_err(|e| SessionError::CompactionFailed(e))
    }

    async fn get_session_info(&self, session_id: &SessionId) -> SessionResult<SessionInfo> {
        self.session_store
            .get(session_id)
            .await
            .ok_or_else(|| SessionError::NotFound(session_id.clone()))
    }

    async fn list_sessions(&self, limit: usize) -> SessionResult<Vec<SessionInfo>> {
        Ok(self.session_store.list_by_provider("claude-code", limit).await)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id() {
        let provider = ClaudeCodeProvider::new();
        assert_eq!(provider.id(), "claude-code");
        assert_eq!(provider.name(), "Claude Code (CLI)");
    }

    #[test]
    fn test_no_pricing() {
        let provider = ClaudeCodeProvider::new();
        assert!(provider.pricing().is_none());
    }

    #[test]
    fn test_supports_streaming() {
        let provider = ClaudeCodeProvider::new();
        assert!(provider.supports_streaming());
        assert!(!provider.supports_embeddings());
    }

    #[test]
    fn test_custom_config() {
        let provider = ClaudeCodeProvider::with_config(
            60,
            Some("claude-sonnet-4-20250514".to_string()),
            None,
        );
        assert_eq!(provider.timeout_secs, 60);
        assert_eq!(
            provider.model,
            Some("claude-sonnet-4-20250514".to_string())
        );
    }

    #[test]
    fn test_default() {
        let provider = ClaudeCodeProvider::default();
        assert_eq!(provider.timeout_secs, 300);
        assert!(provider.model.is_none());
    }

    #[test]
    fn test_build_args_new_session() {
        let provider = ClaudeCodeProvider::new();
        let args = provider.build_args("test", &SessionMode::New, false);
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"test".to_string()));
        assert!(args.contains(&"json".to_string()));
        assert!(!args.contains(&"--continue".to_string()));
        // Should have --model with dynamic selection
        assert!(args.contains(&"--model".to_string()));
    }

    #[test]
    fn test_build_args_continue() {
        let provider = ClaudeCodeProvider::new();
        let args = provider.build_args("test", &SessionMode::Continue, false);
        assert!(args.contains(&"--continue".to_string()));
    }

    #[test]
    fn test_build_args_resume() {
        let provider = ClaudeCodeProvider::new();
        let args = provider.build_args("test", &SessionMode::Resume("abc123".to_string()), false);
        assert!(args.contains(&"--resume".to_string()));
        assert!(args.contains(&"abc123".to_string()));
    }

    #[test]
    fn test_build_args_fork() {
        let provider = ClaudeCodeProvider::new();
        let args = provider.build_args("test", &SessionMode::Fork("abc123".to_string()), false);
        assert!(args.contains(&"--resume".to_string()));
        assert!(args.contains(&"abc123".to_string()));
        assert!(args.contains(&"--fork-session".to_string()));
    }

    #[test]
    fn test_build_args_streaming() {
        let provider = ClaudeCodeProvider::new();
        let args = provider.build_args("test", &SessionMode::New, true);
        assert!(args.contains(&"stream-json".to_string()));
    }

    #[test]
    fn test_build_args_with_model() {
        let provider = ClaudeCodeProvider::with_config(
            300,
            Some("claude-sonnet-4-20250514".to_string()),
            None,
        );
        let args = provider.build_args("test", &SessionMode::New, false);
        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"claude-sonnet-4-20250514".to_string()));
    }

    #[test]
    fn test_is_rate_limit_error() {
        assert!(ClaudeCodeProvider::is_rate_limit_error("Error: rate limit exceeded"));
        assert!(ClaudeCodeProvider::is_rate_limit_error("429 Too Many Requests"));
        assert!(ClaudeCodeProvider::is_rate_limit_error("API is overloaded"));
        assert!(ClaudeCodeProvider::is_rate_limit_error("Server at capacity"));
        assert!(!ClaudeCodeProvider::is_rate_limit_error("Internal server error"));
        assert!(!ClaudeCodeProvider::is_rate_limit_error("Authentication failed"));
    }

    #[tokio::test]
    async fn test_session_store_integration() {
        let store = Arc::new(SessionStore::new());
        let provider = ClaudeCodeProvider::with_session_store(300, None, None, store.clone());

        // Initially no session
        assert!(provider.current_session().await.is_none());

        // Store a session manually
        let info = SessionInfo::new("test-session".to_string(), "claude-code");
        store.store(info).await.unwrap();

        // List sessions
        let sessions = provider.list_sessions(10).await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "test-session");
    }

    #[test]
    fn test_claude_code_status_default() {
        let status = ClaudeCodeStatus::default();
        assert!(!status.installed);
        assert!(!status.logged_in);
        assert!(!status.skill_installed);
        assert!(status.version.is_none());
        assert!(status.error.is_none());
    }

    // ==================== Builder Pattern Tests ====================

    #[test]
    fn test_builder_default() {
        let provider = ClaudeCodeProvider::builder().build();
        assert_eq!(provider.timeout_secs, 300);
        assert!(provider.model.is_none());
        assert!(provider.fallback_model.is_some());
        assert!(provider.auto_fallback);
        assert!(provider.persist_sessions);
    }

    #[test]
    fn test_builder_with_model() {
        let provider = ClaudeCodeProvider::builder()
            .model("claude-sonnet-4-20250514")
            .build();
        assert_eq!(provider.model.as_deref(), Some("claude-sonnet-4-20250514"));
    }

    #[test]
    fn test_builder_with_fallback() {
        let provider = ClaudeCodeProvider::builder()
            .fallback_model("claude-haiku-4-20250514")
            .build();
        assert_eq!(provider.fallback_model.as_deref(), Some("claude-haiku-4-20250514"));
    }

    #[test]
    fn test_builder_no_fallback() {
        let provider = ClaudeCodeProvider::builder()
            .no_fallback()
            .build();
        assert!(provider.fallback_model.is_none());
        assert!(!provider.auto_fallback);
    }

    #[test]
    fn test_builder_full_config() {
        let provider = ClaudeCodeProvider::builder()
            .model("claude-opus-4-20250514")
            .fallback_model("claude-sonnet-4-20250514")
            .timeout_secs(600)
            .working_dir("/tmp/test")
            .persist_sessions(false)
            .auto_fallback(true)
            .build();

        assert_eq!(provider.model.as_deref(), Some("claude-opus-4-20250514"));
        assert_eq!(provider.fallback_model.as_deref(), Some("claude-sonnet-4-20250514"));
        assert_eq!(provider.timeout_secs, 600);
        assert_eq!(provider.working_dir.as_deref(), Some("/tmp/test"));
        assert!(!provider.persist_sessions);
        assert!(provider.auto_fallback);
    }

    // ==================== Rate Limit Detection Tests ====================

    #[test]
    fn test_is_rate_limit_error_429() {
        assert!(ClaudeCodeProvider::is_rate_limit_error("Error 429: Too many requests"));
        assert!(ClaudeCodeProvider::is_rate_limit_error("HTTP 429"));
    }

    #[test]
    fn test_is_rate_limit_error_rate_limit() {
        assert!(ClaudeCodeProvider::is_rate_limit_error("rate limit exceeded"));
        assert!(ClaudeCodeProvider::is_rate_limit_error("Rate Limit Hit"));
    }

    #[test]
    fn test_is_rate_limit_error_quota() {
        assert!(ClaudeCodeProvider::is_rate_limit_error("quota exceeded"));
        assert!(ClaudeCodeProvider::is_rate_limit_error("Quota limit reached"));
    }

    #[test]
    fn test_is_rate_limit_error_not_rate_limit() {
        assert!(!ClaudeCodeProvider::is_rate_limit_error("authentication failed"));
        assert!(!ClaudeCodeProvider::is_rate_limit_error("network error"));
        assert!(!ClaudeCodeProvider::is_rate_limit_error(""));
    }

    #[test]
    fn test_is_rate_limit_error_capacity_indicators() {
        // Capacity/overload indicators for dynamic fallback
        assert!(ClaudeCodeProvider::is_rate_limit_error("API overloaded"));
        assert!(ClaudeCodeProvider::is_rate_limit_error("Server at capacity"));
    }

    // ==================== Model Override Tests ====================

    #[test]
    fn test_build_args_with_model_override() {
        let provider = ClaudeCodeProvider::builder()
            .model("claude-sonnet-4-20250514")
            .build();

        let args = provider.build_args_with_model(
            "test",
            &SessionMode::New,
            false,
            Some("claude-haiku-4-20250514"),
        );

        // Should use override, not the provider's model
        assert!(args.contains(&"claude-haiku-4-20250514".to_string()));
        assert!(!args.contains(&"claude-sonnet-4-20250514".to_string()));
    }

    #[test]
    fn test_build_args_without_model_override() {
        let provider = ClaudeCodeProvider::builder()
            .model("claude-sonnet-4-20250514")
            .build();

        let args = provider.build_args_with_model(
            "test",
            &SessionMode::New,
            false,
            None,
        );

        // Should use provider's model
        assert!(args.contains(&"claude-sonnet-4-20250514".to_string()));
    }
}
