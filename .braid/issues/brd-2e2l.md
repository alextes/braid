---
schema_version: 4
id: brd-2e2l
title: 'bug: brd ls footer shows redundant ''todo'' and ''open'' counts'
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-29T23:34:26.753429Z
updated_at: 2025-12-30T14:50:32.700084Z
---

current output:
```
todo: 38 | done: 8 | skip: 2 | open: 38 | time: 4ms
```

"open" = todo + doing (non-closed issues). when doing=0, it equals todo.

showing both is redundant. options:
1. just show "open: N" (cleaner, includes doing)
2. just show "todo: N" and conditionally "doing: N" (current behavior minus open)

option 1 is simpler.
