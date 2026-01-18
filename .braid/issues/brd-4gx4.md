---
schema_version: 7
id: brd-4gx4
title: set up cargo-dist for automated releases
priority: P2
status: done
deps: []
owner: null
created_at: 2025-12-27T08:47:34.679392Z
updated_at: 2025-12-27T15:37:50.460249Z
acceptance:
- install cargo-dist
- run cargo dist init
- configure for linux/macos/windows
- test release workflow
---

cargo-dist automates building and publishing release binaries to github releases. requires interactive setup.