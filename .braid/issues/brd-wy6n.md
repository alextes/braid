---
schema_version: 6
id: brd-wy6n
title: 'design: rename agent commands to use git-aligned terminology'
priority: P2
status: done
type: design
deps: []
owner: null
created_at: 2026-01-02T08:14:13.63183Z
updated_at: 2026-01-02T10:03:38.489369Z
---

## Problem

Current agent commands use vague terminology:
- `brd agent ship` - what does "ship" mean exactly?
- `brd agent branch` - creates a branch, but for what purpose?
- `brd agent pr` - clearer, but inconsistent with the others

"Ship" is particularly ambiguous - it could mean deploy, release, merge, push, etc.

## Goal

Use git-aligned terminology that makes the action clear:
- "merge to main" - fast-forward merge your work to main
- "open a PR" - create a pull request for review

## Current Commands

```
brd agent init    # set up agent worktree
brd agent ship    # rebase + ff-merge to main
brd agent branch  # create feature branch from main
brd agent pr      # open PR to main
```

## Options to Consider

### Option A: Rename to git verbs
```
brd agent merge   # ff-merge to main (was: ship)
brd agent branch  # create feature branch (unchanged)
brd agent pr      # open PR (unchanged)
```

### Option B: Action-oriented naming
```
brd merge         # merge to main (drop "agent" prefix)
brd pr            # open PR
brd branch        # create branch
```

### Option C: Explicit target naming
```
brd agent merge-main   # merge to main
brd agent pr-main      # PR to main
```

## Consideration: Agents on Main

Agents can work directly on main (no worktree). In this case:
- `brd agent merge` becomes a no-op (already on main)
- `brd agent pr` doesn't make sense (can't PR main to main)

The command should either:
1. **Detect and skip** - "already on main, nothing to merge"
2. **Error clearly** - "merge is for feature branches, you're on main"
3. **Guide to the right flow** - "on main, just push your commits"

This affects naming too - if we call it `merge`, agents might try to use it when working on main and get confused. Current `ship` has the same issue but is vaguer so maybe less confusing?

## Decision

### Workflow Matrix

|  | Agent on main | Agent on feature branch |
|--|---------------|------------------------|
| **Direct merge** | commit → `git push` | commit → `brd agent merge` |
| **PR workflow** | ❌ not supported | commit → `brd agent pr` |

### Command Renames

- `brd agent ship` → `brd agent merge`
- `brd agent pr` - unchanged
- `brd agent branch` - unchanged

### Behavior on Main

- `brd agent merge` on main: warn "already on main, use git push"
- `brd agent pr` on main: error "PRs require a feature branch, use brd agent init"

### Rationale

1. `merge` is git-aligned and unambiguous
2. PR workflow on main doesn't make sense — use a branch
3. Keep `agent` prefix — these are agent-specific workflows
4. No backwards compat alias for `ship` — clean break

## Implementation Issues

- brd-8x1q: rename `brd agent ship` to `brd agent merge`
- brd-hj6a: add main-branch detection to merge/pr commands
- brd-6oxb: update AGENTS.md block with new command names
