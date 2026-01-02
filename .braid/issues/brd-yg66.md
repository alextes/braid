---
schema_version: 4
id: brd-yg66
title: brd next should skip meta-type issues
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-27T08:26:43.753205Z
updated_at: 2025-12-27T08:37:07.879477Z
acceptance:
- 'brd next excludes issues with type: meta'
---

meta issues are tracking containers (like epics), not actionable work items. brd next skips them so agents don't get assigned tracking issues. brd ready still includes them so they show up when all deps are done and can be closed.