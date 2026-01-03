# 🌉 Gemini CLI Bridge

Programmatic integration with Google's Gemini CLI - enables "Claude calling Gemini" (and vice versa!) patterns.

## Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Your Process                              │
│           (Rust app, Python script, Claude, etc.)               │
└────────────────────────────────┬────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Gemini CLI Bridge                             │
│  ┌─────────────────────┐    ┌───────────────────────────────┐   │
│  │    Rust Library     │    │        MCP Server             │   │
│  │   (gemini-cli-rs)   │    │    (gemini-cli-mcp)           │   │
│  └──────────┬──────────┘    └──────────────┬────────────────┘   │
└─────────────┼──────────────────────────────┼────────────────────┘
              │                              │
              └──────────────┬───────────────┘
                             ▼
                    ┌─────────────────┐
                    │  gemini -p ...  │
                    │ (Gemini CLI)    │
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │   Gemini API    │
                    │ (gemini-2.5-pro)│
                    └─────────────────┘
```

## Why Gemini CLI?

| Feature | Benefit |
|---------|---------|
| 🔍 **Google Search** | Native search grounding for real-time info |
| 📚 **1M Token Context** | Handle massive codebases and documents |
| 🆓 **Generous Free Tier** | 1000 requests/day with personal account |
| 🤖 **Alternative Perspective** | Cross-AI validation and diverse insights |
| ⚡ **Fast Flash Model** | Quick responses with gemini-2.5-flash |

## Components

| Component | Language | Description |
|-----------|----------|-------------|
| **gemini-cli-rs** | Rust | Library for invoking Gemini CLI |
| **mcp-server** | TypeScript | MCP server exposing Gemini as tools |
| **skill** | Markdown | Skill file for Claude to use the bridge |

## Quick Start

### 1. Install Gemini CLI

```bash
npm install -g @google/gemini-cli
gemini  # First run: authenticate with Google
```

### 2. Rust Library

```rust
use gemini_cli_rs::GeminiCliClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = GeminiCliClient::new()?;
    
    // Simple prompt
    let response = client.prompt("Explain quantum computing").await?;
    println!("{}", response.text());
    
    // With piped input
    let code = std::fs::read_to_string("main.rs")?;
    let response = client.prompt_with_stdin("Review this code", &code).await?;
    println!("{}", response.text());
    
    Ok(())
}
```

Add to `Cargo.toml`:

```toml
[dependencies]
gemini-cli-rs = { path = "path/to/gemini-cli-bridge/gemini-cli-rs" }
tokio = { version = "1", features = ["full"] }
```

### 3. MCP Server

For Claude Desktop, add to `~/.config/claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "gemini": {
      "command": "node",
      "args": ["/path/to/gemini-cli-bridge/mcp-server/dist/index.js"]
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
- `gemini_prompt` - Send prompts to Gemini
- `gemini_prompt_with_input` - Send prompts with piped content
- `gemini_search` - Use Google Search grounding
- `gemini_version` - Get version info

### 4. Claude Skill

Copy `skill/SKILL.md` to your Claude skills directory for context on using the bridge.

## Use Cases

### 🔍 Real-Time Search

Gemini has native Google Search integration:

```rust
let response = client.prompt("What's trending on Hacker News right now?").await?;
```

### 🤝 Cross-AI Collaboration

Let Claude and Gemini work together:

```
# In Claude Desktop with MCP server configured:
"I'll analyze the architecture, and let's have Gemini review the security aspects"

[gemini_prompt]
prompt: "Review this code for security vulnerabilities"
working_dir: "/project"
```

### 📊 Large Context Analysis

Gemini's 1M token window handles large codebases:

```rust
let client = GeminiCliClientBuilder::new()
    .working_dir("/path/to/large-monorepo")
    .model("gemini-2.5-pro")  // 1M context
    .build()?;

let response = client.prompt("Analyze dependencies across all services").await?;
```

### ⚡ Fast Responses

Use Flash for quick tasks:

```rust
let client = GeminiCliClientBuilder::new()
    .model("gemini-2.5-flash")
    .build()?;

let response = client.prompt("Convert this JSON to YAML").await?;
```

## Configuration

### Rust Client Options

```rust
let client = GeminiCliClientBuilder::new()
    .working_dir("/path/to/project")     // Working directory
    .timeout_secs(300)                   // 5 minute timeout
    .model("gemini-2.5-pro")             // Specific model
    .yolo_mode()                         // Auto-approve tool actions
    .sandbox()                           // Safer execution
    .verbose()                           // Debug output
    .build()?;
```

### Authentication

```bash
# Personal Google Account (recommended)
gemini  # Interactive login

# API Key
export GOOGLE_API_KEY="your-key"

# Vertex AI
export GOOGLE_API_KEY="your-key"
export GOOGLE_GENAI_USE_VERTEXAI=true
```

## Error Handling

```rust
match client.prompt("...").await {
    Ok(response) => {
        println!("{}", response.text());
        
        // Check stats
        if let Some(stats) = response.stats {
            if let Some(tools) = stats.tools {
                println!("Tool calls: {}", tools.total_calls);
            }
        }
    }
    Err(GeminiCliError::AuthenticationError { message }) => {
        eprintln!("Auth failed: {}. Run 'gemini' to authenticate.", message);
    }
    Err(GeminiCliError::RateLimitError { message }) => {
        eprintln!("Rate limited: {}", message);
    }
    Err(GeminiCliError::Timeout { seconds }) => {
        eprintln!("Timed out after {}s", seconds);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## Comparison: Claude Code vs Gemini CLI

| Feature | Claude Code | Gemini CLI |
|---------|-------------|------------|
| **Install** | `npm i -g @anthropic-ai/claude-code` | `npm i -g @google/gemini-cli` |
| **Auth** | Anthropic API key | Google account (free!) |
| **Context Window** | 200K tokens | 1M tokens |
| **Search** | Web search tool | Native Google Search |
| **Free Tier** | No (paid API key required) | 1000 req/day |
| **YOLO Mode** | `--dangerously-skip-permissions` | `--yolo` |
| **Best For** | Coding, reasoning | Search, large context |

## Project Structure

```
gemini-cli-bridge/
├── gemini-cli-rs/          # Rust library
│   ├── src/
│   │   ├── lib.rs          # Entry point
│   │   ├── client.rs       # Main client
│   │   ├── config.rs       # Configuration
│   │   ├── output.rs       # Response parsing
│   │   └── error.rs        # Error types
│   └── examples/
│       ├── simple.rs
│       └── streaming.rs
├── mcp-server/             # TypeScript MCP server
│   └── src/
│       └── index.ts        # Server implementation
└── skill/
    └── SKILL.md            # Claude skill file
```

## Requirements

- **Gemini CLI**: `npm install -g @google/gemini-cli`
- **Rust** (for library): 1.75+
- **Node.js** (for MCP server): 20+
- **Google Account**: For authentication

## License

MIT

---

Built with 🦀 Rust and 📦 TypeScript for multi-AI collaboration
