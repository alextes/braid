---
schema_version: 2
id: brd-4osx
title: prevent reuse of agent worktrees by humans
priority: P2
status: done
deps: []
created_at: 2025-12-26T08:47:02.875414Z
updated_at: 2025-12-26T20:50:37.760605Z
---

humans should use their own worktree (typically the main one) to work on issues. running brd commands from an agent's worktree as a human should be an error or at least a warning.

considerations:
- detect if current worktree has an agent.toml indicating it's an agent worktree
- if human tries to `start` from agent worktree, error out
- maybe add `--force` to override if really needed
- distinguish between "human worktree" and "agent worktree" in claims
