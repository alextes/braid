---
schema_version: 4
id: brd-s5bg
title: add unit tests for init command
priority: P2
status: doing
deps: []
tags:
- testing
owner: agent-three
created_at: 2025-12-28T08:49:24.962784Z
updated_at: 2025-12-28T19:39:14.590517Z
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