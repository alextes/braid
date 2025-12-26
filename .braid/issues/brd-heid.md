---
brd: 1
id: brd-heid
title: implement issue schema migration system
priority: P2
status: doing
deps: []
owner: agent-one
created_at: 2025-12-26T16:49:09.344892Z
updated_at: 2025-12-26T18:46:43.200879Z
acceptance:
- brd migrate command exists
- migrations are versioned and run in order
- brd doctor warns about issues needing migration
- loading old schema issues works (migrate in memory)
---

implement infrastructure for migrating issue files when schema changes.

design:
- `brd: N` (later `schema_version: N`) indicates the schema version
- migrations are functions that transform schema N to N+1
- `brd migrate` rewrites all issues to current schema
- when loading, issues are migrated in memory if old schema
- `brd doctor` reports issues that need migration

this enables backwards-incompatible changes to issue format without breaking existing repos.

future migrations that depend on this:
- brd-2wbe: make owner field required
- brd-c48z: rename brd key to schema_version
