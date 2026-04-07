# MCP Debugging And UI Notes

## Summary

This document summarizes the recent MCP-focused fixes made in `kuse-cowork` to improve:

- failure visibility
- MCP tool naming in the UI
- startup log noise
- agent/tool-call correctness
- MCP server URL hygiene

## Problems That Were Observed

### 1. Failures were hard to diagnose

When an MCP connection or tool call failed, the UI often showed only a generic failure message. The backend was already tracking `last_error` in MCP server status, but the UI did not surface that information reliably enough.

### 2. Tool names looked like internal code

During chat/task execution, the UI displayed raw internal MCP tool identifiers such as:

`mcp_708395ba_babb_4e57_8777_e287d0be42c3_list_projects`

This is correct for internal execution, but not readable for users.

### 3. GitLab MCP appeared connected but agent calls still failed

The GitLab MCP server could report `Connected`, but the agent still replied as if the tool was unavailable or "not found".

The root cause was not a broken GitLab MCP server. The server responded successfully to:

- `initialize`
- `tools/list`
- `tools/call` for `get_user`

The real issue was that the system prompt incorrectly instructed the model to use `server_id:tool_name` style names, while the actual runtime only resolves MCP tools through internal names like:

`mcp_<safe_server_id>_<safe_tool_name>`

That prompt/runtime mismatch caused tool lookup failures.

### 4. Stored MCP URLs could contain hidden whitespace

The saved GitLab MCP URL was found in the database with a leading tab character. That kind of data pollution can create inconsistent behavior across requests and environments.

### 5. Release logs contained routine/noisy output

Routine startup output included:

- API key length debug logs
- successful auto-connect logs for MCP servers

These were useful while debugging, but noisy in normal app runs.

## Changes Made

## Error Visibility

Files:

- `src/components/MCPSettings.tsx`
- `src/components/MCPSettings.css`

Changes:

- Added a top-level MCP error banner in the settings screen
- Displayed `last_error` directly in each server card
- Preserved per-server UI error messages even when backend status refresh is incomplete
- Refreshed MCP status after failed connect/disconnect attempts so the latest backend error is pulled into the UI
- Replaced generic alert text with actual error messages where available

Result:

- users can see why a connection failed
- a failed server now has a visible `Last error` section in its card

## Tool Name Readability

Files:

- `src/lib/tool-display.ts`
- `src/components/TaskPanel.tsx`
- `src/components/Agent.tsx`
- `src/components/Chat.tsx`

Changes:

- Added a formatter for display-only tool labels
- Converted internal tool identifiers into readable labels
- Resolved MCP server IDs back to configured server names using MCP status data

Examples:

- `mcp_708395ba_babb_4e57_8777_e287d0be42c3_list_projects`
  -> `GitLab MCP: List Projects`
- `docker_run`
  -> `Docker: Run`

Result:

- task, agent, and chat tool displays now show readable names instead of internal execution identifiers

## Prompt/Runtime MCP Tool Fix

File:

- `src-tauri/src/commands.rs`

Changes:

- Removed incorrect prompt guidance telling the model to use `server_id:tool_name`
- Replaced it with guidance to use the exact tool names exposed by the tool API

Result:

- reduced mismatch between what the model is told and what the runtime can actually execute
- prevents false "not found" behavior for connected MCP servers

## MCP URL Sanitization

File:

- `src-tauri/src/mcp/storage.rs`

Changes:

- Trimmed `server_url` before saving
- Trimmed `server_url` again when reading from the database

Result:

- removes leading/trailing spaces and tabs from stored MCP URLs
- reduces fragile connection behavior caused by malformed saved values

## Startup Logging Cleanup

Files:

- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`

Changes:

- guarded settings/test debug logs with `#[cfg(debug_assertions)]`
- guarded successful MCP auto-connect logs so they only show in debug builds
- kept actual failure logs intact

Result:

- release runs are quieter
- useful failure logs are still preserved

## Verification

The changes were verified with:

- `npm run build`
- `cargo check`

Both completed successfully after the UI and Rust backend updates.

## Known Limitation

The tool display improvements affect the UI presentation layer. Internal tool identifiers are still used for execution, which is intentional. The app now separates internal execution names from user-facing labels more cleanly, but future work could make this even cleaner by emitting display labels directly from backend events.
