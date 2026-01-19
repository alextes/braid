---
schema_version: 8
id: brd-tmbh
title: add unit tests for repo module
priority: P3
status: done
deps: []
tags:
- testing
owner: null
created_at: 2025-12-28T08:57:23.304532Z
started_at: 2025-12-28T21:10:33.338981Z
completed_at: 2025-12-28T21:10:33.338981Z
---

repo module has complex path resolution logic but only 1 basic test.

## logic to test
- discover() finds repo from subdirectory
- control root resolution (env var > file > fallback)
- agent ID resolution (env var > agent.toml > $USER)
- git_rev_parse wrapper

## test cases
- discover from worktree root
- discover from subdirectory
- control root with env var
- control root with file
- control root fallback
- agent ID priority chain