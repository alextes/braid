---
schema_version: 6
id: brd-ao00
title: add unit tests for rm command
priority: P3
status: done
deps: []
tags:
- testing
owner: null
created_at: 2025-12-28T08:49:45.127324Z
updated_at: 2025-12-28T20:29:57.902627Z
---

rm command deletes issues but has no unit tests.

## logic to test
- file deletion
- safety check (blocks deletion of Doing issues)
- force flag overrides safety check
- dual-write deletion

## test cases
- delete todo issue
- delete done issue
- delete doing issue (should fail)
- delete doing with --force
- delete non-existent issue