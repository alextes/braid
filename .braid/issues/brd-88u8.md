---
schema_version: 9
id: brd-88u8
title: 'refactor(tui): extract shared detail rendering function'
priority: P2
status: done
deps: []
tags:
- tui
- refactor
owner: null
created_at: 2026-01-27T23:29:39.402268Z
started_at: 2026-01-31T20:18:33.2381Z
completed_at: 2026-01-31T20:21:00.297899Z
---

## problem

`draw_detail()` and `draw_detail_overlay()` are 350+ lines of near-identical code:
- both render dependencies, dependents, acceptance criteria, body
- only difference: overlay uses 80% centered rect, pane uses fixed split

## proposal

extract shared rendering logic into a single function:

```rust
fn render_issue_detail(
    f: &mut Frame,
    area: Rect,
    issue: &Issue,
    derived: &DerivedState,
    // ... other params
) { ... }
```

then `draw_detail()` and `draw_detail_overlay()` just compute their areas and call the shared function.

## files affected

- src/tui/ui.rs (lines ~999-1372)