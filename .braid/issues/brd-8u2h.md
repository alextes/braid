---
schema_version: 6
id: brd-8u2h
title: 'design: align workflow-modes.md with init/mode implementation'
priority: P2
status: done
type: design
deps: []
owner: null
created_at: 2026-01-02T10:09:58.536875Z
updated_at: 2026-01-02T10:45:46.829214Z
---

## Problem

`docs/workflow-modes.md` describes 3 modes (git-native, local-sync, external-repo) as distinct choices, but `brd init` uses a 2-question orthogonal flow (issues_branch + auto_sync). The mental models don't align.

Need to reconcile the docs with the actual implementation.
