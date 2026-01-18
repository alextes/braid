---
schema_version: 7
id: brd-km9t
title: 'doctor: detect AGENTS.md block mode mismatch'
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-29T08:25:39.18053Z
updated_at: 2025-12-30T15:31:39.351959Z
---

`brd doctor` should detect when the injected AGENTS.md block doesn't match the current mode.

## Problem

User is in local-sync mode but AGENTS.md has git-native instructions (or vice versa). This causes agents to follow wrong workflow.

## Solution

Add a doctor check:

```
$ brd doctor

checking .braid/ structure... ok
checking config... ok
checking AGENTS.md block... MISMATCH
  current mode: local-sync
  AGENTS.md block: git-native (v1)
  run `brd agents inject` to update
```

## Implementation

- Read AGENTS.md and extract the mode from the block (already have version extraction)
- Compare with current config mode
- Warn if mismatched