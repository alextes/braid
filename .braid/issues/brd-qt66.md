---
schema_version: 6
id: brd-qt66
title: design TUI live view for in-progress issues
priority: P2
status: done
type: design
deps: []
tags:
- tui
owner: null
created_at: 2025-12-28T17:22:29.424528Z
updated_at: 2025-12-28T21:38:38.172667Z
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

## options considered

1. integrate into existing tui (add a "doing" list)
   - add a third list alongside ready/all, filtered to status=doing
   - pros: minimal new ui, reuses current layout
   - cons: still crowded, "live" feel is limited

2. add a dedicated live view mode in the tui (recommended)
   - new `view_mode` in app: normal vs live
   - `l` toggles live view; `r` still refreshes
   - left column shows "in progress" list; optional "recently done" list below; right column keeps detail
   - pros: dashboard feel, focused on in-progress work, avoids cluttering normal view
   - cons: extra mode + keybinding, slightly more code

3. separate command (e.g. `brd tui --live` or `brd watch`)
   - pros: explicit, scriptable entrypoint
   - cons: more surface area; duplicates tui setup

## recommendation

option 2. add a live view mode inside the tui. keep it focused on doing issues with an optional recent-done list. reuse existing detail pane to avoid new ui complexity.

## proposed layout

left column:
- **in progress** list (status=doing)
  - columns: age, id, priority, owner, title
  - age = `now - updated_at` (approx "time in progress" since start sets updated_at)
  - color coding by age: green < 1h, yellow < 1d, red >= 1d
- **recently done** list (optional, last 5–10 by updated_at)

right column:
- existing detail panel for the selected issue

## refresh strategy

- add a periodic refresh tick (e.g. every 2s) in the tui loop
- no filesystem watcher for v1 (keeps deps small)
- avoid spamming "refreshed" message in live view (add a silent reload or suppress message)

## open questions

- include the "recently done" list or keep live view strictly in-progress?
- ok to use `updated_at` as "time in progress" proxy?
- preferred refresh interval (1s, 2s, 5s)?

## decisions

- include recently done list
- use `updated_at` as time in progress proxy
- refresh interval: 5s

## follow-up

- brd-u9vk
