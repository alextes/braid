---
schema_version: 5
id: brd-i2q5
title: add issues_repo config field and schema v5 migration
priority: P2
status: done
deps:
- brd-ucso
owner: null
created_at: 2025-12-30T15:40:06.582474Z
updated_at: 2025-12-30T16:10:53.73031Z
---

Add `issues_repo: Option<String>` to Config struct.

- Update schema to v5
- Add migration from v4 (no-op, just version bump)
- Field is optional, defaults to None (current behavior)
- Validate path exists when set