---
schema_version: 8
id: brd-lgr7
title: allow custom file path for AGENTS block injection
priority: P3
status: done
deps: []
owner: null
created_at: 2026-01-02T15:27:06.520373Z
started_at: 2026-01-12T13:17:18.413539Z
completed_at: 2026-01-12T13:17:18.413539Z
---

## Problem

Currently `brd agent init` always injects the braid workflow block into `AGENTS.md`. Some projects may want this in a different file (e.g., `CLAUDE.md`, `CONTRIBUTING.md`, `.github/AGENTS.md`).

## Proposed Solution

Add an optional argument to specify the target file:

```bash
brd agent init <name>                    # injects into AGENTS.md (default)
brd agent init <name> --agents-file CLAUDE.md  # injects into CLAUDE.md
```

## Implementation Notes

- Default remains `AGENTS.md` for backwards compatibility
- Could also be a config option in `.braid/config.toml` for project-wide preference
- Block detection/update logic should work on any file path