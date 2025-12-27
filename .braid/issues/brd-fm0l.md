---
schema_version: 4
id: brd-fm0l
title: add unit tests for issue ID generation
priority: P2
status: doing
deps: []
owner: agent-one
created_at: 2025-12-26T19:17:29.029972Z
updated_at: 2025-12-27T22:12:40.155729Z
acceptance:
- test ID format matches config (prefix-suffix)
- test suffix length matches config.id_len
- test uniqueness (doesn't collide with existing files)
---

generate_issue_id creates random IDs and handles collisions. currently untested.

test cases:
- generated ID matches expected format (prefix-suffix)
- suffix uses only allowed charset (0-9, a-z)
- suffix length matches config.id_len
- retries on collision with existing file