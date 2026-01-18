---
schema_version: 7
id: brd-6zmi
title: update issues_dir to resolve external repo path
priority: P2
status: done
deps:
- brd-i2q5
owner: null
created_at: 2025-12-30T15:40:13.227377Z
updated_at: 2025-12-30T16:15:49.890526Z
---

When `issues_repo` is set in config:

1. Resolve path (relative to worktree_root or absolute)
2. Discover that repo's RepoPaths
3. Load that repo's Config
4. Return that repo's issues_dir()

Also support BRD_ISSUES_REPO env var override.