---
schema_version: 6
id: brd-lzet
title: update brd start to use config.auto_pull instead of mode check
priority: P2
status: done
deps:
- brd-zeqv
owner: null
created_at: 2025-12-30T17:08:56.592809Z
updated_at: 2025-12-30T19:49:42.659228Z
---

Change `brd start` to check `config.auto_pull` instead of inferring sync behavior from mode.

## Current behavior
- Git-native mode: always syncs (fetch+rebase) before start
- Local-sync mode: never syncs before start

## New behavior
- Check `config.auto_pull`:
  - If `true`: fetch+rebase before picking issue
  - If `false`: skip sync, use local state

## Implementation

In `src/commands/start.rs`:
1. Load config and check `config.auto_pull` instead of `!is_sync_mode`
2. The `--no-sync` flag should still work as override
3. Remove mode-based sync logic, replace with config-based

## Notes
- This decouples sync behavior from branch choice
- Users on separate branch can still auto-sync with remote