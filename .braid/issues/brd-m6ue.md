---
schema_version: 8
id: brd-m6ue
title: add unit tests for pure functions in ui.rs
priority: P3
status: open
deps: []
owner: null
created_at: 2026-01-26T08:55:59.179373Z
---

low-hanging fruit from test coverage analysis (see .plans/tui-test-coverage.md):

- `truncate(s, max_len)` - 3-4 tests for edge cases
- `format_age(timestamp)` - 5-6 tests (minutes, hours, days, weeks, months, years)
- `age_color(duration)` - 3 tests for threshold boundaries
- `update_offset()` - 4-5 tests for scroll behavior
- `centered_rect()` - 2-3 tests for layout calculation

these are pure functions, easy to test in isolation.