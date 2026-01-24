---
schema_version: 8
id: brd-2ftd
title: 'design: enhanced deps display with titles and sorting'
priority: P2
status: open
type: design
deps: []
tags:
- ux
owner: null
created_at: 2026-01-19T23:10:52.639593Z
---

## idea

enhance the `Deps:` field in `brd show` to be more informative:

1. **list format** - show each dep on its own line instead of comma-separated
2. **include title** - show dep title alongside id and status
3. **sort by status** - open deps first, done/skip last

## current
```
Deps:     brd-xnxw (open), brd-a6sf (done)
```

## proposed
```
Deps:
  - brd-xnxw (open): CLI: spawn agent in worktree for issue
  - brd-a6sf (done): add started_at and completed_at, remove updated_at
```

or with sorting:
```
Deps:
  - brd-xnxw (open): CLI: spawn agent in worktree for issue
  - brd-a6sf (done): add started_at and completed_at
```

## use cases

- **meta issues** - tracking issues with many children benefit from seeing titles at a glance
- **context** - when viewing an issue, immediately see what it depends on without extra lookups

## open questions

- should this be the default, or behind a flag like `--verbose`?
- truncate long titles?
- apply same treatment to `Dependents:`?