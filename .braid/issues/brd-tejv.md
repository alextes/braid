---
schema_version: 3
id: brd-tejv
title: deduplicate issue sorting logic
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-26T08:26:05.920903Z
updated_at: 2025-12-26T18:44:45.841225Z
---

the sorting logic (priority → created_at → id) is duplicated:

1. `main.rs` cmd_ls (lines 240-245)
2. `graph.rs` get_ready_issues (lines 118-123)

extract to a single `Issue::cmp_priority` or `sort_issues()` function in `issue.rs` or `graph.rs`.
