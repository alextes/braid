---
schema_version: 8
id: brd-zln0
title: render done deps in brd show slightly darker
priority: P2
status: done
deps: []
owner: null
created_at: 2026-01-26T21:30:10.210023Z
started_at: 2026-01-27T16:53:53.384442Z
completed_at: 2026-01-27T16:57:19.145156Z
---

for visual consistency with the TUI, render done dependencies in `brd show` output with a dimmed/grey color (similar to how blocked issues are slightly dimmed in the TUI).

this helps visually distinguish resolved vs unresolved dependencies at a glance.