---
schema_version: 8
id: brd-0l3d
title: 'design: TUI agent work review view'
priority: P2
status: open
type: design
deps: []
tags:
- tui
owner: null
created_at: 2026-01-25T11:18:03.970999Z
---

## goal

add a new TUI view (alongside dashboard and issues) that lets humans review agent work in progress. key use case: supervising what agents are doing across worktrees.

## requirements

### worktree overview
- list all active agent worktrees
- for each worktree show:
  - agent name / worktree name
  - current issue being worked on
  - file change summary (files changed, insertions, deletions)

### per-worktree file list
- show which files have been modified
- show lines added/removed per file
- allow selecting a file to view its diff

### diff view
- display file diffs in a readable format
- support two modes:
  - split view (old on left, new on right)
  - unified view (inline changes)
- syntax highlighting would be nice but not required for v1

### diff sources
- **dirty working tree**: uncommitted changes in the worktree
- **branch diff**: committed work since branching from main
  - this is the more common case - agent commits as it works
  - diff should be `main..HEAD` or similar

## decisions

- **v1 scope:** unified diff only, split view deferred
- **diff base:** always use `main` - simple `git diff main..HEAD`
- **navigation:** j/k scroll diff, tab or n/p to switch files
- **keybinding:** `3` for agents view

## implementation issues (v1)

1. **brd-ty2x** - add Agents view with worktree list
2. **brd-9sp1** - add diff stat infrastructure to git.rs
3. **brd-mw75** - show file change summary per worktree (depends on 1, 2)
4. **brd-4uqo** - add diff content retrieval and parsing
5. **brd-e67f** - render unified diff view (depends on 3, 4)

## deferred (v2)

- split (side-by-side) diff view
- syntax highlighting
- reviewing completed/merged work

## references

- ratatui has no built-in diff widget, would need custom rendering
- could shell out to `git diff` and parse/display output
- look at how lazygit handles diff display for inspiration
