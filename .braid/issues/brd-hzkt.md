---
schema_version: 7
id: brd-hzkt
title: add unit tests for clear_issues_branch
priority: P1
status: done
deps: []
owner: null
created_at: 2026-01-04T17:49:48.840252Z
updated_at: 2026-01-04T18:01:17.564704Z
---

`clear_issues_branch` in `src/commands/config.rs` has ~150 lines of code with no unit test coverage.

## tests needed
- success: copies issues from worktree back to `.braid/issues/`, removes symlinks, updates config
- error: dirty issues worktree should fail with helpful message
- edge case: no issues to copy (empty worktree)

## risk
could lose issues or fail silently on edge cases when user clears issues-branch config.

## files
- `src/commands/config.rs` - add tests to `mod tests` block