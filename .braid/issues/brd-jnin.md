---
schema_version: 8
id: brd-jnin
title: warn about rebasing agent worktrees after mode switch
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-29T00:06:50.222825Z
started_at: 2025-12-29T21:10:42.101163Z
completed_at: 2025-12-29T21:10:42.101163Z
---

After switching modes (e.g., `brd mode local-sync`), previously created agent worktrees still have the old config until they rebase on main.

## Problem

1. User runs `brd mode local-sync` in main worktree
2. Config is updated and committed to main
3. Existing agent worktrees (e.g., `agent-one`) still have old config
4. Agent continues working with wrong mode until they happen to rebase

## Solution

When `brd mode local-sync` or `brd mode git-native` completes, detect if there are agent worktrees and warn:

```
Switched to local-sync mode.

Warning: Found 2 agent worktree(s) that need to rebase on main:
  - agent-one (at /path/to/worktree)
  - agent-two (at /path/to/worktree)

Run `git rebase main` in each worktree to pick up the new config.
```

## Implementation

- After mode switch commits, check for worktrees with agent branches
- List any that are behind main
- Print warning with paths and instructions