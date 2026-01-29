---
schema_version: 9
id: brd-vryr
title: brd set should not allow status changes, point to start/done/skip/reopen instead
priority: P1
status: done
deps: []
owner: null
created_at: 2026-01-29T13:44:25.743787Z
started_at: 2026-01-29T20:41:11.134207Z
completed_at: 2026-01-29T20:43:22.488598Z
---

## problem

currently `brd set <id> status <value>` allows bypassing all the workflow logic in:
- `brd start` (claiming, owner assignment, timestamps)
- `brd done` (completion validation, design issue checks)
- `brd skip` (skipping workflow)
- `brd reopen` (clearing owner, timestamps)

this is error-prone and defeats the purpose of having dedicated commands.

## solution

when `field == "status"`, reject the operation with a helpful error message:

```
error: cannot set status directly

use these commands instead:
  brd start <id>   - start working (sets status to doing)
  brd done <id>    - mark complete (sets status to done)
  brd skip <id>    - mark skipped (sets status to skip)
  brd reopen <id>  - reopen issue (sets status to open)
```

## critical file

- `src/commands/set.rs`