---
schema_version: 4
id: brd-yc9y
title: interactive brd init with workflow choice
priority: P2
status: todo
deps: []
owner: null
created_at: 2025-12-28T23:16:42.066Z
updated_at: 2025-12-28T23:16:42.066Z
---

Make `brd init` interactive by default, asking about workflow needs.

## Prompt

```
Initializing braid in /path/to/repo...

How will you use braid?

  1. Solo or remote team (git-native) [recommended]
     Issues sync via normal git push/pull. Simple and familiar.

  2. Multiple local agents (local-sync)
     Issues sync instantly via shared worktree. Best for 2+ agents
     on the same machine.

Choice [1]:
```

## Behavior

- Default to git-native (press Enter)
- If local-sync chosen, prompt for branch name (default: `braid-issues`)
- Add `-y`/`--non-interactive` flag to skip (uses git-native)
- Existing `--sync-branch` flag still works (implies non-interactive)

## Files

- `src/cli.rs` - add `-y`/`--non-interactive` flag
- `src/commands/init.rs` - add interactive prompts