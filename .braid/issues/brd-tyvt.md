---
schema_version: 4
id: brd-tyvt
title: add unit tests for migrate command
priority: P3
status: todo
deps: []
owner: null
created_at: 2025-12-28T08:50:04.827801Z
tags:
- testing
updated_at: 2025-12-28T08:50:04.827801Z
---

migrate command applies schema migrations but only has unit tests for the migration logic itself.

## command logic to test
- dry-run mode (no file writes)
- file discovery
- migration summary output
- batch processing
- JSON output

## test cases
- migrate with changes
- migrate no changes needed
- dry-run output
- partial migration (some files need it)