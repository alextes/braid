---
brd: 1
id: brd-2wbe
title: make owner field required, add issue format migration system
priority: P2
status: todo
deps: []
created_at: 2025-12-26T16:44:10.840206Z
updated_at: 2025-12-26T16:44:10.840206Z
acceptance:
- owner field is always present in issue frontmatter (null if unset)
- brd migrate command exists to upgrade old issues
- migration runs automatically on brd init or doctor
---

currently the `owner` field is optional in issue frontmatter. this makes parsing more complex since we need to handle both present and absent cases.

proposal:
1. make `owner` a required field (value can be null)
2. add a migration system for format changes
3. bump schema version when format changes

migration system design:
- `brd: 1` in frontmatter is the schema version
- when loading an issue with old schema, migrate in memory
- `brd migrate` command rewrites all issues to current schema
- `brd doctor` warns about old schema versions

this sets up infrastructure for future format changes without breaking existing repos.

first migration: ensure all issues have `owner: null` if not set.
