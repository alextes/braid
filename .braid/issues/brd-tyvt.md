---
schema_version: 7
id: brd-tyvt
title: add unit tests for migrate command
priority: P3
status: done
deps: []
tags:
- testing
owner: null
created_at: 2025-12-28T08:50:04.827801Z
updated_at: 2025-12-28T21:07:05.947936Z
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