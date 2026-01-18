---
schema_version: 7
id: brd-ghi5
title: 'clarify --dep help text: blocked by vs blocking'
priority: P2
status: done
deps: []
owner: null
created_at: 2026-01-05T21:28:54.830725Z
updated_at: 2026-01-17T12:27:38.158929Z
---

## Problem

In `brd add --help`, the `--dep` flag says:

> add dependency on another issue (can be repeated)

This is ambiguous. Does it mean:
- The new issue is **blocked by** the referenced issue? (correct)
- The new issue **blocks** the referenced issue?

## Fix

Change help text to be explicit:

```
--dep <DEP>  this issue is blocked by DEP (can be repeated)
```

Or:

```
--dep <DEP>  add blocker (this issue waits for DEP to complete)
```