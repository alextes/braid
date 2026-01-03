---
schema_version: 6
id: brd-hv5k
title: 'design: rename status ''todo'' to ''open'' to pair with reopen'
priority: P3
status: todo
type: design
deps: []
owner: null
created_at: 2025-12-29T23:15:06.206602Z
updated_at: 2026-01-01T15:15:50.921015Z
---

if we're adding `brd reopen`, having `status: todo` feels odd. "reopen" suggests "open".

## considerations

- discuss with human before implementing
- would require schema migration
- backwards compatibility: accept both "todo" and "open" during transition?
- grep/tooling that looks for "todo" would break

## if todo → open, should done → closed?

for symmetry: open/closed pairs naturally (like GitHub issues).

current statuses and lengths:
- todo (4) → open (4) ✓
- doing (5)
- done (4) → closed (6) — longer than others
- skip (4)

"closed" is more symmetrical with "open" but breaks the ~4-5 char pattern. alternative: keep "done" since it still makes sense with "reopen" (you can reopen something that's done).
