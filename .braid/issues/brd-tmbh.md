---
schema_version: 4
id: brd-tmbh
title: add unit tests for repo module
priority: P3
status: doing
deps: []
tags:
- testing
owner: agent-three
created_at: 2025-12-28T08:57:23.304532Z
updated_at: 2025-12-28T21:08:59.420406Z
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