---
schema_version: 7
id: brd-3udt
title: 'design: improve error for uninitialized repo'
priority: P2
status: done
type: design
deps: []
owner: null
created_at: 2025-12-30T11:39:57.976026Z
updated_at: 2026-01-02T12:31:36.390827Z
---

Running brd commands in a repo that hasn't been initialized gives cryptic errors.

## Problem

When a user runs commands like `brd ready` or `brd ls` in a repo without `.braid/`, they get confusing error messages that don't explain the actual problem.

## Goal

Detect when a repo hasn't been initialized with braid and show a clear, helpful error:

```
error: this repository hasn't been initialized with braid yet

Run `brd init` to set up issue tracking.
```

## Considerations

- Should work consistently across all commands that require initialization
- Don't duplicate the check in every command - find a coherent architectural approach
- Some commands (like `brd init` itself, `brd --help`) should still work without init
- Consider where in the call flow to do this check (main.rs dispatch? RepoPaths discovery?)

## Tasks

- [ ] Audit which commands require initialization vs which don't
- [ ] Design where the check should live (central vs per-command)
- [ ] Implement the check with clear error message
- [ ] Test the error in various scenarios