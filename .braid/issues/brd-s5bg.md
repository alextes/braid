---
schema_version: 6
id: brd-s5bg
title: add unit tests for init command
priority: P2
status: done
deps: []
tags:
- testing
owner: null
created_at: 2025-12-28T08:49:24.962784Z
updated_at: 2025-12-28T19:43:54.075002Z
---

init command creates repo structure but has no unit tests.

## logic to test
- .braid directory creation
- issues subdirectory creation
- config.toml generation with derived prefix
- .gitignore creation
- agent.toml creation with $USER fallback
- control_root file creation
- idempotency (re-init doesn't overwrite)

## test cases
- fresh init
- re-init existing repo
- missing $USER env var
- JSON output format