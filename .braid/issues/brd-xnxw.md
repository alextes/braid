---
schema_version: 6
id: brd-xnxw
title: 'TUI: spawn ephemeral agent for issue'
priority: P2
status: todo
type: design
deps: []
owner: null
created_at: 2026-01-04T23:31:02.736303Z
updated_at: 2026-01-04T23:31:02.736303Z
---

## Idea

From the TUI, assign an issue to a new ephemeral agent that:
1. Sets up a fresh worktree for the task
2. Programmatically invokes Claude to work on it
3. Shows progress in the TUI
4. Allows human input when agent requests it

## User flow

1. In TUI, select an issue
2. Press hotkey (e.g., `a` for "assign to agent")
3. Braid creates agent worktree, starts Claude on the task
4. TUI shows live output/progress
5. If Claude asks a question, TUI prompts user for input
6. When done, agent worktree can be cleaned up or kept

## Open questions

- How to invoke Claude programmatically? Options:
  - Claude Code CLI (`claude` command)
  - Anthropic API directly
  - Claude Agent SDK
- How to capture and display progress?
  - Stream stdout/stderr
  - Parse structured output
- How to handle agent input requests?
  - Detect when Claude is waiting
  - Switch TUI to input mode
- Worktree lifecycle:
  - Auto-cleanup on success?
  - Keep on failure for debugging?
- Multiple concurrent agents?
  - Show agent status in sidebar
  - Switch between agent views

## Prior art

- Cursor's agent mode
- Aider's watch mode
- Claude Code's interactive mode

## Considerations

- This is a significant feature, may want to prototype outside TUI first
- Could start with CLI-only version: `brd agent spawn <issue-id>`
- TUI integration as follow-up