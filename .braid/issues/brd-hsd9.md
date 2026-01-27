---
schema_version: 8
id: brd-hsd9
title: add brd agent clean command
priority: P2
status: done
deps: []
tags:
- agents
- cli
owner: null
created_at: 2026-01-27T21:21:49.793697Z
started_at: 2026-01-27T21:23:20.214295Z
completed_at: 2026-01-27T21:40:00.992877Z
---

## description

add a command to clean up stale agent session files.

## behavior

`brd agent clean` should:
- remove session files (`.json` and `.log`) for zombie/completed/killed agents
- optionally `--all` to remove all session files
- show what will be deleted before doing it (or `--force` to skip confirmation)

## example

```
$ brd agent clean
found 3 stale sessions:
  agent-a1b2 (zombie) - 2 days old
  agent-c3d4 (completed) - 5 days old  
  agent-e5f6 (killed) - 1 day old
remove these? [y/N]
```