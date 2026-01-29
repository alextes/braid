---
schema_version: 9
id: brd-zshx
title: 'design: resolve TUI keyboard shortcut conflicts'
priority: P2
status: open
type: design
deps: []
tags:
- tui
owner: null
created_at: 2026-01-27T23:29:48.394317Z
---

## problem

the `d` key has conflicting meanings across views:
- Issues view: `d` = mark issue as done
- Agents view: `d` = half-page down

this is confusing and error-prone.

## other inconsistencies noted

- `u` for half-page up only works in diff panel, not main views
- `a` and `n` both add issues (redundant)
- diff panel keys not shown in help overlay

## questions to resolve

- should navigation keys be consistent across all views?
- should action keys (d for done, s for start) be view-specific?
- what's the right key for half-page navigation? (vim uses ctrl+d/u)

## proposal

option A: make `d` always mean "done", use ctrl+d for half-page
option B: make `d` context-aware but show current meaning in footer
option C: use different keys entirely for list navigation vs actions