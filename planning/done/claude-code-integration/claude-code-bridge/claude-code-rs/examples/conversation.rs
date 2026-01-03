//! Example of managing conversations with claude-code-rs.
//!
//! Usage:
//!   cargo run --example conversation
//!
//! Requires Claude Code CLI to be installed.

use claude_code_rs::prelude::*;
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("claude_code_rs=info")
        .init();

    println!("🗣️  Claude Code Conversation Example");
    println!("=====================================");
    println!("Commands: /new (new conversation), /quit (exit)\n");

    let client = ClaudeCodeClientBuilder::new()
        .timeout_secs(120)
        .build()?;

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut first_message = true;

    loop {
        print!("You: ");
        stdout.flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        match input {
            "/quit" | "/exit" => break,
            "/new" => {
                first_message = true;
                println!("📝 Starting new conversation\n");
                continue;
            }
            _ => {}
        }

        let response = if first_message {
            first_message = false;
            client.prompt(input).await
        } else {
            client.continue_conversation(input).await
        };

        match response {
            Ok(resp) => {
                println!("\nClaude: {}\n", resp.text());

                if let Some(usage) = &resp.usage {
                    println!(
                        "  [tokens: {} in / {} out]",
                        usage.input_tokens, usage.output_tokens
                    );
                }
            }
            Err(e) => {
                eprintln!("\n❌ Error: {}\n", e);
            }
        }
    }

    println!("👋 Goodbye!");
    Ok(())
}
