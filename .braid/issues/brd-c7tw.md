---
schema_version: 9
id: brd-c7tw
title: 'design: improved git graph visualization in TUI dashboard'
priority: P2
status: open
type: design
deps: []
tags:
- tui
owner: null
created_at: 2026-01-29T13:32:23.107824Z
---

## goal

replace the current simple text-based git graph with a proper visual graph showing branch points and commit relationships.

**current:**
```
main        ●───●───●───●───● (127 total)
agent-one   └─────── c419bf7 ─ 448b8ce ● (+2)
```

**target:**
```
main (127)  ●──●──●──●──●──●──●──●  ← HEAD
                 ╰─ agent-one (+2)  448b8ce feat: add graph
            ╰───── agent-two (+5)   c3d9f12 wip: fixing tests
```

the fork position on the main line reflects where each branch actually diverged.

## data structure changes

add fields to `BranchGraphInfo` in `src/tui/app.rs`:
- `fork_point: Option<String>` - SHA of merge-base with main
- `main_commits_since_fork: usize` - commits on main since fork

## data loading changes

in `load_git_graph()`, for each agent branch:
- call `merge_base()` to find fork point
- call `commit_count(fork, main)` to get main commits since fork

## rendering algorithm

1. main branch line: commit dots (●) connected with lines, newest on right, "← HEAD" marker
2. agent branches: sort by `main_commits_since_fork` (older forks = more indent)
3. fork position: map `main_commits_since_fork` to horizontal position, use `╰─` connector
4. branch info: name, ahead count (+N), latest commit SHA and message

## edge cases

- many branches (>5): limit display, show "(+N more)"
- same fork point: stack vertically, use `├─` for middle branches
- fork older than visible: show at far left with `···` indicator
- branch at 0 ahead: green (+0) or ✓ to show in sync
- long branch names: truncate with `…`

## unicode characters

```
─  horizontal line     ╰  corner (fork point)
●  commit marker       ├  T-junction (multiple at same point)
←  HEAD indicator
```

## critical files

- `src/tui/app.rs` - BranchGraphInfo struct, load_git_graph()
- `src/tui/ui.rs` - draw_git_graph()
- `src/git.rs` - uses existing merge_base() and commit_count()