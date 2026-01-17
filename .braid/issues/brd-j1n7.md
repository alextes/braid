---
schema_version: 6
id: brd-j1n7
title: add brd set command for quick field updates
priority: P2
status: done
deps: []
owner: null
created_at: 2026-01-04T18:55:43.585849Z
updated_at: 2026-01-04T21:03:02.338284Z
---

## Problem

Changing issue priority requires manually editing the markdown file or using `brd edit` which opens $EDITOR.

## Proposal

Add a `brd set` command for quick field updates:

```bash
brd set <id> priority P2
brd set <id> priority P1 P2 P3   # multiple issues
brd set q62j cakw hv5k -p P2    # alternative syntax
```

Could also support other fields:
```bash
brd set <id> tag +bug           # add tag
brd set <id> tag -wontfix       # remove tag
```

## Alternatives

- `brd edit <id> --priority P2` - extend edit command
- `brd priority <id> P2` - dedicated priority command