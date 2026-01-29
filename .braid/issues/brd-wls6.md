---
schema_version: 9
id: brd-wls6
title: 'perf(tui): add dirty tracking to avoid dashboard recomputation every frame'
priority: P2
status: skip
deps: []
tags:
- tui
- perf
owner: null
created_at: 2026-01-27T23:29:32.71582Z
completed_at: 2026-01-28T21:58:04.628072Z
---

## problem

`draw_dashboard()` recalculates ALL statistics on every 100ms tick:
- counts all issues multiple times (open, doing, done, skip, by priority)
- computes derived state for every issue
- calculates flow metrics (lead/cycle times) for all completed issues
- generates sparklines from 7-day history

additionally, git operations (`git worktree list`, `git diff`) run synchronously and block the UI thread.

## proposal

1. add dirty flag tracking - only recompute when issues actually change
2. cache computed statistics, invalidate on reload
3. move git operations to background thread

## files affected

- src/tui/mod.rs
- src/tui/app.rs
- src/tui/ui.rs (draw_dashboard)