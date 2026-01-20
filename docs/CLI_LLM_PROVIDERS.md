# CLI-Based LLM Providers: A Deep Dive

This document provides a comprehensive analysis of how the **Gemini CLI** and **Claude Code CLI** LLM providers work in the TTRPG Assistant (Sidecar DM) application. It covers installation detection, communication patterns, session management, and provides code examples for implementing similar functionality.

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Provider Implementation](#provider-implementation)
   - [Claude Code CLI Provider](#claude-code-cli-provider)
   - [Gemini CLI Provider](#gemini-cli-provider)
4. [Installation Detection](#installation-detection)
5. [Installation Button Functionality](#installation-button-functionality)
6. [Frontend-Backend Communication](#frontend-backend-communication)
7. [Session Management](#session-management)
8. [Adding This Feature to Your Project](#adding-this-feature-to-your-project)

---

## Overview

CLI-based LLM providers offer an alternative to API-based providers by leveraging existing command-line tools that handle authentication and API communication. This approach provides:

- **No API key management**: Uses CLI tool's existing authentication (OAuth, Google Account, etc.)
- **Free tier access**: Both Gemini CLI and Claude Code offer free usage tiers
- **Full tool capabilities**: Access to file operations, code execution, and tool use
- **Session management**: Native conversation continuity via session IDs

### Providers Comparison

| Feature | Claude Code CLI | Gemini CLI |
|---------|----------------|------------|
| Authentication | OAuth (browser) | Google Account |
| Free Tier | CLI usage included | 1000 req/day |
| Session Support | Native (--resume, --continue) | Local tracking |
| Streaming | stream-json format | stream-json format |
| Tool Use | Full Claude Code tools | Extensions system |
| Installation | npm -g @anthropic-ai/claude-code | npm -g @google/gemini-cli |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Tauri Desktop App                            │
├─────────────────────────────────────────────────────────────────────┤
│  Frontend (Leptos/WASM)                                             │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  LLMSettingsView Component                                   │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │   │
│  │  │ Status Badge │  │ Install Btn  │  │ Login Button     │  │   │
│  │  └──────────────┘  └──────────────┘  └──────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              │                                       │
│                              │ Tauri IPC (invoke)                    │
│                              ▼                                       │
│  Backend (Rust)                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  commands.rs (Tauri Commands)                                │   │
│  │  ┌──────────────────────────────────────────────────────┐   │   │
│  │  │ get_claude_code_status() check_gemini_cli_status()   │   │   │
│  │  │ claude_code_install_cli() launch_gemini_cli_login()  │   │   │
│  │  └──────────────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              │                                       │
│                              ▼                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  core/llm/providers/                                         │   │
│  │  ┌─────────────────────┐  ┌─────────────────────────────┐   │   │
│  │  │ claude_code.rs      │  │ gemini_cli.rs               │   │   │
│  │  │                     │  │                             │   │   │
│  │  │ ClaudeCodeProvider  │  │ GeminiCliProvider           │   │   │
│  │  │ - is_available()    │  │ - is_available()            │   │   │
│  │  │ - get_status()      │  │ - check_status()            │   │   │
│  │  │ - login()           │  │ - launch_login()            │   │   │
│  │  │ - install_cli()     │  │ - install_cli()             │   │   │
│  │  └─────────────────────┘  └─────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              │                                       │
│                              │ tokio::process::Command               │
│                              ▼                                       │
└─────────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     External CLI Tools                               │
│  ┌─────────────────────┐         ┌─────────────────────────────┐   │
│  │ claude              │         │ gemini                       │   │
│  │ (Claude Code CLI)   │         │ (Gemini CLI)                 │   │
│  │                     │         │                              │   │
│  │ ~/.claude/          │         │ ~/.gemini/                   │   │
│  │ ├── credentials.json│         │ └── oauth_creds.json         │   │
│  │ └── commands/       │         │                              │   │
│  └─────────────────────┘         └─────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Provider Implementation

### Claude Code CLI Provider

The Claude Code provider (`src-tauri/src/core/llm/providers/claude_code.rs`) implements the `LLMProvider` trait and provides full access to Claude Code's capabilities.

#### Core Structure

```rust
pub struct ClaudeCodeProvider {
    timeout_secs: u64,
    model: Option<String>,
    fallback_model: Option<String>,
    working_dir: Option<String>,
    session_store: Arc<SessionStore>,
    current_session: RwLock<Option<SessionId>>,
    persist_sessions: bool,
    auto_fallback: bool,
}
```

#### Builder Pattern

The provider uses a builder pattern for flexible configuration:

```rust
// Create with defaults
let provider = ClaudeCodeProvider::new();

// Or with full configuration
let provider = ClaudeCodeProvider::builder()
    .model("claude-sonnet-4-20250514")
    .fallback_model("claude-haiku-4-20250514")
    .timeout_secs(300)
    .working_dir("/path/to/project")
    .persist_sessions(true)
    .auto_fallback(true)
    .build();
```

#### CLI Argument Building

The provider builds CLI arguments based on the session mode:

```rust
fn build_args(&self, prompt: &str, session_mode: &SessionMode, streaming: bool) -> Vec<String> {
    let mut args = vec!["-p".to_string(), prompt.to_string()];

    // Output format
    if streaming {
        args.extend(["--output-format".to_string(), "stream-json".to_string()]);
    } else {
        args.extend(["--output-format".to_string(), "json".to_string()]);
    }

    // Model selection
    let model = self.model.clone().unwrap_or_else(|| DEFAULT_MODEL.to_string());
    args.extend(["--model".to_string(), model]);

    // Session handling
    match session_mode {
        SessionMode::New => {}
        SessionMode::Continue => {
            args.push("--continue".to_string());
        }
        SessionMode::Resume(session_id) => {
            args.extend(["--resume".to_string(), session_id.clone()]);
        }
        SessionMode::Fork(session_id) => {
            args.extend([
                "--resume".to_string(),
                session_id.clone(),
                "--fork-session".to_string(),
            ]);
        }
    }

    args
}
```

#### Executing CLI Commands

```rust
async fn execute_prompt(&self, prompt: &str, session_mode: SessionMode) -> Result<ClaudeCodeResponse> {
    // Find the binary
    let binary = which::which("claude").map_err(|_| {
        LLMError::NotConfigured(
            "Claude Code CLI not found. Install with: npm install -g @anthropic-ai/claude-code"
                .to_string(),
        )
    })?;

    let args = self.build_args(prompt, &session_mode, false);

    let mut cmd = Command::new(binary);
    cmd.args(&args);

    if let Some(ref dir) = self.working_dir {
        cmd.current_dir(dir);
    }

    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Execute with timeout
    let timeout = tokio::time::Duration::from_secs(self.timeout_secs);
    let output = tokio::time::timeout(timeout, cmd.output()).await;

    // Parse JSON response
    match output {
        Ok(Ok(output)) if output.status.success() => {
            let response: ClaudeCodeResponse = serde_json::from_slice(&output.stdout)?;

            // Store session ID if returned
            if let Some(ref session_id) = response.session_id {
                let mut current = self.current_session.write().await;
                *current = Some(session_id.clone());
            }

            Ok(response)
        }
        Ok(Ok(output)) => {
            // Check for rate limiting and retry with fallback
            let stderr = String::from_utf8_lossy(&output.stderr);
            if Self::is_rate_limit_error(&stderr) && self.auto_fallback {
                // Retry with fallback model
                return self.execute_prompt_with_model(prompt, session_mode, &self.fallback_model).await;
            }
            Err(LLMError::ApiError { status: output.status.code().unwrap_or(1), message: stderr.to_string() })
        }
        Err(_) => Err(LLMError::Timeout),
        _ => Err(LLMError::ApiError { status: 0, message: "Unknown error".to_string() }),
    }
}
```

### Gemini CLI Provider

The Gemini CLI provider (`src-tauri/src/core/llm/providers/gemini_cli.rs`) provides access to Google's Gemini models via the Gemini CLI tool.

#### Core Structure

```rust
pub struct GeminiCliProvider {
    model: String,
    fallback_model: Option<String>,
    timeout_secs: u64,
    working_dir: Option<PathBuf>,
    yolo_mode: bool,           // Auto-approve tool actions
    sandbox: bool,             // Safer execution mode
    auto_fallback: bool,
    session_store: Arc<SessionStore>,
    current_session: RwLock<Option<SessionId>>,
    persist_sessions: bool,
}
```

#### Response Parsing

Gemini CLI returns structured JSON responses:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiCliResponse {
    pub response: Option<String>,
    pub stats: Option<GeminiCliStats>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiCliStats {
    pub models: Option<HashMap<String, GeminiModelStats>>,
    pub tools: Option<GeminiToolStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiTokenStats {
    #[serde(alias = "input")]
    pub input_tokens: Option<u32>,
    #[serde(alias = "output")]
    pub output_tokens: Option<u32>,
}
```

#### Extension Management

Gemini CLI supports extensions for additional functionality:

```rust
/// Check if the Sidecar DM extension is installed.
pub async fn check_extension_status() -> (bool, String) {
    let output = Command::new("gemini")
        .args(["extensions", "list"])
        .output()
        .await;

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            if stdout.contains(Self::EXTENSION_NAME) {
                (true, format!("Extension '{}' installed", Self::EXTENSION_NAME))
            } else {
                (false, format!("Extension '{}' not installed", Self::EXTENSION_NAME))
            }
        }
        _ => (false, "Could not list extensions".to_string()),
    }
}

/// Install an extension from a git repository or local path.
pub async fn install_extension(source: &str) -> Result<String, String> {
    let output = Command::new("gemini")
        .args(["extensions", "install", source])
        .output()
        .await
        .map_err(|e| format!("Failed to run install command: {}", e))?;

    if output.status.success() {
        Ok(format!("Extension '{}' installed successfully", Self::EXTENSION_NAME))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Installation failed: {}", stderr.trim()))
    }
}
```

---

## Installation Detection

### Claude Code CLI Detection

The detection mechanism checks:
1. **Binary existence** via `which::which("claude")`
2. **Version availability** via `claude --version`
3. **Authentication status** via a minimal prompt test

```rust
pub async fn get_status() -> ClaudeCodeStatus {
    // Check if skill is installed
    let skill_installed = Self::check_plugin_installed("claude-code-bridge").await;

    // Check if binary exists
    let binary = match which::which("claude") {
        Ok(b) => b,
        Err(_) => {
            return ClaudeCodeStatus {
                installed: false,
                logged_in: false,
                skill_installed,
                version: None,
                user_email: None,
                error: Some("Claude Code CLI not installed".to_string()),
            };
        }
    };

    // Get version
    let version = match Command::new(&binary).arg("--version").output().await {
        Ok(output) if output.status.success() => {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        _ => None,
    };

    // Check authentication by attempting a minimal prompt
    let auth_result = tokio::time::timeout(
        Duration::from_secs(30),
        Command::new(&binary)
            .args(["-p", "hi", "--output-format", "json"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    ).await;

    match auth_result {
        Ok(Ok(output)) => {
            if output.status.success() {
                ClaudeCodeStatus {
                    installed: true,
                    logged_in: true,
                    skill_installed,
                    version,
                    user_email: None,
                    error: None,
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let is_auth_error = stderr.contains("login")
                    || stderr.contains("authenticate")
                    || stderr.contains("unauthorized");

                ClaudeCodeStatus {
                    installed: true,
                    logged_in: false,
                    skill_installed,
                    version,
                    user_email: None,
                    error: if is_auth_error {
                        Some("Not logged in - run 'claude' to authenticate".to_string())
                    } else {
                        Some(stderr.trim().to_string())
                    },
                }
            }
        }
        // ... error handling
    }
}
```

### Gemini CLI Detection

Similar approach with Google-specific authentication checks:

```rust
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
                        || stderr.contains("login")
                    {
                        (true, false, format!("Gemini CLI {} installed but not authenticated", version))
                    } else if check_output.status.success() || stdout.contains("response") {
                        (true, true, format!("Gemini CLI {} ready", version))
                    } else {
                        (true, false, format!("Gemini CLI {} installed (status unclear)", version))
                    }
                }
                Err(_) => (true, false, format!("Gemini CLI {} installed (run 'gemini' to verify)", version)),
            }
        }
        _ => (false, false, "Gemini CLI not installed. Run 'npm i -g @google/gemini-cli'".to_string()),
    }
}
```

---

## Installation Button Functionality

### CLI Installation

Both providers implement cross-platform terminal launching for CLI installation:

```rust
pub async fn install_cli() -> Result<(), String> {
    // Find a package manager
    let npm = which::which("npm")
        .or_else(|_| which::which("pnpm"))
        .or_else(|_| which::which("bun"))
        .map_err(|_| "No package manager found. Please install npm, pnpm, or bun.")?;

    let pkg_manager = npm.file_name().and_then(|n| n.to_str()).unwrap_or("npm");
    let install_cmd = format!("{} install -g @anthropic-ai/claude-code", pkg_manager);

    #[cfg(target_os = "linux")]
    {
        // Try common terminal emulators
        let terminals = [
            ("kitty", vec!["-e", "bash", "-c"]),
            ("gnome-terminal", vec!["--", "bash", "-c"]),
            ("konsole", vec!["-e", "bash", "-c"]),
            ("xterm", vec!["-e", "bash", "-c"]),
            ("alacritty", vec!["-e", "bash", "-c"]),
        ];

        for (term, args) in terminals {
            if which::which(term).is_ok() {
                let mut cmd = Command::new(term);
                for arg in &args {
                    cmd.arg(arg);
                }
                // Add prompt to wait before closing
                cmd.arg(format!(
                    "{}; echo ''; echo 'Press Enter to close...'; read",
                    install_cmd
                ));

                if cmd.spawn().is_ok() {
                    return Ok(());
                }
            }
        }
        Err(format!("Could not open terminal. Please run manually: {}", install_cmd))
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("osascript")
            .args([
                "-e",
                &format!(r#"tell application "Terminal" to do script "{}""#, install_cmd),
            ])
            .spawn()
            .map_err(|e| format!("Failed to open Terminal: {}", e))?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/c", "start", "cmd", "/k", &install_cmd])
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
        Ok(())
    }
}
```

### Login Flow

```rust
pub async fn login() -> Result<(), String> {
    let binary = which::which("claude").map_err(|_| "Claude Code CLI not installed")?;

    #[cfg(target_os = "linux")]
    {
        let terminals = [
            ("kitty", vec!["-e"]),
            ("gnome-terminal", vec!["--"]),
            ("konsole", vec!["-e"]),
            ("xterm", vec!["-e"]),
            ("alacritty", vec!["-e"]),
        ];

        for (term, args) in terminals {
            if which::which(term).is_ok() {
                let mut cmd = Command::new(term);
                for arg in &args {
                    cmd.arg(arg);
                }
                cmd.arg(&binary);

                if cmd.spawn().is_ok() {
                    return Ok(());
                }
            }
        }
        Err("Could not open terminal. Please run 'claude' manually to login.".to_string())
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "Terminal", binary.to_str().unwrap_or("claude")])
            .spawn()
            .map_err(|e| format!("Failed to open Terminal: {}", e))?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/c", "start", "cmd", "/k", binary.to_str().unwrap_or("claude")])
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
        Ok(())
    }
}
```

---

## Frontend-Backend Communication

### Tauri Commands

Commands are defined in `src-tauri/src/commands.rs`:

```rust
/// Get Claude Code CLI status (installed, logged in, version).
#[tauri::command]
pub async fn get_claude_code_status() -> ClaudeCodeStatus {
    ClaudeCodeProvider::get_status().await
}

/// Spawn the Claude Code login flow (opens browser for OAuth).
#[tauri::command]
pub async fn claude_code_login() -> Result<(), String> {
    ClaudeCodeProvider::login().await
}

/// Install Claude Code CLI via npm (opens terminal).
#[tauri::command]
pub async fn claude_code_install_cli() -> Result<(), String> {
    ClaudeCodeProvider::install_cli().await
}

/// Check Gemini CLI installation and authentication status.
#[tauri::command]
pub async fn check_gemini_cli_status() -> GeminiCliStatus {
    let (is_installed, is_authenticated, message) = GeminiCliProvider::check_status().await;
    GeminiCliStatus {
        is_installed,
        is_authenticated,
        message,
    }
}

/// Launch Gemini CLI for authentication.
#[tauri::command]
pub fn launch_gemini_cli_login() -> Result<(), String> {
    GeminiCliProvider::launch_login()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
```

### Frontend Bindings

Type-safe Tauri bindings in `frontend/src/bindings.rs`:

```rust
/// Status returned from Claude Code CLI check
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClaudeCodeStatus {
    pub installed: bool,
    pub logged_in: bool,
    pub skill_installed: bool,
    pub version: Option<String>,
    pub user_email: Option<String>,
    pub error: Option<String>,
}

/// Get Claude Code CLI status (installed, logged in, version)
pub async fn get_claude_code_status() -> Result<ClaudeCodeStatus, String> {
    invoke_no_args("get_claude_code_status").await
}

/// Spawn the Claude Code login flow (opens browser for OAuth)
pub async fn claude_code_login() -> Result<(), String> {
    invoke_void_no_args("claude_code_login").await
}

/// Install Claude Code CLI via npm (opens terminal)
pub async fn claude_code_install_cli() -> Result<(), String> {
    invoke_void_no_args("claude_code_install_cli").await
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiCliStatus {
    pub is_installed: bool,
    pub is_authenticated: bool,
    pub message: String,
}

/// Check Gemini CLI installation and authentication status
pub async fn check_gemini_cli_status() -> Result<GeminiCliStatus, String> {
    invoke_no_args("check_gemini_cli_status").await
}
```

### UI Component (Leptos)

The settings UI in `frontend/src/components/settings/llm.rs`:

```rust
// Status signals
let claude_code_status = RwSignal::new(ClaudeCodeStatus::default());
let claude_code_loading = RwSignal::new(false);

// Refresh function
let refresh_claude_code_status = move || {
    claude_code_loading.set(true);
    spawn_local(async move {
        match get_claude_code_status().await {
            Ok(status) => {
                let is_ready = status.installed && status.logged_in;
                provider_statuses.update(|map| {
                    map.insert("claude-code".to_string(), is_ready);
                });
                claude_code_status.set(status);
            }
            Err(e) => show_error("Claude Code Status", Some(&e), None),
        }
        claude_code_loading.set(false);
    });
};

// UI rendering
view! {
    <div class="p-4 rounded-lg bg-[var(--bg-deep)] border border-[var(--border-subtle)] space-y-3">
        // Status indicators
        <div class="flex flex-wrap gap-2">
            <div class=move || format!(
                "flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium {}",
                if status.installed { "bg-green-500/20 text-green-400" }
                else { "bg-red-500/20 text-red-400" }
            )>
                <span class=move || format!(
                    "w-2 h-2 rounded-full {}",
                    if status.installed { "bg-green-400" } else { "bg-red-400" }
                )></span>
                {if status.installed { "CLI Installed" } else { "CLI Not Installed" }}
            </div>
            // ... more status badges
        </div>

        // Action buttons
        <div class="flex flex-wrap gap-2 pt-2">
            {move || if !status.installed {
                view! {
                    <button
                        class="px-3 py-1.5 text-xs font-medium rounded-lg bg-[var(--accent-primary)] text-white"
                        disabled=is_loading
                        on:click=move |_| {
                            spawn_local(async move {
                                match claude_code_install_cli().await {
                                    Ok(_) => show_success("Installing CLI", Some("Opening terminal...")),
                                    Err(e) => show_error("Install Failed", Some(&e), None),
                                }
                            });
                        }
                    >
                        "Install CLI"
                    </button>
                }.into_any()
            } else if !status.logged_in {
                view! {
                    <button
                        class="px-3 py-1.5 text-xs font-medium rounded-lg bg-[var(--accent-primary)] text-white"
                        disabled=is_loading
                        on:click=move |_| {
                            spawn_local(async move {
                                match claude_code_login().await {
                                    Ok(_) => show_success("Logging In", Some("Opening terminal...")),
                                    Err(e) => show_error("Login Failed", Some(&e), None),
                                }
                            });
                        }
                    >
                        "Login"
                    </button>
                }.into_any()
            } else {
                view! { <span></span> }.into_any()
            }}

            <button
                class="px-3 py-1.5 text-xs font-medium rounded-lg bg-[var(--bg-elevated)]"
                disabled=is_loading
                on:click=move |_| refresh_claude_code_status()
            >
                {if is_loading { "Checking..." } else { "Refresh Status" }}
            </button>
        </div>
    </div>
}
```

---

## Session Management

### Session Store

Both providers use a shared `SessionStore` for tracking conversations:

```rust
pub struct SessionStore {
    sessions: RwLock<HashMap<SessionId, SessionInfo>>,
    storage_path: Option<PathBuf>,
    max_sessions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: SessionId,
    pub provider_id: String,
    pub created_at: u64,
    pub last_used_at: u64,
    pub message_count: u32,
    pub is_compacted: bool,
    pub title: Option<String>,
    pub working_dir: Option<String>,
}
```

### ProviderSession Trait

Both providers implement the `ProviderSession` trait:

```rust
#[async_trait]
pub trait ProviderSession: Send + Sync {
    fn supports_sessions(&self) -> bool;
    async fn current_session(&self) -> Option<SessionId>;
    async fn resume_session(&self, session_id: &SessionId) -> SessionResult<SessionInfo>;
    async fn continue_session(&self) -> SessionResult<SessionInfo>;
    async fn fork_session(&self, session_id: &SessionId) -> SessionResult<SessionId>;
    async fn compact_session(&self) -> SessionResult<()>;
    async fn get_session_info(&self, session_id: &SessionId) -> SessionResult<SessionInfo>;
    async fn list_sessions(&self, limit: usize) -> SessionResult<Vec<SessionInfo>>;
}
```

### Claude Code Session Commands

Claude Code CLI supports native session management:

```bash
# Continue most recent session
claude -p "message" --continue

# Resume specific session
claude -p "message" --resume abc123

# Fork session (create new branch)
claude -p "message" --resume abc123 --fork-session

# Compact session to reduce tokens
claude -p "/compact" --resume abc123
```

---

## Adding This Feature to Your Project

### Step 1: Create the Provider Struct

```rust
// src/providers/my_cli_provider.rs

use async_trait::async_trait;
use tokio::process::Command;
use std::process::Stdio;

pub struct MyCLIProvider {
    timeout_secs: u64,
    model: Option<String>,
}

impl MyCLIProvider {
    pub fn new() -> Self {
        Self {
            timeout_secs: 120,
            model: None,
        }
    }

    /// Check if CLI is available
    pub async fn is_available() -> bool {
        Command::new("my-cli")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Get version
    pub async fn version() -> Option<String> {
        let output = Command::new("my-cli")
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
}
```

### Step 2: Implement Status Detection

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyCLIStatus {
    pub installed: bool,
    pub authenticated: bool,
    pub version: Option<String>,
    pub error: Option<String>,
}

impl MyCLIProvider {
    pub async fn get_status() -> MyCLIStatus {
        // Check installation
        let version = match Command::new("my-cli").arg("--version").output().await {
            Ok(output) if output.status.success() => {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
            _ => {
                return MyCLIStatus {
                    installed: false,
                    authenticated: false,
                    version: None,
                    error: Some("CLI not installed".to_string()),
                };
            }
        };

        // Check authentication with a test command
        let auth_check = Command::new("my-cli")
            .args(["auth", "status"])
            .output()
            .await;

        match auth_check {
            Ok(output) if output.status.success() => MyCLIStatus {
                installed: true,
                authenticated: true,
                version,
                error: None,
            },
            _ => MyCLIStatus {
                installed: true,
                authenticated: false,
                version,
                error: Some("Not authenticated".to_string()),
            },
        }
    }
}
```

### Step 3: Implement Installation

```rust
impl MyCLIProvider {
    pub async fn install_cli() -> Result<(), String> {
        let install_cmd = "npm install -g @vendor/my-cli";

        #[cfg(target_os = "linux")]
        {
            let terminals = [
                ("kitty", vec!["-e", "bash", "-c"]),
                ("gnome-terminal", vec!["--", "bash", "-c"]),
                ("xterm", vec!["-e", "bash", "-c"]),
            ];

            for (term, args) in terminals {
                if which::which(term).is_ok() {
                    let mut cmd = Command::new(term);
                    for arg in &args {
                        cmd.arg(arg);
                    }
                    cmd.arg(format!("{}; echo 'Press Enter...'; read", install_cmd));

                    if cmd.spawn().is_ok() {
                        return Ok(());
                    }
                }
            }
            Err(format!("Run manually: {}", install_cmd))
        }

        #[cfg(target_os = "macos")]
        {
            Command::new("osascript")
                .args(["-e", &format!(r#"tell app "Terminal" to do script "{}""#, install_cmd)])
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(())
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .args(["/c", "start", "cmd", "/k", &install_cmd])
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(())
        }
    }
}
```

### Step 4: Create Tauri Commands

```rust
// src/commands.rs

#[tauri::command]
pub async fn get_my_cli_status() -> MyCLIStatus {
    MyCLIProvider::get_status().await
}

#[tauri::command]
pub async fn my_cli_install() -> Result<(), String> {
    MyCLIProvider::install_cli().await
}

#[tauri::command]
pub async fn my_cli_login() -> Result<(), String> {
    MyCLIProvider::login().await
}
```

### Step 5: Register Commands

```rust
// src/main.rs

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_my_cli_status,
            my_cli_install,
            my_cli_login,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Step 6: Create Frontend Bindings

```typescript
// TypeScript (if using JS frontend)
interface MyCLIStatus {
  installed: boolean;
  authenticated: boolean;
  version: string | null;
  error: string | null;
}

async function getMyCLIStatus(): Promise<MyCLIStatus> {
  return await invoke('get_my_cli_status');
}

async function myCLIInstall(): Promise<void> {
  return await invoke('my_cli_install');
}

async function myCLILogin(): Promise<void> {
  return await invoke('my_cli_login');
}
```

```rust
// Rust (if using Leptos)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MyCLIStatus {
    pub installed: bool,
    pub authenticated: bool,
    pub version: Option<String>,
    pub error: Option<String>,
}

pub async fn get_my_cli_status() -> Result<MyCLIStatus, String> {
    invoke_no_args("get_my_cli_status").await
}
```

### Step 7: Build the UI

```rust
// Leptos component example
#[component]
pub fn MyCLISettings() -> impl IntoView {
    let status = RwSignal::new(MyCLIStatus::default());
    let loading = RwSignal::new(false);

    let refresh_status = move || {
        loading.set(true);
        spawn_local(async move {
            match get_my_cli_status().await {
                Ok(s) => status.set(s),
                Err(e) => log::error!("Failed to get status: {}", e),
            }
            loading.set(false);
        });
    };

    // Initial load
    Effect::new(move |_| refresh_status());

    view! {
        <div class="cli-status">
            {move || {
                let s = status.get();
                if !s.installed {
                    view! {
                        <button on:click=move |_| {
                            spawn_local(async move {
                                let _ = my_cli_install().await;
                            });
                        }>
                            "Install CLI"
                        </button>
                    }.into_any()
                } else if !s.authenticated {
                    view! {
                        <button on:click=move |_| {
                            spawn_local(async move {
                                let _ = my_cli_login().await;
                            });
                        }>
                            "Login"
                        </button>
                    }.into_any()
                } else {
                    view! {
                        <span class="ready">"Ready: v"{s.version}</span>
                    }.into_any()
                }
            }}
            <button on:click=move |_| refresh_status() disabled=loading.get()>
                "Refresh"
            </button>
        </div>
    }
}
```

---

## Summary

This document covered the complete implementation of CLI-based LLM providers:

1. **Provider Architecture**: How providers are structured with builder patterns, session management, and trait implementations
2. **Installation Detection**: Cross-platform binary detection and authentication verification
3. **Installation Automation**: Opening terminals and running package manager commands
4. **IPC Communication**: Tauri commands, frontend bindings, and reactive UI updates
5. **Session Management**: Tracking conversations across CLI invocations

The key design principles are:
- **Cross-platform support**: Handle Linux, macOS, and Windows terminal launching
- **Graceful degradation**: Show helpful messages when CLIs aren't available
- **Reactive UI**: Update status badges in real-time
- **Error handling**: Provide actionable error messages
- **Session persistence**: Store session IDs for conversation continuity

By following these patterns, you can add CLI-based LLM provider support to any Tauri application.
