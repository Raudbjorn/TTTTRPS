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
//! - JSON output parsing for structured responses
//! - Configurable timeout and model selection
//!
//! ## Limitations
//!
//! - No streaming support (full responses only)
//! - Requires Claude Code CLI to be installed
//! - Uses CLI's built-in authentication
//!
//! ## Usage
//!
//! ```rust,no_run
//! use crate::core::llm::providers::ClaudeCodeProvider;
//!
//! let provider = ClaudeCodeProvider::new();
//! // Or with custom timeout
//! let provider = ClaudeCodeProvider::with_config(300, None);
//! ```

use crate::core::llm::cost::ProviderPricing;
use crate::core::llm::router::{
    ChatChunk, ChatRequest, ChatResponse, LLMError, LLMProvider, Result,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

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

/// Claude Code provider using CLI.
pub struct ClaudeCodeProvider {
    timeout_secs: u64,
    model: Option<String>,
    working_dir: Option<String>,
}

impl ClaudeCodeProvider {
    /// Create a new provider with default configuration.
    pub fn new() -> Self {
        Self::with_config(300, None, None)
    }

    /// Create a new provider with custom timeout.
    pub fn with_timeout(timeout_secs: u64) -> Self {
        Self::with_config(timeout_secs, None, None)
    }

    /// Create a new provider with custom configuration.
    pub fn with_config(timeout_secs: u64, model: Option<String>, working_dir: Option<String>) -> Self {
        Self {
            timeout_secs,
            model,
            working_dir,
        }
    }

    /// Build the message content from the request.
    ///
    /// Includes system prompt and conversation history for context.
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
                    // Include system messages in context
                    parts.push(format!("[System: {}]", msg.content));
                }
            }
        }

        // Return joined message with all context preserved
        // If no messages, just return the system prompt or empty string
        if parts.is_empty() {
            String::new()
        } else {
            parts.join("\n\n")
        }
    }

    /// Execute a prompt via Claude Code CLI
    async fn execute_prompt(&self, prompt: &str) -> Result<ClaudeCodeResponse> {
        let binary = which::which("claude").map_err(|_| {
            LLMError::NotConfigured(
                "Claude Code CLI not found. Install with: npm install -g @anthropic-ai/claude-code"
                    .to_string(),
            )
        })?;

        let mut cmd = Command::new(binary);
        cmd.arg("-p").arg(prompt);
        cmd.arg("--output-format").arg("json");

        // Add model if specified
        if let Some(ref model) = self.model {
            cmd.arg("--model").arg(model);
        }

        // Set working directory if specified
        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!(command = ?cmd, "executing Claude Code CLI");

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
                        stderr
                    } else if !stdout.is_empty() {
                        stdout
                    } else {
                        "unknown error".to_string()
                    };

                    error!(status = %status, error = %error_msg, "Claude Code failed");
                    return Err(LLMError::ApiError {
                        status: status.code().unwrap_or(1) as u16,
                        message: error_msg,
                    });
                }

                // Parse JSON response
                serde_json::from_str::<ClaudeCodeResponse>(&stdout).or_else(|_| {
                    // If not JSON, treat as plain text response
                    Ok(ClaudeCodeResponse {
                        session_id: None,
                        result: stdout.trim().to_string(),
                        usage: None,
                        cost: None,
                        error: None,
                    })
                })
            }
            Ok(Err(io_err)) => {
                error!(error = %io_err, "I/O error during Claude Code execution");
                Err(LLMError::ApiError {
                    status: 0,
                    message: format!("I/O error: {}", io_err),
                })
            }
            Err(_) => {
                warn!(
                    timeout_secs = self.timeout_secs,
                    "Claude Code request timed out"
                );
                let _ = child.kill().await;
                Err(LLMError::Timeout)
            }
        }
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
        // Check if binary exists
        let binary = match which::which("claude") {
            Ok(b) => b,
            Err(_) => {
                return ClaudeCodeStatus {
                    installed: false,
                    logged_in: false,
                    version: None,
                    user_email: None,
                    error: Some("Claude Code CLI not installed".to_string()),
                };
            }
        };

        // Get version
        let version = match Command::new(&binary)
            .arg("--version")
            .output()
            .await
        {
            Ok(output) if output.status.success() => {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
            _ => None,
        };

        // Check auth status using `claude auth status`
        let auth_result = Command::new(&binary)
            .args(["auth", "status"])
            .output()
            .await;

        match auth_result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                // Try to parse JSON output, or check for success indicators
                if output.status.success() {
                    // Try to parse as JSON for user email
                    let user_email = serde_json::from_str::<serde_json::Value>(&stdout)
                        .ok()
                        .and_then(|v| v.get("email").and_then(|e| e.as_str()).map(String::from));

                    ClaudeCodeStatus {
                        installed: true,
                        logged_in: true,
                        version,
                        user_email,
                        error: None,
                    }
                } else {
                    // Not logged in
                    let error_msg = if !stderr.is_empty() {
                        stderr.trim().to_string()
                    } else if !stdout.is_empty() {
                        stdout.trim().to_string()
                    } else {
                        "Not authenticated".to_string()
                    };

                    ClaudeCodeStatus {
                        installed: true,
                        logged_in: false,
                        version,
                        user_email: None,
                        error: Some(error_msg),
                    }
                }
            }
            Err(e) => ClaudeCodeStatus {
                installed: true,
                logged_in: false,
                version,
                user_email: None,
                error: Some(format!("Failed to check auth status: {}", e)),
            },
        }
    }

    /// Spawn the Claude Code login flow (opens browser)
    pub async fn login() -> std::result::Result<(), String> {
        let binary = which::which("claude")
            .map_err(|_| "Claude Code CLI not installed")?;

        // Run `claude auth login` which opens browser for OAuth
        let status = Command::new(binary)
            .args(["auth", "login"])
            .status()
            .await
            .map_err(|e| format!("Failed to spawn login: {}", e))?;

        if status.success() {
            Ok(())
        } else {
            Err("Login process failed or was cancelled".to_string())
        }
    }

    /// Logout from Claude Code
    pub async fn logout() -> std::result::Result<(), String> {
        let binary = which::which("claude")
            .map_err(|_| "Claude Code CLI not installed")?;

        let status = Command::new(binary)
            .args(["auth", "logout"])
            .status()
            .await
            .map_err(|e| format!("Failed to logout: {}", e))?;

        if status.success() {
            Ok(())
        } else {
            Err("Logout failed".to_string())
        }
    }
}

/// Status of Claude Code CLI installation and authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeCodeStatus {
    /// Whether the CLI binary is installed
    pub installed: bool,
    /// Whether the user is logged in
    pub logged_in: bool,
    /// CLI version if available
    pub version: Option<String>,
    /// User email if logged in
    pub user_email: Option<String>,
    /// Error message if any
    pub error: Option<String>,
}

impl Default for ClaudeCodeProvider {
    fn default() -> Self {
        Self::new()
    }
}

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
        Self::is_available()
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        // Uses your Claude Code account pricing (subscription or API)
        None
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let message = self.build_message(&request);
        debug!(message_len = message.len(), "sending message to Claude Code");

        let start = Instant::now();

        let response = self.execute_prompt(&message).await?;

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
            session_id = ?response.session_id,
            "received response from Claude Code"
        );

        Ok(ChatResponse {
            content: response.result,
            model: self.model.clone().unwrap_or_else(|| "claude-code".to_string()),
            provider: "claude-code".to_string(),
            usage: response.usage.map(|u| crate::core::llm::cost::TokenUsage {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
            }),
            finish_reason: Some("stop".to_string()),
            latency_ms,
            cost_usd: response.cost.map(|c| c.usd),
        })
    }

    async fn stream_chat(&self, _request: ChatRequest) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        // Claude Code CLI doesn't support streaming - returns full response
        warn!("streaming not supported for Claude Code provider");
        Err(LLMError::StreamingNotSupported("claude-code".to_string()))
    }

    fn supports_streaming(&self) -> bool {
        false
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
    fn test_no_streaming() {
        let provider = ClaudeCodeProvider::new();
        assert!(!provider.supports_streaming());
        assert!(!provider.supports_embeddings());
    }

    #[test]
    fn test_custom_config() {
        let provider = ClaudeCodeProvider::with_config(60, Some("claude-sonnet-4-20250514".to_string()), None);
        assert_eq!(provider.timeout_secs, 60);
        assert_eq!(provider.model, Some("claude-sonnet-4-20250514".to_string()));
    }

    #[test]
    fn test_default() {
        let provider = ClaudeCodeProvider::default();
        assert_eq!(provider.timeout_secs, 300);
        assert!(provider.model.is_none());
    }
}
