---
schema_version: 8
id: brd-ily8
title: brd mode external-repo --init to create and initialize external repo
priority: P3
status: open
deps: []
tags:
- cli
owner: null
created_at: 2025-12-30T15:40:32.253103Z
---

Convenience feature: `brd mode external-repo --init <path>`

1. Create directory if not exists
2. Run git init
3. Run brd init (inherit prefix from code repo?)
4. Create initial commit
5. Set issues_repo in code repo config

Makes setup a one-liner instead of multiple steps.