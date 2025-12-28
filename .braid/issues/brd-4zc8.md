---
schema_version: 4
id: brd-4zc8
title: add integration tests for dual-write behavior
priority: P2
status: todo
deps: []
owner: null
created_at: 2025-12-28T08:57:41.115931Z
tags:
- testing
updated_at: 2025-12-28T08:57:41.115931Z
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