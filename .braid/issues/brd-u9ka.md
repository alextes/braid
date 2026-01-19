---
schema_version: 8
id: brd-u9ka
title: integration tests for workflow mode switching and init
priority: P2
status: done
deps: []
owner: null
created_at: 2026-01-02T13:35:13.813972Z
started_at: 2026-01-04T15:22:38.390019Z
completed_at: 2026-01-04T15:22:38.390019Z
---

## Goal

Create integration tests for core workflow paths from brd-e27d.

## Critical Paths to Test

### Mode Switching
- `brd mode local-sync` from git-native creates issues branch
- `brd mode git-native` from local-sync copies issues back
- switching modes preserves existing issues

### Init Behavior
- `brd init` on fresh repo works
- `brd init` on already-initialized repo errors (see brd-1ml1)
- `brd init -y` uses defaults (local-sync + auto-sync)

### Claim/Done Flow
- `brd start` claims issue, sets status to doing
- `brd done` marks complete, sets status to done
- claimed issues not claimable by another agent

## Implementation Notes

These would be end-to-end tests that:
- create temp git repos
- run actual brd commands
- verify file system state and command output

Could use a test harness similar to existing tests or a dedicated integration test setup.

## Related

- brd-e27d: manual/assisted testing checklist
- brd-1ml1: design for init-when-already-initialized