---
schema_version: 8
id: brd-shiw
title: 'design: consolidate commit-push-retry logic'
priority: P2
status: open
type: design
deps: []
tags:
- refactor
owner: null
created_at: 2026-01-20T11:23:41.788473Z
---

## problem

two nearly identical functions in `start.rs` handle commit-push-retry:
- `commit_and_push_main_with_action` (lines 231-292)
- `commit_and_push_issues_branch_with_action` (lines 305-365)

both share:
- same retry loop (MAX_RETRIES = 2)
- same commit message format: `chore(braid): {action} {issue_id}`
- same fetch → rebase → abort on failure pattern
- same error handling

only differences:
- target branch (main vs issues branch)
- working directory (worktree_root vs issues_wt)
- push command (`HEAD:main` vs branch name)

## considerations

- functions are already parameterized by `action` and `issue_id`
- could add `branch` and `working_dir` parameters
- need to handle the "no origin" case (main checks `remote_main`, issues branch doesn't)

## possible approaches

1. **unified function** - single `commit_and_push_branch(paths, branch, worktree, action, issue_id, cli)`
2. **shared retry helper** - extract just the retry loop, keep two entry points
3. **trait-based** - define `PushTarget` trait with implementations for main/issues

unified function seems simplest since the logic is nearly identical
