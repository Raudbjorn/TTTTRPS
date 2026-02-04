---
name: claude-code-bridge
description: Integrate with Claude Code CLI for delegating complex coding tasks. Use when you need to spawn a separate Claude Code instance to work on files, run tests, or perform multi-step coding operations in a specific directory. Enables "Claude calling Claude" patterns for parallel work or specialized contexts.
---

# Claude Code Bridge

Delegate coding tasks to Claude Code CLI from any context.

## When to Use

- **Parallel Work**: Spawn Claude Code to work on a subtask while you continue
- **Different Context**: Need Claude Code's file access in a specific directory
- **Specialized Tasks**: Let Claude Code handle complex refactoring, testing, or debugging
- **Isolation**: Keep risky operations in a separate session

## Usage Patterns

### Via MCP Server (Recommended)

If the `claude-code-mcp` server is configured, use the MCP tools:

```
Use claude_prompt to ask Claude Code: "Refactor the authentication module to use JWT"
```

Tools available:
- `claude_prompt` - Send a prompt to Claude Code
- `claude_continue` - Continue most recent conversation
- `claude_resume` - Resume a specific session by ID
- `claude_version` - Get version info

### Via CLI (Direct)

Execute Claude Code directly:

```bash
# Single prompt
claude -p "List all TODO comments in this project" --output-format json

# Continue conversation
claude -p "Now fix the first TODO" --continue

# Resume specific session
claude -p "What files did we modify?" --resume <session-id>
```

### Via Rust Library

```rust
use claude_code_rs::ClaudeCodeClient;

let client = ClaudeCodeClient::new()?;
let response = client.prompt("Analyze this codebase").await?;
println!("{}", response.text());
```

## Configuration

### MCP Server Setup

Add to Claude Desktop's `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "claude-code": {
      "command": "npx",
      "args": ["@anthropic/claude-code-mcp"]
    }
  }
}
```

Or for Claude Code's `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "claude-code-bridge": {
      "command": "node",
      "args": ["/path/to/mcp-server/dist/index.js"]
    }
  }
}
```

## Best Practices

1. **Specify Working Directory**: Always set `working_dir` to give Claude Code proper context
2. **Use Descriptive Prompts**: Be specific about what you want done
3. **Capture Session IDs**: Save session IDs from responses to resume conversations
4. **Set Timeouts**: Complex tasks may need longer timeouts (default: 5 min)
5. **Check Results**: Verify Claude Code's output before proceeding

## Example Workflows

### Code Review Delegation

```
I'll use Claude Code to review the PR changes:

[claude_prompt]
prompt: "Review the diff in pr-123.patch for security issues and code quality. Provide specific feedback."
working_dir: "/home/user/project"
```

### Test Generation

```
Let me have Claude Code generate tests:

[claude_prompt]  
prompt: "Generate comprehensive unit tests for src/auth/*.ts. Use vitest and aim for 90% coverage."
working_dir: "/home/user/project"
```

### Refactoring with Followup

```
[claude_prompt]
prompt: "Identify all uses of deprecated API v1 in this codebase"
working_dir: "/home/user/project"

# After reviewing results:
[claude_continue]
prompt: "Now migrate the first 3 files you found to API v2"
```

## Error Handling

- **Not Found**: Ensure Claude Code is installed (`npm install -g @anthropic-ai/claude-code`)
- **Timeout**: Increase `timeout_secs` for complex tasks
- **Permission Denied**: Claude Code may need approval for file writes; use interactive mode for sensitive operations
