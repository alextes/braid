---
schema_version: 8
id: brd-r9hf
title: 'refactor: split agent.rs into focused modules'
priority: P3
status: open
deps: []
tags:
- refactor
owner: null
created_at: 2026-01-20T10:38:45.677108Z
---

## Problem

`src/commands/agent.rs` is 1,354 lines - the second largest file. It handles 5 distinct operations plus AGENTS.md block generation logic.

## Proposal

Split into:
- `agent_init.rs` - worktree creation (~300 lines)
- `agent_merge.rs` - merge workflow (~200 lines)
- `agent_pr.rs` - PR creation (~110 lines)
- `agents_block.rs` - AGENTS.md generation (~400 lines)

Keep `agent.rs` as dispatcher routing to submodules.

## Benefits

- AGENTS.md generation becomes reusable
- Each operation is self-contained
- Easier to test individual workflows