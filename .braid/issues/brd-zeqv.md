---
schema_version: 6
id: brd-zeqv
title: add auto_pull and auto_push config fields with schema v6 migration
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-30T17:08:24.421968Z
updated_at: 2025-12-30T19:45:59.27573Z
---

Add two new boolean config fields for controlling sync behavior:

```toml
# .braid/config.toml
auto_pull = true   # fetch+rebase before brd start
auto_push = true   # commit+push after brd done
```

## Implementation

1. Add fields to `Config` struct in `src/config.rs`:
   - `auto_pull: bool` with `#[serde(default = "default_true")]`
   - `auto_push: bool` with `#[serde(default = "default_true")]`

2. Add schema v6 migration in `src/migrate.rs`:
   - Bump `CURRENT_SCHEMA` to 6
   - Migration adds `auto_pull = true` and `auto_push = true` to existing configs

3. Update config validation if needed

## Notes
- Both default to `true` for safety (prevents conflicts with remote collaborators)
- Users who want to avoid commit churn can set to `false`