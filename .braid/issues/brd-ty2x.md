---
schema_version: 8
id: brd-ty2x
title: 'TUI: add Agents view with worktree list'
priority: P2
status: done
deps:
- brd-0l3d
tags:
- tui
owner: null
created_at: 2026-01-25T21:30:37.222105Z
started_at: 2026-01-26T13:33:21.720306Z
completed_at: 2026-01-26T13:36:48.034079Z
acceptance:
- press 3 to see list of agent worktrees
- shows agent name, current branch, current issue (if any)
---

foundation for agent work review. add new TUI view that lists active agent worktrees.

## scope
- add `View::Agents` to enum in `app.rs`
- add `draw_agents_view()` in `ui.rs` - simple list of worktrees
- add `3` keybinding in `event.rs`
- update footer and help text
- discover worktrees via `~/.braid/worktrees/<repo>/*/`
