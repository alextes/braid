---
schema_version: 6
id: brd-pezq
title: add unit tests for agent init command
priority: P2
status: done
deps: []
tags:
- testing
owner: null
created_at: 2025-12-28T08:49:25.186681Z
updated_at: 2025-12-28T20:15:57.101204Z
---

agent init creates worktrees but has no unit tests.

## logic to test
- agent name validation (alphanumeric, hyphen, underscore only)
- worktree directory creation
- branch creation
- agent.toml generation

## test cases
- valid agent name
- invalid agent name (special chars)
- duplicate agent name
- worktree structure verification