---
schema_version: 8
id: brd-tbk4
title: brd agent init should refuse when run from an existing worktree
priority: P1
status: done
deps: []
owner: null
created_at: 2026-01-26T13:56:35.632828Z
started_at: 2026-01-26T21:31:59.673282Z
completed_at: 2026-01-26T21:33:12.019765Z
---

when running `brd agent init` from what's already a worktree instead of `main`, brd should refuse with an error.

currently it creates a new worktree as a branch of the branch, which while potentially useful in some edge cases, is far more likely to be a mistake - the user probably meant to run the command from main and accidentally did it from the wrong directory.

### expected behavior

- detect if current directory is already an agent worktree (check for `.braid/agent.toml`)
- if so, return an error with a helpful message explaining the issue
- suggest the user `cd` to main and retry

### acceptance criteria

- `brd agent init` returns an error when run from an existing agent worktree
- error message is clear and actionable