---
schema_version: 8
id: brd-8kbe
title: check whether fs2 dep is still needed or can be dropped
priority: P3
status: done
deps: []
owner: null
created_at: 2025-12-26T19:21:01.108756Z
started_at: 2025-12-26T21:55:15.869234Z
completed_at: 2025-12-26T21:55:15.869234Z
---

## finding

fs2 is still needed. `LockGuard` (which uses fs2 for cross-platform file locking) is actively used in:

- `src/commands/add.rs` - mutex when creating issues
- `src/commands/dep.rs` - mutex when modifying dependencies
- `src/commands/done.rs` - mutex when marking issues done
- `src/commands/start.rs` - mutex when starting issues
- `src/tui/app.rs` - mutex for TUI operations

this prevents race conditions when multiple agents modify issues concurrently.
