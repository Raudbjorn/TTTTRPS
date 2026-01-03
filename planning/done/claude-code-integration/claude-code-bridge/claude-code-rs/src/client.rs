//! Claude Code client for programmatic CLI interaction.

use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, error, info, instrument, warn};

use crate::config::{ClaudeCodeConfig, PermissionMode};
use crate::error::{ClaudeCodeError, Result};
use crate::output::{parse_response, ClaudeResponse};

/// Client for interacting with Claude Code CLI.
#[derive(Debug, Clone)]
pub struct ClaudeCodeClient {
    config: ClaudeCodeConfig,
    binary_path: PathBuf,
}

impl ClaudeCodeClient {
    /// Create a new client with default configuration.
    pub fn new() -> Result<Self> {
        Self::with_config(ClaudeCodeConfig::default())
    }

    /// Create a new client with custom configuration.
    pub fn with_config(config: ClaudeCodeConfig) -> Result<Self> {
        let binary_path = config
            .binary_path
            .clone()
            .or_else(|| which::which("claude").ok())
            .ok_or(ClaudeCodeError::NotFound)?;

        debug!(binary = %binary_path.display(), "found Claude Code binary");

        Ok(Self {
            config,
            binary_path,
        })
    }

    /// Get the current configuration.
    pub fn config(&self) -> &ClaudeCodeConfig {
        &self.config
    }

    /// Update the configuration.
    pub fn set_config(&mut self, config: ClaudeCodeConfig) -> Result<()> {
        if let Some(ref path) = config.binary_path {
            if !path.exists() {
                return Err(ClaudeCodeError::NotFound);
            }
            self.binary_path = path.clone();
        }
        self.config = config;
        Ok(())
    }

    /// Send a prompt to Claude Code and get a response.
    #[instrument(skip(self, prompt), fields(prompt_len = prompt.len()))]
    pub async fn prompt(&self, prompt: &str) -> Result<ClaudeResponse> {
        self.execute(prompt, None).await
    }

    /// Continue the most recent conversation.
    #[instrument(skip(self, prompt), fields(prompt_len = prompt.len()))]
    pub async fn continue_conversation(&self, prompt: &str) -> Result<ClaudeResponse> {
        self.execute_with_flags(prompt, None, &["--continue"]).await
    }

    /// Resume a specific conversation by session ID.
    #[instrument(skip(self, prompt), fields(prompt_len = prompt.len(), session_id = %session_id))]
    pub async fn resume(&self, prompt: &str, session_id: &str) -> Result<ClaudeResponse> {
        // Validate session ID format (basic validation)
        if session_id.is_empty() || session_id.contains(' ') {
            return Err(ClaudeCodeError::InvalidSessionId(session_id.to_string()));
        }

        self.execute_with_flags(prompt, None, &["--resume", session_id])
            .await
    }

    /// Execute a prompt with optional files as context.
    #[instrument(skip(self, prompt, files), fields(prompt_len = prompt.len(), file_count = files.as_ref().map(|f| f.len()).unwrap_or(0)))]
    pub async fn execute(
        &self,
        prompt: &str,
        files: Option<&[PathBuf]>,
    ) -> Result<ClaudeResponse> {
        self.execute_with_flags(prompt, files, &[]).await
    }

    /// Execute a prompt with additional CLI flags.
    async fn execute_with_flags(
        &self,
        prompt: &str,
        files: Option<&[PathBuf]>,
        extra_flags: &[&str],
    ) -> Result<ClaudeResponse> {
        let mut cmd = Command::new(&self.binary_path);

        // Core flags for non-interactive mode
        cmd.arg("-p").arg(prompt);
        cmd.arg("--output-format").arg(self.config.output_format.as_str());

        // Optional configuration
        if let Some(ref model) = self.config.model {
            cmd.arg("--model").arg(model);
        }

        if let Some(tokens) = self.config.max_tokens {
            cmd.arg("--max-tokens").arg(tokens.to_string());
        }

        if let Some(ref system) = self.config.system_prompt {
            cmd.arg("--system-prompt").arg(system);
        }

        if let Some(ref tools) = self.config.allowed_tools {
            for tool in tools {
                cmd.arg("--allowedTools").arg(tool);
            }
        }

        if let Some(ref tools) = self.config.disallowed_tools {
            for tool in tools {
                cmd.arg("--disallowedTools").arg(tool);
            }
        }

        if let Some(ref mcp_config) = self.config.mcp_config {
            cmd.arg("--mcp-config").arg(mcp_config);
        }

        // Permission mode
        match self.config.permission_mode {
            PermissionMode::AcceptAll => {
                cmd.arg("--dangerously-skip-permissions");
            }
            PermissionMode::RejectAll => {
                // No flag - will fail on permission requests in non-interactive mode
            }
            PermissionMode::Default => {}
        }

        if self.config.verbose {
            cmd.arg("--verbose");
        }

        // Add extra flags
        for flag in extra_flags {
            cmd.arg(flag);
        }

        // Add files
        if let Some(file_list) = files {
            for file in file_list {
                cmd.arg(file);
            }
        }

        // Working directory
        if let Some(ref dir) = self.config.working_dir {
            if !dir.exists() {
                return Err(ClaudeCodeError::WorkingDirNotAccessible {
                    path: dir.display().to_string(),
                });
            }
            cmd.current_dir(dir);
        }

        // Configure process
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!(command = ?cmd, "executing Claude Code");

        // Spawn and wait with timeout
        let mut child = cmd.spawn().map_err(ClaudeCodeError::SpawnFailed)?;

        let result = timeout(self.config.timeout, async {
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
                    // Check if stderr contains useful error info
                    let error_msg = if !stderr.is_empty() {
                        stderr.clone()
                    } else if !stdout.is_empty() {
                        // Sometimes errors are in stdout for JSON mode
                        stdout.clone()
                    } else {
                        "unknown error".to_string()
                    };

                    error!(status = %status, error = %error_msg, "Claude Code failed");
                    return Err(ClaudeCodeError::ProcessFailed {
                        status,
                        stdout,
                        stderr,
                    });
                }

                // Parse the output
                parse_response(&stdout).map_err(|e| {
                    warn!(error = %e, output = %stdout, "failed to parse response");
                    e
                })
            }
            Ok(Err(io_err)) => {
                error!(error = %io_err, "I/O error during Claude Code execution");
                Err(ClaudeCodeError::IoError(io_err))
            }
            Err(_) => {
                // Timeout - try to kill the process
                warn!(
                    timeout_secs = self.config.timeout.as_secs(),
                    "Claude Code request timed out"
                );
                let _ = child.kill().await;
                Err(ClaudeCodeError::Timeout {
                    seconds: self.config.timeout.as_secs(),
                })
            }
        }
    }

    /// Check if Claude Code is available and working.
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> Result<bool> {
        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("--version");
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        match cmd.output().await {
            Ok(output) => {
                let success = output.status.success();
                if success {
                    let version = String::from_utf8_lossy(&output.stdout);
                    info!(version = %version.trim(), "Claude Code health check passed");
                } else {
                    warn!("Claude Code health check failed");
                }
                Ok(success)
            }
            Err(e) => {
                error!(error = %e, "Claude Code health check failed");
                Err(ClaudeCodeError::SpawnFailed(e))
            }
        }
    }

    /// Get Claude Code version.
    #[instrument(skip(self))]
    pub async fn version(&self) -> Result<String> {
        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("--version");
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd.output().await.map_err(ClaudeCodeError::SpawnFailed)?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(ClaudeCodeError::ProcessFailed {
                status: output.status,
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }
}

impl Default for ClaudeCodeClient {
    fn default() -> Self {
        Self::new().expect("Claude Code CLI not found")
    }
}

/// Builder for creating a configured ClaudeCodeClient.
#[derive(Debug, Default)]
pub struct ClaudeCodeClientBuilder {
    config: ClaudeCodeConfig,
}

impl ClaudeCodeClientBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the binary path.
    pub fn binary_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config = self.config.binary_path(path);
        self
    }

    /// Set the working directory.
    pub fn working_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.config = self.config.working_dir(path);
        self
    }

    /// Set the timeout in seconds.
    pub fn timeout_secs(mut self, secs: u64) -> Self {
        self.config = self.config.timeout_secs(secs);
        self
    }

    /// Set the model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.config = self.config.model(model);
        self
    }

    /// Set max tokens.
    pub fn max_tokens(mut self, tokens: u32) -> Self {
        self.config = self.config.max_tokens(tokens);
        self
    }

    /// Set the system prompt.
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.config = self.config.system_prompt(prompt);
        self
    }

    /// Set permission mode to accept all (dangerous!).
    pub fn accept_all_permissions(mut self) -> Self {
        self.config = self.config.permission_mode(PermissionMode::AcceptAll);
        self
    }

    /// Enable verbose mode.
    pub fn verbose(mut self) -> Self {
        self.config = self.config.verbose(true);
        self
    }

    /// Build the client.
    pub fn build(self) -> Result<ClaudeCodeClient> {
        ClaudeCodeClient::with_config(self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        let builder = ClaudeCodeClientBuilder::new()
            .timeout_secs(60)
            .model("claude-3-sonnet")
            .verbose();

        assert_eq!(builder.config.timeout.as_secs(), 60);
        assert_eq!(builder.config.model, Some("claude-3-sonnet".to_string()));
        assert!(builder.config.verbose);
    }
}
