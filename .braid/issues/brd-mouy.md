---
schema_version: 7
id: brd-mouy
title: prevent dependency cycles and handle design issue closure transfer
priority: P1
status: done
deps: []
owner: null
created_at: 2025-12-30T15:17:13.951418Z
updated_at: 2025-12-30T15:52:20.869597Z
acceptance:
- brd dep add rejects self deps and dependency cycles with a clear error
- brd done for design issues transfers deps to implementation issues and replaces dependents
- cli has tests covering cycle prevention and design close transfer
---
