---
schema_version: 4
id: brd-ztwj
title: design PR-based multi-agent workflow
priority: P2
status: doing
type: design
deps: []
owner: agent-two
created_at: 2025-12-27T22:04:41.412418Z
updated_at: 2025-12-28T19:56:00.663136Z
---

current workflow: agents work in worktrees and ship directly to main via `brd agent ship` (rebase + fast-forward push).

proposed alternative: PR-based workflow where each agent:
1. works in its own worktree
2. creates feature branches for each issue
3. opens PRs for review/CI
4. PRs get merged (by human or automation)

## questions to resolve

- how does an agent know when to create a new branch vs continue on current?
- branch naming convention? (e.g., `agent-one/brd-xyz`, `feat/brd-xyz`)
- who merges PRs — human review, auto-merge after CI, or agent self-merge?
- how to handle conflicts when multiple agents have open PRs?
- should agents be able to review each other's PRs?
- how does issue state sync work with branches instead of direct main commits?

## potential commands

- `brd agent pr` — create PR for current work
- `brd agent branch <issue-id>` — create/switch to branch for issue
- config option to choose workflow style (direct vs PR-based)

## trade-offs

direct workflow:
- simpler, faster iteration
- requires trust in agents
- conflicts resolved at ship time

PR workflow:
- better audit trail
- CI gates before merge
- more overhead per change
- familiar github-flow model