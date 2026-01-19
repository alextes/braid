---
schema_version: 8
id: brd-ngpk
title: issues-branch mode
priority: P2
status: skip
type: meta
deps:
- brd-vwel
- brd-6lme
- brd-8vcu
- brd-ft2e
- brd-72xm
owner: null
created_at: 2025-12-28T23:59:53.476334Z
started_at: 2025-12-30T15:54:10.472427Z
completed_at: 2025-12-30T15:54:10.472427Z
---

Meta issue tracking implementation of issues-branch mode - a workflow mode that keeps issues on a separate branch while maintaining simple git semantics.

## Goal
Clean main branch history by storing issues on an orphan branch, accessed via git show/ls-tree for reads and branch switching for writes.

## Implementation issues
- brd-vwel: config field + schema migration
- brd-6lme: read infrastructure (git show/ls-tree)
- brd-8vcu: write infrastructure (branch switching)
- brd-ft2e: brd mode issues-branch command
- brd-72xm: update write commands