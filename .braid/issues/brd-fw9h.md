---
schema_version: 4
id: brd-fw9h
title: add tests for TUI app and ui modules
priority: P3
status: todo
deps: []
tags:
- tests
owner: null
created_at: 2025-12-28T22:57:32.292647Z
updated_at: 2025-12-28T22:57:32.292647Z
---

The TUI modules `app.rs` and `ui.rs` have no test coverage (~23K LOC combined). This creates regression risk as the TUI evolves.

## Suggested approach
- Use `ratatui::backend::TestBackend` for snapshot tests
- Test key state transitions in `app.rs`
- Test rendering output for different issue states

## Acceptance criteria
- [ ] Basic tests for app state management
- [ ] Snapshot tests for UI rendering
- [ ] Event handling tests (some exist in event.rs already)