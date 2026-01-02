---
schema_version: 4
id: brd-6hxh
title: add unit tests for ship command (git operations)
priority: P1
status: done
deps: []
tags:
- testing
owner: null
created_at: 2025-12-28T08:48:43.212119Z
updated_at: 2025-12-28T16:40:26.435399Z
---

the ship command does complex git operations (fetch, rebase, push, reset) with no test coverage.

## risk
- touches git state directly
- rebase can fail in many ways
- used by all agents to push work

## test cases needed
- successful ship flow (mock git commands)
- rebase conflict handling
- dirty worktree detection
- remote push failure
- branch state verification