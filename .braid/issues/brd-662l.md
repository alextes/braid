---
schema_version: 8
id: brd-662l
title: add timing output to doctor command
priority: P3
status: done
deps: []
owner: null
created_at: 2025-12-28T22:57:56.683906Z
started_at: 2026-01-24T20:22:11.090627Z
completed_at: 2026-01-24T20:23:45.324411Z
---

Add a `took: Nms` line to the doctor command output, similar to what brd ls has. This helps identify if graph operations are slow.