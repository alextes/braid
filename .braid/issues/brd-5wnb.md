---
schema_version: 6
id: brd-5wnb
title: Extend doctor to check CLAUDE.md and CLAUDE.local.md
priority: P2
status: done
deps: []
owner: null
created_at: 2026-01-12T13:15:55.218723Z
updated_at: 2026-01-12T14:16:16.639764Z
acceptance:
- doctor checks for braid instruction block in AGENTS.md
- doctor checks for braid instruction block in CLAUDE.md
- doctor checks for braid instruction block in CLAUDE.local.md
- Pass if the block is found in any of the three files
- Provide clear feedback about which file(s) contain the block
---

The doctor command currently only checks whether the braid instruction block has been injected in AGENTS.md. It should also check CLAUDE.md and CLAUDE.local.md since these are valid locations for Claude Code instructions.