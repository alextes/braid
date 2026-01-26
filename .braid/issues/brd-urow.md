---
schema_version: 8
id: brd-urow
title: consider TUI test coverage strategy
priority: P3
status: done
deps: []
owner: null
created_at: 2026-01-26T08:08:33.795438Z
started_at: 2026-01-26T08:26:16.504468Z
completed_at: 2026-01-26T08:56:02.449375Z
---

evaluate our current TUI test coverage, particularly for:
- new details pane toggle feature (Tab key, overlay)
- rendering logic (draw_* functions)
- state transitions

consider:
- snapshot testing for UI rendering
- property-based testing for state machines
- integration tests that exercise full key sequences