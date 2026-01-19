---
schema_version: 8
id: brd-dje7
title: 'brd mode: add sync-local and default subcommands'
priority: P2
status: done
deps:
- brd-ozah
owner: null
created_at: 2025-12-28T16:21:56.472458Z
started_at: 2025-12-28T21:09:24.505952Z
completed_at: 2025-12-28T21:09:24.505952Z
---

add mode switching to `brd mode` command.

## commands

### `brd mode sync-local [branch-name]`

switch to local sync mode:
1. create sync branch from HEAD if not exists
2. move existing issues to sync branch  
3. set up shared worktree at `<git-common-dir>/brd/issues/`
4. update main's config with `sync_branch = "<name>"`
5. commit changes

default branch name: `braid-issues`

### `brd mode default`

switch back to git-native mode:
1. merge sync branch into current branch (brings issues back)
2. remove `sync_branch` from config
3. remove shared worktree
4. commit changes

## notes

- reuse logic from `brd init --sync-branch` for sync-local
- handle case where issues already exist in both locations
- warn if there are uncommitted changes in issues worktree

## files

- `src/cli.rs` - add ModeAction enum
- `src/commands/mode.rs` - implement switching

## parent

part of brd-jpux (workflow modes design)