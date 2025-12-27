---
schema_version: 2
id: brd-yg66
title: brd next should skip meta-type issues
priority: P2
status: done
deps: []
created_at: 2025-12-27T08:26:43.753205Z
updated_at: 2025-12-27T08:37:07.879477Z
acceptance:
- 'brd next excludes issues with type: meta'
- brd ready also excludes meta issues
---

meta issues are tracking containers (like epics), not actionable work items. brd next and brd ready should skip them and only return actionable issues.