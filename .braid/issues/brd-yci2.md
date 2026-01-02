---
schema_version: 6
id: brd-yci2
title: design better PR-based workflow support
priority: P2
status: done
type: design
deps: []
owner: null
created_at: 2025-12-29T17:22:49.510666Z
updated_at: 2025-12-29T23:24:19.017794Z
---

Currently agents need to figure out from AGENTS.md how to work with PRs. We could provide helpful commands that guide agents through PR workflows.

## Problem

- Agents start issues on main/sync-branch but work on feature branches
- Closing an issue while on a feature branch is awkward
- No guidance on when to mark done (before or after PR merge?)
- Race conditions between issue state and PR state

## Ideas to explore

1. **PR-aware start/done**
   - `brd start` on a feature branch could note the branch
   - `brd done` could handle being on a different branch than where issue lives

2. **Explicit PR commands**
   - `brd pr start <issue>` - create feature branch, link to issue
   - `brd pr done <issue>` - mark done from feature branch, handle sync

3. **Mode variant**
   - `brd mode pr-based` or config option for PR workflow
   - Adjusts behavior of start/done for PR context

4. **Better AGENTS.md instructions**
   - Mode-specific workflow guidance
   - Clear steps for PR-based work

## analysis

### current state (what already works)

the commands actually work well:

1. `brd agent branch <issue>` - claims issue (pushes to main), creates `pr/<agent>/<issue>` branch
2. work on feature branch, commit as usual
3. `brd done <issue>` - marks done
   - local-sync mode: writes to shared worktree (works from any branch)
   - git-native mode: writes to feature branch (merges with PR)
4. `brd agent pr` - pushes branch, creates PR

when PR merges in git-native mode, the done state merges too. correct behavior.

### actual problems

**1. documentation gap**
agents don't know:
- the expected sequence of commands
- that done state flows with the PR
- how to handle edge cases

**2. PR rejection recovery**
- agent marks done, creates PR
- PR is rejected (needs changes)
- issue is "done" on branch but work isn't done
- need to re-open the issue (currently manual edit)

**3. no PR tracking**
- once PR is created, no link back to issue
- can't easily find PR for an issue or vice versa

### potential improvements

**A. `brd reopen <issue>` command**
- sets status back to doing
- useful when PR is rejected or work needs to continue

**B. combine done + pr into single command**
- `brd agent pr` could auto-mark done
- or `brd agent pr --done` flag
- makes "done and create PR" atomic

**C. PR URL tracking**
- add `pr_url` field to issue frontmatter
- `brd agent pr` writes this automatically
- `brd show` displays it
- helps agents and humans track PR status

**D. workflow validation (covered in brd-fzk1)**
- `brd done` warns if on main but workflow=pr
- helps catch mistakes

## recommendation

1. **documentation first** - update AGENTS.md to clearly explain PR workflow (brd-fmiv covers this)

2. **add `brd reopen`** - simple command for PR rejection recovery

3. **add PR URL tracking** - `brd agent pr` stores URL in issue

4. **consider combining done+pr** - reduces steps, less chance of forgetting one

skip the "auto-done on pr" for now - explicit is better. agents should mark done then create PR.

## output

implementation issues:
- brd-atsr: add brd reopen command
- brd-y2dn: graceful fallback when gh CLI is not installed

design issues (lower priority, edge cases):
- brd-hv5k: rename status 'todo' to 'open' to pair with reopen
- brd-1i52: track PR URL in issue frontmatter