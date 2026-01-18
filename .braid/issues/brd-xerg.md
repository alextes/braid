---
schema_version: 7
id: brd-xerg
title: add unit tests for commit.rs
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-31T08:27:38.233639Z
updated_at: 2026-01-02T10:49:05.593304Z
---

commit.rs has 153 lines and 0 tests. This module handles git commit operations directly.

Risk: Untested commit logic could lead to:
- Accidental commits
- Lost work
- Incorrect commit messages
- Status check failures

Test areas needed:
- Commit message generation and formatting
- Status checking before commit
- Error handling for git failures
- Edge cases (empty commits, staged changes)