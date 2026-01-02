---
schema_version: 6
id: brd-rijv
title: gh repo create integration for external-repo setup
priority: P3
status: todo
deps:
- brd-ily8
owner: null
created_at: 2025-12-30T15:40:40.861126Z
updated_at: 2025-12-30T15:40:40.861126Z
---

If gh CLI is installed, offer to create GitHub repo for external issues:

`brd mode external-repo --init <path> --github`

1. Create local repo and init braid
2. Run `gh repo create` (prompt for public/private)
3. Push initial commit
4. Set issues_repo in code repo config

Depends on --init feature (brd-ily8).