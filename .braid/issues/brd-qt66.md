---
schema_version: 4
id: brd-qt66
title: design TUI live view for in-progress issues
priority: P2
status: doing
type: design
deps: []
tags:
- tui
owner: agent-two
created_at: 2025-12-28T17:22:29.424528Z
updated_at: 2025-12-28T21:13:14.182345Z
---

design a TUI view that makes it easy to follow the live state of in-progress issues.

## motivation

when multiple agents are working in parallel, it's valuable to have a dashboard-style view showing:
- which issues are currently being worked on
- who owns each in-progress issue
- how long each has been in progress
- quick status/health indicators

## design questions

- should this be a separate view mode or integrated into existing ls view?
- what refresh rate / live update mechanism?
- what information to show per issue?
- should it show recent activity (last N completed issues)?
- keyboard shortcuts for quick actions (jump to issue, refresh, etc.)

## possible features

- auto-refresh on file system changes
- color coding by duration (green = recent, yellow = stale, red = very old)
- owner column with agent names
- elapsed time since started
- compact vs detailed view toggle
