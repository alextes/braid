---
schema_version: 4
id: brd-2bi5
title: add unit tests for issue parsing edge cases
priority: P2
status: done
deps: []
owner: agent-two
created_at: 2025-12-26T19:16:45.860751Z
updated_at: 2025-12-27T22:12:58.108103Z
acceptance:
  - test parsing with missing optional fields
  - test parsing with malformed frontmatter
  - test parsing with empty body
  - test round-trip serialization (parse → save → parse)
---

the existing test_parse_issue test covers the happy path. edge cases to add:

- missing optional fields (deps, labels, owner, acceptance)
- malformed yaml frontmatter
- empty or whitespace-only body
- verify serialization round-trip preserves all fields
