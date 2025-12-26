---
brd: 1
id: brd-vp1p
title: extend doctor to print passing checks with descriptions
priority: P2
status: done
deps: []
created_at: 2025-12-26T07:42:40.025085Z
updated_at: 2025-12-26T18:35:38.160091Z
---

currently `brd doctor` only prints "✓ All checks passed" on success. extend it to print each individual check as it passes, with a short description of what was validated.

example output:
```
✓ .braid directory exists
✓ config.toml is valid
✓ all issue files parse correctly
✓ no missing dependencies
✓ no dependency cycles
```

for json mode, include a `checks` array with each check's name, status, and description.
