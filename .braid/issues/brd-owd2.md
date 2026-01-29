---
schema_version: 9
id: brd-owd2
title: 'design: dependency graph visualization command'
priority: P2
status: open
type: design
deps: []
tags:
- ux
owner: null
created_at: 2026-01-28T15:35:02.125537Z
---

explore adding a command to visualize dependency relationships between issues.

## motivation

`brd show <id>` shows immediate deps and dependents, but understanding the full dependency graph (transitive relationships, blocked chains, etc.) requires multiple lookups.

## possible directions

### 1. tree view command
```
brd deps <id>       # show what this issue depends on (upward)
brd deps --rev <id> # show what depends on this issue (downward)
```

ascii tree output:
```
brd-abc1 (open): parent feature
├── brd-def2 (done): prerequisite work
└── brd-ghi3 (open): another prereq
    └── brd-jkl4 (open): nested dep
```

### 2. full graph command
```
brd graph           # show all issues and their relationships
brd graph --dot     # output graphviz DOT format
```

### 3. blocked chain view
```
brd blocked         # show issues blocked by open deps
brd blocked --chain # show the full blocking chain
```

### 4. TUI integration
- dedicated view in TUI for exploring the graph interactively
- navigate deps/dependents with keyboard

## open questions

- which direction is most useful day-to-day?
- ASCII art vs external tool (graphviz, mermaid)?
- should this replace or supplement `--context` flag?
- how to handle cycles gracefully in visualization?