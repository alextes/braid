---
schema_version: 8
id: brd-rx4v
title: implement TUI unit tests
priority: P2
status: done
deps: []
tags:
- testing
owner: null
created_at: 2025-12-28T17:00:38.214605Z
started_at: 2025-12-28T17:14:27.002296Z
completed_at: 2025-12-28T17:14:27.002296Z
---

implement the testing approach designed in brd-48y6.

## tasks

1. extract `rebuild_lists()` helper from `reload_issues()` (not needed)
2. add `#[cfg(test)] App::with_state()` constructor (not needed)
3. add unit tests in `src/tui/app.rs` (implemented in `src/tui/event.rs` instead):
   - `test_move_up_down`
   - `test_switch_pane`
   - `test_selection_clamping`
   - `test_add_issue_flow`
   - `test_edit_issue_flow`
   - `test_cancel_returns_to_normal`

## implementation notes
- added `handle_key_event` in `src/tui/event.rs` and unit tests covering input modes, navigation, selection clamping, help gating, and quit keys
- kept `App` unchanged; tests drive key handling with temp repo paths for i/o-dependent paths

## reference

see brd-48y6 for full design analysis.
