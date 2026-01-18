---
schema_version: 7
id: brd-wx1e
title: add cancelled status for abandoned issues
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-27T13:45:21.728941Z
updated_at: 2025-12-27T16:10:10.987735Z
acceptance:
- 'status: cancelled is valid in frontmatter'
- cancelled issues shown dim like done in brd ls
- brd cancel <id> command to mark issue cancelled
---

for issues you don't want to do anymore but want to keep for historical reference.