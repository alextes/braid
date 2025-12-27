---
schema_version: 3
id: brd-sbd8
title: rename labels field to tags in schema
priority: P2
status: todo
deps: []
owner: null
created_at: 2025-12-27T14:51:20.551097Z
updated_at: 2025-12-27T14:51:20.551097Z
acceptance:
- schema uses "tags" instead of "labels"
- migration renames labels→tags in existing issues
- CLI flags updated (--label → --tag)
---

"tags" is more natural than "labels". requires schema v4 migration.
