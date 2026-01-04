#!/usr/bin/env node
/**
 * Gemini CLI MCP Server
 *
 * Exposes Google's Gemini CLI as MCP tools, allowing other AI agents
 * (including Claude!) to delegate tasks to Gemini.
 *
 * Tools:
 * - gemini_prompt: Send a prompt to Gemini CLI
 * - gemini_prompt_with_input: Send a prompt with stdin content
 * - gemini_version: Get Gemini CLI version info
 * - gemini_search: Use Gemini with Google Search grounding
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

interface GeminiResponse {
  response: string | null;
  stats?: {
    models?: Record<string, {
      tokens?: {
        prompt: number;
        candidates: number;
        cached: number;
        total: number;
      };
      api?: {
        totalRequests: number;
        totalLatencyMs: number;
      };
    }>;
    tools?: {
      totalCalls: number;
      totalSuccess: number;
      totalFail: number;
    };
  };
  error?: {
    type: string;
    message: string;
  };
}

interface ExecuteOptions {
  prompt: string;
  workingDir?: string;
  stdinInput?: string;
  model?: string;
  yoloMode?: boolean;
  sandbox?: boolean;
  timeoutMs?: number;
}

// ============================================================================
// Gemini CLI Executor
// ============================================================================

async function executeGemini(options: ExecuteOptions): Promise<GeminiResponse> {
  const {
    prompt,
    workingDir,
    stdinInput,
    model,
    yoloMode = false,
    sandbox = false,
    timeoutMs = 300000, // 5 minutes default
  } = options;

  const args: string[] = ["-p", prompt, "--output-format", "json"];

  if (model) {
    args.push("--model", model);
  }

  if (yoloMode) {
    args.push("--yolo");
  }

  if (sandbox) {
    args.push("--sandbox");
  }

  return new Promise((resolve, reject) => {
    let settled = false;
    const child = spawn("gemini", args, {
      cwd: workingDir || process.cwd(),
      stdio: ["pipe", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";

    // Manual timeout handling with proper cleanup
    const timeoutId = setTimeout(() => {
      if (!settled) {
        settled = true;
        child.kill("SIGTERM");
        reject(new Error(`Gemini CLI timed out after ${timeoutMs}ms`));
      }
    }, timeoutMs);

    const cleanup = () => {
      clearTimeout(timeoutId);
      child.stdout.removeAllListeners();
      child.stderr.removeAllListeners();
      child.removeAllListeners();
    };

    // Write stdin if provided
    if (stdinInput) {
      child.stdin.write(stdinInput);
    }
    child.stdin.end();

    child.stdout.on("data", (data: Buffer) => {
      stdout += data.toString();
    });

    child.stderr.on("data", (data: Buffer) => {
      stderr += data.toString();
    });

    child.on("error", (err) => {
      if (settled) return;
      settled = true;
      cleanup();
      child.kill();
      reject(new Error(`Failed to spawn Gemini CLI: ${err.message}`));
    });

    child.on("close", (code, signal) => {
      if (settled) return;
      settled = true;
      cleanup();

      // Check if killed by signal (e.g., timeout)
      if (signal) {
        // Already handled by timeout
        return;
      }

      // Try to parse as JSON first
      try {
        const response = JSON.parse(stdout) as GeminiResponse;

        // Check for errors in the response
        if (response.error) {
          // Still resolve, but include the error
          resolve(response);
          return;
        }

        resolve(response);
      } catch {
        // If not JSON, check exit code
        if (code !== 0) {
          reject(new Error(`Gemini CLI exited with code ${code}: ${stderr || stdout}`));
          return;
        }

        // Treat as plain text response
        resolve({
          response: stdout.trim(),
        });
      }
    });
  });
}

async function getGeminiVersion(): Promise<string> {
  return new Promise((resolve, reject) => {
    const child = spawn("gemini", ["--version"], {
      stdio: ["ignore", "pipe", "pipe"],
    });

    let stdout = "";

    child.stdout.on("data", (data: Buffer) => {
      stdout += data.toString();
    });

    child.on("error", (err) => {
      reject(new Error(`Failed to get Gemini CLI version: ${err.message}`));
    });

    child.on("close", (code) => {
      if (code !== 0) {
        reject(new Error("Failed to get Gemini CLI version"));
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
  prompt: z.string().describe("The prompt to send to Gemini CLI"),
  working_dir: z.string().optional().describe("Working directory for Gemini CLI (affects file access)"),
  model: z.string().optional().describe("Model to use (e.g., gemini-2.5-pro, gemini-2.5-flash)"),
  yolo_mode: z.boolean().optional().describe("Auto-approve all tool actions (dangerous!)"),
  sandbox: z.boolean().optional().describe("Run in sandbox mode for safer execution"),
  timeout_secs: z.number().optional().describe("Timeout in seconds (default: 300)"),
});

const PromptWithInputSchema = z.object({
  prompt: z.string().describe("The prompt to send to Gemini CLI"),
  stdin_input: z.string().describe("Content to pipe to Gemini via stdin (e.g., file contents, logs)"),
  working_dir: z.string().optional().describe("Working directory for Gemini CLI"),
  model: z.string().optional().describe("Model to use"),
  timeout_secs: z.number().optional().describe("Timeout in seconds (default: 300)"),
});

const SearchInputSchema = z.object({
  query: z.string().describe("Search query to run with Google Search grounding"),
  working_dir: z.string().optional().describe("Working directory for Gemini CLI"),
  timeout_secs: z.number().optional().describe("Timeout in seconds (default: 300)"),
});

// ============================================================================
// Tool Definitions
// ============================================================================

const tools: Tool[] = [
  {
    name: "gemini_prompt",
    description: `Send a prompt to Google's Gemini CLI and get a response.

Gemini CLI is an agentic AI assistant that can:
- Read and write files in the working directory
- Execute shell commands
- Search the web with Google Search
- Analyze code and generate documentation
- And much more

Use this tool to delegate tasks to Gemini, especially for:
- Tasks requiring Google Search grounding
- Alternative perspective on a problem
- Parallel work on a subtask
- Tasks in a different working directory`,
    inputSchema: {
      type: "object" as const,
      properties: {
        prompt: { type: "string", description: "The prompt to send to Gemini CLI" },
        working_dir: { type: "string", description: "Working directory for Gemini CLI" },
        model: { type: "string", description: "Model to use (gemini-2.5-pro or gemini-2.5-flash)" },
        yolo_mode: { type: "boolean", description: "Auto-approve all tool actions (dangerous!)" },
        sandbox: { type: "boolean", description: "Run in sandbox mode" },
        timeout_secs: { type: "number", description: "Timeout in seconds (default: 300)" },
      },
      required: ["prompt"],
    },
  },
  {
    name: "gemini_prompt_with_input",
    description: `Send a prompt to Gemini CLI with piped stdin input.

Use this for:
- Analyzing file contents without Gemini reading from disk
- Processing log output
- Reviewing code snippets
- Any task where you want to provide the content directly`,
    inputSchema: {
      type: "object" as const,
      properties: {
        prompt: { type: "string", description: "The prompt to send to Gemini CLI" },
        stdin_input: { type: "string", description: "Content to pipe via stdin" },
        working_dir: { type: "string", description: "Working directory for Gemini CLI" },
        model: { type: "string", description: "Model to use" },
        timeout_secs: { type: "number", description: "Timeout in seconds (default: 300)" },
      },
      required: ["prompt", "stdin_input"],
    },
  },
  {
    name: "gemini_search",
    description: `Use Gemini with Google Search grounding to answer a query.

This leverages Gemini's integration with Google Search for:
- Current events and news
- Real-time information
- Web research
- Fact verification`,
    inputSchema: {
      type: "object" as const,
      properties: {
        query: { type: "string", description: "Search query" },
        working_dir: { type: "string", description: "Working directory" },
        timeout_secs: { type: "number", description: "Timeout in seconds (default: 300)" },
      },
      required: ["query"],
    },
  },
  {
    name: "gemini_version",
    description: "Get Gemini CLI version information.",
    inputSchema: {
      type: "object" as const,
      properties: {},
      required: [],
    },
  },
];

// ============================================================================
// Helper Functions
// ============================================================================

function formatStats(stats: GeminiResponse["stats"]): string {
  if (!stats) return "";

  const parts: string[] = [];

  if (stats.models) {
    for (const [modelName, modelStats] of Object.entries(stats.models)) {
      if (modelStats.tokens) {
        parts.push(
          `${modelName}: ${modelStats.tokens.prompt} prompt / ${modelStats.tokens.candidates} response tokens`
        );
        if (modelStats.tokens.cached > 0) {
          parts.push(`  (cached: ${modelStats.tokens.cached})`);
        }
      }
    }
  }

  if (stats.tools && stats.tools.totalCalls > 0) {
    parts.push(`Tools: ${stats.tools.totalSuccess}/${stats.tools.totalCalls} successful`);
  }

  return parts.length > 0 ? `\n\n---\nStats: ${parts.join(", ")}` : "";
}

// ============================================================================
// Server Setup
// ============================================================================

const server = new Server(
  {
    name: "gemini-cli-mcp",
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
      case "gemini_prompt": {
        const input = PromptInputSchema.parse(args);
        const response = await executeGemini({
          prompt: input.prompt,
          workingDir: input.working_dir,
          model: input.model,
          yoloMode: input.yolo_mode,
          sandbox: input.sandbox,
          timeoutMs: (input.timeout_secs ?? 300) * 1000,
        });

        if (response.error) {
          return {
            content: [{
              type: "text",
              text: `Error (${response.error.type}): ${response.error.message}`,
            }],
            isError: true,
          };
        }

        const content: TextContent[] = [
          {
            type: "text",
            text: (response.response || "No response") + formatStats(response.stats),
          },
        ];

        return { content };
      }

      case "gemini_prompt_with_input": {
        const input = PromptWithInputSchema.parse(args);
        const response = await executeGemini({
          prompt: input.prompt,
          workingDir: input.working_dir,
          stdinInput: input.stdin_input,
          model: input.model,
          timeoutMs: (input.timeout_secs ?? 300) * 1000,
        });

        if (response.error) {
          return {
            content: [{
              type: "text",
              text: `Error (${response.error.type}): ${response.error.message}`,
            }],
            isError: true,
          };
        }

        return {
          content: [{
            type: "text",
            text: (response.response || "No response") + formatStats(response.stats),
          }],
        };
      }

      case "gemini_search": {
        const input = SearchInputSchema.parse(args);
        // Use a prompt that leverages Google Search grounding
        const response = await executeGemini({
          prompt: `Using Google Search, answer: ${input.query}`,
          workingDir: input.working_dir,
          timeoutMs: (input.timeout_secs ?? 300) * 1000,
        });

        if (response.error) {
          return {
            content: [{
              type: "text",
              text: `Error (${response.error.type}): ${response.error.message}`,
            }],
            isError: true,
          };
        }

        return {
          content: [{
            type: "text",
            text: (response.response || "No response") + formatStats(response.stats),
          }],
        };
      }

      case "gemini_version": {
        const version = await getGeminiVersion();
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
  // Verify Gemini CLI is available
  try {
    const version = await getGeminiVersion();
    console.error(`Gemini CLI MCP Server starting (Gemini CLI ${version})`);
  } catch {
    console.error("Warning: Gemini CLI not found. Install with: npm install -g @google/gemini-cli");
  }

  const transport = new StdioServerTransport();
  await server.connect(transport);

  console.error("Gemini CLI MCP Server running on stdio");
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
