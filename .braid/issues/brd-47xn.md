---
schema_version: 8
id: brd-47xn
title: implement brd mode external-repo command
priority: P2
status: done
deps:
- brd-6zmi
owner: null
created_at: 2025-12-30T15:40:20.443884Z
started_at: 2025-12-30T16:19:27.037549Z
completed_at: 2025-12-30T16:19:27.037549Z
---

Add `brd mode external-repo <path>` subcommand:

1. Validate external repo exists and has .braid/config.toml
2. Set issues_repo in local config
3. Commit config change
4. Print success message with usage hints

Also add to `brd mode` (no args) output to show current external repo if set.