---
schema_version: 4
id: brd-vkc4
title: add unit tests for doctor command
priority: P1
status: todo
deps: []
owner: null
created_at: 2025-12-28T08:48:43.455202Z
tags:
- testing
updated_at: 2025-12-28T08:48:43.455202Z
---

doctor performs many validation checks but only has integration coverage.

## checks to unit test
- .braid directory exists check
- config.toml validity check
- issue file parsing check
- schema version check (needs migration detection)
- missing dependency detection
- cycle detection
- AGENTS.md block version check

## test cases
- each check passing
- each check failing with correct error
- JSON output format
- error aggregation