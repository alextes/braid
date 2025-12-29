---
schema_version: 5
id: brd-xiq2
title: rename 'default' to 'git-native' in brd mode command
priority: P3
status: done
deps: []
owner: null
created_at: 2025-12-28T23:03:30.547642Z
updated_at: 2025-12-30T16:54:45.802221Z
---

The `brd mode` command uses the term `default` for the git-native workflow mode. This is vague - `git-native` is more descriptive and consistent with documentation (e.g., workflow-modes.md).

Change references from `default` to `git-native` in:
- CLI subcommand name (`brd mode git-native` instead of `brd mode default`)
- Help text and output messages

Can still note that git-native is the default mode where relevant.