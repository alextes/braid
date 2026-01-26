---
schema_version: 8
id: brd-gnn5
title: 'dashboard: velocity sparkline (7-day trend)'
priority: P2
status: open
deps: []
tags:
- tui
owner: null
created_at: 2026-01-25T10:16:32.946666Z
---
add a velocity section to the TUI dashboard showing issue completion and creation trends over the past 7 days using ASCII sparklines.

## mockup

```
┌─ Velocity (7d) ────────────────────────┐
│ completed ▂▄█▆▃▅▇  23 total (+5)       │
│ created   ▅▃▂▄▆▃▄  18 total            │
└────────────────────────────────────────┘
```

## data source

- group issues by `completed_at` date (last 7 days) for completion sparkline
- group issues by `created_at` date (last 7 days) for creation sparkline
- show total count and delta vs previous 7-day period

## sparkline rendering

use braille or block characters for the sparkline:
- `▁▂▃▄▅▆▇█` (block elements, 8 levels)
- scale to max value in the 7-day window
- each character = 1 day

## implementation notes

- add new section to `draw_dashboard()` in `src/tui/ui.rs`
- position below the "Recent (24h)" section or replace it with this richer view
- helper function: `fn make_sparkline(values: &[usize]) -> String`

## acceptance

- [ ] sparkline shows 7 days of completion data
- [ ] sparkline shows 7 days of creation data
- [ ] totals displayed alongside sparklines
- [ ] delta vs previous period shown (e.g. "+5" or "-3")
