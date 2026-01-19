---
schema_version: 8
id: brd-fait
title: Agent Grid TUI - Headless JSON Approach
priority: P2
status: open
type: design
deps: []
owner: null
created_at: 2026-01-17T09:59:31.778931Z
---

## Context

We explored a PTY-based approach for the agent grid TUI (branch `agent-grid-tui`) that embeds real terminal emulators using `portable-pty` and `vt100`. While functional, this approach has complexity around terminal emulation, resize handling, and key routing.

## Alternative: Claude Headless + JSON Output

Instead of embedding full PTY terminals, use Claude Code's headless mode with JSON output:

```bash
claude --print --output-format stream-json
```

### Benefits
- No terminal emulation complexity
- Structured data (tool calls, responses, status)
- Easier to render custom UI widgets
- Better control over what's displayed
- Simpler architecture

### Architecture

```
┌─────────────────────────────────────────────┐
│  ratatui (custom widgets for claude output) │
├─────────────────────────────────────────────┤
│  JSON parser (stream-json format)           │
├─────────────────────────────────────────────┤
│  claude --print --output-format stream-json │
└─────────────────────────────────────────────┘
```

### Key Differences from PTY Approach
- Parse JSON events instead of ANSI escape sequences
- Render structured data (tool names, file paths, code blocks)
- Custom UI for different event types (thinking, tool use, response)
- No need for vt100 crate or terminal emulation

### Implementation Notes
- Spawn claude with `--print --output-format stream-json`
- Parse newline-delimited JSON events
- Map events to UI state updates
- Render with ratatui widgets (lists, paragraphs, syntax highlighting)

### Reference
- PTY approach WIP: branch `agent-grid-tui`
- Claude Code JSON output format: `claude --help` or docs