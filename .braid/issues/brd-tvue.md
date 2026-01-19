---
schema_version: 8
id: brd-tvue
title: mode-aware AGENTS.md injection
priority: P3
status: done
deps: []
owner: null
created_at: 2025-12-28T16:21:56.615665Z
started_at: 2025-12-28T21:15:21.058183Z
completed_at: 2025-12-28T21:15:21.058183Z
---

update `brd agents inject` to generate mode-aware content.

## current

static block with generic workflow info.

## new structure

```markdown
<!-- braid:agents:start v2 -->

## braid workflow

[STATIC: brd commands - ls, start, done, etc.]

## syncing issues

[DYNAMIC: depends on current mode]

<!-- braid:agents:end -->
```

## dynamic section by mode

**git-native:**
```
issues sync via git. after changing issue state:
1. commit: `git add .braid && git commit -m "..."`
2. push to main or merge your branch
3. others rebase to get updates
```

**local-sync:**
```
issues sync via shared worktree. changes are visible to all local agents instantly.
- `brd sync` - commit and optionally push to remote
- no need to manually commit issue changes
```

## implementation

1. bump AGENTS_BLOCK_VERSION to 2
2. add mode detection in injection
3. generate appropriate sync section
4. update `check_agents_block` to handle v1→v2 upgrade prompt

## files

- `src/commands/agents.rs`

## parent

part of brd-jpux (workflow modes design)