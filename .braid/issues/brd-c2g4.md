---
schema_version: 7
id: brd-c2g4
title: 'brd sync: detect upstream, support local-only'
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-28T16:21:42.963077Z
updated_at: 2025-12-28T20:23:41.323509Z
---

update `brd sync` to work in local-only mode when no upstream exists.

## current behavior

always tries to fetch/push to origin, fails if no remote.

## new behavior

1. check if sync branch has upstream (`git rev-parse --abbrev-ref @{u}`)
2. if no upstream: just commit locally, skip fetch/push
3. if upstream exists: current behavior (fetch, rebase, commit, push)
4. add `--push` flag to set upstream and push (first-time remote setup)

## implementation

```rust
fn has_upstream(branch: &str, cwd: &Path) -> bool {
    git_output(&["rev-parse", "--abbrev-ref", &format!("{}@{{u}}", branch)], cwd)
        .is_ok()
}
```

## files

- `src/commands/sync.rs`
- `src/cli.rs` (add --push flag to Sync command)

## parent

part of brd-jpux (workflow modes design)