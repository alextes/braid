---
schema_version: 8
id: brd-n61z
title: agent send outputs raw JSON and doesn't update log file
priority: P2
status: done
deps: []
tags:
- agents
- bug
owner: null
created_at: 2026-01-27T13:35:04.69071Z
completed_at: 2026-01-27T13:35:55.433916Z
---

## description

`brd agent send` has two problems:

1. outputs raw stream-json to terminal instead of human-readable text
2. resumed conversation doesn't append to the session's log file

## fix

1. remove `--output-format stream-json` - let claude output normally
2. append stdout/stderr to the session log file (like spawn does)