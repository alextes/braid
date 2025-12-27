---
schema_version: 2
id: brd-9luu
title: add -p shorthand for --priority flag
priority: P3
status: done
deps: []
created_at: 2025-12-26T08:53:15.981035Z
updated_at: 2025-12-26T08:56:29.00914Z
---

`brd ls -p P1` should work as shorthand for `brd ls --priority P1`.

in cli.rs, add `short = 'p'` to the priority flag on Ls command.
