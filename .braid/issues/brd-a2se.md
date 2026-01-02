---
schema_version: 6
id: brd-a2se
title: automatic cleanup/housekeeping command
priority: P3
status: todo
type: design
deps: []
owner: null
created_at: 2025-12-27T13:45:21.834025Z
updated_at: 2025-12-27T13:45:21.834025Z
acceptance:
- brd cleanup command exists
- auto-cancels issues older than 180 days (configurable)
- runs automatically with doctor or ls/ready if not run in 30 days
- disabled by default, opt-in via config
---

housekeeping for stale issues. should be configurable and off by default. consider what other cleanup tasks might be useful.
