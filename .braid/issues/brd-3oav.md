---
schema_version: 8
id: brd-3oav
title: simplify brd mode - separate config from migration
priority: P3
status: done
type: design
deps: []
owner: null
created_at: 2025-12-29T17:22:49.63266Z
started_at: 2025-12-30T11:39:14.836764Z
completed_at: 2025-12-30T11:39:14.836764Z
---

`brd mode` currently does several things:
1. Changes config (the actual mode switch)
2. Creates branches if needed
3. Migrates issues between locations
4. Handles git commits on both branches

These could be separated for clarity.

## Current state

`brd mode local-sync`:
- Creates sync branch (if needed)
- Creates shared worktree
- Moves issues to sync branch
- Commits changes
- Updates config

## Potential simplification

**Mode as pure config:**
```bash
brd config set mode local-sync  # just changes config
```

**Separate migration command:**
```bash
brd migrate-issues --to-branch braid-issues  # moves issues
```

**Or keep combined but clearer:**
- `brd mode` shows current mode
- `brd mode switch local-sync` does the full switch with migration
- Config change is implementation detail

## Questions

- Is the current UX actually confusing?
- Would separation add unnecessary complexity?
- What's the most intuitive mental model?

## Output

Decision on whether to refactor mode command structure.