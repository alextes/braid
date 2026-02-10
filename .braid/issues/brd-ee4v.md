---
schema_version: 9
id: brd-ee4v
title: 'TUI: color tags in issues list like CLI does'
priority: P1
status: done
deps: []
tags:
- tui
- ux
owner: null
created_at: 2026-02-01T23:00:01.542443Z
started_at: 2026-02-06T22:28:38.611779Z
completed_at: 2026-02-06T22:30:33.700411Z
---

the CLI `brd ls` output renders tags with distinct colors (cyan for most, red for #bug). the TUI issues list should match this behavior for visual consistency.

## current behavior
tags in the TUI issues list are rendered in plain text without color distinction.

## expected behavior
tags should be colored the same way as in CLI output:
- most tags: cyan
- #bug tag: red

## related
- brd-xj5z: render #bug tag in red in brd ls (done)
- brd-kxnu: make tags visually distinct in brd ls