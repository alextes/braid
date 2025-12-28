---
schema_version: 4
id: brd-48y6
title: add unit tests for TUI state management
priority: P1
status: todo
type: design
deps: []
owner: null
created_at: 2025-12-28T08:48:43.326404Z
tags:
- testing
updated_at: 2025-12-28T08:48:43.326404Z
---

the TUI has 7 input modes and complex state transitions with zero test coverage.

## InputMode states
- Normal
- Title (new issue)
- Priority (new issue)
- EditSelect
- EditTitle
- EditPriority
- EditStatus

## test cases needed
- state transitions between modes
- selection clamping on reload
- pane switching
- input handling in each mode
- message display/clearing

## note
may need to refactor App to be more testable (dependency injection for paths/io)