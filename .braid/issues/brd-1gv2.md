---
schema_version: 8
id: brd-1gv2
title: 'improve init: clearer agent instructions injection flow'
priority: P1
status: done
deps: []
tags:
- ux
- cli
owner: null
created_at: 2026-01-26T13:22:00.958403Z
started_at: 2026-01-26T13:59:31.174048Z
completed_at: 2026-01-26T14:03:52.024255Z
---

## problem

after `brd init`, the suggestion to run `brd agent inject` is easy to miss. users (especially AI agents) may not realize they need to inject the braid workflow instructions.

## proposed changes

1. **offer injection during init**: after setup completes, prompt:
   ```
   inject braid instructions into AGENTS.md? [Y/n]:
   ```
   
2. **allow custom target**: if user declines or wants a different file:
   ```
   inject into different file? (enter path or leave empty to skip):
   ```

3. **post-injection reminder**: after successful injection, print:
   ```
   note: if you have running agents, ask them to re-read AGENTS.md
   to pick up the new braid workflow instructions.
   ```

## acceptance criteria

- [ ] init prompts to inject instructions (default: yes)
- [ ] supports custom injection target file
- [ ] reminds about running agents after injection
- [ ] works with -y flag (auto-inject to AGENTS.md)
- [ ] works in non-interactive mode (auto-inject to AGENTS.md)