---
schema_version: 9
id: brd-ylt3
title: differentiate P3 priority color from background in dashboard bar
priority: P3
status: open
type: design
deps: []
owner: null
created_at: 2026-01-29T22:05:36.121352Z
---

the P3 priority uses DarkGray which blends into the terminal background,
making it hard to see in the priority bar.

options to consider:
- use a slightly lighter grey (e.g., Color::Rgb(80, 80, 80))
- use a different character (e.g., ▒ instead of █)
- use a muted blue or other subtle color to distinguish from grey

note: this is the same underlying issue as the skip status color problem.
could potentially solve both with a consistent approach.