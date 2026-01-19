---
schema_version: 8
id: brd-8h03
title: add brd status command
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-28T23:16:57.75444Z
started_at: 2025-12-30T15:19:39.018868Z
completed_at: 2025-12-30T15:19:39.018868Z
---

New command showing repo state at a glance.

## Output

```
braid status
─────────────
Mode:     git-native
Agent:    agent-one
Prefix:   brd
Issues:   12 open, 5 doing, 23 done

# or for local-sync:
Mode:     local-sync (branch: braid-issues)
Agent:    agent-one
Prefix:   brd
Issues:   12 open, 5 doing, 23 done
Sync:     up to date with origin/braid-issues
```

## Files

- `src/cli.rs` - add Status command
- `src/commands/status.rs` - new file
- `src/commands/mod.rs` - export module
- `src/main.rs` - route command