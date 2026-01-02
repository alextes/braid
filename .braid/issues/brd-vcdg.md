---
schema_version: 6
id: brd-vcdg
title: add unit tests for issue ID resolution
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-26T19:15:38.163194Z
updated_at: 2025-12-26T21:44:12.567977Z
acceptance:
- test exact ID matching
- test partial ID matching (suffix, contains)
- test ambiguous ID detection and error
- test missing ID error
---

resolve_issue_id handles partial matching which has subtle edge cases:
- exact match should take precedence
- partial matches by suffix or contains
- ambiguous matches should error with list of candidates
- missing IDs should error appropriately

this logic is currently untested.