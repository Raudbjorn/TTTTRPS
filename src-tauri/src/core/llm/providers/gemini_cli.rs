//! Gemini CLI Provider
//!
//! Provides LLM access through Google's Gemini CLI tool.
//! This leverages the `gemini` command-line tool for accessing Gemini models
//! using Google account authentication (no API key required).
//!
//! ## Features
//!
//! - Uses existing Google account authentication (free tier: 1000 req/day)
//! - Supports streaming via `--output-format stream-json`
//! - Native Google Search grounding
//! - 1M token context window with gemini-2.5-pro
//!
//! ## Usage
//!
//! ```rust,no_run
//! use crate::core::llm::providers::GeminiCliProvider;
//!
//! // Create provider with default config
//! let provider = GeminiCliProvider::new();
//!
//! // Or with custom settings
//! let provider = GeminiCliProvider::builder()
//!     .model("gemini-2.5-flash")
//!     .timeout_secs(300)
//!     .yolo_mode(true)
//!     .build();
//! ```

use crate::core::llm::cost::{ProviderPricing, TokenUsage};
use crate::core::llm::router::{
    ChatChunk, ChatRequest, ChatResponse, LLMError, LLMProvider, Result,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Command as StdCommand, Stdio};
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Default timeout in seconds for CLI operations.
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Default model to use (prefer Pro for quality).
const DEFAULT_MODEL: &str = "gemini-3-pro-preview";

/// Fallback model when rate limited (faster, higher quota).
const FALLBACK_MODEL: &str = "gemini-3-flash-preview";

/// Gemini CLI JSON response structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiCliResponse {
    pub response: Option<String>,
    pub stats: Option<GeminiCliStats>,
    pub error: Option<String>,
}

/// Statistics from Gemini CLI response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiCliStats {
    pub models: Option<std::collections::HashMap<String, GeminiModelStats>>,
    pub tools: Option<GeminiToolStats>,
}

/// Model-specific statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiModelStats {
    pub tokens: Option<GeminiTokenStats>,
    pub api: Option<serde_json::Value>,
}

/// Token statistics from Gemini.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiTokenStats {
    #[serde(alias = "input")]
    pub input_tokens: Option<u32>,
    #[serde(alias = "output")]
    pub output_tokens: Option<u32>,
}

/// Tool call statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiToolStats {
    pub total_calls: Option<u32>,
    pub by_name: Option<std::collections::HashMap<String, u32>>,
}

/// Streaming chunk from Gemini CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiCliStreamChunk {
    #[serde(rename = "type")]
    pub chunk_type: Option<String>,
    pub content: Option<String>,
    pub delta: Option<String>,
    pub done: Option<bool>,
    pub error: Option<String>,
}

/// Gemini CLI provider configuration.
#[derive(Debug, Clone)]
pub struct GeminiCliProvider {
    model: String,
    fallback_model: Option<String>,
    timeout_secs: u64,
    working_dir: Option<PathBuf>,
    yolo_mode: bool,
    sandbox: bool,
    /// Whether to automatically fallback on rate limit errors.
    auto_fallback: bool,
}

/// Builder for GeminiCliProvider.
#[derive(Debug)]
pub struct GeminiCliProviderBuilder {
    model: Option<String>,
    fallback_model: Option<String>,
    timeout_secs: Option<u64>,
    working_dir: Option<PathBuf>,
    yolo_mode: bool,
    sandbox: bool,
    auto_fallback: bool,
}

impl Default for GeminiCliProviderBuilder {
    fn default() -> Self {
        Self {
            model: None,
            fallback_model: Some(FALLBACK_MODEL.to_string()),
            timeout_secs: None,
            working_dir: None,
            yolo_mode: false,
            sandbox: false,
            auto_fallback: true, // Enable by default
        }
    }
}

impl GeminiCliProviderBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the model to use.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the timeout in seconds.
    pub fn timeout_secs(mut self, timeout: u64) -> Self {
        self.timeout_secs = Some(timeout);
        self
    }

    /// Set the working directory for the CLI.
    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Enable YOLO mode (auto-approve tool actions).
    pub fn yolo_mode(mut self, enabled: bool) -> Self {
        self.yolo_mode = enabled;
        self
    }

    /// Enable sandbox mode for safer execution.
    pub fn sandbox(mut self, enabled: bool) -> Self {
        self.sandbox = enabled;
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

    /// Build the provider.
    pub fn build(self) -> GeminiCliProvider {
        GeminiCliProvider {
            model: self.model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            fallback_model: self.fallback_model,
            timeout_secs: self.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS),
            working_dir: self.working_dir,
            yolo_mode: self.yolo_mode,
            sandbox: self.sandbox,
            auto_fallback: self.auto_fallback,
        }
    }
}

impl GeminiCliProvider {
    /// Create a new provider with default settings.
    pub fn new() -> Self {
        Self::builder().build()
    }

    /// Create a new builder.
    pub fn builder() -> GeminiCliProviderBuilder {
        GeminiCliProviderBuilder::new()
    }

    /// Create a provider with custom model and timeout.
    pub fn with_config(model: String, timeout_secs: u64) -> Self {
        Self::builder()
            .model(model)
            .timeout_secs(timeout_secs)
            .build()
    }

    /// Check if Gemini CLI is installed and available.
    pub async fn is_available() -> bool {
        Command::new("gemini")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Get the version of the installed Gemini CLI.
    pub async fn version() -> Option<String> {
        let output = Command::new("gemini")
            .arg("--version")
            .output()
            .await
            .ok()?;

        if output.status.success() {
            String::from_utf8(output.stdout)
                .ok()
                .map(|s| s.trim().to_string())
        } else {
            None
        }
    }

    /// Check the current status of the Gemini CLI installation and authentication.
    /// Returns a tuple of (is_installed, is_authenticated, status_message).
    pub async fn check_status() -> (bool, bool, String) {
        // First check if installed
        let version_output = Command::new("gemini")
            .arg("--version")
            .output()
            .await;

        match version_output {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();

                // Try a simple prompt to check if authenticated
                // Using a minimal prompt with JSON output to detect auth status
                let auth_check = Command::new("gemini")
                    .args(["-p", "ping", "--output-format", "json"])
                    .output()
                    .await;

                match auth_check {
                    Ok(check_output) => {
                        let stderr = String::from_utf8_lossy(&check_output.stderr);
                        let stdout = String::from_utf8_lossy(&check_output.stdout);

                        // Check for authentication-related errors
                        if stderr.contains("not logged in")
                            || stderr.contains("authentication required")
                            || stderr.contains("sign in")
                            || stderr.contains("login") {
                            (true, false, format!("Gemini CLI {} installed but not authenticated. Run 'gemini' to log in.", version))
                        } else if check_output.status.success() || stdout.contains("response") {
                            (true, true, format!("Gemini CLI {} ready", version))
                        } else {
                            // Unknown state - don't assume authenticated, may be in error state
                            (true, false, format!("Gemini CLI {} installed (status unclear, try running 'gemini')", version))
                        }
                    }
                    Err(_) => {
                        // Couldn't run auth check - don't assume authenticated
                        (true, false, format!("Gemini CLI {} installed (run 'gemini' to verify auth)", version))
                    }
                }
            }
            Ok(_) => {
                // Command ran but failed
                (false, false, "Gemini CLI not installed. Run 'npm i -g @google/gemini-cli'".to_string())
            }
            Err(_) => {
                (false, false, "Gemini CLI not installed. Run 'npm i -g @google/gemini-cli'".to_string())
            }
        }
    }

    /// Launch the Gemini CLI in interactive mode for authentication.
    /// Returns the child process handle for the spawned terminal.
    #[cfg(target_os = "linux")]
    pub fn launch_login() -> std::io::Result<std::process::Child> {
        // Try common terminal emulators
        let terminals = ["kitty", "gnome-terminal", "konsole", "xterm", "x-terminal-emulator"];

        for terminal in terminals {
            let result = match terminal {
                "kitty" => StdCommand::new("kitty")
                    .args(["--", "gemini"])
                    .spawn(),
                "gnome-terminal" => StdCommand::new("gnome-terminal")
                    .args(["--", "gemini"])
                    .spawn(),
                "konsole" => StdCommand::new("konsole")
                    .args(["-e", "gemini"])
                    .spawn(),
                "xterm" | "x-terminal-emulator" => StdCommand::new(terminal)
                    .args(["-e", "gemini"])
                    .spawn(),
                _ => continue,
            };

            if result.is_ok() {
                return result;
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No terminal emulator found"
        ))
    }

    #[cfg(target_os = "macos")]
    pub fn launch_login() -> std::io::Result<std::process::Child> {
        // Use osascript to open Terminal and run gemini
        StdCommand::new("osascript")
            .args([
                "-e",
                "tell application \"Terminal\" to do script \"gemini\"",
                "-e",
                "tell application \"Terminal\" to activate",
            ])
            .spawn()
    }

    #[cfg(target_os = "windows")]
    pub fn launch_login() -> std::io::Result<std::process::Child> {
        StdCommand::new("cmd")
            .args(["/c", "start", "cmd", "/k", "gemini"])
            .spawn()
    }

    /// The name of the Sidecar DM extension for Gemini CLI.
    pub const EXTENSION_NAME: &'static str = "sidecar-dm";

    /// Check if the Sidecar DM extension is installed.
    /// Returns (is_installed, extension_version_or_message).
    pub async fn check_extension_status() -> (bool, String) {
        let output = Command::new("gemini")
            .args(["extensions", "list"])
            .output()
            .await;

        match output {
            Ok(result) if result.status.success() => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                // Check if our extension is in the list
                if stdout.contains(Self::EXTENSION_NAME) {
                    // Try to extract version if present
                    for line in stdout.lines() {
                        if line.contains(Self::EXTENSION_NAME) {
                            return (true, format!("Extension '{}' installed", Self::EXTENSION_NAME));
                        }
                    }
                    (true, format!("Extension '{}' installed", Self::EXTENSION_NAME))
                } else {
                    (false, format!("Extension '{}' not installed", Self::EXTENSION_NAME))
                }
            }
            Ok(_) => {
                (false, "Could not list extensions".to_string())
            }
            Err(e) => {
                (false, format!("Error checking extensions: {}", e))
            }
        }
    }

    /// Install the Sidecar DM extension from a git repository or local path.
    /// Returns Ok(message) on success, Err(error) on failure.
    pub async fn install_extension(source: &str) -> std::result::Result<String, String> {
        info!("Installing Gemini CLI extension from: {}", source);

        let output = Command::new("gemini")
            .args(["extensions", "install", source])
            .output()
            .await
            .map_err(|e| format!("Failed to run install command: {}", e))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            info!("Extension installed successfully: {}", stdout.trim());
            Ok(format!("Extension '{}' installed successfully", Self::EXTENSION_NAME))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            error!("Extension installation failed: {} {}", stdout, stderr);
            Err(format!("Installation failed: {} {}", stdout.trim(), stderr.trim()))
        }
    }

    /// Link a local extension directory for development.
    pub async fn link_extension(path: &str) -> std::result::Result<String, String> {
        info!("Linking local Gemini CLI extension from: {}", path);

        let output = Command::new("gemini")
            .args(["extensions", "link", path])
            .output()
            .await
            .map_err(|e| format!("Failed to link extension: {}", e))?;

        if output.status.success() {
            Ok(format!("Extension linked from '{}'", path))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Link failed: {}", stderr.trim()))
        }
    }

    /// Uninstall the Sidecar DM extension.
    pub async fn uninstall_extension() -> std::result::Result<String, String> {
        info!("Uninstalling Gemini CLI extension: {}", Self::EXTENSION_NAME);

        let output = Command::new("gemini")
            .args(["extensions", "uninstall", Self::EXTENSION_NAME])
            .output()
            .await
            .map_err(|e| format!("Failed to uninstall extension: {}", e))?;

        if output.status.success() {
            Ok(format!("Extension '{}' uninstalled", Self::EXTENSION_NAME))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Uninstall failed: {}", stderr.trim()))
        }
    }

    /// Build command arguments for a chat request with a specific model.
    fn build_args_with_model(&self, prompt: &str, model: &str) -> Vec<String> {
        let mut args = vec![
            "-p".to_string(),
            prompt.to_string(),
            "--output-format".to_string(),
            "json".to_string(),
            "--model".to_string(),
            model.to_string(),
        ];

        if self.yolo_mode {
            args.push("--yolo".to_string());
        }

        if self.sandbox {
            args.push("--sandbox".to_string());
        }

        args
    }

    /// Build command arguments for a chat request.
    fn build_args(&self, prompt: &str) -> Vec<String> {
        self.build_args_with_model(prompt, &self.model)
    }

    /// Build command arguments for streaming with a specific model.
    fn build_stream_args_with_model(&self, prompt: &str, model: &str) -> Vec<String> {
        let mut args = vec![
            "-p".to_string(),
            prompt.to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--model".to_string(),
            model.to_string(),
        ];

        if self.yolo_mode {
            args.push("--yolo".to_string());
        }

        if self.sandbox {
            args.push("--sandbox".to_string());
        }

        args
    }

    /// Build command arguments for streaming.
    fn build_stream_args(&self, prompt: &str) -> Vec<String> {
        self.build_stream_args_with_model(prompt, &self.model)
    }

    /// Check if an error indicates a rate limit / quota exhaustion.
    fn is_rate_limit_error(stderr: &str, exit_code: Option<i32>) -> bool {
        // Check for common rate limit indicators (case-insensitive)
        let lower = stderr.to_lowercase();
        lower.contains("429")
            || lower.contains("resource exhausted")
            || lower.contains("resource_exhausted")
            || lower.contains("quota")
            || lower.contains("rate limit")
            || lower.contains("too many requests")
            || exit_code == Some(8) // Common exit code for rate limiting
    }

    /// Execute a chat request with a specific model.
    async fn execute_chat(&self, prompt: &str, model: &str) -> Result<(String, Option<TokenUsage>, u64)> {
        let args = self.build_args_with_model(prompt, model);

        debug!(
            prompt_len = prompt.len(),
            model = model,
            args = ?args,
            "executing Gemini CLI"
        );

        let start = Instant::now();

        let mut cmd = Command::new("gemini");
        cmd.args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout_secs),
            cmd.output(),
        )
        .await
        .map_err(|_| LLMError::Timeout)?
        .map_err(|e| LLMError::ApiError {
            status: 0,
            message: format!("Failed to execute Gemini CLI: {}", e),
        })?;

        let latency_ms = start.elapsed().as_millis() as u64;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!(
                model = model,
                exit_code = ?output.status.code(),
                stderr = %stderr,
                "Gemini CLI failed"
            );

            if stderr.contains("authentication") || stderr.contains("not logged in") {
                return Err(LLMError::AuthError(
                    "Not authenticated. Run 'gemini' to log in with your Google account."
                        .to_string(),
                ));
            }

            // Check for rate limiting
            if Self::is_rate_limit_error(&stderr, output.status.code()) {
                return Err(LLMError::RateLimited { retry_after_secs: 60 });
            }

            return Err(LLMError::ApiError {
                status: output.status.code().unwrap_or(1) as u16,
                message: stderr.to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let (content, usage) = self.parse_response(&stdout)?;

        Ok((content, usage, latency_ms))
    }

    /// Build the full prompt from a ChatRequest.
    fn build_prompt(&self, request: &ChatRequest) -> String {
        let mut parts = Vec::new();

        // Add system prompt if present
        if let Some(ref system) = request.system_prompt {
            parts.push(format!("[System Instructions]\n{}\n", system));
        }

        // Add conversation history
        for msg in &request.messages {
            match msg.role {
                crate::core::llm::router::MessageRole::System => {
                    // System messages handled above
                }
                crate::core::llm::router::MessageRole::User => {
                    parts.push(format!("User: {}", msg.content));
                }
                crate::core::llm::router::MessageRole::Assistant => {
                    parts.push(format!("Assistant: {}", msg.content));
                }
            }
        }

        // Return the last user message or full context
        if let Some(last_user) = request
            .messages
            .iter()
            .rev()
            .find(|m| matches!(m.role, crate::core::llm::router::MessageRole::User))
        {
            // For simple single-turn, just use the last user message
            if request.messages.len() == 1 {
                return last_user.content.clone();
            }
        }

        parts.join("\n\n")
    }

    /// Parse the JSON response from Gemini CLI.
    fn parse_response(&self, output: &str) -> Result<(String, Option<TokenUsage>)> {
        let response: GeminiCliResponse = serde_json::from_str(output).map_err(|e| {
            error!(error = %e, output = %output, "failed to parse Gemini CLI response");
            LLMError::ApiError {
                status: 0,
                message: format!("Failed to parse response: {}", e),
            }
        })?;

        // Check for errors
        if let Some(error) = response.error {
            if error.contains("authentication") || error.contains("auth") {
                return Err(LLMError::AuthError(error));
            }
            if error.contains("rate limit") || error.contains("quota") {
                return Err(LLMError::RateLimited { retry_after_secs: 60 });
            }
            return Err(LLMError::ApiError {
                status: 0,
                message: error,
            });
        }

        let content = response.response.unwrap_or_default();

        // Extract token usage if available
        let usage = response.stats.and_then(|stats| {
            stats.models.and_then(|models| {
                // Get the first model's stats
                models.values().next().and_then(|model_stats| {
                    model_stats.tokens.as_ref().map(|tokens| TokenUsage::new(
                        tokens.input_tokens.unwrap_or(0),
                        tokens.output_tokens.unwrap_or(0),
                    ))
                })
            })
        });

        Ok((content, usage))
    }
}

impl Default for GeminiCliProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LLMProvider for GeminiCliProvider {
    fn id(&self) -> &str {
        "gemini-cli"
    }

    fn name(&self) -> &str {
        "Gemini CLI"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn health_check(&self) -> bool {
        Self::is_available().await
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        // No per-token pricing - uses Google account auth with free tier
        None
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        if !Self::is_available().await {
            return Err(LLMError::NotConfigured(
                "Gemini CLI not installed. Run 'npm i -g @google/gemini-cli' and authenticate."
                    .to_string(),
            ));
        }

        let prompt = self.build_prompt(&request);

        // Try primary model first
        let primary_model = &self.model;
        match self.execute_chat(&prompt, primary_model).await {
            Ok((content, usage, latency_ms)) => {
                info!(
                    model = primary_model,
                    response_len = content.len(),
                    latency_ms,
                    usage = ?usage,
                    "received response from Gemini CLI"
                );

                return Ok(ChatResponse {
                    content,
                    model: primary_model.clone(),
                    provider: "gemini-cli".to_string(),
                    usage,
                    finish_reason: Some("stop".to_string()),
                    latency_ms,
                    cost_usd: None,
                    tool_calls: None,
                });
            }
            Err(LLMError::RateLimited { .. }) if self.auto_fallback => {
                // Rate limited - try fallback model if available
                if let Some(ref fallback) = self.fallback_model {
                    warn!(
                        primary_model = primary_model,
                        fallback_model = fallback,
                        "Rate limited on primary model, switching to fallback"
                    );

                    let (content, usage, latency_ms) = self.execute_chat(&prompt, fallback).await?;

                    info!(
                        model = fallback,
                        response_len = content.len(),
                        latency_ms,
                        usage = ?usage,
                        "received response from fallback model"
                    );

                    return Ok(ChatResponse {
                        content,
                        model: fallback.clone(),
                        provider: "gemini-cli".to_string(),
                        usage,
                        finish_reason: Some("stop".to_string()),
                        latency_ms,
                        cost_usd: None,
                        tool_calls: None,
                    });
                } else {
                    // No fallback configured, propagate rate limit error
                    return Err(LLMError::RateLimited { retry_after_secs: 60 });
                }
            }
            Err(e) => return Err(e),
        }
    }

    async fn stream_chat(&self, request: ChatRequest) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        if !Self::is_available().await {
            return Err(LLMError::NotConfigured(
                "Gemini CLI not installed. Run 'npm i -g @google/gemini-cli' and authenticate."
                    .to_string(),
            ));
        }

        let prompt = self.build_prompt(&request);
        let args = self.build_stream_args(&prompt);

        debug!(
            prompt_len = prompt.len(),
            args = ?args,
            "executing Gemini CLI (streaming)"
        );

        let mut cmd = Command::new("gemini");
        cmd.args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        let mut child = cmd.spawn().map_err(|e| LLMError::ApiError {
            status: 0,
            message: format!("Failed to spawn Gemini CLI: {}", e),
        })?;

        let stdout = child.stdout.take().ok_or_else(|| LLMError::ApiError {
            status: 0,
            message: "Failed to capture stdout".to_string(),
        })?;

        let (tx, rx) = mpsc::channel(32);
        let model = self.model.clone();
        let stream_id = uuid::Uuid::new_v4().to_string();

        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut index: u32 = 0;

            while let Ok(Some(line)) = lines.next_line().await {
                if line.is_empty() {
                    continue;
                }

                // Try to parse as stream chunk
                match serde_json::from_str::<GeminiCliStreamChunk>(&line) {
                    Ok(chunk) => {
                        // Get the content from either content or delta field
                        let content = chunk
                            .delta
                            .or(chunk.content)
                            .unwrap_or_default();

                        let is_done = chunk.done.unwrap_or(false);

                        if !content.is_empty() || is_done {
                            let chat_chunk = ChatChunk {
                                stream_id: stream_id.clone(),
                                content,
                                provider: "gemini-cli".to_string(),
                                model: model.clone(),
                                is_final: is_done,
                                finish_reason: if is_done {
                                    Some("stop".to_string())
                                } else {
                                    None
                                },
                                usage: None,
                                index,
                            };

                            if tx.send(Ok(chat_chunk)).await.is_err() {
                                break;
                            }
                            index += 1;
                        }

                        // Check for errors
                        if let Some(error) = chunk.error {
                            let _ = tx
                                .send(Err(LLMError::ApiError {
                                    status: 0,
                                    message: error,
                                }))
                                .await;
                            break;
                        }

                        // Check if done
                        if is_done {
                            break;
                        }
                    }
                    Err(e) => {
                        warn!(line = %line, error = %e, "failed to parse stream chunk");
                        // Try to use the line as raw content
                        if !line.starts_with('{') {
                            let chat_chunk = ChatChunk {
                                stream_id: stream_id.clone(),
                                content: line,
                                provider: "gemini-cli".to_string(),
                                model: model.clone(),
                                is_final: false,
                                finish_reason: None,
                                usage: None,
                                index,
                            };
                            if tx.send(Ok(chat_chunk)).await.is_err() {
                                break;
                            }
                            index += 1;
                        }
                    }
                }
            }

            // Wait for process to complete
            let _ = child.wait().await;
        });

        Ok(rx)
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_embeddings(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id() {
        let provider = GeminiCliProvider::new();
        assert_eq!(provider.id(), "gemini-cli");
        assert_eq!(provider.name(), "Gemini CLI");
    }

    #[test]
    fn test_default_model() {
        let provider = GeminiCliProvider::new();
        assert_eq!(provider.model(), "gemini-3-pro-preview");
    }

    #[test]
    fn test_custom_model() {
        let provider = GeminiCliProvider::builder()
            .model("gemini-3-flash-preview")
            .build();
        assert_eq!(provider.model(), "gemini-3-flash-preview");
    }

    #[test]
    fn test_fallback_model_default() {
        let provider = GeminiCliProvider::new();
        assert_eq!(provider.fallback_model.as_deref(), Some("gemini-3-flash-preview"));
        assert!(provider.auto_fallback);
    }

    #[test]
    fn test_no_fallback() {
        let provider = GeminiCliProvider::builder()
            .no_fallback()
            .build();
        assert!(provider.fallback_model.is_none());
        assert!(!provider.auto_fallback);
    }

    #[test]
    fn test_custom_fallback() {
        let provider = GeminiCliProvider::builder()
            .fallback_model("gemini-2.5-flash")
            .build();
        assert_eq!(provider.fallback_model.as_deref(), Some("gemini-2.5-flash"));
    }

    #[test]
    fn test_no_pricing() {
        let provider = GeminiCliProvider::new();
        assert!(provider.pricing().is_none());
    }

    #[test]
    fn test_supports_streaming() {
        let provider = GeminiCliProvider::new();
        assert!(provider.supports_streaming());
        assert!(!provider.supports_embeddings());
    }

    #[test]
    fn test_build_args() {
        let provider = GeminiCliProvider::new();
        let args = provider.build_args("test prompt");
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"test prompt".to_string()));
        assert!(args.contains(&"json".to_string()));
    }

    #[test]
    fn test_build_args_with_yolo() {
        let provider = GeminiCliProvider::builder().yolo_mode(true).build();
        let args = provider.build_args("test");
        assert!(args.contains(&"--yolo".to_string()));
    }

    #[test]
    fn test_builder_chain() {
        let provider = GeminiCliProvider::builder()
            .model("gemini-2.5-flash")
            .timeout_secs(300)
            .yolo_mode(true)
            .sandbox(true)
            .build();

        assert_eq!(provider.model, "gemini-2.5-flash");
        assert_eq!(provider.timeout_secs, 300);
        assert!(provider.yolo_mode);
        assert!(provider.sandbox);
    }

    #[test]
    fn test_parse_response() {
        let provider = GeminiCliProvider::new();
        let json = r#"{"response": "Hello!", "stats": null, "error": null}"#;
        let (content, usage) = provider.parse_response(json).unwrap();
        assert_eq!(content, "Hello!");
        assert!(usage.is_none());
    }

    #[test]
    fn test_parse_response_with_stats() {
        let provider = GeminiCliProvider::new();
        let json = r#"{
            "response": "Hello!",
            "stats": {
                "models": {
                    "gemini-2.5-pro": {
                        "tokens": {
                            "input": 10,
                            "output": 5,
                            "total": 15
                        }
                    }
                }
            },
            "error": null
        }"#;
        let (content, usage) = provider.parse_response(json).unwrap();
        assert_eq!(content, "Hello!");
        assert!(usage.is_some());
        let usage = usage.unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 5);
        assert_eq!(usage.total(), 15);
    }

    #[test]
    fn test_parse_response_with_error() {
        let provider = GeminiCliProvider::new();
        let json = r#"{"response": null, "stats": null, "error": "rate limit exceeded"}"#;
        let result = provider.parse_response(json);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LLMError::RateLimited { .. }));
    }

    // ==================== Rate Limit Detection Tests ====================

    #[test]
    fn test_is_rate_limit_error_429() {
        assert!(GeminiCliProvider::is_rate_limit_error("Error 429: Too many requests", None));
        assert!(GeminiCliProvider::is_rate_limit_error("HTTP 429", None));
    }

    #[test]
    fn test_is_rate_limit_error_resource_exhausted() {
        assert!(GeminiCliProvider::is_rate_limit_error("RESOURCE_EXHAUSTED", None));
        assert!(GeminiCliProvider::is_rate_limit_error("Resource exhausted: quota exceeded", None));
    }

    #[test]
    fn test_is_rate_limit_error_quota() {
        assert!(GeminiCliProvider::is_rate_limit_error("quota exceeded", None));
        assert!(GeminiCliProvider::is_rate_limit_error("You have exceeded your quota", None));
    }

    #[test]
    fn test_is_rate_limit_error_rate_limit() {
        assert!(GeminiCliProvider::is_rate_limit_error("rate limit exceeded", None));
        assert!(GeminiCliProvider::is_rate_limit_error("Rate limit hit", None));
    }

    #[test]
    fn test_is_rate_limit_error_too_many_requests() {
        assert!(GeminiCliProvider::is_rate_limit_error("too many requests", None));
    }

    #[test]
    fn test_is_rate_limit_error_exit_code_8() {
        assert!(GeminiCliProvider::is_rate_limit_error("", Some(8)));
        assert!(GeminiCliProvider::is_rate_limit_error("some other error", Some(8)));
    }

    #[test]
    fn test_is_rate_limit_error_not_rate_limit() {
        assert!(!GeminiCliProvider::is_rate_limit_error("authentication failed", None));
        assert!(!GeminiCliProvider::is_rate_limit_error("invalid model", None));
        assert!(!GeminiCliProvider::is_rate_limit_error("network error", Some(1)));
        assert!(!GeminiCliProvider::is_rate_limit_error("", None));
    }

    // ==================== Args Building Tests ====================

    #[test]
    fn test_build_args_with_model() {
        let provider = GeminiCliProvider::new();
        let args = provider.build_args_with_model("hello", "gemini-3-flash-preview");

        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"hello".to_string()));
        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"gemini-3-flash-preview".to_string()));
        assert!(args.contains(&"json".to_string()));
    }

    #[test]
    fn test_build_args_with_model_includes_yolo() {
        let provider = GeminiCliProvider::builder()
            .yolo_mode(true)
            .build();
        let args = provider.build_args_with_model("test", "gemini-3-pro-preview");

        assert!(args.contains(&"--yolo".to_string()));
        assert!(args.contains(&"gemini-3-pro-preview".to_string()));
    }

    #[test]
    fn test_build_args_with_model_includes_sandbox() {
        let provider = GeminiCliProvider::builder()
            .sandbox(true)
            .build();
        let args = provider.build_args_with_model("test", "gemini-3-pro-preview");

        assert!(args.contains(&"--sandbox".to_string()));
    }

    #[test]
    fn test_build_stream_args_with_model() {
        let provider = GeminiCliProvider::new();
        let args = provider.build_stream_args_with_model("hello", "gemini-3-flash-preview");

        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"hello".to_string()));
        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"gemini-3-flash-preview".to_string()));
        assert!(args.contains(&"stream-json".to_string()));
    }

    // ==================== Prompt Building Tests ====================

    #[test]
    fn test_build_prompt_simple() {
        use crate::core::llm::router::ChatMessage;

        let provider = GeminiCliProvider::new();
        let request = ChatRequest::new(vec![ChatMessage::user("Hello, world!")]);

        let prompt = provider.build_prompt(&request);
        assert_eq!(prompt, "Hello, world!");
    }

    #[test]
    fn test_build_prompt_with_system() {
        use crate::core::llm::router::ChatMessage;

        let provider = GeminiCliProvider::new();
        let request = ChatRequest {
            messages: vec![
                ChatMessage::user("Hi"),
                ChatMessage::assistant("Hello!"),
                ChatMessage::user("How are you?"),
            ],
            system_prompt: Some("You are a helpful assistant.".to_string()),
            temperature: None,
            max_tokens: None,
            provider: None,
            tools: None,
            tool_choice: None,
        };

        let prompt = provider.build_prompt(&request);
        assert!(prompt.contains("[System Instructions]"));
        assert!(prompt.contains("You are a helpful assistant."));
        assert!(prompt.contains("User: Hi"));
        assert!(prompt.contains("Assistant: Hello!"));
        assert!(prompt.contains("User: How are you?"));
    }

    // ==================== Error Parsing Tests ====================

    #[test]
    fn test_parse_response_auth_error() {
        let provider = GeminiCliProvider::new();
        let json = r#"{"response": null, "stats": null, "error": "authentication required"}"#;
        let result = provider.parse_response(json);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LLMError::AuthError(_)));
    }

    #[test]
    fn test_parse_response_quota_error() {
        let provider = GeminiCliProvider::new();
        let json = r#"{"response": null, "stats": null, "error": "quota exceeded"}"#;
        let result = provider.parse_response(json);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LLMError::RateLimited { .. }));
    }

    #[test]
    fn test_parse_response_generic_error() {
        let provider = GeminiCliProvider::new();
        let json = r#"{"response": null, "stats": null, "error": "unknown error occurred"}"#;
        let result = provider.parse_response(json);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LLMError::ApiError { .. }));
    }

    #[test]
    fn test_parse_response_invalid_json() {
        let provider = GeminiCliProvider::new();
        let result = provider.parse_response("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_response_empty_response() {
        let provider = GeminiCliProvider::new();
        let json = r#"{"response": null, "stats": null, "error": null}"#;
        let (content, _) = provider.parse_response(json).unwrap();
        assert_eq!(content, "");
    }

    // ==================== Builder Combination Tests ====================

    #[test]
    fn test_builder_full_chain() {
        let provider = GeminiCliProvider::builder()
            .model("gemini-3-pro-preview")
            .fallback_model("gemini-2.5-flash")
            .timeout_secs(180)
            .yolo_mode(true)
            .sandbox(false)
            .auto_fallback(true)
            .build();

        assert_eq!(provider.model, "gemini-3-pro-preview");
        assert_eq!(provider.fallback_model.as_deref(), Some("gemini-2.5-flash"));
        assert_eq!(provider.timeout_secs, 180);
        assert!(provider.yolo_mode);
        assert!(!provider.sandbox);
        assert!(provider.auto_fallback);
    }

    #[test]
    fn test_builder_disable_auto_fallback_keeps_fallback_model() {
        let provider = GeminiCliProvider::builder()
            .auto_fallback(false)
            .build();

        // Fallback model is still set, but auto_fallback is disabled
        assert!(provider.fallback_model.is_some());
        assert!(!provider.auto_fallback);
    }

    #[test]
    fn test_working_dir() {
        use std::path::PathBuf;

        let provider = GeminiCliProvider::builder()
            .working_dir("/tmp/test")
            .build();

        assert_eq!(provider.working_dir, Some(PathBuf::from("/tmp/test")));
    }

    // ==================== Stream Chunk Parsing Tests ====================

    #[test]
    fn test_parse_stream_chunk_with_content() {
        let json = r#"{"type": "content", "content": "Hello", "done": false}"#;
        let chunk: GeminiCliStreamChunk = serde_json::from_str(json).unwrap();

        assert_eq!(chunk.chunk_type.as_deref(), Some("content"));
        assert_eq!(chunk.content.as_deref(), Some("Hello"));
        assert_eq!(chunk.done, Some(false));
    }

    #[test]
    fn test_parse_stream_chunk_with_delta() {
        let json = r#"{"type": "delta", "delta": " world", "done": false}"#;
        let chunk: GeminiCliStreamChunk = serde_json::from_str(json).unwrap();

        assert_eq!(chunk.delta.as_deref(), Some(" world"));
    }

    #[test]
    fn test_parse_stream_chunk_done() {
        let json = r#"{"type": "done", "done": true}"#;
        let chunk: GeminiCliStreamChunk = serde_json::from_str(json).unwrap();

        assert_eq!(chunk.done, Some(true));
    }

    #[test]
    fn test_parse_stream_chunk_with_error() {
        let json = r#"{"type": "error", "error": "rate limit", "done": true}"#;
        let chunk: GeminiCliStreamChunk = serde_json::from_str(json).unwrap();

        assert_eq!(chunk.error.as_deref(), Some("rate limit"));
        assert_eq!(chunk.done, Some(true));
    }

    // ==================== Token Stats Parsing Tests ====================

    #[test]
    fn test_parse_token_stats() {
        let json = r#"{"input": 100, "output": 50}"#;
        let stats: GeminiTokenStats = serde_json::from_str(json).unwrap();

        assert_eq!(stats.input_tokens, Some(100));
        assert_eq!(stats.output_tokens, Some(50));
    }

    #[test]
    fn test_parse_token_stats_with_aliases() {
        // Test that both "input" and "input_tokens" work
        let json = r#"{"input_tokens": 100, "output_tokens": 50}"#;
        let stats: GeminiTokenStats = serde_json::from_str(json).unwrap();

        assert_eq!(stats.input_tokens, Some(100));
        assert_eq!(stats.output_tokens, Some(50));
    }
}
