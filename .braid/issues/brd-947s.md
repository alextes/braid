---
schema_version: 4
id: brd-947s
title: document workflow modes
priority: P3
status: doing
deps:
- brd-ozah
- brd-c2g4
- brd-dje7
- brd-tvue
owner: agent-one
created_at: 2025-12-28T16:21:56.751882Z
updated_at: 2025-12-28T21:36:37.568014Z
---

create user-facing documentation for workflow modes.

## new file: docs/workflow-modes.md

overview of all modes:
1. solo mode - one branch, no sync needed
2. branch-per-feature - issues in branches, merge to share
3. multi-human/remote - git push/pull coordination
4. local-sync - shared worktree for local agents

when to use each, how to set up, how to switch.

## updates

- `docs/sync-branch.md` - rename/refocus as "local sync mode" docs
- `README.md` - add brief modes section with link to full docs
- `AGENTS.md` - update braid-specific docs list

## parent

part of brd-jpux (workflow modes design)