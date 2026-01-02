---
schema_version: 6
id: brd-mfzr
title: 'design: orthogonal workflow configuration (branch + auto-sync)'
priority: P2
status: done
type: design
deps: []
owner: null
created_at: 2025-12-30T17:06:21.463511Z
updated_at: 2025-12-30T17:10:17.481058Z
---

## Key Insight: Coordination vs Storage are Orthogonal

Two independent choices:

1. **Coordination** (`auto_pull`/`auto_push`) - git-based sync for ALL agents (local or remote)
2. **Storage** (`issues_branch`) - where issues live (with code vs separate branch)

**Critical correction:** `issues_branch` is NOT required for multi-agent. Git roundtrip via `auto_pull`/`auto_push` coordinates local agents just fine. The `issues_branch` is purely a storage preference.

## Config

```toml
# .braid/config.toml
schema_version = 6
issues_branch = "braid-issues"  # optional, for cleaner history
auto_pull = true   # fetch+rebase before brd start
auto_push = true   # commit+push after brd done
```

## User Story Mapping

| Story | Recommended Config | Notes |
|-------|-------------------|-------|
| Solo dev, one process | auto=yes, storage=any | Safe default |
| Solo dev, many local agents | auto=yes, storage=any | Git roundtrip coordinates |
| Remote team | auto=yes, storage=any | Git roundtrip coordinates |
| Wants clean history | auto=any, storage=separate | Personal preference |

## Init: Two Orthogonal Questions

```
Q1: Auto-sync issues with git? (default: yes)
├─ Yes → auto_pull=true, auto_push=true
└─ No  → auto_pull=false, auto_push=false

Q2: Where should issues live? (default: with code)
├─ With code → issues_branch=None
└─ Separate branch → issues_branch="braid-issues"
```

## Implemented

- `src/config.rs` - Added `auto_pull: bool`, `auto_push: bool` (default: true)
- `src/commands/init.rs` - 2-question flow with trade-off explanations
- `src/commands/start.rs` - Uses `config.auto_pull`
- `src/commands/done.rs` - Uses `config.auto_push`
- `src/migrate.rs` - Schema v6 migration