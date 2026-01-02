---
schema_version: 6
id: brd-mc5q
title: update outdated dependencies
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-26T18:46:47.133569Z
updated_at: 2025-12-26T18:54:43.752163Z
---

Several dependencies have newer versions available:

- crossterm: 0.28.1 → 0.29.0
- rand: 0.8.5 → 0.9.2
- ratatui: 0.29.0 → 0.30.0
- thiserror: 1.0.69 → 2.0.17
- toml: 0.8.23 → 0.9.10
- unicode-width: 0.2.0 → 0.2.2

Tasks:
- [ ] Review changelog/breaking changes for each dependency
- [ ] Update Cargo.toml with new versions
- [ ] Run tests to verify compatibility
- [ ] Fix any breaking changes