---
schema_version: 8
id: brd-bynh
title: 'agent send fails: --output-format=stream-json requires --verbose'
priority: P2
status: done
deps: []
tags:
- agents
- bug
owner: null
created_at: 2026-01-27T13:28:52.234464Z
completed_at: 2026-01-27T13:30:09.317436Z
---

## description

`brd agent send` fails with:

```
Error: When using --print, --output-format=stream-json requires --verbose
claude exited with code 1
```

## cause

`cmd_agent_send` passes `-p` and `--output-format stream-json` but omits `--verbose`.

## fix

add `--verbose` to the claude command args in `cmd_agent_send`.