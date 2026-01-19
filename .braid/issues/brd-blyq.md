---
schema_version: 8
id: brd-blyq
title: add unit tests for agents command
priority: P3
status: done
deps: []
tags:
- testing
owner: null
created_at: 2025-12-28T08:50:04.939819Z
started_at: 2025-12-28T21:08:42.800674Z
completed_at: 2025-12-28T21:08:42.800674Z
---

agents show/inject commands manage AGENTS.md but have no unit tests.

## logic to test
- block generation
- block version detection
- inject into existing file
- inject into new file
- update outdated block

## test cases
- show outputs correct block
- inject creates block
- inject updates outdated block
- version detection