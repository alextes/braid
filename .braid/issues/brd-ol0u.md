---
schema_version: 8
id: brd-ol0u
title: 'design: visualize and clarify dependency relationships'
priority: P2
status: open
type: design
deps: []
owner: null
created_at: 2026-01-18T20:36:28.54195Z
---

Explore better ways to show dependencies between issues.

## areas to consider

### dedicated command for dep visualization
- `brd deps` or `brd graph` to show dependency tree/graph?
- ascii tree output?
- which direction: blockers vs dependents?

### improve brd show output
- current: `State: BLOCKED` is confusing alongside `Status:`
- proposal: just show `Blocked: <ids>` when blocked, hide otherwise
- should we show dependents too? (issues blocked by this one)
- show dep status inline? e.g. `Deps: brd-foo (done), brd-bar (open)`

### related questions
- how to discover "what's blocking X"?
- how to see "what does completing X unblock"?