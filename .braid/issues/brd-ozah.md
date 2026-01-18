---
schema_version: 7
id: brd-ozah
title: add brd mode command (show only)
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-28T16:21:42.842145Z
updated_at: 2025-12-28T16:46:20.675055Z
---

implement `brd mode` command that shows current mode.

## behavior

```
$ brd mode
Mode: git-native (default)

Issues sync via git - merge to main, rebase to get updates.
Good for: solo work, small teams, remote agents.
```

or if sync branch is configured:

```
$ brd mode
Mode: local-sync
Branch: braid-issues
Remote: origin/braid-issues (tracking)

Issues sync via shared worktree. All local agents see changes instantly.
Remote sync: run `brd sync` to push/pull.
```

## implementation

1. add `Mode` subcommand to CLI (no subcommands yet, just show)
2. detect mode from config (`sync_branch` present → local-sync, else git-native)
3. check if sync branch has upstream tracking
4. print mode info

## files

- `src/cli.rs` - add Mode command
- `src/commands/mode.rs` - new file
- `src/commands/mod.rs` - export
- `src/main.rs` - wire up

## parent

part of brd-jpux (workflow modes design)