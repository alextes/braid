---
schema_version: 8
id: brd-zjo3
title: add unit tests for show command
priority: P3
status: done
deps: []
tags:
- testing
owner: null
created_at: 2025-12-28T08:49:45.008596Z
started_at: 2025-12-28T20:28:20.222964Z
completed_at: 2025-12-28T20:28:20.222964Z
---

show command displays issue details but has no unit tests.

## logic to test
- issue loading
- derived state computation display
- dependency status display
- JSON output format
- missing issue error

## test cases
- show existing issue
- show with deps (some done, some not)
- show blocked issue
- show ready issue
- JSON output structure