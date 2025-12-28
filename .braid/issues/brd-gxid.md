---
schema_version: 4
id: brd-gxid
title: add unit tests for error module
priority: P3
status: doing
deps: []
tags:
- testing
owner: agent-three
created_at: 2025-12-28T08:57:40.989672Z
updated_at: 2025-12-28T21:10:50.25078Z
---

error module defines error types but has no tests for Display/formatting.

## logic to test
- error message formatting
- error code generation (for exit codes)
- From implementations

## test cases
- each error variant formats correctly
- error messages are user-friendly