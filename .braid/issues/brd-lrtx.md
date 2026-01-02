---
schema_version: 6
id: brd-lrtx
title: lowercase brd dep output messages
priority: P3
status: todo
deps: []
owner: null
created_at: 2026-01-02T13:33:45.977655Z
updated_at: 2026-01-02T13:33:45.977655Z
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