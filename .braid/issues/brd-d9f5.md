---
schema_version: 4
id: brd-d9f5
title: add auto-commit helpers
priority: P3
status: skip
deps: []
owner: null
created_at: 2025-12-25T21:46:28.161703Z
updated_at: 2025-12-27T21:44:40.334274Z
acceptance:
- brd commit command to commit .braid changes
- optional auto-commit on issue state changes
---

## Implementation Notes (2025-12-27)

Prototyped but reverted. Key learnings for future implementation:

### `brd commit` command
- stages `.braid` with `git add .braid`
- checks for staged changes with `git diff --cached --quiet .braid`
- **must use `git commit -m <msg> -- .braid`** to only commit .braid (not other staged files)
- auto-generates messages like `chore(braid): add 1, update 2 issues (brd-xxxx, brd-yyyy)`

### auto-commit config
- add `auto_commit: bool` to Config with `#[serde(default)]` for backwards compat
- call `maybe_auto_commit()` from main.rs after state-changing commands succeed
- state-changing commands: `add`, `done`, `start`, `skip`, `rm`, `dep add/rm`

### concerns
- auto-commit feels too magical / surprising for users
- complicates mental model of when commits happen
- manual `brd commit` might be sufficient if needed at all
