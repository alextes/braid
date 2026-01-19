---
schema_version: 8
id: brd-xtam
title: '''brd ls'' error for dir with git, without brd'
priority: P2
status: open
deps: []
owner: null
created_at: 2026-01-09T19:41:29.693793Z
---

check what error brd ls gives on a dir with a git repo but where brd has not been initialized yet. it should be more informative than 'error: io error: No such file or directory (os error 2)'. this may have been fixed already in the latest version