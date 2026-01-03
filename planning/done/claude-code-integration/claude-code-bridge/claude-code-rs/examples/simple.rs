//! Simple example of using claude-code-rs.
//!
//! Usage:
//!   cargo run --example simple
//!
//! Requires Claude Code CLI to be installed.

use claude_code_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("claude_code_rs=debug")
        .init();

    // Create client
    let client = ClaudeCodeClient::new()?;

    // Check version
    println!("Checking Claude Code version...");
    let version = client.version().await?;
    println!("✅ Claude Code version: {}\n", version);

    // Send a simple prompt
    println!("Sending prompt...");
    let response = client.prompt("What is the capital of Iceland?").await?;

    println!("📝 Response:");
    println!("{}", response.text());

    if let Some(session_id) = response.session_id() {
        println!("\n📌 Session ID: {}", session_id);
    }

    if let Some(usage) = &response.usage {
        println!(
            "💰 Tokens: {} in / {} out",
            usage.input_tokens, usage.output_tokens
        );
    }

    Ok(())
}
