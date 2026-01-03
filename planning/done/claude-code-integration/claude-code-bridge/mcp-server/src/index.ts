#!/usr/bin/env node
/**
 * Claude Code MCP Server
 *
 * Exposes Claude Code CLI as MCP tools, allowing other AI agents
 * to delegate tasks to Claude Code.
 *
 * Tools:
 * - claude_prompt: Send a prompt to Claude Code
 * - claude_continue: Continue the most recent conversation
 * - claude_resume: Resume a specific conversation by session ID
 * - claude_version: Get Claude Code version info
 */

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  Tool,
  TextContent,
} from "@modelcontextprotocol/sdk/types.js";
import { z } from "zod";
import { spawn } from "child_process";

// ============================================================================
// Types
// ============================================================================

interface ClaudeResponse {
  sessionId?: string;
  result: string;
  usage?: {
    inputTokens: number;
    outputTokens: number;
  };
  cost?: {
    usd: number;
  };
  toolUses?: Array<{
    name: string;
    result?: string;
    error?: string;
  }>;
  error?: string;
}

interface ExecuteOptions {
  prompt: string;
  workingDir?: string;
  continueConversation?: boolean;
  resumeSession?: string;
  model?: string;
  systemPrompt?: string;
  maxTokens?: number;
  timeoutMs?: number;
}

// ============================================================================
// Claude Code Executor
// ============================================================================

async function executeClaudeCode(options: ExecuteOptions): Promise<ClaudeResponse> {
  const {
    prompt,
    workingDir,
    continueConversation,
    resumeSession,
    model,
    systemPrompt,
    maxTokens,
    timeoutMs = 300000, // 5 minutes default
  } = options;

  const args: string[] = ["-p", prompt, "--output-format", "json"];

  if (continueConversation) {
    args.push("--continue");
  }

  if (resumeSession) {
    args.push("--resume", resumeSession);
  }

  if (model) {
    args.push("--model", model);
  }

  if (systemPrompt) {
    args.push("--system-prompt", systemPrompt);
  }

  if (maxTokens) {
    args.push("--max-tokens", maxTokens.toString());
  }

  return new Promise((resolve, reject) => {
    const child = spawn("claude", args, {
      cwd: workingDir || process.cwd(),
      stdio: ["ignore", "pipe", "pipe"],
      timeout: timeoutMs,
    });

    let stdout = "";
    let stderr = "";

    child.stdout.on("data", (data: Buffer) => {
      stdout += data.toString();
    });

    child.stderr.on("data", (data: Buffer) => {
      stderr += data.toString();
    });

    child.on("error", (err) => {
      reject(new Error(`Failed to spawn Claude Code: ${err.message}`));
    });

    child.on("close", (code) => {
      if (code !== 0) {
        reject(new Error(`Claude Code exited with code ${code}: ${stderr || stdout}`));
        return;
      }

      try {
        // Try to parse as JSON
        const response = JSON.parse(stdout) as ClaudeResponse;
        resolve(response);
      } catch {
        // If not JSON, treat as plain text
        resolve({
          result: stdout.trim(),
        });
      }
    });

    // Handle timeout
    setTimeout(() => {
      child.kill("SIGTERM");
      reject(new Error(`Claude Code timed out after ${timeoutMs}ms`));
    }, timeoutMs);
  });
}

async function getClaudeVersion(): Promise<string> {
  return new Promise((resolve, reject) => {
    const child = spawn("claude", ["--version"], {
      stdio: ["ignore", "pipe", "pipe"],
    });

    let stdout = "";

    child.stdout.on("data", (data: Buffer) => {
      stdout += data.toString();
    });

    child.on("error", (err) => {
      reject(new Error(`Failed to get Claude Code version: ${err.message}`));
    });

    child.on("close", (code) => {
      if (code !== 0) {
        reject(new Error("Failed to get Claude Code version"));
        return;
      }
      resolve(stdout.trim());
    });
  });
}

// ============================================================================
// Input Schemas
// ============================================================================

const PromptInputSchema = z.object({
  prompt: z.string().describe("The prompt to send to Claude Code"),
  working_dir: z.string().optional().describe("Working directory for Claude Code (affects file access)"),
  model: z.string().optional().describe("Model to use (e.g., claude-sonnet-4-20250514)"),
  system_prompt: z.string().optional().describe("Custom system prompt"),
  max_tokens: z.number().optional().describe("Maximum tokens for response"),
  timeout_secs: z.number().optional().describe("Timeout in seconds (default: 300)"),
});

const ContinueInputSchema = z.object({
  prompt: z.string().describe("The follow-up prompt"),
  working_dir: z.string().optional().describe("Working directory for Claude Code"),
  timeout_secs: z.number().optional().describe("Timeout in seconds (default: 300)"),
});

const ResumeInputSchema = z.object({
  prompt: z.string().describe("The prompt to send"),
  session_id: z.string().describe("The session ID to resume"),
  working_dir: z.string().optional().describe("Working directory for Claude Code"),
  timeout_secs: z.number().optional().describe("Timeout in seconds (default: 300)"),
});

// ============================================================================
// Tool Definitions
// ============================================================================

const tools: Tool[] = [
  {
    name: "claude_prompt",
    description: `Send a prompt to Claude Code CLI and get a response.

Claude Code is an agentic coding assistant that can:
- Read and write files
- Execute shell commands
- Search codebases
- Run tests
- And much more

Use this tool to delegate complex coding tasks to Claude Code.
The working_dir parameter controls which directory Claude Code operates in.`,
    inputSchema: {
      type: "object" as const,
      properties: {
        prompt: { type: "string", description: "The prompt to send to Claude Code" },
        working_dir: { type: "string", description: "Working directory for Claude Code" },
        model: { type: "string", description: "Model to use" },
        system_prompt: { type: "string", description: "Custom system prompt" },
        max_tokens: { type: "number", description: "Maximum tokens for response" },
        timeout_secs: { type: "number", description: "Timeout in seconds (default: 300)" },
      },
      required: ["prompt"],
    },
  },
  {
    name: "claude_continue",
    description: `Continue the most recent Claude Code conversation.

Use this to send follow-up messages in an ongoing conversation
without needing to track session IDs.`,
    inputSchema: {
      type: "object" as const,
      properties: {
        prompt: { type: "string", description: "The follow-up prompt" },
        working_dir: { type: "string", description: "Working directory for Claude Code" },
        timeout_secs: { type: "number", description: "Timeout in seconds (default: 300)" },
      },
      required: ["prompt"],
    },
  },
  {
    name: "claude_resume",
    description: `Resume a specific Claude Code conversation by session ID.

Use this to return to a previous conversation when you have
the session ID from a prior interaction.`,
    inputSchema: {
      type: "object" as const,
      properties: {
        prompt: { type: "string", description: "The prompt to send" },
        session_id: { type: "string", description: "The session ID to resume" },
        working_dir: { type: "string", description: "Working directory for Claude Code" },
        timeout_secs: { type: "number", description: "Timeout in seconds (default: 300)" },
      },
      required: ["prompt", "session_id"],
    },
  },
  {
    name: "claude_version",
    description: "Get Claude Code CLI version information.",
    inputSchema: {
      type: "object" as const,
      properties: {},
      required: [],
    },
  },
];

// ============================================================================
// Server Setup
// ============================================================================

const server = new Server(
  {
    name: "claude-code-mcp",
    version: "0.1.0",
  },
  {
    capabilities: {
      tools: {},
    },
  }
);

// Handle tool listing
server.setRequestHandler(ListToolsRequestSchema, async () => {
  return { tools };
});

// Handle tool execution
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  try {
    switch (name) {
      case "claude_prompt": {
        const input = PromptInputSchema.parse(args);
        const response = await executeClaudeCode({
          prompt: input.prompt,
          workingDir: input.working_dir,
          model: input.model,
          systemPrompt: input.system_prompt,
          maxTokens: input.max_tokens,
          timeoutMs: (input.timeout_secs ?? 300) * 1000,
        });

        const content: TextContent[] = [
          {
            type: "text",
            text: response.result,
          },
        ];

        // Add metadata if available
        if (response.sessionId || response.usage || response.cost) {
          const metadata: Record<string, unknown> = {};
          if (response.sessionId) metadata.session_id = response.sessionId;
          if (response.usage) metadata.usage = response.usage;
          if (response.cost) metadata.cost = response.cost;

          content.push({
            type: "text",
            text: `\n\n---\nMetadata: ${JSON.stringify(metadata)}`,
          });
        }

        return { content };
      }

      case "claude_continue": {
        const input = ContinueInputSchema.parse(args);
        const response = await executeClaudeCode({
          prompt: input.prompt,
          workingDir: input.working_dir,
          continueConversation: true,
          timeoutMs: (input.timeout_secs ?? 300) * 1000,
        });

        return {
          content: [{ type: "text", text: response.result }],
        };
      }

      case "claude_resume": {
        const input = ResumeInputSchema.parse(args);
        const response = await executeClaudeCode({
          prompt: input.prompt,
          workingDir: input.working_dir,
          resumeSession: input.session_id,
          timeoutMs: (input.timeout_secs ?? 300) * 1000,
        });

        return {
          content: [{ type: "text", text: response.result }],
        };
      }

      case "claude_version": {
        const version = await getClaudeVersion();
        return {
          content: [{ type: "text", text: version }],
        };
      }

      default:
        return {
          content: [{ type: "text", text: `Unknown tool: ${name}` }],
          isError: true,
        };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    return {
      content: [{ type: "text", text: `Error: ${message}` }],
      isError: true,
    };
  }
});

// ============================================================================
// Main
// ============================================================================

async function main() {
  // Verify Claude Code is available
  try {
    const version = await getClaudeVersion();
    console.error(`Claude Code MCP Server starting (Claude Code ${version})`);
  } catch {
    console.error("Warning: Claude Code CLI not found. Install with: npm install -g @anthropic-ai/claude-code");
  }

  const transport = new StdioServerTransport();
  await server.connect(transport);

  console.error("Claude Code MCP Server running on stdio");
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
