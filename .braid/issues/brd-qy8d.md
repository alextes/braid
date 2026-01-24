---
schema_version: 8
id: brd-qy8d
title: 'design: extract load-lock-modify-save helper'
priority: P3
status: open
type: design
deps: []
tags:
- refactor
owner: null
created_at: 2026-01-20T11:23:41.618731Z
---

## problem

the load-lock-modify-save pattern is repeated 9+ times across 7 command files:
- `done.rs:23-27`
- `skip.rs:13-17`
- `set.rs:13-17`
- `start.rs:376-399`
- `dep.rs:13-18` (cmd_dep_add) and `dep.rs:56-61` (cmd_dep_rm)
- `add.rs:13, 39-41`

typical sequence:
```rust
let config = Config::load(&paths.config_path())?;
let _lock = LockGuard::acquire(&paths.lock_path())?;
let mut issues = load_all_issues(paths, &config)?;
let full_id = resolve_issue_id(id, &issues)?;
// ... modify issue ...
issue.save(&issue_path)?;
```

## considerations

- some commands only need read access (no save)
- some commands modify multiple issues (dep add/rm)
- lock scope needs careful thought (hold during modification vs. release early)
- error handling varies slightly between commands

## possible approaches

1. **helper function** - `with_issue_mut(paths, id, |issue| { ... })` style closure
2. **builder pattern** - `IssueTransaction::new(paths).lock().load().resolve(id).modify(...).save()`
3. **macro** - generate the boilerplate at compile time

need to decide which approach balances ergonomics vs flexibility
