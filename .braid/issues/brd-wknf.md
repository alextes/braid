---
schema_version: 3
id: brd-wknf
title: implement verbose logging flag
priority: P2
status: todo
deps:
- brd-pwez
owner: null
created_at: 2025-12-27T13:46:26.201963Z
updated_at: 2025-12-27T13:46:26.201963Z
acceptance:
- brd -v ls shows verbose output to stderr
- BRD_VERBOSE=1 brd ls also enables verbose mode
- verbose output includes config discovery and issue loading info
- json output still works cleanly with verbose enabled
---

implement the verbose logging design from brd-pwez.

## tasks

1. add `--verbose/-v` flag to Cli struct with `env = "BRD_VERBOSE"`
2. add `verbose!` macro to lib.rs
3. add verbose logging to config loading and issue discovery
4. test with `brd -v ls` and `BRD_VERBOSE=1 brd ls`
