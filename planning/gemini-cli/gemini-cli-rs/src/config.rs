//! Configuration for Gemini CLI client.

use std::path::PathBuf;
use std::time::Duration;

/// Configuration for the Gemini CLI client.
#[derive(Debug, Clone)]
pub struct GeminiCliConfig {
    /// Path to the Gemini CLI binary (default: searches PATH for "gemini").
    pub binary_path: Option<PathBuf>,

    /// Working directory for Gemini CLI (default: current directory).
    pub working_dir: Option<PathBuf>,

    /// Timeout for each request (default: 5 minutes).
    pub timeout: Duration,

    /// Output format (default: JSON for structured parsing).
    pub output_format: OutputFormat,

    /// Model to use (default: let Gemini CLI decide, usually gemini-2.5-pro).
    pub model: Option<String>,

    /// Enable YOLO mode - auto-approve all tool actions (dangerous!).
    pub yolo_mode: bool,

    /// Custom system prompt via GEMINI.md content.
    pub system_prompt: Option<String>,

    /// Enable Google Search grounding.
    pub enable_search: bool,

    /// Enable verbose output for debugging.
    pub verbose: bool,

    /// Sandbox mode for safer execution.
    pub sandbox: bool,
}

impl Default for GeminiCliConfig {
    fn default() -> Self {
        Self {
            binary_path: None,
            working_dir: None,
            timeout: Duration::from_secs(300), // 5 minutes
            output_format: OutputFormat::Json,
            model: None,
            yolo_mode: false,
            system_prompt: None,
            enable_search: true,
            verbose: false,
            sandbox: false,
        }
    }
}

impl GeminiCliConfig {
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

    /// Enable YOLO mode (auto-approve all tool actions).
    /// 
    /// # Warning
    /// This is dangerous! Only use in trusted environments.
    pub fn yolo_mode(mut self, enabled: bool) -> Self {
        self.yolo_mode = enabled;
        self
    }

    /// Set a custom system prompt.
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Enable or disable Google Search grounding.
    pub fn enable_search(mut self, enabled: bool) -> Self {
        self.enable_search = enabled;
        self
    }

    /// Enable verbose output.
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Enable sandbox mode.
    pub fn sandbox(mut self, sandbox: bool) -> Self {
        self.sandbox = sandbox;
        self
    }
}

/// Output format for Gemini CLI responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Plain text output.
    Text,
    /// JSON structured output (recommended for programmatic use).
    #[default]
    Json,
    /// Streaming JSON (newline-delimited JSON events).
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
