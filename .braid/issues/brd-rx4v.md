---
schema_version: 4
id: brd-rx4v
title: implement TUI unit tests
priority: P2
status: todo
deps: []
tags:
- testing
owner: null
created_at: 2025-12-28T17:00:38.214605Z
updated_at: 2025-12-28T17:00:38.214605Z
---

implement the testing approach designed in brd-48y6.

## tasks

1. extract `rebuild_lists()` helper from `reload_issues()`
2. add `#[cfg(test)] App::with_state()` constructor
3. add unit tests in `src/tui/app.rs`:
   - `test_move_up_down`
   - `test_switch_pane`
   - `test_selection_clamping`
   - `test_add_issue_flow`
   - `test_edit_issue_flow`
   - `test_cancel_returns_to_normal`

## reference

see brd-48y6 for full design analysis.