---
schema_version: 4
id: brd-u5am
title: add unit tests for ready command
priority: P3
status: todo
deps: []
owner: null
created_at: 2025-12-28T08:49:45.246164Z
tags:
- testing
updated_at: 2025-12-28T08:49:45.246164Z
---

ready command filters issues but has no unit tests.

## logic to test
- filters to only ready issues (status=todo, deps resolved)
- sorting by priority
- output formatting

## test cases
- all ready (no deps)
- some blocked
- none ready
- priority ordering