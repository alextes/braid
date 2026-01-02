---
schema_version: 6
id: brd-l66t
title: rename agents command to avoid confusion with agent
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-28T10:50:53.232495Z
updated_at: 2025-12-28T18:11:45.462063Z
---

currently we have:
- `brd agent init` / `brd agent ship` — worktree management
- `brd agents show` / `brd agents inject` — AGENTS.md management

this is confusing. options:
1. rename `agents` to `agentsmd` or `agents-md`
2. fold `agents` subcommands under `agent` (e.g. `brd agent inject`)
3. rename to something else entirely (e.g. `brd instructions`)