---
schema_version: 9
id: brd-an6r
title: 'TUI: dep selector resets to index 0 on redraw'
priority: P2
status: done
deps: []
tags:
- bug
owner: null
created_at: 2026-01-28T21:59:04.729032Z
started_at: 2026-01-28T22:01:19.081215Z
completed_at: 2026-01-28T22:03:25.975365Z
---

when selecting a dependency issue in the TUI issues view using `l` and `h` to navigate, the selector automatically jumps back to index 0 on redraw/rerender, making it impossible to rest on a higher index.