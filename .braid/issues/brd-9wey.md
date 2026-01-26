---
schema_version: 8
id: brd-9wey
title: show dirty file count in brd status sync field
priority: P3
status: done
deps: []
owner: null
created_at: 2026-01-20T11:27:08.940831Z
started_at: 2026-01-24T20:42:00.278438Z
completed_at: 2026-01-24T20:45:50.71125Z
---

currently `brd status` shows `(dirty)` when there are uncommitted changes in the issues worktree.

enhance to show the count of dirty files:
- `(1 dirty)` or `(3 dirty)` instead of just `(dirty)`

implementation: use `git status --porcelain` and count lines, or similar.