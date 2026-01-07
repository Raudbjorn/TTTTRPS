//! Claude Desktop connection manager with lifecycle management.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use super::client::{ClaudeClient, ConnectionState, Message};
use super::config::{ClaudeConfig, CLAUDE_BINARY_PATHS};
use super::error::{ClaudeCdpError, Result};

/// Status information for the Claude Desktop connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeDesktopStatus {
    /// Current connection state.
    pub state: String,
    /// Whether connected to Claude Desktop.
    pub connected: bool,
    /// The CDP port being used.
    pub port: u16,
    /// Whether we launched Claude Desktop ourselves.
    pub launched_by_us: bool,
    /// Path to Claude binary (if detected).
    pub binary_path: Option<String>,
}

/// Manager for Claude Desktop CDP connections.
///
/// Handles connection lifecycle, auto-launch, and provides a thread-safe
/// interface for sending messages.
pub struct ClaudeDesktopManager {
    client: Arc<Mutex<ClaudeClient>>,
    config: Arc<RwLock<ClaudeConfig>>,
    process: Arc<Mutex<Option<Child>>>,
    binary_path: Arc<RwLock<Option<String>>>,
}

impl ClaudeDesktopManager {
    /// Create a new manager with default configuration.
    pub fn new() -> Self {
        Self::with_config(ClaudeConfig::default())
    }

    /// Create a new manager with custom configuration.
    pub fn with_config(config: ClaudeConfig) -> Self {
        Self {
            client: Arc::new(Mutex::new(ClaudeClient::with_config(config.clone()))),
            config: Arc::new(RwLock::new(config)),
            process: Arc::new(Mutex::new(None)),
            binary_path: Arc::new(RwLock::new(None)),
        }
    }

    /// Get the current connection status.
    pub async fn status(&self) -> ClaudeDesktopStatus {
        let client = self.client.lock().await;
        let config = self.config.read().await;
        let process = self.process.lock().await;
        let binary_path = self.binary_path.read().await;

        ClaudeDesktopStatus {
            state: format!("{:?}", client.state()),
            connected: client.is_connected(),
            port: config.port,
            launched_by_us: process.is_some(),
            binary_path: binary_path.clone(),
        }
    }

    /// Check if connected to Claude Desktop.
    pub async fn is_connected(&self) -> bool {
        let client = self.client.lock().await;
        client.is_connected()
    }

    /// Get the current connection state.
    pub async fn state(&self) -> ConnectionState {
        let client = self.client.lock().await;
        client.state()
    }

    /// Detect the Claude Desktop binary path.
    pub fn detect_claude_binary() -> Option<String> {
        for path in CLAUDE_BINARY_PATHS {
            if Path::new(path).exists() {
                info!(path, "found Claude Desktop binary");
                return Some(path.to_string());
            }
        }

        // Try using `which` command as fallback
        if let Ok(output) = Command::new("which")
            .arg("claude")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() && Path::new(&path).exists() {
                    info!(path, "found Claude Desktop binary via which");
                    return Some(path);
                }
            }
        }

        warn!("Claude Desktop binary not found");
        None
    }

    /// Connect to a running Claude Desktop instance.
    pub async fn connect(&self) -> Result<()> {
        let mut client = self.client.lock().await;
        client.connect().await
    }

    /// Connect to Claude Desktop, or launch it if not running.
    pub async fn connect_or_launch(&self) -> Result<()> {
        // First, try to connect to existing instance
        info!("attempting to connect to Claude Desktop");
        let connect_result = self.connect().await;

        if connect_result.is_ok() {
            info!("connected to existing Claude Desktop instance");
            return Ok(());
        }

        // Connection failed, try to launch
        info!("no running Claude Desktop found, attempting to launch");

        let binary_path = Self::detect_claude_binary().ok_or_else(|| {
            ClaudeCdpError::BinaryNotFound {
                paths: CLAUDE_BINARY_PATHS.iter().map(|s| s.to_string()).collect(),
            }
        })?;

        // Store the binary path
        {
            let mut path = self.binary_path.write().await;
            *path = Some(binary_path.clone());
        }

        // Launch Claude Desktop with CDP enabled
        self.launch(&binary_path).await?;

        // Wait for it to start and connect
        self.wait_and_connect().await
    }

    /// Launch Claude Desktop with CDP enabled.
    ///
    /// Note: stdout/stderr are discarded to avoid blocking on output.
    /// If debugging launch issues, run Claude Desktop manually with:
    /// `claude --remote-debugging-port=9333`
    async fn launch(&self, binary_path: &str) -> Result<()> {
        let config = self.config.read().await;
        let port = config.port;

        info!(binary_path, port, "launching Claude Desktop with CDP");

        let child = Command::new(binary_path)
            .arg(format!("--remote-debugging-port={}", port))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| ClaudeCdpError::LaunchFailed {
                reason: e.to_string(),
            })?;

        let pid = child.id();
        debug!(pid, "Claude Desktop process spawned");

        let mut process = self.process.lock().await;
        *process = Some(child);

        info!(pid, "Claude Desktop launched successfully");
        Ok(())
    }

    /// Wait for Claude Desktop to be ready and connect.
    async fn wait_and_connect(&self) -> Result<()> {
        const MAX_ATTEMPTS: u32 = 30;
        const POLL_INTERVAL: Duration = Duration::from_millis(500);

        info!("waiting for Claude Desktop to be ready");

        for attempt in 1..=MAX_ATTEMPTS {
            sleep(POLL_INTERVAL).await;

            debug!(attempt, max = MAX_ATTEMPTS, "connection attempt");

            let result = self.connect().await;
            if result.is_ok() {
                info!(attempt, "connected to Claude Desktop");
                return Ok(());
            }
        }

        error!("Claude Desktop did not become ready within timeout");
        Err(ClaudeCdpError::ResponseTimeout {
            seconds: (MAX_ATTEMPTS as u64 * POLL_INTERVAL.as_millis() as u64) / 1000,
        })
    }

    /// Disconnect from Claude Desktop.
    pub async fn disconnect(&self) {
        let mut client = self.client.lock().await;
        client.disconnect().await;
    }

    /// Disconnect and kill the launched Claude Desktop process (if we launched it).
    pub async fn disconnect_and_kill(&self) {
        self.disconnect().await;

        let mut process = self.process.lock().await;
        if let Some(mut child) = process.take() {
            info!("killing Claude Desktop process");
            if let Err(e) = child.kill() {
                error!(error = %e, "failed to kill Claude Desktop process");
            }
        }
    }

    /// Send a message to Claude and wait for response.
    pub async fn send_message(&self, message: &str) -> Result<String> {
        let client = self.client.lock().await;
        client.send_message(message).await
    }

    /// Start a new conversation.
    pub async fn new_conversation(&self) -> Result<()> {
        let client = self.client.lock().await;
        client.new_conversation().await
    }

    /// Get the current conversation history.
    pub async fn get_conversation(&self) -> Result<Vec<Message>> {
        let client = self.client.lock().await;
        client.get_conversation().await
    }

    /// Update the configuration (requires reconnection).
    pub async fn update_config(&self, port: Option<u16>, timeout_secs: Option<u64>) {
        let mut config = self.config.write().await;

        if let Some(p) = port {
            config.port = p;
        }
        if let Some(t) = timeout_secs {
            config.response_timeout_secs = t;
        }

        // Update the client with new config
        let mut client = self.client.lock().await;
        *client = ClaudeClient::with_config(config.clone());

        info!(port = config.port, timeout = config.response_timeout_secs, "configuration updated");
    }

    /// Health check - verify CDP connection is still valid.
    pub async fn health_check(&self) -> bool {
        let client = self.client.lock().await;
        if !client.is_connected() {
            return false;
        }

        // Try to get conversation as a health check
        match client.get_conversation().await {
            Ok(_) => true,
            Err(e) => {
                debug!(error = %e, "health check failed");
                false
            }
        }
    }
}

impl Default for ClaudeDesktopManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ClaudeDesktopManager {
    fn drop(&mut self) {
        // Try to kill the process if we launched it
        if let Ok(mut process) = self.process.try_lock() {
            if let Some(mut child) = process.take() {
                let _ = child.kill();
            }
        }
    }
}
