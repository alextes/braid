---
schema_version: 8
id: brd-1qzs
title: prevent old brd versions from creating issues in upgraded repos
priority: P2
status: done
deps: []
tags:
- design
owner: null
created_at: 2025-12-26T21:19:28.81185Z
started_at: 2025-12-26T22:26:40.712921Z
completed_at: 2025-12-26T22:26:40.712921Z
---

## problem

when a repo is migrated to a newer schema version (e.g. v2), old versions of brd that only understand v1 can still create new issues using the old format. this creates inconsistency and broken issues.

## proposed solution

store the repo's schema version in config.toml:

```toml
[braid]
schema_version = 2
```

on startup, brd checks:
1. read `schema_version` from config.toml
2. compare against brd's max supported schema version
3. if repo schema > brd's max, error with: "this repo uses schema v{X}, please upgrade brd"

this prevents old brd versions from making any changes to upgraded repos.

## alternatives considered

- **doctor command check**: only catches issues after the fact, doesn't prevent them
- **per-issue validation**: more complex, doesn't prevent creation of malformed issues
