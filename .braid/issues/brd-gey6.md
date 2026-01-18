---
schema_version: 7
id: brd-gey6
title: detect AI agent execution and refuse interactive commands without --force
priority: P2
status: open
type: design
deps: []
owner: null
created_at: 2026-01-17T10:18:09.269942Z
updated_at: 2026-01-17T10:18:09.269942Z
---

Commands like `brd edit` open an interactive editor, which doesn't work for AI agents (Claude, Codex, etc.).

## investigation needed

- Can we detect common AI agent environments? (e.g. CLAUDE_CODE, CODEX env vars, or similar signals)
- Should we refuse interactive commands when detected, requiring --force to proceed?
- What commands should this apply to? (edit, tui, others?)
- Is there a standard way to detect non-interactive/headless execution?