---
schema_version: 9
id: brd-lnsm
title: differentiate skip status color from background in dashboard bar
priority: P3
status: open
type: design
deps: []
owner: null
created_at: 2026-01-29T22:05:29.713822Z
---

the skip status uses DarkGray which blends into the terminal background,
making it hard to see in the status bar.

options to consider:
- use a slightly lighter grey (e.g., Color::Rgb(80, 80, 80))
- use a different character (e.g., ▒ instead of █)
- use a subtle pattern or different shade