---
schema_version: 8
id: brd-lrtx
title: lowercase brd dep output messages
priority: P2
status: done
deps: []
owner: null
created_at: 2026-01-02T13:33:45.977655Z
started_at: 2026-01-17T10:20:27.359108Z
completed_at: 2026-01-17T10:20:27.359108Z
---

Currently `brd dep add` outputs:
```
Added dependency: brd-x blocked by brd-y
```

Should be lowercase for consistency:
```
added dependency: brd-x blocked by brd-y
```

Same for `brd dep rm`.