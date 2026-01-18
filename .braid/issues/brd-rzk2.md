---
schema_version: 7
id: brd-rzk2
title: add tests for TUI modules
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-31T08:27:38.467381Z
updated_at: 2026-01-02T21:09:35.389916Z
---

The TUI module has 2,375 lines across 3 files with only 1 test:
- app.rs (827 lines) - 0 tests
- ui.rs (896 lines) - 0 tests  
- event.rs (652 lines) - 1 test

This is the primary user-facing interface.

Risk: Broken navigation, filtering, or rendering would severely impact UX.

Test areas needed:
- Event handling and keyboard navigation
- State management (filtering, selection, scrolling)
- Rendering logic for different issue states
- Filter mode interactions
- Edge cases (empty lists, long titles, many dependencies)