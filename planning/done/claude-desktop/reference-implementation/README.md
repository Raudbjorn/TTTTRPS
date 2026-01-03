# 🌉 Claude Bridge

A Tauri + Leptos application that bridges local processes to Claude Desktop via Chrome DevTools Protocol (CDP).

**Use Case:** When you need to communicate with Claude from a process that can't (or shouldn't) handle API authentication directly, Claude Bridge lets you piggyback on your existing Claude Desktop session.

```
┌─────────────────┐     CDP/IPC      ┌──────────────────┐     HTTPS     ┌─────────────────┐
│  Your Process   │ ───────────────► │  Claude Bridge   │ ───────────► │  Claude Desktop  │
│  (no auth)      │                  │  (this app)      │               │  (authenticated) │
└─────────────────┘                  └──────────────────┘               └─────────────────┘
```

## 📦 Project Structure

```
claude-bridge/
├── claude-cdp/          # Core CDP client library
│   └── src/
│       ├── lib.rs       # Library entry point
│       ├── client.rs    # CDP client implementation
│       └── error.rs     # Error types
├── src-tauri/           # Tauri backend
│   └── src/
│       ├── main.rs      # Entry point
│       ├── lib.rs       # App setup
│       └── commands.rs  # Tauri commands
├── frontend/            # Leptos frontend
│   └── src/
│       ├── lib.rs       # Main app component
│       ├── tauri.rs     # Tauri bindings
│       └── components/  # UI components
└── scripts/
    └── launch-claude-debug.sh
```

## 🚀 Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (1.75+)
- [Trunk](https://trunkrs.dev/) for building the frontend: `cargo install trunk`
- [Tauri CLI](https://tauri.app/): `cargo install tauri-cli`
- Claude Desktop installed

### 1. Launch Claude Desktop with CDP

```bash
# Using the provided script
./scripts/launch-claude-debug.sh

# Or manually
claude-desktop --remote-debugging-port=9222

# On Arch Linux, you might need to find the binary:
/opt/Claude/claude --remote-debugging-port=9222
```

### 2. Build and Run

```bash
# Development mode
cargo tauri dev

# Production build
cargo tauri build
```

## 🔧 Using the CDP Library Directly

The `claude-cdp` crate can be used standalone in your Rust projects:

```rust
use claude_cdp::{ClaudeClient, ClaudeConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client with default config (port 9222)
    let mut client = ClaudeClient::new();
    
    // Or with custom config
    let mut client = ClaudeClient::with_config(
        ClaudeConfig::default()
            .with_port(9333)
            .with_timeout(120)
    );
    
    // Connect to Claude Desktop
    client.connect().await?;
    
    // Send a message and get response
    let response = client.send_message("Explain monads in one sentence").await?;
    println!("Claude: {}", response);
    
    // Start a new conversation
    client.new_conversation().await?;
    
    // Get conversation history
    let messages = client.get_conversation().await?;
    for msg in messages {
        println!("[{}]: {}", msg.role, msg.content);
    }
    
    // Disconnect
    client.disconnect().await;
    
    Ok(())
}
```

Add to your `Cargo.toml`:

```toml
[dependencies]
claude-cdp = { path = "path/to/claude-bridge/claude-cdp" }
tokio = { version = "1", features = ["full"] }
```

## 🏗️ Architecture

### CDP Communication

Claude Desktop is an Electron app, which means it runs on Chromium. When launched with `--remote-debugging-port`, it exposes the Chrome DevTools Protocol over WebSocket.

The `claude-cdp` crate:

1. **Connects** to the CDP WebSocket endpoint
2. **Finds** the Claude conversation page
3. **Injects JavaScript** to interact with the UI:
   - Find the message input element
   - Type the message
   - Click send / press Enter
   - Wait for and extract the response

### Tauri Commands

The Tauri backend exposes these commands to the frontend:

| Command | Description |
|---------|-------------|
| `connect` | Connect to Claude Desktop via CDP |
| `disconnect` | Disconnect from Claude Desktop |
| `get_status` | Get current connection status |
| `send_message` | Send a message and wait for response |
| `new_conversation` | Start a new conversation |
| `get_conversation` | Get conversation history |
| `update_config` | Update CDP configuration |

### Frontend (Leptos)

The Leptos frontend provides:

- Connection status indicator
- Message display with user/assistant differentiation
- Auto-scrolling message area
- Auto-resizing textarea input
- Configuration panel for CDP settings

## ⚠️ Limitations

1. **UI Selectors**: The CDP client relies on CSS selectors to find UI elements. If Claude Desktop updates its UI, selectors may need updating.

2. **Rate Limiting**: You're subject to the same rate limits as the Claude Desktop app itself.

3. **Single Session**: Only one bridge connection per Claude Desktop instance.

4. **No Streaming**: Currently waits for full response (streaming support could be added).

## 🔐 Security Considerations

- CDP only listens on localhost by default
- No API keys are stored or transmitted
- Uses your existing Claude Desktop authentication

## 🛠️ Development

### Running Tests

```bash
# Test the CDP library
cd claude-cdp && cargo test

# Test with logging
RUST_LOG=debug cargo test -- --nocapture
```

### Building for Release

```bash
cargo tauri build
```

The built application will be in `src-tauri/target/release/bundle/`.

## 📝 License

MIT

---

Built with 🦀 Rust, ⚡ Tauri, and 🎯 Leptos
