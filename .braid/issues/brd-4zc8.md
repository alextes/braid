---
schema_version: 8
id: brd-4zc8
title: add integration tests for dual-write behavior
priority: P2
status: skip
deps: []
tags:
- testing
owner: null
created_at: 2025-12-28T08:57:41.115931Z
started_at: 2025-12-28T17:04:25.864734Z
completed_at: 2025-12-28T17:04:25.864734Z
---

dual-write syncs issue files between worktree and control root but is untested.

## behavior to test
- add writes to both locations
- start/done/skip write to both locations
- rm removes from both locations
- changes in control root visible to other worktrees

## test cases
- create issue in worktree, verify in control root
- modify issue status, verify both updated
- delete issue, verify both removed
- simulate multi-worktree scenario