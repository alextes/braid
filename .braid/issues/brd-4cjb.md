---
schema_version: 3
id: brd-4cjb
title: implement full-text search for issues
priority: P3
status: todo
type: design
deps: []
owner: null
created_at: 2025-12-27T08:14:33.487622Z
updated_at: 2025-12-27T08:14:33.487622Z
acceptance:
- define search scope (title, body, frontmatter fields)
- decide on search backend (grep, built-in, sqlite fts)
- design CLI interface (brd search <query> vs brd ls --search)
- consider filtering by status, priority, labels in search
---

currently brd search just prints instructions to use grep/rg. a proper search feature would be nice for larger issue sets. low priority since grep works fine.