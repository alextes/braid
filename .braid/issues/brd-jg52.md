---
schema_version: 9
id: brd-jg52
title: 'TUI: filter should search issue IDs, not just titles'
priority: P2
status: done
deps: []
tags:
- tui
owner: null
created_at: 2026-01-28T15:45:05.742302Z
completed_at: 2026-01-31T20:20:12.571199Z
---

filtering in the TUI only searches issue titles, not IDs.

## problem

user sees a dependency like `brd-1i52` and wants to filter to find it.
typing `1i52` or `brd-1i52` in the filter returns no results because
filter only matches against title text.

## expected behavior

filter should match against:
- issue ID (brd-xxxx)
- title
- possibly tags too

## current behavior

only matches title, so filtering by ID fails silently (shows empty list).