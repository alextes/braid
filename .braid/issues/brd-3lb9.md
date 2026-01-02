---
schema_version: 4
id: brd-3lb9
title: fix doctor schema check to read raw file version
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-27T08:36:16.221517Z
updated_at: 2025-12-27T08:38:15.906444Z
acceptance:
- doctor checks on-disk schema version, not in-memory migrated version
- correctly reports issues needing migration
---
