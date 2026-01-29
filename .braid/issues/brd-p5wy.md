---
schema_version: 9
id: brd-p5wy
title: drill down to dependencies in issues view
priority: P2
status: open
type: design
deps: []
owner: null
created_at: 2026-01-28T16:20:54.804629Z
---

## context

when navigating a blocked issue, it's useful to see its dependency chain. currently you can use `h`/`l` to select deps in the detail pane and hit enter to jump to one — but this isn't ideal. jumping can land you far down the issue list, losing context of where you were. there's no way to focus the main list on just the dependencies.

a filter or sub-selection approach would keep you oriented: you'd see only the relevant deps, work through them, then pop back to where you started.

## desired behavior

a way to "drill down" so the issues list shows only the dependencies (and possibly transitive deps) of the current issue. this would make it easy to:
- see the full blocking chain at a glance
- work through blockers systematically
- understand what's holding up a specific issue

## design questions

1. **scope** - show only direct deps, or transitive deps too?
2. **trigger** - key binding? (e.g., `D` for drill down)
3. **visual indicator** - how to show we're in drill-down mode?
4. **navigation** - can you drill down further from a dep? stack-based?
5. **escape** - how to return to full list? (esc? same key toggles?)

## possible approaches

### A: simple filter
- `D` sets filter to show only direct deps of current issue
- filter bar shows "deps of brd-xxxx"
- esc clears like any filter

### B: stack-based drill
- `D` pushes current context, shows deps
- can drill further into a dep's deps
- `u` or backspace pops back up the stack
- breadcrumb trail shows path

### C: tree view mode
- toggle to tree visualization
- shows issue with deps indented below
- expand/collapse nodes