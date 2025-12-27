---
schema_version: 2
id: brd-2wbe
title: make owner field required in issue frontmatter
priority: P2
status: todo
deps:
- brd-heid
created_at: 2025-12-26T16:44:10.840206Z
updated_at: 2025-12-26T16:50:17.79961Z
acceptance:
- owner field is always present in issue frontmatter (null if unset)
- migration adds owner field to old issues
---

currently the `owner` field is optional in issue frontmatter. this makes parsing more complex since we need to handle both present and absent cases.

make `owner` a required field (value can be null). add migration to ensure all existing issues have `owner: null` if not set.

depends on brd-heid for migration infrastructure.
