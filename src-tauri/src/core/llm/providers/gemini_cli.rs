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
use std::process::Stdio;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Default timeout in seconds for CLI operations.
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Default model to use.
const DEFAULT_MODEL: &str = "gemini-2.5-pro";

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
    timeout_secs: u64,
    working_dir: Option<PathBuf>,
    yolo_mode: bool,
    sandbox: bool,
}

/// Builder for GeminiCliProvider.
#[derive(Debug, Default)]
pub struct GeminiCliProviderBuilder {
    model: Option<String>,
    timeout_secs: Option<u64>,
    working_dir: Option<PathBuf>,
    yolo_mode: bool,
    sandbox: bool,
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

    /// Build the provider.
    pub fn build(self) -> GeminiCliProvider {
        GeminiCliProvider {
            model: self.model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            timeout_secs: self.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS),
            working_dir: self.working_dir,
            yolo_mode: self.yolo_mode,
            sandbox: self.sandbox,
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
                            // Unknown state, assume authenticated but there might be an issue
                            (true, true, format!("Gemini CLI {} installed", version))
                        }
                    }
                    Err(_) => {
                        // Couldn't run auth check, assume installed but status unknown
                        (true, false, format!("Gemini CLI {} installed (auth status unknown)", version))
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
        use std::process::Command as StdCommand;

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
        use std::process::Command as StdCommand;

        StdCommand::new("open")
            .args(["-a", "Terminal", "--args", "-e", "gemini"])
            .spawn()
    }

    #[cfg(target_os = "windows")]
    pub fn launch_login() -> std::io::Result<std::process::Child> {
        use std::process::Command as StdCommand;

        StdCommand::new("cmd")
            .args(["/c", "start", "cmd", "/k", "gemini"])
            .spawn()
    }

    /// Build command arguments for a chat request.
    fn build_args(&self, prompt: &str) -> Vec<String> {
        let mut args = vec![
            "-p".to_string(),
            prompt.to_string(),
            "--output-format".to_string(),
            "json".to_string(),
        ];

        // Add model if not default
        if self.model != DEFAULT_MODEL {
            args.push("--model".to_string());
            args.push(self.model.clone());
        }

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
        let mut args = vec![
            "-p".to_string(),
            prompt.to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
        ];

        if self.model != DEFAULT_MODEL {
            args.push("--model".to_string());
            args.push(self.model.clone());
        }

        if self.yolo_mode {
            args.push("--yolo".to_string());
        }

        if self.sandbox {
            args.push("--sandbox".to_string());
        }

        args
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
        let args = self.build_args(&prompt);

        debug!(
            prompt_len = prompt.len(),
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

            return Err(LLMError::ApiError {
                status: output.status.code().unwrap_or(1) as u16,
                message: stderr.to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let (content, usage) = self.parse_response(&stdout)?;

        info!(
            response_len = content.len(),
            latency_ms,
            usage = ?usage,
            "received response from Gemini CLI"
        );

        Ok(ChatResponse {
            content,
            model: self.model.clone(),
            provider: "gemini-cli".to_string(),
            usage,
            finish_reason: Some("stop".to_string()),
            latency_ms,
            cost_usd: None, // Free tier / subscription based
        })
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
        assert_eq!(provider.model(), "gemini-2.5-pro");
    }

    #[test]
    fn test_custom_model() {
        let provider = GeminiCliProvider::builder()
            .model("gemini-2.5-flash")
            .build();
        assert_eq!(provider.model(), "gemini-2.5-flash");
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
}
