---
name: gemini-cli-bridge
description: Integrate with Google's Gemini CLI for delegating tasks to Gemini. Use when you need an alternative AI perspective, Google Search grounding, parallel task execution, or want to leverage Gemini's 1M token context window. Enables "Claude calling Gemini" patterns for diverse AI collaboration.
---

# Gemini CLI Bridge

Delegate tasks to Google's Gemini CLI from any context.

## When to Use

- **Google Search Grounding**: Gemini has native Google Search integration
- **Alternative Perspective**: Get a different AI's take on a problem
- **Large Context**: Gemini 2.5 Pro has a 1M token context window
- **Parallel Work**: Have Gemini work on a subtask while you continue
- **Code Review**: Cross-AI code review for better coverage

## Key Differences from Claude Code

| Feature | Claude Code | Gemini CLI |
|---------|-------------|------------|
| **Search** | Web search tool | Native Google Search |
| **Context** | 200K tokens | 1M tokens |
| **Free tier** | API key required | 1000 req/day free |
| **Strengths** | Coding, analysis | Search, multimodal |

## Usage Patterns

### Via MCP Server (Recommended)

If the `gemini-cli-mcp` server is configured, use the MCP tools:

```
Use gemini_prompt to ask Gemini: "What are the latest developments in WebGPU?"
```

Tools available:
- `gemini_prompt` - Send a prompt to Gemini CLI
- `gemini_prompt_with_input` - Send prompt with piped content
- `gemini_search` - Use Google Search grounding
- `gemini_version` - Get version info

### Via CLI (Direct)

Execute Gemini CLI directly:

```bash
# Single prompt
gemini -p "Explain this error message" --output-format json

# Pipe input
cat error.log | gemini -p "Analyze these errors"

# With Google Search
gemini -p "What's the current price of Bitcoin?"

# YOLO mode (auto-approve all actions)
gemini -p "Fix all linting errors" --yolo
```

### Via Rust Library

```rust
use gemini_cli_rs::GeminiCliClient;

let client = GeminiCliClient::new()?;
let response = client.prompt("Analyze this codebase").await?;
println!("{}", response.text());
```

## Configuration

### MCP Server Setup

Add to Claude Desktop's `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "gemini": {
      "command": "npx",
      "args": ["@svnbjrn/gemini-cli-mcp"]
    }
  }
}
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `GOOGLE_API_KEY` | API key for usage-based billing |
| `GEMINI_SYSTEM_MD` | Path to custom system prompt |

## Best Practices

1. **Specify Working Directory**: Always set `working_dir` for file operations
2. **Use YOLO Sparingly**: Only enable for trusted, safe operations
3. **Leverage Search**: Gemini excels at queries needing real-time info
4. **Check Token Stats**: Monitor cached tokens for cost optimization
5. **Sandbox Mode**: Use `sandbox: true` for safer execution

## Example Workflows

### Current Events Research

```
I'll use Gemini for real-time search:

[gemini_search]
query: "What companies announced layoffs this week?"
```

### Cross-AI Code Review

```
Let me get Gemini's perspective on this code:

[gemini_prompt_with_input]
prompt: "Review this code for bugs and suggest improvements"
stdin_input: "<paste code here>"
```

### Large File Analysis

```
Gemini can handle larger context:

[gemini_prompt]
prompt: "Analyze the architecture of this entire codebase"
working_dir: "/home/user/large-project"
```

### Parallel Documentation

```
[gemini_prompt]
prompt: "Generate API documentation for all public functions in src/"
working_dir: "/home/user/project"
yolo_mode: true
```

## Error Handling

- **Not Found**: Install with `npm install -g @google/gemini-cli`
- **Auth Failed**: Run `gemini` interactively to authenticate with Google
- **Rate Limit**: Free tier allows 60 req/min, 1000 req/day
- **Timeout**: Increase `timeout_secs` for complex tasks

## Authentication

Gemini CLI supports multiple auth methods:

1. **Personal Google Account** (recommended for personal use)
   - Run `gemini` and select "Login with Google"
   - 1000 requests/day free

2. **Google AI Studio API Key**
   ```bash
   export GOOGLE_API_KEY="your-key"
   ```

3. **Vertex AI** (for enterprise)
   ```bash
   export GOOGLE_API_KEY="your-key"
   export GOOGLE_GENAI_USE_VERTEXAI=true
   ```
