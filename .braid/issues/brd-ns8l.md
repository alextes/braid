---
schema_version: 8
id: brd-ns8l
title: lightweight version update check
priority: P3
status: open
type: design
deps: []
tags:
- integration
owner: null
created_at: 2026-01-04T17:05:14.799101Z
---

## Problem

Users don't know when a new version of brd is available.

## Proposal

Periodically check for new versions and hint when an update is available.

**Behavior:**
- every ~32 invocations, check if a newer version exists
- if yes, print a one-line hint: `brd v0.7.0 available (you have v0.6.0)`
- `brd doctor` could also include a version check

**Open questions:**
- what's the lightest way to check? options:
  - GitHub releases API (`api.github.com/repos/alextes/braid/releases/latest`)
  - crates.io API
  - HEAD request to a known URL
- how to track invocation count? `.braid/runtime/` or `~/.braid/`?
- should there be a way to disable the check? (env var, config)
- timeout/failure behavior — must not slow down normal usage

**Constraints:**
- must be fast (non-blocking or very short timeout)
- must fail silently (no errors if offline)
- no dependencies on external services for core functionality