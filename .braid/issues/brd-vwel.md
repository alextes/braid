---
schema_version: 6
id: brd-vwel
title: add issues_branch config field and schema v5 migration
priority: P2
status: skip
deps: []
owner: null
created_at: 2025-12-28T23:58:15.656691Z
updated_at: 2025-12-30T15:54:10.486666Z
---

Add `issues_branch: Option<String>` field to Config struct and migrate schema from v4 to v5.

## Changes

**src/config.rs:**
```rust
pub struct Config {
    pub schema_version: u32,
    pub id_prefix: String,
    pub id_len: u32,
    pub sync_branch: Option<String>,
    pub issues_branch: Option<String>,  // NEW
}
```

**src/migrate.rs:**
- Add v4 → v5 migration (no-op, just version bump)
- Update CURRENT_SCHEMA to 5

## Acceptance criteria
- [ ] Config parses with new optional field
- [ ] Old configs (v4) auto-migrate to v5
- [ ] Tests pass