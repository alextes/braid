---
schema_version: 3
id: brd-pwez
title: add verbose logging flag for debugging
priority: P2
status: todo
deps: []
labels:
- design
owner: null
created_at: 2025-12-26T20:35:50.61416Z
updated_at: 2025-12-26T20:35:50.61416Z
---

## context

when debugging brd behavior, it would be helpful to have verbose output showing what the tool is doing internally.

## design questions

- **flag design**: `--verbose` / `-v`? or `--debug`? support multiple levels (`-vv`)?
- **output destination**: stderr (keeps stdout clean for piping) or stdout?
- **what to log**: file operations, git commands, issue parsing, config loading?
- **implementation**: use `tracing` crate? simple eprintln? env var (`BRD_VERBOSE=1`)?
- **integration with --json**: should verbose mode work alongside JSON output?
