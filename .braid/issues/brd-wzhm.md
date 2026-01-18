---
schema_version: 7
id: brd-wzhm
title: add unit tests for lock module
priority: P2
status: done
deps: []
tags:
- testing
owner: null
created_at: 2025-12-28T08:49:25.071656Z
updated_at: 2025-12-28T20:00:24.784307Z
---

file locking is critical for multi-agent coordination but untested.

## LockGuard behavior
- acquire() blocks until lock available
- try_acquire() returns None if locked
- drop releases lock
- lock file is created

## test cases
- successful acquire
- successful try_acquire
- try_acquire when locked returns None
- lock released on drop
- lock file cleanup