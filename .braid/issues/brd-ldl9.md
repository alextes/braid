---
schema_version: 6
id: brd-ldl9
title: TUI for live agent monitoring and interaction
priority: P2
status: todo
type: design
deps: []
owner: null
created_at: 2026-01-08T15:03:42.290274Z
updated_at: 2026-01-08T15:03:42.290274Z
---

explore a TUI interface for live viewing of issue progress.

## current state
we have something limited - basically just a list of issues in progress. not a bad start.

## vision
- only part of the screen dedicated to issues in progress
- easy to see if an agent has stalled and needs user input
- efficient to hop to the agent to give the input it needs

## design options to explore

### tmux integration
since brd and agents typically run in tmux:
- brd TUI in one tmux pane
- agent in a secondary tmux pane
- TUI can swap the secondary pane to whichever agent needs input or is highlighted

### embedded claude code CLI
run the claude code CLI directly inside the TUI

### programmatic claude code
go further and use claude code programmatically - full control over the interaction instead of having users interact via claude code CLI directly

### desktop app or web interface
instead of a TUI, build a proper desktop app or web interface. this would require exposing brd functionality via some interface (API, IPC, etc.) that a GUI could consume