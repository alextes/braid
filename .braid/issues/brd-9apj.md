---
schema_version: 8
id: brd-9apj
title: migrate remaining test modules to use TestRepo utilities
priority: P3
status: done
deps:
- brd-unwz
tags:
- refactor
owner: null
created_at: 2026-01-22T21:21:52.773777Z
started_at: 2026-01-24T13:38:14.436102Z
completed_at: 2026-01-24T14:52:00.741442Z
---

migrate these test modules from local `create_test_repo()`/`write_issue()`/`make_cli()` helpers to use the shared `TestRepo`, `IssueBuilder`, and `test_cli()` from `src/test_utils.rs`:

- [x] done.rs
- [x] skip.rs
- [x] start.rs (non-git tests migrated, git-specific helpers kept)
- [x] dep.rs
- [x] add.rs
- [x] agent.rs (replaced make_cli with test_cli, kept git-specific helpers)

set.rs was already migrated as proof of concept in brd-unwz.
