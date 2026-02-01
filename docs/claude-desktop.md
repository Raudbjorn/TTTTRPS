# Claude Desktop CDP Provider

> **⚠️ ARCHIVED**: This feature has been removed from the codebase. This documentation is preserved for historical reference only. The `claude_cdp/` module, `ClaudeDesktopProvider`, and all related Tauri commands have been deleted. See PR #55 for details.

> Complete implementation guide for integrating with Claude Desktop via Chrome DevTools Protocol (CDP)

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [How CDP Works](#how-cdp-works)
4. [Core Implementation](#core-implementation)
   - [Module Structure](#module-structure)
   - [Configuration](#configuration)
   - [CDP Client](#cdp-client)
   - [Desktop Manager](#desktop-manager)
   - [Error Handling](#error-handling)
5. [LLM Provider Integration](#llm-provider-integration)
6. [Tauri Commands](#tauri-commands)
7. [Frontend Integration](#frontend-integration)
8. [Binary Detection](#binary-detection)
9. [DOM Interaction via JavaScript](#dom-interaction-via-javascript)
10. [Session Lifecycle](#session-lifecycle)
11. [Implementation Guide](#implementation-guide)
12. [Troubleshooting](#troubleshooting)
13. [Limitations](#limitations)

---

## Overview

The Claude Desktop CDP Provider enables communication with Claude Desktop (Anthropic's Electron-based desktop application) through the Chrome DevTools Protocol. This approach leverages your existing Claude Desktop subscription without requiring separate API credits.

### Key Benefits

- **No API Costs**: Uses your Claude Pro/Team subscription
- **Full Claude Access**: Same models available in Claude Desktop
- **Persistent Authentication**: Inherits Claude Desktop's logged-in session
- **Desktop Integration**: Works alongside your normal Claude Desktop usage

### How It Works

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Your Application                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────┐     ┌──────────────────┐     ┌────────────────────┐   │
│  │ ClaudeDesktop   │────▶│ ClaudeDesktop    │────▶│ ClaudeClient       │   │
│  │ Provider        │     │ Manager          │     │ (CDP Client)       │   │
│  │ (LLMProvider)   │     │ (Lifecycle)      │     │ (chromiumoxide)    │   │
│  └─────────────────┘     └──────────────────┘     └────────────────────┘   │
│                                                              │              │
└──────────────────────────────────────────────────────────────┼──────────────┘
                                                               │
                                                               ▼
                                              ┌─────────────────────────────┐
                                              │     Chrome DevTools         │
                                              │     Protocol (CDP)          │
                                              │     WebSocket Connection    │
                                              └─────────────────────────────┘
                                                               │
                                                               ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                           Claude Desktop (Electron)                          │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                     Chromium Renderer Process                          │ │
│  ├────────────────────────────────────────────────────────────────────────┤ │
│  │  ┌──────────────┐  ┌───────────────────┐  ┌────────────────────────┐  │ │
│  │  │ Chat Input   │  │ Message Display   │  │ DOM Elements           │  │ │
│  │  │ [textarea]   │  │ .font-claude-msg  │  │ (CSS Selectors)        │  │ │
│  │  │ contentEdit. │  │ [data-streaming]  │  │                        │  │ │
│  │  └──────────────┘  └───────────────────┘  └────────────────────────┘  │ │
│  │                                                                        │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌─────────────────────────┐                                                │
│  │ --remote-debugging-port │                                                │
│  │         =9333           │◀─── Enabled at launch                          │
│  └─────────────────────────┘                                                │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Architecture

### Communication Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Message Send Flow                                 │
└─────────────────────────────────────────────────────────────────────────────┘

  User Input                    CDP Commands                     DOM Operations
      │                              │                                  │
      ▼                              ▼                                  ▼
┌───────────┐   chat()    ┌──────────────────┐  evaluate()   ┌───────────────┐
│ ChatRequest│───────────▶│ ClaudeDesktop    │──────────────▶│ JavaScript    │
│           │             │ Provider         │               │ Injection     │
└───────────┘             └──────────────────┘               └───────────────┘
                                  │                                  │
                                  │                                  ▼
                                  │                          ┌───────────────┐
                                  │                          │ Fill Input    │
                                  │                          │ Click Send    │
                                  │                          │ Poll Response │
                                  │                          └───────────────┘
                                  │                                  │
                                  │◀─────────────────────────────────┘
                                  │
                                  ▼
                          ┌───────────────┐
                          │ ChatResponse  │
                          │ (Full text)   │
                          └───────────────┘
```

### Module Hierarchy

```
src-tauri/src/core/
├── claude_cdp/
│   ├── mod.rs         # Module re-exports
│   ├── config.rs      # Configuration constants and ClaudeConfig
│   ├── client.rs      # Low-level CDP client (ClaudeClient)
│   ├── manager.rs     # High-level manager with lifecycle (ClaudeDesktopManager)
│   └── error.rs       # Error types (ClaudeCdpError)
│
└── llm/providers/
    └── claude_desktop.rs  # LLMProvider trait implementation
```

---

## How CDP Works

### Chrome DevTools Protocol Basics

CDP is the protocol Chrome uses for its DevTools. Electron apps (like Claude Desktop) inherit this capability since they embed Chromium.

**Key Concepts:**

1. **WebSocket Connection**: CDP uses WebSocket for real-time bidirectional communication
2. **JSON-RPC**: Commands and events use JSON-RPC format
3. **Domains**: Functionality is organized into domains (Runtime, DOM, Page, etc.)
4. **Targets**: Each tab/page is a "target" you can connect to

### Enabling CDP in Claude Desktop

Claude Desktop must be launched with the `--remote-debugging-port` flag:

```bash
# Linux
/opt/Claude/claude --remote-debugging-port=9333

# macOS
/Applications/Claude.app/Contents/MacOS/Claude --remote-debugging-port=9333

# Windows
"C:\Program Files\Claude\Claude.exe" --remote-debugging-port=9333
```

### CDP Endpoints

Once enabled, these HTTP endpoints become available:

| Endpoint | Description |
|----------|-------------|
| `http://127.0.0.1:9333/json` | List all available targets |
| `http://127.0.0.1:9333/json/version` | Browser version info |
| `http://127.0.0.1:9333/json/new` | Open new tab |
| `ws://127.0.0.1:9333/devtools/page/{id}` | WebSocket for specific page |

### chromiumoxide Crate

This implementation uses the `chromiumoxide` crate for CDP communication:

```toml
# Cargo.toml
[dependencies]
chromiumoxide = { version = "0.7", default-features = false, features = ["tokio-runtime"] }
```

**Why chromiumoxide?**

- Pure Rust implementation
- Async/await native with Tokio
- Type-safe CDP command wrappers
- Automatic reconnection handling
- Well-maintained and documented

---

## Core Implementation

### Module Structure

```rust
// src-tauri/src/core/claude_cdp/mod.rs

//! Claude Desktop CDP Bridge
//!
//! Enables communication with Claude Desktop via Chrome DevTools Protocol (CDP).

mod client;
mod config;
mod error;
mod manager;

pub use client::{ClaudeClient, ConnectionState, Message};
pub use config::{ClaudeConfig, DEFAULT_CDP_PORT, DEFAULT_TIMEOUT_SECS, CLAUDE_BINARY_PATHS};
pub use error::{ClaudeCdpError, Result};
pub use manager::{ClaudeDesktopManager, ClaudeDesktopStatus};
```

### Configuration

The configuration module defines constants and the `ClaudeConfig` struct:

```rust
// src-tauri/src/core/claude_cdp/config.rs

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
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port.max(MIN_PORT);
        self
    }

    /// Create a new config with a custom timeout.
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
```

### CDP Client

The client handles the low-level CDP communication:

```rust
// src-tauri/src/core/claude_cdp/client.rs

use std::time::Duration;
use chromiumoxide::{Browser, Page};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, instrument, warn};

use super::config::ClaudeConfig;
use super::error::{ClaudeCdpError, Result};

/// A message sent to or received from Claude.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: Option<String>,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
            timestamp: Some(Self::now()),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            timestamp: Some(Self::now()),
        }
    }

    fn now() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        format!("{}000", duration.as_secs())
    }
}

/// Connection state for the Claude Desktop bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

/// The Claude Desktop CDP bridge client.
pub struct ClaudeClient {
    config: ClaudeConfig,
    browser: Option<Browser>,
    page: Option<Page>,
    state: ConnectionState,
}

impl ClaudeClient {
    pub fn new() -> Self {
        Self::with_config(ClaudeConfig::default())
    }

    pub fn with_config(config: ClaudeConfig) -> Self {
        Self {
            config,
            browser: None,
            page: None,
            state: ConnectionState::Disconnected,
        }
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    pub fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected && self.page.is_some()
    }

    /// Connect to Claude Desktop via CDP.
    #[instrument(skip(self), fields(port = self.config.port))]
    pub async fn connect(&mut self) -> Result<()> {
        if self.is_connected() {
            debug!("already connected to Claude Desktop");
            return Ok(());
        }

        self.state = ConnectionState::Connecting;
        info!(port = self.config.port, "connecting to Claude Desktop via CDP");

        let ws_url = self.config.ws_url();

        // Connect to the browser via CDP
        let (browser, mut handler) = Browser::connect(&ws_url).await.map_err(|e| {
            self.state = ConnectionState::Failed;
            ClaudeCdpError::ConnectionFailed {
                url: ws_url.clone(),
                source: Box::new(e),
            }
        })?;

        // Spawn the CDP event handler task
        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if let Err(e) = event {
                    error!("CDP event handler error: {}", e);
                    break;
                }
            }
        });

        // Find the Claude conversation page
        let page = self.find_claude_page(&browser).await?;

        self.browser = Some(browser);
        self.page = Some(page);
        self.state = ConnectionState::Connected;

        info!("successfully connected to Claude Desktop");
        Ok(())
    }

    /// Find the Claude conversation page in the browser.
    async fn find_claude_page(&self, browser: &Browser) -> Result<Page> {
        let mut pages = browser.pages().await.map_err(|e| {
            ClaudeCdpError::ProtocolError(format!("failed to list pages: {e}"))
        })?;

        // Look for a page with Claude in the URL or title
        for page in pages.iter_mut() {
            if let Ok(url) = page.url().await {
                let url_str = url.map(|u| u.to_string()).unwrap_or_default();
                debug!(url = %url_str, "checking page");

                // Claude Desktop URLs typically contain "claude" or "anthropic"
                if url_str.contains("claude") || url_str.contains("anthropic") {
                    info!(url = %url_str, "found Claude page");
                    return Ok(page.clone());
                }
            }
        }

        if pages.is_empty() {
            return Err(ClaudeCdpError::NoClaudePageFound);
        }

        warn!(page_count = pages.len(), "no explicit Claude page found");
        Err(ClaudeCdpError::NoClaudePageFound)
    }

    /// Disconnect from Claude Desktop.
    pub async fn disconnect(&mut self) {
        if let Some(browser) = self.browser.take() {
            drop(browser);
        }
        self.page = None;
        self.state = ConnectionState::Disconnected;
        info!("disconnected from Claude Desktop");
    }

    /// Send a message to Claude and wait for the response.
    #[instrument(skip(self, message), fields(message_len = message.len()))]
    pub async fn send_message(&self, message: &str) -> Result<String> {
        let page = self.page.as_ref().ok_or(ClaudeCdpError::NotConnected)?;

        info!("sending message to Claude");

        // Get the current message count before sending
        let initial_count = self.get_assistant_message_count(page).await?;
        debug!(initial_count, "current assistant message count");

        // Find and fill the input element
        self.fill_input(page, message).await?;

        // Submit the message
        self.submit_message(page).await?;

        // Wait for Claude's response
        let response = self.wait_for_response(page, initial_count).await?;

        info!(response_len = response.len(), "received response from Claude");
        Ok(response)
    }

    // ... additional methods follow
}
```

### Desktop Manager

The manager provides a high-level interface with lifecycle management:

```rust
// src-tauri/src/core/claude_cdp/manager.rs

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
    pub state: String,
    pub connected: bool,
    pub port: u16,
    pub launched_by_us: bool,
    pub binary_path: Option<String>,
}

/// Manager for Claude Desktop CDP connections.
pub struct ClaudeDesktopManager {
    client: Arc<Mutex<ClaudeClient>>,
    config: Arc<RwLock<ClaudeConfig>>,
    process: Arc<Mutex<Option<Child>>>,
    binary_path: Arc<RwLock<Option<String>>>,
}

impl ClaudeDesktopManager {
    pub fn new() -> Self {
        Self::with_config(ClaudeConfig::default())
    }

    pub fn with_config(config: ClaudeConfig) -> Self {
        Self {
            client: Arc::new(Mutex::new(ClaudeClient::with_config(config.clone()))),
            config: Arc::new(RwLock::new(config)),
            process: Arc::new(Mutex::new(None)),
            binary_path: Arc::new(RwLock::new(None)),
        }
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

    // ... additional methods
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
```

### Error Handling

Comprehensive error types for CDP operations:

```rust
// src-tauri/src/core/claude_cdp/error.rs

use thiserror::Error;

/// Errors that can occur when communicating with Claude Desktop via CDP.
#[derive(Error, Debug)]
pub enum ClaudeCdpError {
    /// Failed to connect to Claude Desktop.
    #[error("failed to connect to Claude Desktop at {url}: {source}")]
    ConnectionFailed {
        url: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Claude Desktop is not running or CDP is not enabled.
    #[error("Claude Desktop not reachable. Ensure it's running with --remote-debugging-port={port}")]
    NotReachable { port: u16 },

    /// No Claude page/tab found in the browser.
    #[error("no Claude conversation page found in browser")]
    NoClaudePageFound,

    /// Failed to find the message input element.
    #[error("could not locate message input element: {details}")]
    InputElementNotFound { details: String },

    /// Failed to send a message.
    #[error("failed to send message: {reason}")]
    SendFailed { reason: String },

    /// Timeout waiting for Claude's response.
    #[error("timeout after {seconds}s waiting for Claude's response")]
    ResponseTimeout { seconds: u64 },

    /// Failed to extract response text.
    #[error("failed to extract response from page: {details}")]
    ResponseExtractionFailed { details: String },

    /// JavaScript execution failed.
    #[error("JavaScript execution failed: {script_hint} - {error}")]
    JsExecutionFailed { script_hint: String, error: String },

    /// The connection was closed unexpectedly.
    #[error("CDP connection closed unexpectedly")]
    ConnectionClosed,

    /// Generic CDP protocol error.
    #[error("CDP protocol error: {0}")]
    ProtocolError(String),

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Claude Desktop binary not found.
    #[error("Claude Desktop binary not found. Checked: {paths:?}")]
    BinaryNotFound { paths: Vec<String> },

    /// Failed to launch Claude Desktop.
    #[error("failed to launch Claude Desktop: {reason}")]
    LaunchFailed { reason: String },

    /// Not connected to Claude Desktop.
    #[error("not connected to Claude Desktop")]
    NotConnected,
}

pub type Result<T> = std::result::Result<T, ClaudeCdpError>;
```

---

## LLM Provider Integration

The provider wraps the manager to implement the `LLMProvider` trait:

```rust
// src-tauri/src/core/llm/providers/claude_desktop.rs

use crate::core::claude_cdp::{ClaudeConfig, ClaudeDesktopManager};
use crate::core::llm::cost::ProviderPricing;
use crate::core::llm::router::{
    ChatChunk, ChatRequest, ChatResponse, LLMError, LLMProvider, Result,
};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info, warn};

/// Claude Desktop provider using CDP.
pub struct ClaudeDesktopProvider {
    manager: Arc<Mutex<ClaudeDesktopManager>>,
    port: u16,
    timeout_secs: u64,
}

impl ClaudeDesktopProvider {
    pub fn new() -> Self {
        Self::with_config(9333, 120)
    }

    pub fn with_port(port: u16) -> Self {
        Self::with_config(port, 120)
    }

    pub fn with_config(port: u16, timeout_secs: u64) -> Self {
        let config = ClaudeConfig::default()
            .with_port(port)
            .with_timeout(timeout_secs);

        Self {
            manager: Arc::new(Mutex::new(ClaudeDesktopManager::with_config(config))),
            port,
            timeout_secs,
        }
    }

    /// Connect to Claude Desktop.
    pub async fn connect(&self) -> std::result::Result<(), crate::core::claude_cdp::ClaudeCdpError> {
        let manager = self.manager.lock().await;
        manager.connect().await
    }

    /// Connect or launch Claude Desktop.
    pub async fn connect_or_launch(&self) -> std::result::Result<(), crate::core::claude_cdp::ClaudeCdpError> {
        let manager = self.manager.lock().await;
        manager.connect_or_launch().await
    }

    /// Build the message content from the request.
    fn build_message(&self, request: &ChatRequest) -> String {
        let mut parts = Vec::new();

        // Add system prompt if present
        if let Some(ref system) = request.system_prompt {
            parts.push(format!("[System: {}]\n", system));
        }

        // Add conversation context
        for msg in request.messages.iter() {
            match msg.role {
                crate::core::llm::router::MessageRole::User => {
                    parts.push(format!("User: {}", msg.content));
                }
                crate::core::llm::router::MessageRole::Assistant => {
                    parts.push(format!("Assistant: {}", msg.content));
                }
                crate::core::llm::router::MessageRole::System => {
                    // Skip system messages (already added above)
                }
            }
        }

        // Return the last user message
        if let Some(last_user_msg) = request.messages.iter().rev().find(|m| {
            matches!(m.role, crate::core::llm::router::MessageRole::User)
        }) {
            last_user_msg.content.clone()
        } else {
            parts.join("\n\n")
        }
    }
}

#[async_trait]
impl LLMProvider for ClaudeDesktopProvider {
    fn id(&self) -> &str {
        "claude-desktop"
    }

    fn name(&self) -> &str {
        "Claude Desktop (CDP)"
    }

    fn model(&self) -> &str {
        // We don't know which model the user has selected in Claude Desktop
        "claude-desktop"
    }

    async fn health_check(&self) -> bool {
        let manager = self.manager.lock().await;
        manager.is_connected().await && manager.health_check().await
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        // No per-token pricing - subscription based
        None
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let manager = self.manager.lock().await;

        if !manager.is_connected().await {
            return Err(LLMError::NotConfigured(
                "Claude Desktop not connected. Call connect() or connect_or_launch() first.".to_string()
            ));
        }

        let message = self.build_message(&request);
        debug!(message_len = message.len(), "sending message to Claude Desktop");

        let start = Instant::now();

        let response_text = manager.send_message(&message).await.map_err(|e| {
            LLMError::ApiError {
                status: 0,
                message: e.to_string(),
            }
        })?;

        let latency_ms = start.elapsed().as_millis() as u64;

        info!(
            response_len = response_text.len(),
            latency_ms,
            "received response from Claude Desktop"
        );

        Ok(ChatResponse {
            content: response_text,
            model: "claude-desktop".to_string(),
            provider: "claude-desktop".to_string(),
            usage: None, // Can't track tokens via CDP
            finish_reason: Some("stop".to_string()),
            latency_ms,
            cost_usd: None, // Subscription-based
            tool_calls: None,
        })
    }

    async fn stream_chat(
        &self,
        _request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        // CDP doesn't support streaming
        warn!("streaming not supported for Claude Desktop provider");
        Err(LLMError::StreamingNotSupported("claude-desktop".to_string()))
    }

    fn supports_streaming(&self) -> bool {
        false
    }

    fn supports_embeddings(&self) -> bool {
        false
    }
}
```

---

## Tauri Commands

The Tauri commands expose Claude Desktop functionality to the frontend:

```rust
// src-tauri/src/commands.rs

use crate::core::claude_cdp::{ClaudeDesktopManager, ClaudeDesktopStatus};

// ============================================================================
// Claude Desktop CDP Commands
// ============================================================================

/// Connect to a running Claude Desktop instance via CDP.
#[tauri::command]
pub async fn connect_claude_desktop(
    port: Option<u16>,
    state: State<'_, AppState>,
) -> Result<ClaudeDesktopStatus, String> {
    let manager = state.claude_desktop_manager.clone();

    // Update port if specified
    if let Some(p) = port {
        let guard = manager.read().await;
        guard.update_config(Some(p), None).await;
    }

    // Connect
    {
        let guard = manager.read().await;
        guard.connect().await.map_err(|e| e.to_string())?;
    }

    let guard = manager.read().await;
    let status = guard.status().await;
    Ok(status)
}

/// Launch Claude Desktop with CDP enabled and connect.
#[tauri::command]
pub async fn launch_claude_desktop(
    state: State<'_, AppState>,
) -> Result<ClaudeDesktopStatus, String> {
    let manager = state.claude_desktop_manager.clone();

    // Connect or launch
    {
        let guard = manager.read().await;
        guard.connect_or_launch().await.map_err(|e| e.to_string())?;
    }

    let guard = manager.read().await;
    let status = guard.status().await;
    Ok(status)
}

/// Try to connect to Claude Desktop, launch if not running.
#[tauri::command]
pub async fn connect_or_launch_claude_desktop(
    port: Option<u16>,
    state: State<'_, AppState>,
) -> Result<ClaudeDesktopStatus, String> {
    let manager = state.claude_desktop_manager.clone();

    if let Some(p) = port {
        let guard = manager.read().await;
        guard.update_config(Some(p), None).await;
    }

    {
        let guard = manager.read().await;
        guard.connect_or_launch().await.map_err(|e| e.to_string())?;
    }

    let guard = manager.read().await;
    let status = guard.status().await;
    Ok(status)
}

/// Disconnect from Claude Desktop.
#[tauri::command]
pub async fn disconnect_claude_desktop(
    kill_if_launched: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = state.claude_desktop_manager.clone();
    let guard = manager.read().await;

    if kill_if_launched.unwrap_or(false) {
        guard.disconnect_and_kill().await;
    } else {
        guard.disconnect().await;
    }

    Ok(())
}

/// Get Claude Desktop connection status.
#[tauri::command]
pub async fn get_claude_desktop_status(
    state: State<'_, AppState>,
) -> Result<ClaudeDesktopStatus, String> {
    let manager = state.claude_desktop_manager.clone();
    let guard = manager.read().await;
    let status = guard.status().await;
    Ok(status)
}

/// Start a new conversation in Claude Desktop.
#[tauri::command]
pub async fn claude_desktop_new_conversation(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = state.claude_desktop_manager.clone();
    let guard = manager.read().await;
    guard.new_conversation().await.map_err(|e| e.to_string())
}

/// Get conversation history from Claude Desktop.
#[tauri::command]
pub async fn claude_desktop_get_history(
    state: State<'_, AppState>,
) -> Result<Vec<crate::core::claude_cdp::Message>, String> {
    let manager = state.claude_desktop_manager.clone();
    let guard = manager.read().await;
    guard.get_conversation().await.map_err(|e| e.to_string())
}

/// Check if Claude Desktop binary is installed.
#[tauri::command]
pub fn detect_claude_desktop() -> Option<String> {
    ClaudeDesktopManager::detect_claude_binary()
}

/// Send a message to Claude Desktop and get response.
#[tauri::command]
pub async fn claude_desktop_send_message(
    message: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let manager = state.claude_desktop_manager.clone();
    let guard = manager.read().await;
    guard.send_message(&message).await.map_err(|e| e.to_string())
}

/// Update Claude Desktop CDP configuration.
#[tauri::command]
pub async fn configure_claude_desktop(
    port: Option<u16>,
    timeout_secs: Option<u64>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = state.claude_desktop_manager.clone();
    let guard = manager.read().await;
    guard.update_config(port, timeout_secs).await;
    Ok(())
}
```

### Command Registration

Commands are registered in `main.rs`:

```rust
// src-tauri/src/main.rs

.invoke_handler(tauri::generate_handler![
    // ... other commands ...

    // Claude Desktop CDP
    commands::connect_claude_desktop,
    commands::launch_claude_desktop,
    commands::connect_or_launch_claude_desktop,
    commands::disconnect_claude_desktop,
    commands::get_claude_desktop_status,
    commands::claude_desktop_new_conversation,
    commands::claude_desktop_get_history,
    commands::detect_claude_desktop,
    commands::claude_desktop_send_message,
    commands::configure_claude_desktop,
])
```

---

## Frontend Integration

### Provider Enum

The frontend defines Claude Desktop as a provider option:

```rust
// frontend/src/components/settings/llm.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LLMProvider {
    Claude,
    OpenAI,
    Gemini,
    Cohere,
    Groq,
    Together,
    DeepSeek,
    Ollama,
    Meilisearch,
    ClaudeCode,
    ClaudeDesktop,  // <-- CDP Provider
    Claude,
    GeminiCli,
}

impl std::fmt::Display for LLMProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMProvider::ClaudeDesktop => write!(f, "Claude Desktop"),
            // ... other providers
        }
    }
}

impl LLMProvider {
    pub fn api_key_label(&self) -> &str {
        match self {
            LLMProvider::ClaudeDesktop => "Status", // No API key needed
            // ... other providers
        }
    }

    pub fn api_key_placeholder(&self) -> &str {
        match self {
            LLMProvider::ClaudeDesktop => "Uses Desktop authentication",
            // ... other providers
        }
    }

    pub fn default_model(&self) -> &str {
        match self {
            LLMProvider::ClaudeDesktop => "claude-sonnet-4-20250514",
            // ... other providers
        }
    }

    pub fn requires_api_key(&self) -> Option<bool> {
        match self {
            LLMProvider::ClaudeDesktop => None, // Uses Desktop authentication
            // ... other providers
        }
    }

    pub fn color(&self) -> &str {
        match self {
            LLMProvider::ClaudeDesktop => "text-orange-400", // Anthropic Sienna
            // ... other providers
        }
    }
}
```

### Invoking Commands from Frontend

```rust
// Example: Connecting to Claude Desktop from Leptos

use leptos::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn ClaudeDesktopControl() -> impl IntoView {
    let (status, set_status) = create_signal(None::<ClaudeDesktopStatus>);
    let (connecting, set_connecting) = create_signal(false);
    let (error, set_error) = create_signal(None::<String>);

    let connect = move |_| {
        set_connecting.set(true);
        set_error.set(None);

        spawn_local(async move {
            let result = tauri_invoke::<ClaudeDesktopStatus>(
                "connect_or_launch_claude_desktop",
                serde_json::json!({ "port": 9333 }),
            ).await;

            match result {
                Ok(s) => set_status.set(Some(s)),
                Err(e) => set_error.set(Some(e)),
            }
            set_connecting.set(false);
        });
    };

    let disconnect = move |_| {
        spawn_local(async move {
            let _ = tauri_invoke::<()>(
                "disconnect_claude_desktop",
                serde_json::json!({ "kill_if_launched": false }),
            ).await;
            set_status.set(None);
        });
    };

    view! {
        <div class="claude-desktop-control">
            <Show
                when=move || status.get().map(|s| s.connected).unwrap_or(false)
                fallback=move || view! {
                    <button
                        on:click=connect
                        disabled=connecting.get()
                    >
                        {move || if connecting.get() { "Connecting..." } else { "Connect" }}
                    </button>
                }
            >
                <div class="connected-status">
                    <span class="status-dot green"></span>
                    <span>"Connected to Claude Desktop"</span>
                    <button on:click=disconnect>"Disconnect"</button>
                </div>
            </Show>

            <Show when=move || error.get().is_some()>
                <div class="error">
                    {move || error.get().unwrap_or_default()}
                </div>
            </Show>
        </div>
    }
}

// Helper function for Tauri invocation
async fn tauri_invoke<T: serde::de::DeserializeOwned>(
    cmd: &str,
    args: serde_json::Value,
) -> Result<T, String> {
    let window = web_sys::window().unwrap();
    let tauri = js_sys::Reflect::get(&window, &JsValue::from_str("__TAURI__"))
        .map_err(|_| "Tauri not available")?;
    let invoke = js_sys::Reflect::get(&tauri, &JsValue::from_str("invoke"))
        .map_err(|_| "invoke not available")?;

    let promise = js_sys::Reflect::apply(
        invoke.unchecked_ref(),
        &tauri,
        &js_sys::Array::of2(
            &JsValue::from_str(cmd),
            &JsValue::from_serde(&args).unwrap(),
        ),
    ).map_err(|e| format!("{:?}", e))?;

    let result = wasm_bindgen_futures::JsFuture::from(
        js_sys::Promise::from(promise)
    ).await.map_err(|e| format!("{:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("deserialize error: {:?}", e))
}
```

---

## Binary Detection

### Cross-Platform Detection

```
// Detection flow:

   ┌─────────────────────────────────────────┐
   │       detect_claude_binary()            │
   └─────────────────────────────────────────┘
                      │
                      ▼
   ┌─────────────────────────────────────────┐
   │   Check platform-specific paths:        │
   │                                         │
   │   Linux:                                │
   │   - /opt/Claude/claude                  │
   │   - /opt/claude-desktop/claude          │
   │   - /usr/bin/claude                     │
   │   - /usr/local/bin/claude               │
   │                                         │
   │   macOS:                                │
   │   - /Applications/Claude.app/.../Claude │
   │   - /usr/local/bin/claude               │
   │                                         │
   │   Windows:                              │
   │   - C:\Program Files\Claude\Claude.exe  │
   │   - C:\Program Files (x86)\Claude\...   │
   └─────────────────────────────────────────┘
                      │
              Path found? ───No──┐
                      │          │
                     Yes         ▼
                      │   ┌──────────────────┐
                      │   │ Fallback: `which`│
                      │   │ (Unix only)      │
                      │   └──────────────────┘
                      │          │
                      ▼          ▼
              ┌───────────────────────────────┐
              │    Return Some(path) or None  │
              └───────────────────────────────┘
```

### Implementation

```rust
/// Detect the Claude Desktop binary path.
pub fn detect_claude_binary() -> Option<String> {
    // Check known platform-specific paths
    for path in CLAUDE_BINARY_PATHS {
        if Path::new(path).exists() {
            info!(path, "found Claude Desktop binary");
            return Some(path.to_string());
        }
    }

    // Unix fallback: try `which` command
    #[cfg(unix)]
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
```

---

## DOM Interaction via JavaScript

### The Challenge

Claude Desktop's UI uses React with dynamic class names and structure. The CDP approach must:
1. Find the input element (contenteditable, textarea, or ProseMirror)
2. Fill it with the message
3. Submit (button click or Enter key)
4. Wait for the streaming response to complete
5. Extract the response text

### Finding Input Elements

```javascript
// Multiple selector strategies for resilience
const selectors = [
    '[contenteditable="true"]',  // Modern rich text inputs
    'div[contenteditable]',      // Generic contenteditable
    'textarea',                  // Classic textarea
    '[data-placeholder]',        // Placeholder-marked inputs
    '.ProseMirror',              // ProseMirror editor
    '[role="textbox"]'           // ARIA textbox role
];

for (const selector of selectors) {
    const input = document.querySelector(selector);
    if (input) {
        input.focus();

        if (input.contentEditable === 'true') {
            input.innerHTML = '';
            input.textContent = message;
            input.dispatchEvent(new InputEvent('input', { bubbles: true }));
            return { success: true, selector };
        }

        if (input.tagName === 'TEXTAREA') {
            input.value = message;
            input.dispatchEvent(new InputEvent('input', { bubbles: true }));
            return { success: true, selector };
        }
    }
}
```

### Submitting Messages

```javascript
// Try button click first, then keyboard event
const buttonSelectors = [
    'button[type="submit"]',
    'button[aria-label*="send" i]',
    'button[aria-label*="Send" i]',
    '[data-testid="send-button"]',
    'button svg[class*="send"]'
];

for (const selector of buttonSelectors) {
    const btn = document.querySelector(selector);
    if (btn) {
        const button = btn.closest('button') || btn;
        if (!button.disabled) {
            button.click();
            return { success: true, method: 'button', selector };
        }
    }
}

// Fallback: simulate Enter key
const input = document.querySelector('[contenteditable="true"], textarea');
if (input) {
    const enterEvent = new KeyboardEvent('keydown', {
        key: 'Enter',
        code: 'Enter',
        keyCode: 13,
        which: 13,
        bubbles: true
    });
    input.dispatchEvent(enterEvent);
    return { success: true, method: 'enter' };
}
```

### Detecting Response Completion

```javascript
// Poll until streaming indicator disappears and new message appears
(() => {
    // Check if still streaming
    const streaming = document.querySelector('[data-is-streaming="true"]');
    if (streaming) {
        return { done: false, streaming: true };
    }

    // Look for new assistant messages
    const messages = document.querySelectorAll(
        '.font-claude-message, [class*="assistant-message"], [data-message-author="assistant"]'
    );

    if (messages.length > initialCount) {
        const lastMsg = messages[messages.length - 1];
        const text = lastMsg.textContent || lastMsg.innerText || '';
        return { done: true, text: text.trim() };
    }

    // Alternative: check prose/markdown containers
    const allContent = document.querySelectorAll('[class*="prose"], [class*="markdown"]');
    if (allContent.length > initialCount) {
        const lastContent = allContent[allContent.length - 1];
        const text = lastContent.textContent || lastContent.innerText || '';
        return { done: true, text: text.trim() };
    }

    return { done: false, streaming: false };
})()
```

### Extracting Conversation History

```javascript
(() => {
    const messages = [];

    const containers = document.querySelectorAll(
        '[data-message-author], [class*="message-"]'
    );

    containers.forEach(container => {
        const isAssistant = container.getAttribute('data-message-author') === 'assistant'
            || container.classList.contains('font-claude-message')
            || container.querySelector('.font-claude-message');

        const text = container.textContent || container.innerText || '';

        if (text.trim()) {
            messages.push({
                role: isAssistant ? 'assistant' : 'user',
                content: text.trim()
            });
        }
    });

    return messages;
})()
```

---

## Session Lifecycle

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Session Lifecycle                                   │
└─────────────────────────────────────────────────────────────────────────────┘

                    ┌───────────────────┐
                    │   Disconnected    │
                    └─────────┬─────────┘
                              │
                              │ connect() or connect_or_launch()
                              ▼
                    ┌───────────────────┐
                    │    Connecting     │
                    └─────────┬─────────┘
                              │
              ┌───────────────┴───────────────┐
              │                               │
              ▼                               ▼
    ┌───────────────────┐           ┌───────────────────┐
    │    Connected      │           │      Failed       │
    └─────────┬─────────┘           └─────────┬─────────┘
              │                               │
              │ disconnect()                  │ retry after delay
              │ or connection lost            │
              ▼                               │
    ┌───────────────────┐                     │
    │   Disconnected    │◀────────────────────┘
    └───────────────────┘


┌─────────────────────────────────────────────────────────────────────────────┐
│                       Process Management                                     │
└─────────────────────────────────────────────────────────────────────────────┘

          detect_claude_binary()
                    │
                    ▼
           ┌───────────────┐
           │ Binary Found? │
           └───────┬───────┘
                   │
       ┌───────────┴───────────┐
       │ Yes                   │ No
       ▼                       ▼
  ┌─────────────┐     ┌─────────────────┐
  │ Launch with │     │ BinaryNotFound  │
  │ CDP enabled │     │ Error           │
  └──────┬──────┘     └─────────────────┘
         │
         ▼
  ┌──────────────────┐
  │ wait_and_connect │──────▶ Poll every 500ms
  │ (max 30 attempts)│       │
  └────────┬─────────┘       │
           │                 │
           │ ◀───────────────┘
           ▼
    ┌─────────────┐
    │  Connected  │
    │ (or timeout)│
    └─────────────┘


┌─────────────────────────────────────────────────────────────────────────────┐
│                       Cleanup on Drop                                        │
└─────────────────────────────────────────────────────────────────────────────┘

  impl Drop for ClaudeDesktopManager {
      fn drop(&mut self) {
          // Kill process if we launched it
          if let Ok(mut process) = self.process.try_lock() {
              if let Some(mut child) = process.take() {
                  let _ = child.kill();
              }
          }
      }
  }
```

---

## Implementation Guide

### Step 1: Add Dependencies

```toml
# Cargo.toml
[dependencies]
chromiumoxide = { version = "0.7", default-features = false, features = ["tokio-runtime"] }
tokio = { version = "1", features = ["full"] }
futures-util = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tracing = "0.1"
async-trait = "0.1"
```

### Step 2: Create Module Structure

```
src/
├── cdp/
│   ├── mod.rs
│   ├── config.rs
│   ├── client.rs
│   ├── manager.rs
│   └── error.rs
└── provider.rs
```

### Step 3: Implement Configuration

Copy the configuration module with platform-specific binary paths for your target platform.

### Step 4: Implement CDP Client

The client handles:
- WebSocket connection to CDP endpoint
- Finding the correct page/tab
- JavaScript evaluation for DOM interaction
- Message sending and response polling

### Step 5: Implement Manager

The manager provides:
- Binary detection
- Process launching with CDP flag
- Connection retry logic
- Lifecycle management (connect, disconnect, kill)

### Step 6: Implement Provider Trait

Wrap the manager in your LLM provider interface:

```rust
#[async_trait]
impl LLMProvider for ClaudeDesktopProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        // 1. Ensure connected
        // 2. Build message from request
        // 3. Send via manager
        // 4. Return response
    }
}
```

### Step 7: Add Tauri Commands (if applicable)

Expose the functionality via Tauri commands for frontend access.

### Step 8: Handle UI Selectors Gracefully

The DOM selectors may change with Claude Desktop updates. Design your JavaScript to:
- Try multiple selector strategies
- Log which selectors work
- Fail gracefully with clear error messages

---

## Troubleshooting

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| Connection refused | Claude Desktop not running with CDP | Launch with `--remote-debugging-port=9333` |
| No Claude page found | Wrong target/tab | Ensure Claude Desktop is on conversation page |
| Input element not found | UI changed | Update selectors based on current DOM |
| Response timeout | Long response or network issue | Increase timeout, check Claude Desktop |
| Message not submitting | Button disabled or wrong selector | Check for rate limiting, update selectors |

### Debugging Tips

1. **Enable Tracing**:
   ```rust
   tracing_subscriber::fmt()
       .with_max_level(tracing::Level::DEBUG)
       .init();
   ```

2. **Inspect CDP Targets**:
   ```bash
   curl http://127.0.0.1:9333/json
   ```

3. **Manual Testing**:
   - Open Chrome DevTools on Claude Desktop
   - Test JavaScript selectors in console
   - Watch network tab for issues

4. **Check Process**:
   ```bash
   # Linux/macOS
   ps aux | grep claude
   lsof -i :9333

   # Windows
   netstat -ano | findstr 9333
   ```

---

## Limitations

### Technical Limitations

| Limitation | Impact | Workaround |
|------------|--------|------------|
| **No Streaming** | Full response only after completion | Use loading indicator, increase timeout |
| **No Token Counting** | Can't track usage/costs | N/A (subscription model) |
| **UI Dependent** | Selectors may break with updates | Design resilient selectors, log failures |
| **Slower** | DOM polling overhead | Acceptable for most use cases |
| **Single Session** | Shares Claude Desktop state | Start new conversation when needed |

### Reliability Considerations

- **UI Changes**: Claude Desktop updates may change DOM structure
- **Rate Limiting**: Claude may rate-limit heavy usage
- **Authentication**: Depends on user being logged in
- **Focus**: Some operations may require Claude Desktop to have focus

### When to Use This vs API

| Use CDP When | Use API When |
|--------------|--------------|
| Development/testing | Production applications |
| Personal projects | Commercial products |
| Cost-sensitive work | High-volume usage |
| Already have subscription | Need token tracking |
| Acceptable latency | Need low latency |
| Don't need streaming | Need streaming |

---

## Summary

The Claude Desktop CDP Provider enables integration with Claude Desktop through the Chrome DevTools Protocol, offering a cost-effective way to leverage your Claude subscription for development and testing.

**Key Components**:
1. **chromiumoxide** - Rust CDP client library
2. **ClaudeClient** - Low-level CDP operations
3. **ClaudeDesktopManager** - Lifecycle and process management
4. **ClaudeDesktopProvider** - LLMProvider trait implementation
5. **JavaScript injection** - DOM interaction for input/output

**Files Created**:
- `src-tauri/src/core/claude_cdp/mod.rs`
- `src-tauri/src/core/claude_cdp/config.rs`
- `src-tauri/src/core/claude_cdp/client.rs`
- `src-tauri/src/core/claude_cdp/manager.rs`
- `src-tauri/src/core/claude_cdp/error.rs`
- `src-tauri/src/core/llm/providers/claude_desktop.rs`

This approach trades some capabilities (streaming, token tracking) for cost savings and integration with your existing Claude Desktop workflow.
