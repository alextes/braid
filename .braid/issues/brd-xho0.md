---
schema_version: 7
id: brd-xho0
title: manage growing brd ls output
priority: P1
status: done
type: design
deps: []
tags:
- visual
owner: null
created_at: 2025-12-26T19:00:09.808782Z
updated_at: 2025-12-26T19:07:30.797082Z
acceptance:
- design decision documented
- implementation approach chosen
---

as issues accumulate, `brd ls` becomes unwieldy. need a strategy to keep the default view useful.

## options to explore

### 1. smart defaults for `brd ls`

- limit output to N issues (e.g. 32)
- always sort non-done before done
- add `--all` flag to show everything

### 2. compaction / archiving

- move done issues to `.braid/archive/` after some threshold
- archived issues not loaded by default
- `brd ls --archived` to see them
- could auto-archive on some trigger (e.g. >50 done issues)

### 3. time-based filtering

- hide done issues older than N days by default
- `--since` / `--until` flags

### 4. status quo with better filtering

- just improve `--status` filtering
- maybe `brd ls --active` as shorthand for `--status todo --status doing`

## decision

**chosen approach: smart defaults (option 1)**

`brd ls` behavior:

1. show todo/doing issues first (sorted by priority → created_at → id)
2. then show at most 10 most recently completed done issues
3. add `--all` flag to show everything without limits

no physical archiving for now - just display filtering. keeps implementation simple and avoids complexity around dependency resolution for archived issues.

## implementation issues

- brd-ur48: implement smart brd ls defaults
