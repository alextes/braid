---
schema_version: 8
id: brd-u9qm
title: add unit tests for dep command
priority: P2
status: done
deps: []
tags:
- testing
owner: null
created_at: 2025-12-28T08:49:05.883055Z
started_at: 2025-12-28T17:39:25.464875Z
completed_at: 2025-12-28T17:39:25.464875Z
---

dep add/rm commands have cycle prevention logic but only integration test coverage.

## current state
- cycle detection logic in graph.rs is well tested
- but dep.rs command logic is not unit tested

## test cases needed
- self-dependency rejection
- cycle error message format
- successful add (no cycle)
- duplicate add (idempotent)
- remove existing dep
- remove non-existent dep (no-op)
- partial ID resolution in dep commands