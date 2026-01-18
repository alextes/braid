---
schema_version: 7
id: brd-pqjd
title: revise brd init interactive prompt with orthogonal questions
priority: P2
status: done
deps:
- brd-zeqv
owner: null
created_at: 2025-12-30T17:08:56.470184Z
updated_at: 2025-12-30T20:27:22.502855Z
---

Replace the mode-based prompt with two orthogonal questions.

## Key Insight

Coordination and storage are ORTHOGONAL choices:
1. **Coordination** (`auto_pull`/`auto_push`) - git-based sync works for ALL agents
2. **Storage** (`issues_branch`) - purely about where issues live

Neither is required for multi-agent. Both are personal preferences.

## New (2-question flow)
```
Initializing braid in /path/to/repo...

Q1: Auto-sync issues with git?
    (pull before start, push after done)
      1. Yes - stay in sync, prevents duplicate claims [default]
      2. No - manual sync only (brd sync)

    → Auto-sync enabled.
    Trade-off: Network dependency, more commits.

Q2: Where should issues live?
      1. With code - issues in .braid/ on each branch [default]
      2. Separate branch - single source of truth, cleaner history

    → Issues stored with code.

Creating .braid/ with:
  auto_pull = true
  auto_push = true
```

## Config Mapping

| Auto-sync | Storage | auto_pull | auto_push | issues_branch |
|-----------|---------|-----------|-----------|---------------|
| Yes | With code | true | true | (none) |
| Yes | Separate | true | true | "braid-issues" |
| No | With code | false | false | (none) |
| No | Separate | false | false | "braid-issues" |

## Implementation

Updated `src/commands/init.rs`:
1. `determine_workflow_config()` uses 2-question flow
2. Q1: Auto-sync (default: yes) → sets auto_pull/auto_push
3. Q2: Storage (default: with code) → sets issues_branch
4. Both questions independent, no conditional skipping

## Notes
- `-y`/`--non-interactive` uses defaults: no issues_branch, auto_pull=true, auto_push=true
- `--issues-branch` flag still works as explicit override
- External repo deferred to `brd mode external-repo` command