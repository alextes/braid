---
schema_version: 7
id: brd-m7vy
title: add cycle prevention on dep edits
priority: P3
status: done
deps: []
owner: null
created_at: 2025-12-25T21:46:28.262547Z
updated_at: 2025-12-27T21:37:35.090005Z
acceptance:
- brd dep add rejects if it would create a cycle
- clear error message showing the cycle
---
