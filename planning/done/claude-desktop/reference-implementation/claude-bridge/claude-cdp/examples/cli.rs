//! Simple CLI example for using claude-cdp directly.
//!
//! Usage:
//!   cargo run --example cli -- "Your message here"
//!
//! Make sure Claude Desktop is running with:
//!   claude-desktop --remote-debugging-port=9222

use claude_cdp::{ClaudeClient, ClaudeConfig, ClaudeCdpError};
use std::env;
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("claude_cdp=info".parse().unwrap()),
        )
        .init();

    // Get port from env or use default
    let port: u16 = env::var("CLAUDE_CDP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(9222);

    println!("🌉 Claude Bridge CLI");
    println!("   Connecting to port {}...", port);

    // Create and connect client
    let config = ClaudeConfig::default().with_port(port).with_timeout(120);
    let mut client = ClaudeClient::with_config(config);

    client.connect().await?;
    println!("   ✅ Connected to Claude Desktop\n");

    // Check for command line argument
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        // Single message mode
        let message = args[1..].join(" ");
        println!("You: {}", message);
        
        let response = client.send_message(&message).await?;
        println!("\nClaude: {}\n", response);
    } else {
        // Interactive mode
        println!("Interactive mode. Type your messages (Ctrl+D to exit).\n");

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            print!("You: ");
            stdout.flush()?;

            let mut line = String::new();
            if stdin.lock().read_line(&mut line)? == 0 {
                // EOF
                break;
            }

            let message = line.trim();
            if message.is_empty() {
                continue;
            }

            // Handle special commands
            match message {
                "/new" => {
                    client.new_conversation().await?;
                    println!("📝 Started new conversation\n");
                    continue;
                }
                "/quit" | "/exit" => break,
                "/help" => {
                    println!("Commands:");
                    println!("  /new   - Start new conversation");
                    println!("  /quit  - Exit");
                    println!("  /help  - Show this help\n");
                    continue;
                }
                _ => {}
            }

            // Send message
            match client.send_message(message).await {
                Ok(response) => {
                    println!("\nClaude: {}\n", response);
                }
                Err(ClaudeCdpError::ResponseTimeout { seconds }) => {
                    println!("\n⏱️  Timeout after {}s waiting for response\n", seconds);
                }
                Err(e) => {
                    println!("\n❌ Error: {}\n", e);
                }
            }
        }
    }

    client.disconnect().await;
    println!("👋 Disconnected");

    Ok(())
}
