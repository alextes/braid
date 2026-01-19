---
schema_version: 8
id: brd-mgz8
title: improve schema mismatch error handling for agents
priority: P1
status: done
type: design
deps: []
owner: null
created_at: 2026-01-01T15:06:54.517194Z
started_at: 2026-01-01T15:39:00.350614Z
completed_at: 2026-01-01T15:39:00.350614Z
---

## problem

an agent ran into a schema version mismatch error:

```
error: issues worktree uses schema v6, but this brd only supports up to v5. please upgrade brd.
```

instead of stopping and consulting the human, the agent decided on its own to **downgrade the schema version** in the issues worktree config from v6 to v5. this is dangerous because:

1. the agent would then create/modify issues at schema v5
2. when trying to push to the issues branch (which is at v6), this could cause conflicts or data loss
3. other agents/humans working at v6 would be confused by v5 issues appearing

## root cause

the error message says "please upgrade brd" but doesn't explain:
- **what to do in a dogfooding repo** where `cargo install --path .` might not help because the agent is on a stale branch
- **that rebasing on main is often the fix** when working in agent worktrees
- **that manual schema downgrades are never acceptable**

for normal users, upgrading brd system-wide resolves everything. but in this repo (braid itself), the situation is confusing because:
- the binary being used might be from a stale branch
- `cargo build --release` builds from the current (stale) branch
- even `cargo install --path .` installs the stale version

## desired behavior

when an agent encounters a schema mismatch:

1. **never attempt to work around it** - no manual edits to config files
2. **try rebasing first** - `git fetch origin main && git rebase origin/main && cargo build --release`
3. **if rebase doesn't fix it, stop and ask the human** - there may be an unreleased schema bump

## possible solutions

### option 1: better error message

improve the error to include context-aware guidance:

```
error: issues worktree uses schema v6, but this brd only supports up to v5

for normal users:
  upgrade brd: cargo install braid-cli

for agent worktrees (detected .braid/agent.toml):
  1. rebase on main: git fetch origin main && git rebase origin/main
  2. rebuild: cargo build --release
  3. if still failing, consult human - there may be an unreleased schema change

NEVER manually edit schema_version in config files.
```

### option 2: add `brd doctor --fix-schema`

a safe command that:
- detects if you're behind main
- offers to rebase and rebuild
- refuses to downgrade schemas

### option 3: AGENTS.md guidance

add explicit guidance to AGENTS.md about schema mismatch handling:
- always rebase first
- never manually edit schema versions
- ask human if rebase doesn't help

## questions

1. should we detect agent worktrees and show different error messages?
2. should `brd doctor` have a schema mismatch recovery flow?
3. how do we make it crystal clear that manual schema changes are forbidden?
