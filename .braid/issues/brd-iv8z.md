---
schema_version: 7
id: brd-iv8z
title: refactor dep command to use blocker/blocked terminology
priority: P2
status: done
deps: []
owner: null
created_at: 2026-01-01T15:23:27.084527Z
updated_at: 2026-01-01T15:31:51.537655Z
---

## Problem

Current `brd dep` uses "parent/child" terminology which is semantically confusing:
- `brd dep add <CHILD> <PARENT>` - "child depends on parent"
- Natural reading: parent is the big task, children are subtasks that complete first
- Actual meaning: parent blocks child (opposite intuition)

## Solution

Replace with **blocker/blocked** terminology:

```
brd dep add <BLOCKED> <BLOCKER>
# "BLOCKED is blocked by BLOCKER"
```

## Changes

1. Rename CLI arguments from `<CHILD> <PARENT>` to `<BLOCKED> <BLOCKER>`
2. Update help text: "add a dependency (blocked is blocked by blocker)"
3. Update any user-facing messages that reference parent/child
4. Update `brd dep rm` similarly

## Scope

- CLI argument names and help text only
- Internal field names can stay as-is (no schema migration needed)
- No behavior change, just terminology
