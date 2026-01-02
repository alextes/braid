---
schema_version: 6
id: brd-twye
title: implement issues-branch mode
priority: P2
status: done
type: design
deps: []
owner: null
created_at: 2025-12-28T23:55:59.853809Z
updated_at: 2025-12-29T00:00:08.170503Z
---

A new workflow mode that keeps issues on a separate branch while maintaining simple git semantics.

## Problem

In git-native mode, issue commits pollute main branch history. This is problematic for:
- Remote agents who don't "own" main
- Human collaborators who want clean main history
- PR-based workflows where issue churn is noise

## Design

### Mode name: `issues-branch`

Config:
```toml
issues_branch = "braid-issues"  # enables this mode
```

Three modes total:
- git-native: issues in `.braid/issues/` on current branch
- issues-branch: issues on separate orphan branch
- local-sync: shared worktree (for local multi-agent)

### Reading (`brd ls`, `brd show`)

Use `git show`/`git ls-tree` to read directly from the issues branch - no checkout needed.

### Writing (`brd start`, `brd done`, `brd add`)

1. Stash uncommitted changes (if any)
2. Switch to issues branch
3. Make changes, commit
4. Switch back to original branch
5. Pop stash

### Syncing

Regular git push/pull on the issues branch. No special `brd sync` command needed.

### Setup

`brd mode issues-branch [branch-name]` creates orphan branch with full `.braid/` directory.

## Files to modify

- `src/config.rs` - add `issues_branch: Option<String>`
- `src/repo.rs` - git show/ls-tree helpers, branch switching
- `src/issue.rs` - update `load_issues()` for this mode
- `src/commands/mode.rs` - add `issues-branch` subcommand
- `src/commands/*.rs` - update write operations
- `src/migrate.rs` - schema v4 → v5