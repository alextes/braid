---
schema_version: 8
id: brd-71rv
title: 'dashboard: cycle time and lead time stats'
priority: P2
status: open
deps: []
tags:
- tui
owner: null
created_at: 2026-01-25T10:16:33.006938Z
---
add cycle time and lead time statistics to the TUI dashboard to help understand how quickly work moves through the system.

## definitions

- **lead time**: time from issue creation to completion (`completed_at - created_at`)
- **cycle time**: time from starting work to completion (`completed_at - started_at`)

lead time measures total wait + work time. cycle time measures active work time.

## mockup

```
┌─ Flow Metrics ─────────────────────────┐
│ lead time   avg 4.2d  median 2.8d      │
│ cycle time  avg 1.3d  median 0.9d      │
│ (based on 47 completed issues)         │
└────────────────────────────────────────┘
```

## data source

- filter to issues with `status == Done` and valid timestamps
- compute lead time: `completed_at - created_at` for each
- compute cycle time: `completed_at - started_at` for each (skip if `started_at` is None)
- calculate average and median for each metric

## implementation notes

- add new section to `draw_dashboard()` in `src/tui/ui.rs`
- helper functions for duration formatting (e.g. "4.2d", "2h", "< 1h")
- consider filtering to recent issues (e.g. last 30 days) to keep metrics relevant
- handle edge cases: no completed issues, missing `started_at`

## optional enhancements

- show trend indicator (up/down arrow) comparing to previous period
- break down by priority (P0 issues might have faster cycle time)
- histogram of cycle times

## acceptance

- [ ] average lead time displayed
- [ ] median lead time displayed
- [ ] average cycle time displayed
- [ ] median cycle time displayed
- [ ] sample size shown
- [ ] handles edge case of no completed issues gracefully
