---
schema_version: 8
id: brd-mw75
title: 'TUI: show file change summary per worktree'
priority: P2
status: done
deps:
- brd-ty2x
- brd-9sp1
tags:
- tui
owner: null
created_at: 2026-01-25T21:30:42.54198Z
started_at: 2026-01-26T13:38:54.322935Z
completed_at: 2026-01-26T13:41:52.705223Z
acceptance:
- select worktree, see file list with +/- counts
- works for both dirty and committed changes
---

extend Agents view to show change stats for selected worktree.

## scope
- for selected worktree, show:
  - files changed count
  - total insertions/deletions
  - list of changed files with per-file stats
- compute diff appropriately:
  - if dirty: uncommitted changes
  - if clean but ahead of main: branch diff
