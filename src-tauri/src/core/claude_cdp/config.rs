//! Configuration for the Claude Desktop CDP bridge.

use serde::{Deserialize, Serialize};

/// Default CDP port for Claude Desktop (non-standard to avoid Chrome conflicts).
pub const DEFAULT_CDP_PORT: u16 = 9333;

/// Default response timeout in seconds.
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Default polling interval when waiting for responses.
pub const DEFAULT_POLL_INTERVAL_MS: u64 = 500;

/// Maximum allowed timeout in seconds.
pub const MAX_TIMEOUT_SECS: u64 = 600;

/// Minimum allowed port (avoid privileged ports).
pub const MIN_PORT: u16 = 1024;

/// Known paths where Claude Desktop binary might be installed (Linux).
#[cfg(target_os = "linux")]
pub const CLAUDE_BINARY_PATHS: &[&str] = &[
    "/opt/Claude/claude",           // Arch Linux (AUR)
    "/opt/claude-desktop/claude",   // Alternative Linux
    "/usr/bin/claude",              // System install
    "/usr/local/bin/claude",        // Local install
];

/// Known paths where Claude Desktop binary might be installed (macOS).
#[cfg(target_os = "macos")]
pub const CLAUDE_BINARY_PATHS: &[&str] = &[
    "/Applications/Claude.app/Contents/MacOS/Claude",
    "/usr/local/bin/claude",
];

/// Known paths where Claude Desktop binary might be installed (Windows).
#[cfg(target_os = "windows")]
pub const CLAUDE_BINARY_PATHS: &[&str] = &[
    r"C:\Program Files\Claude\Claude.exe",
    r"C:\Program Files (x86)\Claude\Claude.exe",
];

/// Fallback for other platforms.
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub const CLAUDE_BINARY_PATHS: &[&str] = &[];

/// Configuration for connecting to Claude Desktop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeConfig {
    /// The CDP debugging port (default: 9333).
    pub port: u16,
    /// Host address (default: "127.0.0.1").
    pub host: String,
    /// Timeout in seconds for waiting for responses.
    pub response_timeout_secs: u64,
    /// Polling interval in milliseconds when waiting for responses.
    pub poll_interval_ms: u64,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_CDP_PORT,
            host: "127.0.0.1".to_string(),
            response_timeout_secs: DEFAULT_TIMEOUT_SECS,
            poll_interval_ms: DEFAULT_POLL_INTERVAL_MS,
        }
    }
}

impl ClaudeConfig {
    /// Create a new config with a custom port.
    ///
    /// Port must be >= 1024 (non-privileged). Values below MIN_PORT are clamped.
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port.max(MIN_PORT);
        self
    }

    /// Create a new config with a custom timeout.
    ///
    /// Timeout is clamped to 1..=MAX_TIMEOUT_SECS (600 seconds).
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.response_timeout_secs = secs.clamp(1, MAX_TIMEOUT_SECS);
        self
    }

    /// Get the WebSocket debugger URL for chromiumoxide.
    pub fn ws_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    /// Get the CDP endpoint URL.
    pub fn cdp_url(&self) -> String {
        format!("http://{}:{}/json", self.host, self.port)
    }
}
