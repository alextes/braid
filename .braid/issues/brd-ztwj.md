---
schema_version: 8
id: brd-ztwj
title: design PR-based multi-agent workflow
priority: P2
status: done
type: design
deps: []
owner: null
created_at: 2025-12-27T22:04:41.412418Z
started_at: 2025-12-28T19:59:59.717975Z
completed_at: 2025-12-28T19:59:59.717975Z
---

PR-based workflow as an alternative to direct `brd agent ship` for teams wanting CI gates and human review.

## overview

current workflows:
1. **direct mode**: `brd agent ship` → rebase + fast-forward push to main
2. **sync branch mode**: issues on separate branch, code to main

PR-based adds a third option:
- agents work in feature branches
- create PRs for human review
- CI runs before merge
- familiar GitHub Flow model

## design decisions

### 1. branch creation: explicit command

new command: `brd agent branch <issue-id>`
- creates branch `<agent-name>/<issue-id>` from main
- `brd start` does NOT auto-create branch (separation of concerns)
- agent must explicitly create branch before working

rationale: keeps `brd start` focused on claiming issues, branch creation is a separate workflow concern.

### 2. issue state sync: mode-dependent

**without sync branch (simple mode):**
- `brd start` modifies issue file in the feature branch
- issue state (doing/done) merges with the PR
- simple, but humans responsible for avoiding conflicts if multiple agents claim same issue

**with sync branch (coordinated mode):**
- `brd start` pushes claim to sync branch immediately
- other agents see the claim before PR merges
- code goes to feature branch → PR to main
- after PR merges, run `brd done` + `brd sync`

### 3. PR merge strategy: human review required

- agent creates PR via `brd agent pr`
- human must approve and merge
- no auto-merge (keeps human in the loop)
- after PR merges, human/agent runs `brd done` manually

### 4. new commands

```
brd agent branch <id>     # create feature branch <agent>/<id> from main
brd agent pr              # create PR from current branch to main
```

`brd agent pr` behavior:
- uses `gh pr create` under the hood
- auto-generates title: "feat: <issue-title> (<issue-id>)"
- body includes issue description
- targets main branch

## workflows

### simple PR workflow (no sync branch)

```bash
brd agent branch brd-xyz   # create branch agent-one/brd-xyz
brd start brd-xyz          # claim issue (committed in branch)
# ... do work, commit as usual ...
brd done brd-xyz           # mark done (committed in branch)
brd agent pr               # create PR
# human reviews and merges PR
# issue state merges with code
```

### PR workflow with sync branch

```bash
brd agent branch brd-xyz   # create branch
brd start brd-xyz          # claim pushed to sync branch (visible to all)
# ... do work, commit as usual ...
brd agent pr               # create PR for code only
# human reviews and merges PR
brd done brd-xyz           # mark done in sync branch
brd sync                   # push done status to remote
```

## trade-offs

| aspect | direct workflow | PR workflow |
|--------|-----------------|-------------|
| speed | fast iteration | slower (review gate) |
| audit | commit history only | PR history + reviews |
| trust | requires trust in agents | human verification |
| conflicts | resolved at ship time | resolved before merge |
| CI | optional | gates merge |

## implementation issues

after this design is approved, create implementation issues for:
- [ ] `brd agent branch` command
- [ ] `brd agent pr` command
- [ ] documentation updates