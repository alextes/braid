---
schema_version: 6
id: brd-fsqm
title: 'TUI: workers view for multi-agent management'
priority: P3
status: skip
type: design
deps: []
tags:
- tui
owner: null
created_at: 2025-12-27T21:27:55.788447Z
updated_at: 2025-12-27T21:29:00.389027Z
acceptance:
- F-key switching between issues view and workers view
- NxM pane grid (default 1x2 horizontal split)
- each pane spawns a shell process
- pane shows agent ID metadata when in agent worktree
- TUI config file persists layout preferences
---

## overview

second view in brd tui focused on managing multiple agents working in parallel. the issues view is for creating/refining issues; the workers view is for monitoring agents.

## layout

- default: 1x2 (two horizontal panes stacked)
- configurable: any NxM grid
- not dynamic - grid size fixed at startup

## panes

each pane is a full shell:
- spawn shell process per pane (bash/zsh based on $SHELL)
- user can `cd` to agent worktree or run `brd agent init`
- full terminal capability (run agents, vim, etc.)

## navigation

- alt+1, alt+2, etc. to jump between panes
- F1/F2 (or similar) to switch between issues view and workers view
- visual indicator for focused pane

## agent awareness

when pane cwd is an agent worktree:
- show agent ID in pane header/border
- optionally show current issue being worked on

## configuration

persist in `.braid/tui.toml` or similar:
- default grid size (rows x cols)
- last used view
- possibly per-worktree pane assignments

## implementation notes

- look into `portable-pty` crate for PTY handling
- ratatui can handle the split layouts
- shell spawning adds complexity - may need async