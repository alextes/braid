---
schema_version: 4
id: brd-jpux
title: design braid workflow modes and configuration
priority: P1
status: todo
type: design
deps: []
owner: null
created_at: 2025-12-28T16:05:42.310892Z
updated_at: 2025-12-28T16:05:42.310892Z
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

## open questions

### for mode 4 (local multi-agent)
- should the sync branch be purely local, or also pushed to remote?
- if local-only, how do we prevent accidental push?
- should there be a `local_sync = true` option that skips remote operations?
- how does `brd sync` behave in local-only mode? (just commit, no push?)

### mode switching
- can we switch from mode 1/2/3 to mode 4 seamlessly?
- what happens to existing issues on main when enabling sync branch?
- should there be a `brd mode` command to switch/show current mode?
- how do we validate the switch won't cause data loss?

### AGENTS.md injection
- current injection is static
- should have mode-aware sections
- static part: how to use brd commands
- dynamic part: how to sync in current mode
- need to update `brd agents inject` to be mode-aware

## design goals

1. **clear mental models**: each mode should be easy to explain
2. **seamless transitions**: switching modes shouldn't break things
3. **git-native**: leverage git's model, don't fight it
4. **progressive complexity**: simple cases stay simple

## tasks

- [ ] document all four workflow modes in detail
- [ ] design mode switching command (`brd mode`?)
- [ ] design local-only sync branch variant
- [ ] update AGENTS.md injection to be mode-aware
- [ ] create user-facing docs explaining when to use each mode
- [ ] implementation issues for each piece

## notes

current sync branch implementation (just completed) handles mode 4 partially, but assumes remote sync. need to think through local-only variant.

mode 3 vs mode 4 distinction: remote agents MUST use git for coordination (mode 3). local agents CAN use sync branch for smoother UX (mode 4). the key insight is that local agents share filesystem access.