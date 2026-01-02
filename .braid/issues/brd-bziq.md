---
schema_version: 6
id: brd-bziq
title: git_rev_parse passes multi-word args as single argument
priority: P1
status: done
deps: []
owner: null
created_at: 2025-12-29T23:35:31.749637Z
updated_at: 2025-12-29T23:36:34.278089Z
---

## bug

In repo.rs, `git_rev_parse` uses `.arg(arg)` which passes the whole string as one argument.

When called with `"--abbrev-ref HEAD"`, git interprets it literally and returns `--abbrev-ref HEAD` instead of the branch name.

This breaks `ensure_issues_worktree` branch detection, causing `brd sync` to fail with:
```
error: issues worktree exists but is on branch '--abbrev-ref HEAD', expected 'braid-issues'
```

Fix: split the argument or use `.args()` for multi-word inputs.