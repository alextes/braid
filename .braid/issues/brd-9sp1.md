---
schema_version: 8
id: brd-9sp1
title: add diff stat infrastructure to git.rs
priority: P2
status: doing
deps: []
tags:
- tui
owner: agent-one
created_at: 2026-01-25T21:30:37.47433Z
started_at: 2026-01-26T08:59:00.821953Z
acceptance:
- can get diff stats for any worktree
- works for both uncommitted and committed changes
---

add functions to get diff statistics from git.

## scope
- `git::diff_stat(cwd, base, head)` → `DiffStat { files_changed, insertions, deletions }`
- `git::diff_files(cwd, base, head)` → `Vec<FileDiff { path, status, insertions, deletions }>`
- handle both:
  - dirty tree: `git diff --stat` (no base/head)
  - branch diff: `git diff --stat main..HEAD`
