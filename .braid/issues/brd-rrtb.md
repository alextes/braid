---
schema_version: 7
id: brd-rrtb
title: handle uncommitted changes when starting issues
priority: P1
status: done
type: design
deps: []
owner: null
created_at: 2026-01-09T11:32:14.399513Z
updated_at: 2026-01-17T10:06:37.976023Z
---

currently brd start refuses to run when there are uncommitted changes in the working tree:

```
syncing with origin/main...
error: working tree has uncommitted changes outside .braid - commit or stash first
```

## tension
- being cautious makes sense because of auto-commit functionality
- but it's not crazy to have unrelated changes in the working tree and still want to start a new issue

## consider
- should we auto-stash and restore?
- allow a --force flag to proceed anyway?
- smarter detection of whether changes conflict with the issue being started?
- better guidance in the error message?