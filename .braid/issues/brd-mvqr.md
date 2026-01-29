---
schema_version: 9
id: brd-mvqr
title: 'design: scrollable detail pane in TUI'
priority: P2
status: open
type: design
deps: []
tags:
- tui
owner: null
created_at: 2026-01-29T11:38:38.14388Z
---

## problem

when an issue's body is longer than the detail pane can display, there's no way to scroll it. the content is simply cut off.

## constraint

the detail pane already uses `h`/`l` (or arrow keys) to select dependencies. any scrolling mechanism needs to coexist with dependency selection.

## design questions

1. **mode-based?** - enter a "scroll mode" for the detail pane vs dependency selection mode?
2. **separate keys?** - use different keys for scrolling (e.g., `ctrl+j/k`, `{`/`}`, `pgup`/`pgdn`)?
3. **context-aware?** - if no deps, h/l scroll? if deps exist, h/l select deps and something else scrolls?
4. **focus model?** - explicit focus switch between list and detail pane?

## possible approaches

### A: page up/down keys
- `PgUp`/`PgDn` or `ctrl+u`/`ctrl+d` scroll detail pane
- h/l continue to select deps
- simple, no mode switching

### B: focus-based
- `Tab` already toggles detail pane visibility
- could use `Tab` or another key to switch focus to detail pane
- when focused: j/k scroll, h/l select deps
- visual indicator shows which pane has focus

### C: auto-scroll on dep selection
- selecting a dep auto-scrolls to show it
- separate scroll keys (`{`/`}` or `ctrl+j/k`) for manual scroll
- deps section always visible at top?

### D: expandable detail overlay
- enter on issue opens full-screen detail (already exists)
- that overlay could be scrollable
- keep side pane as preview only (no scroll needed)