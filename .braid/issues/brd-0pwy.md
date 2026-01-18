---
schema_version: 7
id: brd-0pwy
title: improve agent init output with agent start instructions
priority: P3
status: done
deps: []
owner: null
created_at: 2025-12-26T12:01:59.561764Z
updated_at: 2025-12-27T08:11:05.503376Z
---

the "To use this agent" instructions after `brd agent init` should mention starting an agent before running `brd next`.

current:

```
To use this agent:
  cd /path/to/worktree
  brd next  # get next issue to work on
```

suggested:

```
To use this agent:
  cd /path/to/worktree
  # start your agent (claude, codex, gemini, etc.)
  brd next  # get next issue to work on
```
