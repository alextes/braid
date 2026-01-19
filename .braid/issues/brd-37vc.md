---
schema_version: 8
id: brd-37vc
title: refactor cmd_add to fix clippy::too_many_arguments
priority: P3
status: done
deps: []
owner: null
created_at: 2025-12-26T19:03:19.008249Z
started_at: 2025-12-26T21:54:36.241215Z
completed_at: 2025-12-26T21:54:36.241215Z
acceptance:
- cmd_add takes 7 or fewer arguments
- 'remove #[allow(clippy::too_many_arguments)]'
---

cmd_add currently takes 8 arguments and uses #[allow(clippy::too_many_arguments)].

refactor to use a struct like `AddOptions` or pass the parsed `Command::Add` variant directly.