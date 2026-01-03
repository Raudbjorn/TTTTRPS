//! Simple example of using gemini-cli-rs.
//!
//! Usage:
//!   cargo run --example simple
//!
//! Requires Gemini CLI to be installed and authenticated.

use gemini_cli_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("gemini_cli_rs=debug")
        .init();

    // Create client
    let client = GeminiCliClient::new()?;

    // Check version
    println!("🔍 Checking Gemini CLI version...");
    let version = client.version().await?;
    println!("✅ Gemini CLI version: {}\n", version);

    // Send a simple prompt
    println!("💬 Sending prompt...\n");
    let response = client.prompt("What is the capital of Iceland? Reply in one sentence.").await?;

    println!("🤖 Response:");
    println!("{}\n", response.text());

    // Show stats if available
    if let Some(stats) = &response.stats {
        if let Some(tools) = &stats.tools {
            println!("🔧 Tool calls: {}", tools.total_calls);
        }
        if let Some(models) = &stats.models {
            for (model_name, model_stats) in models {
                if let Some(tokens) = &model_stats.tokens {
                    println!(
                        "📊 {}: {} prompt / {} response tokens (cached: {})",
                        model_name, tokens.prompt, tokens.candidates, tokens.cached
                    );
                }
            }
        }
    }

    Ok(())
}
