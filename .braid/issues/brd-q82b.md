---
schema_version: 9
id: brd-q82b
title: 'TUI: mark blocking issues in main list'
priority: P2
status: open
deps: []
tags:
- tui
- ux
owner: null
created_at: 2026-01-28T15:44:58.092472Z
---

when viewing an issue that is blocked, show markers in the main issue list indicating which issues are blocking it.

## idea

show a red `>` marker next to any issue in the visible list that is blocking the currently highlighted issue.

```
  brd-atsr P2  4w  -  add brd reopen command #cli        ← selected, BLOCKED
  brd-y2dn P2  4w  -  graceful fallback when gh... 
> brd-1i52 P3  4w  -  design: track PR URL...            ← red marker: blocks selected
  brd-cnd8 P2  1w  -  TUI: spawn and manage agents
```

## benefits

- immediately see blockers without opening detail pane
- works even when blocker is far down the list
- reinforces the relationship visually

## considerations

- what if multiple issues are blocking? show marker on all of them
- marker should be visually distinct (red, or different symbol like `!` or `⊳`)
- only show when selected issue is actually blocked