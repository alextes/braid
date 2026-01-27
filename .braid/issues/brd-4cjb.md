---
schema_version: 8
id: brd-4cjb
title: implement full-text search for issues
priority: P3
status: open
type: design
deps: []
tags:
- cli
owner: null
created_at: 2025-12-27T08:14:33.487622Z
acceptance:
- search scope is title + body (regex)
- simple built-in backend (iterate files, no sqlite)
- CLI is brd search with filtering flags
- show snippets/context around matches
---

currently `brd search` just prints instructions to use grep/rg. a proper search would be nice for larger issue sets.

## direction (from discussion)

**backend**: simple built-in - iterate markdown files, regex match. sqlite FTS is overkill for 10-100 issues.

**CLI**: `brd search <query>` (not `brd ls --search`)

**filtering**: the real value over grep - combine text search with structured filters:
```
brd search "auth" --status open --tag backend
brd search "bug" --priority P1
```

**output**: show snippets/context around matches so you know *why* something matched without opening the file.

## example output

```
brd-abc1  fix authentication bug  [P1] [open] #backend
  ...token validation **auth** fails when...

brd-xyz2  add oauth support  [P2] [open] #backend #auth
  ...implement **auth** flow for third-party...
```