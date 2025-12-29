---
schema_version: 4
id: brd-ft2e
title: implement brd mode issues-branch command
priority: P2
status: skip
deps:
- brd-6lme
- brd-8vcu
owner: null
created_at: 2025-12-28T23:59:41.070542Z
updated_at: 2025-12-30T15:54:10.528483Z
---

Add `brd mode issues-branch [branch-name]` subcommand to switch to issues-branch mode.

## Behavior

```
$ brd mode issues-branch

Setting up issues-branch mode...

This will:
  • Create orphan branch 'braid-issues' for issue storage
  • Move 12 issues from .braid/issues/ to that branch
  • Configure brd to read/write issues on that branch

Continue? [Y/n]:
```

## Implementation

1. Create orphan branch: `git checkout --orphan {branch}`
2. Copy full `.braid/` directory to the branch
3. Commit: `git commit -m "chore(braid): initialize issues branch"`
4. Return to original branch
5. Update config with `issues_branch = Some(branch_name)`
6. Remove `.braid/issues/` from main (optional? or keep as cache?)

## Acceptance criteria
- [ ] Creates orphan branch with .braid/ directory
- [ ] Updates config correctly
- [ ] Confirmation prompt with -y to skip
- [ ] Shows helpful error if already in issues-branch mode