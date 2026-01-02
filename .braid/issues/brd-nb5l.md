---
schema_version: 6
id: brd-nb5l
title: move helper functions to appropriate modules
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-26T08:26:05.824842Z
updated_at: 2025-12-26T18:51:24.765013Z
---

move helper functions out of main.rs to their appropriate modules:

- `git_rev_parse` → `repo.rs` (duplicates existing logic there)
- `load_all_issues` → `issue.rs` or new `store.rs`
- `resolve_issue_id` → `issue.rs`
- `generate_issue_id` → `issue.rs` or `config.rs`
- `issue_to_json` → `issue.rs` (serialization belongs with the type)
