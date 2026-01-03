//! Gemini CLI client for programmatic CLI interaction.

use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, error, info, instrument, warn};

use crate::config::{GeminiCliConfig, OutputFormat};
use crate::error::{GeminiCliError, Result};
use crate::output::{parse_response, GeminiResponse};

/// Client for interacting with Gemini CLI.
#[derive(Debug, Clone)]
pub struct GeminiCliClient {
    config: GeminiCliConfig,
    binary_path: PathBuf,
}

impl GeminiCliClient {
    /// Create a new client with default configuration.
    pub fn new() -> Result<Self> {
        Self::with_config(GeminiCliConfig::default())
    }

    /// Create a new client with custom configuration.
    pub fn with_config(config: GeminiCliConfig) -> Result<Self> {
        let binary_path = config
            .binary_path
            .clone()
            .or_else(|| which::which("gemini").ok())
            .ok_or(GeminiCliError::NotFound)?;

        debug!(binary = %binary_path.display(), "found Gemini CLI binary");

        Ok(Self {
            config,
            binary_path,
        })
    }

    /// Get the current configuration.
    pub fn config(&self) -> &GeminiCliConfig {
        &self.config
    }

    /// Update the configuration.
    pub fn set_config(&mut self, config: GeminiCliConfig) -> Result<()> {
        if let Some(ref path) = config.binary_path {
            if !path.exists() {
                return Err(GeminiCliError::NotFound);
            }
            self.binary_path = path.clone();
        }
        self.config = config;
        Ok(())
    }

    /// Send a prompt to Gemini CLI and get a response.
    #[instrument(skip(self, prompt), fields(prompt_len = prompt.len()))]
    pub async fn prompt(&self, prompt: &str) -> Result<GeminiResponse> {
        self.execute(prompt, None).await
    }

    /// Send a prompt with stdin input (e.g., file contents).
    #[instrument(skip(self, prompt, stdin_input), fields(prompt_len = prompt.len(), stdin_len = stdin_input.len()))]
    pub async fn prompt_with_stdin(&self, prompt: &str, stdin_input: &str) -> Result<GeminiResponse> {
        self.execute_with_stdin(prompt, None, Some(stdin_input)).await
    }

    /// Execute a prompt with optional files as context.
    #[instrument(skip(self, prompt, files), fields(prompt_len = prompt.len(), file_count = files.as_ref().map(|f| f.len()).unwrap_or(0)))]
    pub async fn execute(
        &self,
        prompt: &str,
        files: Option<&[PathBuf]>,
    ) -> Result<GeminiResponse> {
        self.execute_with_stdin(prompt, files, None).await
    }

    /// Execute a prompt with optional files and stdin input.
    async fn execute_with_stdin(
        &self,
        prompt: &str,
        files: Option<&[PathBuf]>,
        stdin_input: Option<&str>,
    ) -> Result<GeminiResponse> {
        let mut cmd = Command::new(&self.binary_path);

        // Headless mode with prompt
        cmd.arg("-p").arg(prompt);

        // Output format
        cmd.arg("--output-format").arg(self.config.output_format.as_str());

        // Optional model
        if let Some(ref model) = self.config.model {
            cmd.arg("--model").arg(model);
        }

        // YOLO mode (auto-approve all tool actions)
        if self.config.yolo_mode {
            cmd.arg("--yolo");
        }

        // Sandbox mode
        if self.config.sandbox {
            cmd.arg("--sandbox");
        }

        // Verbose mode
        if self.config.verbose {
            cmd.arg("--verbose");
        }

        // Add files as positional arguments
        if let Some(file_list) = files {
            for file in file_list {
                cmd.arg(file);
            }
        }

        // Working directory
        if let Some(ref dir) = self.config.working_dir {
            if !dir.exists() {
                return Err(GeminiCliError::WorkingDirNotAccessible {
                    path: dir.display().to_string(),
                });
            }
            cmd.current_dir(dir);
        }

        // Configure stdin
        if stdin_input.is_some() {
            cmd.stdin(Stdio::piped());
        } else {
            cmd.stdin(Stdio::null());
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        debug!(command = ?cmd, "executing Gemini CLI");

        // Spawn the process
        let mut child = cmd.spawn().map_err(GeminiCliError::SpawnFailed)?;

        // Write stdin if provided
        if let Some(input) = stdin_input {
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                stdin.write_all(input.as_bytes()).await?;
                stdin.flush().await?;
                drop(stdin);
            }
        }

        // Wait with timeout
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
                    "Gemini CLI process completed"
                );

                // Parse the response
                let response = parse_response(&stdout)?;

                // Check for errors in the response
                if let Some(ref err) = response.error {
                    // Check for authentication errors
                    if err.error_type.contains("Authentication") {
                        return Err(GeminiCliError::AuthenticationError {
                            message: err.message.clone(),
                        });
                    }

                    // Check for rate limit errors
                    if err.error_type.contains("RateLimit") || err.message.contains("rate limit") {
                        return Err(GeminiCliError::RateLimitError {
                            message: err.message.clone(),
                        });
                    }

                    // Generic Gemini error
                    return Err(GeminiCliError::GeminiError {
                        error_type: err.error_type.clone(),
                        message: err.message.clone(),
                    });
                }

                // Check process exit status (even if we got a response)
                if !status.success() && response.response.is_none() {
                    error!(status = %status, stderr = %stderr, "Gemini CLI failed");
                    return Err(GeminiCliError::ProcessFailed {
                        status,
                        stdout,
                        stderr,
                    });
                }

                info!(
                    response_len = response.response.as_ref().map(|r| r.len()).unwrap_or(0),
                    "received response from Gemini CLI"
                );

                Ok(response)
            }
            Ok(Err(io_err)) => {
                error!(error = %io_err, "I/O error during Gemini CLI execution");
                Err(GeminiCliError::IoError(io_err))
            }
            Err(_) => {
                warn!(
                    timeout_secs = self.config.timeout.as_secs(),
                    "Gemini CLI request timed out"
                );
                let _ = child.kill().await;
                Err(GeminiCliError::Timeout {
                    seconds: self.config.timeout.as_secs(),
                })
            }
        }
    }

    /// Check if Gemini CLI is available and working.
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
                    info!(version = %version.trim(), "Gemini CLI health check passed");
                } else {
                    warn!("Gemini CLI health check failed");
                }
                Ok(success)
            }
            Err(e) => {
                error!(error = %e, "Gemini CLI health check failed");
                Err(GeminiCliError::SpawnFailed(e))
            }
        }
    }

    /// Get Gemini CLI version.
    #[instrument(skip(self))]
    pub async fn version(&self) -> Result<String> {
        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("--version");
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd.output().await.map_err(GeminiCliError::SpawnFailed)?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(GeminiCliError::ProcessFailed {
                status: output.status,
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }

    /// Execute a prompt with streaming output.
    /// 
    /// Returns an async stream of events. Use `OutputFormat::StreamJson` for best results.
    #[instrument(skip(self, prompt), fields(prompt_len = prompt.len()))]
    pub async fn prompt_streaming(
        &self,
        prompt: &str,
    ) -> Result<impl tokio::io::AsyncBufRead + Send> {
        let mut cmd = Command::new(&self.binary_path);

        cmd.arg("-p").arg(prompt);
        cmd.arg("--output-format").arg("stream-json");

        if let Some(ref model) = self.config.model {
            cmd.arg("--model").arg(model);
        }

        if self.config.yolo_mode {
            cmd.arg("--yolo");
        }

        if let Some(ref dir) = self.config.working_dir {
            if !dir.exists() {
                return Err(GeminiCliError::WorkingDirNotAccessible {
                    path: dir.display().to_string(),
                });
            }
            cmd.current_dir(dir);
        }

        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd.spawn().map_err(GeminiCliError::SpawnFailed)?;

        let stdout = child.stdout.ok_or_else(|| {
            GeminiCliError::IoError(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "failed to capture stdout",
            ))
        })?;

        Ok(tokio::io::BufReader::new(stdout))
    }
}

impl Default for GeminiCliClient {
    fn default() -> Self {
        Self::new().expect("Gemini CLI not found")
    }
}

/// Builder for creating a configured GeminiCliClient.
#[derive(Debug, Default)]
pub struct GeminiCliClientBuilder {
    config: GeminiCliConfig,
}

impl GeminiCliClientBuilder {
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

    /// Enable YOLO mode (auto-approve all tool actions).
    /// 
    /// # Warning
    /// This is dangerous! Only use in trusted environments.
    pub fn yolo_mode(mut self) -> Self {
        self.config = self.config.yolo_mode(true);
        self
    }

    /// Enable verbose mode.
    pub fn verbose(mut self) -> Self {
        self.config = self.config.verbose(true);
        self
    }

    /// Enable sandbox mode.
    pub fn sandbox(mut self) -> Self {
        self.config = self.config.sandbox(true);
        self
    }

    /// Build the client.
    pub fn build(self) -> Result<GeminiCliClient> {
        GeminiCliClient::with_config(self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        let builder = GeminiCliClientBuilder::new()
            .timeout_secs(60)
            .model("gemini-2.5-flash")
            .verbose();

        assert_eq!(builder.config.timeout.as_secs(), 60);
        assert_eq!(builder.config.model, Some("gemini-2.5-flash".to_string()));
        assert!(builder.config.verbose);
    }
}
