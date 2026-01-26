---
schema_version: 8
id: brd-e67f
title: 'TUI: render unified diff view'
priority: P2
status: open
deps:
- brd-mw75
- brd-4uqo
tags:
- tui
owner: null
created_at: 2026-01-25T21:30:47.184718Z
acceptance:
- select file, see colored diff
- can scroll through long diffs
- press Esc or q to go back to file list
---

display file diff in TUI with colored unified diff format.

## scope
- new sub-view when file is selected
- unified diff format (like `git diff` output)
- color coding: green for additions, red for deletions
- scrollable for long diffs
- navigation: j/k scroll, tab or n/p switch files
