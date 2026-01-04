---
schema_version: 6
id: brd-0fyx
title: hint about --all flag when listing done issues
priority: P2
status: todo
deps: []
owner: null
created_at: 2026-01-04T17:02:41.69323Z
updated_at: 2026-01-04T17:02:41.69323Z
---

## Problem

When running `brd ls --status done`, results are limited but there's no indication that more exist or how to see them.

## Proposal

Show a hint when done issues are truncated:

```
brd-abc1  done  fix the bug
brd-def2  done  add feature
...
showing 15 of 47 done issues (use --all to show all)
```

Similar to how `brd ls` shows "+N more" for todo issues.