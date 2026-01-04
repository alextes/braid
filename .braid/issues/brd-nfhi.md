---
schema_version: 6
id: brd-nfhi
title: add unit test for external-repo set success
priority: P1
status: done
deps: []
owner: null
created_at: 2026-01-04T17:49:48.965642Z
updated_at: 2026-01-04T18:01:17.581822Z
---

`cmd_config_external_repo` success path in `src/commands/config.rs` is untested. only error cases have tests (4 tests).

## tests needed
- success: validates external repo, loads config, commits

## risk
path resolution bugs or repo validation failures could go unnoticed.

## files
- `src/commands/config.rs` - add test to `mod tests` block