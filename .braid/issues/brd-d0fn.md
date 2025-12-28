---
schema_version: 4
id: brd-d0fn
title: add confirmation prompts to mode switching
priority: P2
status: todo
deps: []
owner: null
created_at: 2025-12-28T23:16:57.871755Z
updated_at: 2025-12-28T23:16:57.871755Z
---

When switching modes, show what will happen and confirm.

## Example

```
$ brd mode sync-local

Switching to local-sync mode...

This will:
  • Create branch 'braid-issues' for issue storage
  • Set up shared worktree at .git/brd/issues
  • Move 12 issues from .braid/issues/ to the worktree
  • Commit the changes

Continue? [Y/n]:
```

## Behavior

- Add `-y`/`--yes` flag to skip confirmation
- Keep existing logic, just wrap with confirmation
- Show issue count in the prompt

## Files

- `src/cli.rs` - add `-y` flag to mode subcommands
- `src/commands/mode.rs` - add confirmation prompts