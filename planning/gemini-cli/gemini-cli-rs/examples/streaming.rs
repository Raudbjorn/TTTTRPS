//! Streaming example - shows how to process Gemini CLI output in real-time.
//!
//! Usage:
//!   cargo run --example streaming
//!
//! Requires Gemini CLI to be installed and authenticated.

use gemini_cli_rs::prelude::*;
use tokio::io::{AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("gemini_cli_rs=info")
        .init();

    println!("🌊 Gemini CLI Streaming Example");
    println!("================================\n");

    let client = GeminiCliClientBuilder::new()
        .timeout_secs(120)
        .build()?;

    println!("💬 Prompt: Explain the benefits of Rust in 3 bullet points\n");
    println!("📡 Streaming response:\n");

    let reader = client
        .prompt_streaming("Explain the benefits of Rust in 3 bullet points")
        .await?;

    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        // Try to parse as a stream event
        match serde_json::from_str::<StreamEvent>(&line) {
            Ok(event) => match event {
                StreamEvent::Init { model, session_id, .. } => {
                    println!("🚀 Session started: {} (model: {})", session_id, model);
                }
                StreamEvent::Message { role, content, delta, .. } => {
                    if role == "assistant" {
                        if delta {
                            print!("{}", content);
                        } else {
                            println!("\n📝 {}", content);
                        }
                    }
                }
                StreamEvent::ToolUse { tool_name, .. } => {
                    println!("\n🔧 Using tool: {}", tool_name);
                }
                StreamEvent::ToolResult { tool_id, status, .. } => {
                    println!("   ✓ Tool {} completed: {}", tool_id, status);
                }
                StreamEvent::Result { stats, .. } => {
                    println!("\n\n✅ Complete!");
                    if let Some(s) = stats {
                        if let Some(tools) = s.tools {
                            println!("   Tool calls: {}", tools.total_calls);
                        }
                    }
                }
                StreamEvent::Error { message, .. } => {
                    eprintln!("\n❌ Error: {}", message);
                }
            },
            Err(_) => {
                // Not a valid event, might be partial output
                print!("{}", line);
            }
        }
    }

    println!("\n\n👋 Done!");
    Ok(())
}
