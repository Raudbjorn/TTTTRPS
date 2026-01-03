# Claude Desktop CDP Bridge Integration

**Status:** Complete
**Completed:** 2026-01-03
**PR:** #12

## Overview

Integrated the `claude-cdp` library to enable communication with Claude Desktop via Chrome DevTools Protocol (CDP). This provides an alternative to API-based Claude access.

## Use Case

Development convenience (not a first-class LLM provider):
- Uses existing Claude Desktop authentication (no API keys needed)
- Works with Claude Pro subscription
- Useful for development/testing without API costs

## Limitations

- No streaming (full response only)
- No token counting
- No model control (uses whatever's selected in Claude Desktop)
- Fragile (depends on UI selectors)

## Implementation

### Files Created

| File | Description |
|------|-------------|
| `src-tauri/src/core/claude_cdp/mod.rs` | Module exports |
| `src-tauri/src/core/claude_cdp/client.rs` | CDP client implementation |
| `src-tauri/src/core/claude_cdp/error.rs` | Error types |
| `src-tauri/src/core/claude_cdp/config.rs` | Configuration with platform-specific binary paths |
| `src-tauri/src/core/claude_cdp/manager.rs` | Connection manager with process lifecycle |
| `src-tauri/src/core/llm/providers/claude_desktop.rs` | LLMProvider implementation |

### Files Modified

| File | Changes |
|------|---------|
| `src-tauri/Cargo.toml` | Added `chromiumoxide` dependency |
| `src-tauri/src/core/mod.rs` | Added `claude_cdp` module |
| `src-tauri/src/core/llm/providers/mod.rs` | Added `ClaudeDesktop` variant to `ProviderConfig` |
| `src-tauri/src/commands.rs` | Added CDP Tauri commands |
| `src-tauri/src/main.rs` | Initialized `ClaudeDesktopManager` in AppState |

### Tauri Commands

- `connect_claude_desktop(port)` - Connect to running instance
- `launch_claude_desktop()` - Launch Claude Desktop with CDP enabled
- `connect_or_launch_claude_desktop(port)` - Try connect, launch if needed
- `disconnect_claude_desktop()` - Disconnect
- `get_claude_desktop_status()` - Get connection state
- `claude_desktop_new_conversation()` - Start new chat
- `claude_desktop_get_history()` - Get conversation history
- `detect_claude_desktop()` - Check if binary found

### Platform Support

- **Linux:** `/opt/Claude/claude`, `/opt/claude-desktop/claude`, `/usr/bin/claude`, `/usr/local/bin/claude`
- **macOS:** `/Applications/Claude.app/Contents/MacOS/Claude`
- **Windows:** `C:\Program Files\Claude\Claude.exe`, `C:\Program Files (x86)\Claude\Claude.exe`

## Reference Implementation

The `reference-implementation/` directory contains the original claude-cdp reference code used for this integration.
