---
schema_version: 7
id: brd-uft9
title: add unit tests for clear_external_repo
priority: P1
status: done
deps: []
owner: null
created_at: 2026-01-04T17:49:49.082482Z
updated_at: 2026-01-04T18:01:17.598308Z
---

`clear_external_repo` in `src/commands/config.rs` has ~80 lines of code with no unit test coverage.

## tests needed
- success: updates config, commits, warns about agents
- edge case: already cleared (no-op)

## risk
silent failures when clearing external repo config.

## files
- `src/commands/config.rs` - add tests to `mod tests` block