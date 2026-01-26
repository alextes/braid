---
schema_version: 8
id: brd-kih6
title: 'refactor: split config.rs into subcommand modules'
priority: P3
status: done
deps: []
tags:
- refactor
owner: null
created_at: 2026-01-20T10:38:40.019771Z
started_at: 2026-01-24T21:03:51.289918Z
completed_at: 2026-01-24T21:10:02.18374Z
---

## Problem

`src/commands/config.rs` is 1,620 lines - the largest file in the codebase. It handles 4 distinct subcommands with complex interactive workflows mixed together.

## Proposal

Split into separate files by subcommand:

- `config_show.rs` - display current config
- `config_issues_branch.rs` - issues branch setup/teardown
- `config_external_repo.rs` - external repo setup
- `config_auto_sync.rs` - auto-sync toggle

Keep `config.rs` as a thin dispatcher that routes to submodules.

## Benefits

- Each file ~400 lines instead of 1,620
- Easier to navigate and understand
- Clearer ownership of each workflow
