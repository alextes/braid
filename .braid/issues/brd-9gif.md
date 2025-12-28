---
schema_version: 4
id: brd-9gif
title: warn when agent claims new issue while another is still doing
priority: P1
status: done
deps: []
owner: null
created_at: 2025-12-28T23:15:19.305303Z
updated_at: 2025-12-28T23:21:54.492506Z
---

when running `brd start`, warn if the current agent already has an issue claimed (status: doing) that hasn't been completed.

this helps catch:
- forgotten issues that were never finished
- agents picking up new work without closing out previous work

should print a warning but not block the claim (agent may be intentionally switching).