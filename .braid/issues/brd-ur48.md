---
schema_version: 2
id: brd-ur48
title: implement smart brd ls defaults
priority: P2
status: done
deps:
- brd-xho0
created_at: 2025-12-26T19:06:49.983355Z
updated_at: 2025-12-26T19:23:05.316873Z
acceptance:
- todo/doing shown before done
- max 10 done issues shown by default
- add --all flag to show everything
---

implement the design from brd-xho0:

1. sort todo/doing before done issues
2. limit done issues to 10 most recent (by updated_at)
3. add `--all` flag to bypass limits and show everything

sorting within each group remains: priority → created_at → id