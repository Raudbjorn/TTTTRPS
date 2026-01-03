//! Configuration for Claude Code client.

use std::path::PathBuf;
use std::time::Duration;

/// Configuration for the Claude Code client.
#[derive(Debug, Clone)]
pub struct ClaudeCodeConfig {
    /// Path to the Claude Code binary (default: searches PATH for "claude").
    pub binary_path: Option<PathBuf>,

    /// Working directory for Claude Code (default: current directory).
    pub working_dir: Option<PathBuf>,

    /// Timeout for each request (default: 5 minutes).
    pub timeout: Duration,

    /// Output format (default: JSON for structured parsing).
    pub output_format: OutputFormat,

    /// Model to use (default: let Claude Code decide).
    pub model: Option<String>,

    /// Maximum tokens for response (default: let Claude Code decide).
    pub max_tokens: Option<u32>,

    /// System prompt override.
    pub system_prompt: Option<String>,

    /// Allowed tools filter.
    pub allowed_tools: Option<Vec<String>>,

    /// Disallowed tools filter.
    pub disallowed_tools: Option<Vec<String>>,

    /// MCP configuration file path.
    pub mcp_config: Option<PathBuf>,

    /// Permission mode for tool calls.
    pub permission_mode: PermissionMode,

    /// Enable verbose output for debugging.
    pub verbose: bool,
}

impl Default for ClaudeCodeConfig {
    fn default() -> Self {
        Self {
            binary_path: None,
            working_dir: None,
            timeout: Duration::from_secs(300), // 5 minutes
            output_format: OutputFormat::Json,
            model: None,
            max_tokens: None,
            system_prompt: None,
            allowed_tools: None,
            disallowed_tools: None,
            mcp_config: None,
            permission_mode: PermissionMode::Default,
            verbose: false,
        }
    }
}

impl ClaudeCodeConfig {
    /// Create a new config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the binary path.
    pub fn binary_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.binary_path = Some(path.into());
        self
    }

    /// Set the working directory.
    pub fn working_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(path.into());
        self
    }

    /// Set the timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set timeout in seconds.
    pub fn timeout_secs(mut self, secs: u64) -> Self {
        self.timeout = Duration::from_secs(secs);
        self
    }

    /// Set the output format.
    pub fn output_format(mut self, format: OutputFormat) -> Self {
        self.output_format = format;
        self
    }

    /// Set the model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set max tokens.
    pub fn max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Set the system prompt.
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set allowed tools.
    pub fn allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = Some(tools);
        self
    }

    /// Set disallowed tools.
    pub fn disallowed_tools(mut self, tools: Vec<String>) -> Self {
        self.disallowed_tools = Some(tools);
        self
    }

    /// Set MCP config path.
    pub fn mcp_config(mut self, path: impl Into<PathBuf>) -> Self {
        self.mcp_config = Some(path.into());
        self
    }

    /// Set permission mode.
    pub fn permission_mode(mut self, mode: PermissionMode) -> Self {
        self.permission_mode = mode;
        self
    }

    /// Enable verbose output.
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

/// Output format for Claude Code responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Plain text output.
    Text,
    /// JSON structured output (recommended for programmatic use).
    #[default]
    Json,
    /// Stream JSON objects as they arrive.
    StreamJson,
}

impl OutputFormat {
    /// Convert to CLI flag value.
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputFormat::Text => "text",
            OutputFormat::Json => "json",
            OutputFormat::StreamJson => "stream-json",
        }
    }
}

/// Permission mode for tool execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionMode {
    /// Default behavior - may prompt for permissions.
    #[default]
    Default,
    /// Accept all tool calls automatically (use with caution!).
    AcceptAll,
    /// Reject all tool calls that require permission.
    RejectAll,
}
