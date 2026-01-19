---
schema_version: 8
id: brd-2gj9
title: after init, recommend injecting agent instructions
priority: P1
status: done
deps: []
owner: null
created_at: 2026-01-04T15:54:54.816228Z
started_at: 2026-01-04T16:18:53.993489Z
completed_at: 2026-01-04T16:18:53.993489Z
---

## Problem

After `brd init`, users (especially those setting up for AI agents) don't know they should inject agent instructions into their repo.

## Proposal

After successful init, show a recommendation:

```
braid initialized!

next steps:
  brd agent inject    # add workflow instructions to AGENTS.md
  brd add "my task"   # create your first issue
```

Could also offer to do it interactively:

```
inject agent instructions into AGENTS.md? [Y/n]
```

## Considerations

- `brd agent inject` doesn't exist yet - it's currently done via `brd agent init`
- might need a standalone inject command that doesn't create a worktree
- or rename/refactor the AGENTS.md injection logic