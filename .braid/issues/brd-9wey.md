---
schema_version: 8
id: brd-9wey
title: show dirty file count in brd status sync field
priority: P3
status: open
deps: []
owner: null
created_at: 2026-01-20T11:27:08.940831Z
---

currently `brd status` shows `(dirty)` when there are uncommitted changes in the issues worktree.

enhance to show the count of dirty files:
- `(1 dirty)` or `(3 dirty)` instead of just `(dirty)`

implementation: use `git status --porcelain` and count lines, or similar.