---
schema_version: 4
id: brd-sbd8
title: rename labels field to tags in schema
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-27T14:51:20.551097Z
updated_at: 2025-12-27T15:08:09.642639Z
acceptance:
- schema uses "tags" instead of "labels"
- migration renames labels→tags in existing issues
- CLI flags updated (--label → --tag)
---

"tags" is more natural than "labels". requires schema v4 migration.
