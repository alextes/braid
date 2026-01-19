---
schema_version: 8
id: brd-x00g
title: consolidate agent.rs and agents.rs modules
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-29T23:28:14.369515Z
started_at: 2025-12-30T15:00:00.27295Z
completed_at: 2025-12-30T15:00:00.27295Z
---

the CLI was unified under `brd agent` (brd-l66t), but the source files are still split:

- `src/commands/agent.rs` - worktree commands (init, branch, pr, ship)
- `src/commands/agents.rs` - AGENTS.md commands (inject, instructions)

this is confusing when working on the code.

## options

1. merge `agents.rs` into `agent.rs` (single file)
2. rename `agents.rs` to something clearer like `agent_instructions.rs`
3. create `src/commands/agent/` directory with `mod.rs`, `worktree.rs`, `instructions.rs`

option 1 is simplest if the combined file isn't too large.
