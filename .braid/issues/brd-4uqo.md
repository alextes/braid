---
schema_version: 8
id: brd-4uqo
title: add diff content retrieval and parsing to git.rs
priority: P2
status: done
deps: []
tags:
- tui
owner: null
created_at: 2026-01-25T21:30:42.801225Z
started_at: 2026-01-26T10:54:29.970756Z
completed_at: 2026-01-26T10:56:20.867463Z
acceptance:
- can retrieve parsed diff for any file
- handles context lines, additions, deletions
---

get actual diff content for display and parse into structured form.

## scope
- `git::diff_content(cwd, base, head, file)` → `String` (raw diff)
- parse into structured form:
  ```rust
  struct DiffHunk {
      old_start: u32,
      old_count: u32,
      new_start: u32,
      new_count: u32,
      lines: Vec<DiffLine>,
  }
  enum DiffLine { Context(String), Add(String), Remove(String) }
  ```
