---
schema_version: 8
id: brd-xnxw
title: 'CLI: spawn agent in worktree for issue'
priority: P2
status: open
type: design
deps:
- brd-qas3
tags:
- agent
owner: null
created_at: 2026-01-04T23:31:02.736303Z
---

## goal

Add CLI command to spawn a claude agent working on an issue in a fresh worktree.

```
brd agent spawn <issue-id>
```

## what it does

1. creates a new worktree for the issue (like `brd agent init`)
2. fires up claude code in that worktree with the issue context
3. returns control to user (agent runs in background? foreground?)

## open questions

- how to invoke claude code programmatically?
  - `claude` CLI with `--print` or similar?
  - need to pass issue context as prompt
- foreground vs background execution?
  - foreground: simpler, user sees output directly
  - background: allows spawning multiple agents
- how to pass issue context to claude?
  - read issue file, format as prompt
  - include deps context?

## out of scope (separate issues)

- TUI integration (brd-cnd8)
- interacting with running agent (brd-qas3)
- progress streaming / output capture
