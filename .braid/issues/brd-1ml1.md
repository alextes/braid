---
schema_version: 6
id: brd-1ml1
title: 'design: behavior when brd init called on already-initialized repo'
priority: P2
status: todo
type: design
deps: []
owner: null
created_at: 2026-01-02T13:27:04.292405Z
updated_at: 2026-01-02T13:27:04.292405Z
---

## Problem

What should `brd init` do if braid is already initialized in the repo?

Currently unclear - need to define the expected behavior.

## Options

### Option A: Error with clear message
```
$ brd init
error: braid is already initialized in this repo
hint: use `brd mode` to change configuration
hint: use `brd doctor` to check repo health
```

### Option B: Offer to reconfigure
```
$ brd init
braid is already initialized. reconfigure? [y/N]
```

### Option C: Silent success if config matches
Only error if trying to change configuration.

## Recommendation

Option A seems cleanest - `init` means "initialize", not "reconfigure". Users have `brd mode` for changes.

## Implementation

- Check for existing .braid/config.toml at start of init
- If exists, print error and suggest alternatives
- Exit with non-zero status