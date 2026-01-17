---
schema_version: 6
id: brd-662l
title: add timing output to doctor command
priority: P3
status: todo
deps: []
owner: null
created_at: 2025-12-28T22:57:56.683906Z
updated_at: 2026-01-04T21:02:41.707578Z
---

Add a `took: Nms` line to the doctor command output, similar to what brd ls has. This helps identify if graph operations are slow.