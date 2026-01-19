---
schema_version: 8
id: brd-esqt
title: change State field in brd show to only show Blocked when blocked
priority: P2
status: done
deps:
- brd-a6sf
owner: null
created_at: 2026-01-18T20:43:17.506521Z
started_at: 2026-01-19T17:44:19.971056Z
completed_at: 2026-01-19T17:45:16.567015Z
---

Currently `brd show` outputs:
```
State:    BLOCKED
  open:   brd-xnxw
```

This is confusing alongside `Status:`. Change to only show when blocked:
```
Blocked:  brd-xnxw (open)
```

When not blocked, omit the field entirely (issue is implicitly ready if status is open and no blockers shown).