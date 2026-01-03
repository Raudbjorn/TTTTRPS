# 🌉 Claude Code Bridge

Programmatic integration with Claude Code CLI - enables "Claude calling Claude" patterns.

## Overview

```
┌─────────────────┐                    ┌─────────────────┐
│  Your Process   │                    │  Claude Desktop │
│  (any language) │                    │  (MCP client)   │
└────────┬────────┘                    └────────┬────────┘
         │                                      │
         │ ┌────────────────────────────────────┘
         │ │
         ▼ ▼
┌─────────────────────────────────────────────────────────┐
│              Claude Code Bridge                          │
│  ┌─────────────────┐    ┌─────────────────────────────┐ │
│  │   Rust Library  │    │       MCP Server            │ │
│  │  (claude-code-rs)    │   (claude-code-mcp)         │ │
│  └────────┬────────┘    └────────────┬────────────────┘ │
└───────────┼──────────────────────────┼──────────────────┘
            │                          │
            └──────────┬───────────────┘
                       ▼
              ┌─────────────────┐
              │  claude -p ...  │
              │ (Claude Code CLI)
              └─────────────────┘
```

## Components

| Component | Language | Description |
|-----------|----------|-------------|
| **claude-code-rs** | Rust | Library for invoking Claude Code CLI |
| **mcp-server** | TypeScript | MCP server exposing Claude Code as tools |
| **skill** | Markdown | Claude skill for using the bridge |

## Quick Start

### 1. Rust Library

```rust
use claude_code_rs::ClaudeCodeClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClaudeCodeClient::new()?;
    
    // Simple prompt
    let response = client.prompt("Explain ownership in Rust").await?;
    println!("{}", response.text());
    
    // Continue conversation
    let response = client.continue_conversation("Give me an example").await?;
    println!("{}", response.text());
    
    Ok(())
}
```

Add to `Cargo.toml`:

```toml
[dependencies]
claude-code-rs = { path = "path/to/claude-code-bridge/claude-code-rs" }
tokio = { version = "1", features = ["full"] }
```

### 2. MCP Server

For Claude Desktop, add to `~/.config/claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "claude-code": {
      "command": "node",
      "args": ["/path/to/claude-code-bridge/mcp-server/dist/index.js"]
    }
  }
}
```

Build and run:

```bash
cd mcp-server
npm install
npm run build
```

Then Claude Desktop can use:
- `claude_prompt` - Send prompts to Claude Code
- `claude_continue` - Continue conversations
- `claude_resume` - Resume by session ID
- `claude_version` - Get version info

### 3. Claude Skill

Copy `skill/SKILL.md` to your Claude skills directory for context on using the bridge.

## Use Cases

### 🔀 Parallel Work

Let Claude Code work on a subtask while you continue with something else:

```
I'll have Claude Code refactor the auth module while we discuss the API design.

[claude_prompt]
prompt: "Refactor src/auth to use async/await throughout. Run tests after."
working_dir: "/home/user/project"
```

### 🎯 Specialized Context

Give Claude Code access to a specific directory:

```rust
let client = ClaudeCodeClientBuilder::new()
    .working_dir("/home/user/other-project")
    .build()?;

let response = client.prompt("What's the tech stack of this project?").await?;
```

### 🔄 Conversation Chains

```rust
// First message
let r1 = client.prompt("List all deprecated functions").await?;
let session = r1.session_id().unwrap().to_string();

// Later...
let r2 = client.resume("Now fix the first one", &session).await?;
```

### 🧪 Test Delegation

```json
{
  "tool": "claude_prompt",
  "arguments": {
    "prompt": "Generate unit tests for src/utils.ts with >90% coverage",
    "working_dir": "/home/user/project",
    "timeout_secs": 600
  }
}
```

## Configuration

### Rust Client Options

```rust
let client = ClaudeCodeClientBuilder::new()
    .working_dir("/path/to/project")     // Working directory
    .timeout_secs(300)                   // 5 minute timeout
    .model("claude-sonnet-4-20250514")   // Specific model
    .system_prompt("You are a code reviewer")
    .max_tokens(4096)
    .verbose()                           // Debug output
    .build()?;
```

### MCP Server Environment

| Variable | Description |
|----------|-------------|
| `CLAUDE_CODE_PATH` | Custom path to Claude Code binary |
| `CLAUDE_CODE_TIMEOUT` | Default timeout in seconds |

## Error Handling

Both the Rust library and MCP server use errors-as-values:

```rust
match client.prompt("...").await {
    Ok(response) => {
        if let Some(err) = response.error {
            eprintln!("Claude Code error: {}", err);
        } else {
            println!("{}", response.text());
        }
    }
    Err(ClaudeCodeError::Timeout { seconds }) => {
        eprintln!("Timed out after {}s", seconds);
    }
    Err(ClaudeCodeError::NotFound) => {
        eprintln!("Claude Code CLI not installed");
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## Requirements

- **Claude Code CLI**: `npm install -g @anthropic-ai/claude-code`
- **Rust** (for library): 1.75+
- **Node.js** (for MCP server): 20+

## Project Structure

```
claude-code-bridge/
├── claude-code-rs/          # Rust library
│   ├── src/
│   │   ├── lib.rs           # Entry point
│   │   ├── client.rs        # Main client
│   │   ├── config.rs        # Configuration
│   │   ├── output.rs        # Response parsing
│   │   └── error.rs         # Error types
│   └── examples/
│       ├── simple.rs
│       └── conversation.rs
├── mcp-server/              # TypeScript MCP server
│   └── src/
│       └── index.ts         # Server implementation
└── skill/
    └── SKILL.md             # Claude skill file
```

## Comparison with CDP Bridge

| Feature | Claude Desktop (CDP) | Claude Code (CLI) |
|---------|---------------------|-------------------|
| Authentication | Uses your session | Uses your session |
| Integration | Chrome DevTools Protocol | Direct CLI invocation |
| Complexity | Higher (browser automation) | Lower (spawn process) |
| Reliability | UI selectors may break | Stable CLI interface |
| Features | Full Desktop features | Full Code features |
| Best for | Desktop-specific workflows | Coding tasks |

## License

MIT

---

Built with 🦀 Rust and 📦 TypeScript for the Claude ecosystem
