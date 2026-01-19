---
schema_version: 8
id: brd-ww8p
title: implement brd v0.1 CLI
priority: P0
status: done
deps: []
owner: null
created_at: 2025-12-25T22:06:30.190214Z
started_at: 2025-12-25T22:06:44.997826Z
completed_at: 2025-12-25T22:06:44.997826Z
---

implement the core brd CLI as specified in plans/v0.1.md.

## modules

- **cli**: clap-based command definitions with subcommands and flags
- **config**: toml config with id_prefix and id_len settings
- **error**: error types with exit codes per spec section 13
- **issue**: yaml frontmatter parsing, priority/status enums, markdown body
- **graph**: dependency state computation, cycle detection, ready issue sorting
- **claims**: claim struct and agent ID resolution (commands stubbed)
- **lock**: file-based locking via fs2 for multi-agent safety
- **repo**: git worktree discovery, control root resolution

## commands

- `init`: initialize .braid/ directory and config
- `add`: create new issue with title, priority, deps, acceptance criteria
- `ls`: list issues with filtering by status/priority/ready/blocked
- `show`: display full issue details
- `ready`: list issues ready to work on
- `next`: get highest priority ready issue
- `dep add/rm`: manage issue dependencies
- `start`: transition issue to doing status
- `done`: transition issue to done status
- `doctor`: validate issue graph integrity
- `completions`: generate shell completions

## features

- partial ID resolution (e.g. `ww8p` matches `brd-ww8p`)
- json output mode via `--json` flag for scripting
- deterministic sorting by priority, created_at, then id
- cycle detection in dependency graph
- missing dependency detection
