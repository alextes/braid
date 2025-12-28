---
schema_version: 4
id: brd-1k3w
title: design workflow preference signaling for agents
priority: P2
status: todo
type: design
deps: []
owner: null
created_at: 2025-12-28T23:37:12.230185Z
updated_at: 2025-12-28T23:37:12.230185Z
---

braid supports two workflows:
1. direct ship: `brd start` → work → `brd agent ship` (ff-merge to main)
2. PR-based: `brd agent branch` → work → `brd agent pr` (human review)

both work well, but agents don't know which the directing human prefers.

## questions
- how should a human signal their preference?
- should it be repo-wide config, per-issue, or per-agent?
- should agents infer from context (e.g., CI presence)?

## options to explore
1. config option in `.braid/config.toml` (e.g., `workflow = "pr"`)
2. AGENTS.md instruction (already possible, but not machine-readable)
3. issue-level flag (some issues need review, others don't)
4. agent-level config (trusted agents ship direct, new agents use PR)

## outcome
decide on approach and create implementation issues.