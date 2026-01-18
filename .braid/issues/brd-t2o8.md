---
schema_version: 7
id: brd-t2o8
title: smarter brd start with auto-sync
priority: P1
status: done
type: design
deps: []
owner: null
created_at: 2025-12-28T17:30:56.763758Z
updated_at: 2025-12-28T17:45:40.592713Z
---

make `brd start` handle syncing automatically to prevent stale claims and race conditions.

## modes 1-3 (git-native)

```
brd start [id]
  1. check for local done issues not in main → prompt "ship first?"
  2. git fetch origin main
  3. git rebase origin/main (fail gracefully if dirty)
  4. pick issue (highest priority ready, or specified id)
  5. verify issue still unclaimed after rebase
  6. write claim (status: doing, owner: agent-id)
  7. git add .braid && git commit -m "start: <id>"
  8. git push origin main
  9. if push rejected:
     - pull --rebase
     - check if issue still ours (or unclaimed)
     - retry push (up to 2 retries)
     - after 2 failures, explain and hand back to user
```

## mode 4 (sync branch)

```
brd start [id]
  1. claim issue in shared worktree
  2. commit to sync branch
  3. push to upstream (always - sync branch must have upstream)
  4. handle conflicts similar to above (2 retries)
```

## flags

- `--no-sync` - skip fetch/rebase, trust local state
- `--no-push` - claim locally but don't push yet

## design decisions

- sync branch should always have upstream (losing local state = losing issues)
- push after every claim in sync mode (simple, safe)
- 2 auto-retries on push failure, then hand back with explanation
- prompt about unshipped done issues before claiming new work

## implementation notes

- check for done issues: compare local .braid/issues with origin/main
- "still unclaimed" check: re-read issue file after rebase
- retry logic: exponential backoff not needed (git is fast)