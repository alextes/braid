---
schema_version: 6
id: brd-jpux
title: design braid workflow modes and configuration
priority: P1
status: done
type: design
deps: []
owner: null
created_at: 2025-12-28T16:05:42.310892Z
updated_at: 2025-12-28T17:10:17.693318Z
---

braid needs clear documentation and configuration for different workflow modes. current implementation has sync branch mode, but the full picture of supported workflows isn't coherent.

## identified workflows

### 1. solo mode (simplest)
- one branch (main), one person or one agent
- issues live in `.braid/issues/` on main
- no sync needed, no conflicts possible
- **config**: default, no special options

### 2. branch-per-feature mode
- issues created in feature branches
- issues only visible in that branch until merge
- follows git's natural model
- good for: solo dev with branches, small teams
- **config**: default, no special options (git handles it)

### 3. multi-human / remote-agent mode
- multiple humans in clones, or agents on remote machines
- must sync issue state through git (merge to main, rebase)
- claim conflicts resolved by "first push wins"
- **config**: default mode works, but requires discipline

### 4. multi-agent local mode
- multiple agents on same machine
- can share a local worktree for instant issue visibility
- no push/pull needed for issue coordination
- sync branch provides the shared state
- **config**: `sync_branch` mode with local worktree

## design decisions

### mode 4 (local multi-agent)
- remote sync inferred from git upstream tracking (no explicit config needed)
- if sync branch has upstream → sync with remote
- if no upstream → local-only (just commit, no push)
- `brd sync --push` to set upstream for first-time remote setup

### mode switching
- `brd mode` command shows current mode
- `brd mode sync-local [branch]` enables local sync mode
- `brd mode default` switches back, merges sync branch into current branch
- guided migration handles issue movement

### AGENTS.md injection
- split into static (brd commands) + dynamic (sync instructions)
- dynamic section generated based on current mode
- bump to v2 format

## implementation issues

- [x] brd-ozah - add brd mode command (show only)
- [ ] brd-c2g4 - brd sync: detect upstream, support local-only
- [ ] brd-dje7 - brd mode: add sync-local and default subcommands
- [ ] brd-tvue - mode-aware AGENTS.md injection
- [ ] brd-947s - document workflow modes