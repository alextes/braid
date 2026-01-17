---
schema_version: 6
id: brd-cakw
title: show issue type more clearly in brd show output
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-28T10:19:23.832329Z
updated_at: 2026-01-04T20:40:04.491421Z
---

currently `brd show` doesn't prominently display the issue type (design/meta).

## current output
the type field is either missing or not shown at all in the output.

## desired
- show "Type: design" or "Type: meta" when set
- perhaps with color coding to match `brd ls` (italic for design)

## acceptance
- type is visible in normal output
- type is included in JSON output