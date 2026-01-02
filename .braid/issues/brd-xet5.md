---
schema_version: 6
id: brd-xet5
title: design smoother workflow mode management and onboarding
priority: P2
status: done
type: design
deps:
- brd-jpux
owner: null
created_at: 2025-12-28T21:06:17.924989Z
updated_at: 2025-12-28T23:17:08.281406Z
---

the workflow modes from brd-jpux (solo, branch-per-feature, multi-human/remote, local multi-agent) are powerful but the UX for discovering, choosing, and switching between them could be smoother.

## problems to solve

1. **discovery** - how do new users learn which mode fits their use case?
2. **onboarding** - guided setup for each mode (especially sync-branch mode)
3. **switching** - transitioning between modes as needs evolve
4. **visibility** - making current mode clear without running `brd mode`

## areas to explore

- interactive `brd init` that asks about workflow and configures appropriately
- `brd mode` wizard for switching modes with migration help
- better error messages that suggest mode changes when hitting friction
- documentation/help text improvements

## output

design decisions and implementation issues for improving the modes UX.