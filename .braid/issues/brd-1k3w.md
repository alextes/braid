---
schema_version: 8
id: brd-1k3w
title: design workflow preference signaling for agents
priority: P2
status: done
type: design
deps: []
owner: null
created_at: 2025-12-28T23:37:12.230185Z
started_at: 2025-12-29T21:07:54.854094Z
completed_at: 2025-12-29T21:07:54.854094Z
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

## analysis

### option 1: config.toml (recommended as default)
```toml
workflow = "direct"  # or "pr"
```
- simple, machine-readable, repo-wide
- most repos have one consistent preference
- backwards compatible: default to "direct" (current behavior)

### option 2: AGENTS.md (not recommended)
could add `<!-- braid:workflow:pr -->` marker, but:
- parsing markdown comments is brittle
- config is the right place for config
- AGENTS.md should describe *how* to use the workflow, not *which* one

### option 3: issue-level flag (recommended as override)
```yaml
---
workflow: pr
---
```
- fine-grained when needed (security fixes need review, typos don't)
- optional - uses repo default if not specified

### option 4: agent trust level (not recommended for v1)
- complex to implement and reason about
- trust is subjective and changes over time
- could layer this on later if needed

## recommendation

**layered approach:**

1. **repo default** in `config.toml`:
   - `workflow = "direct" | "pr"`
   - default: "direct" (backwards compatible)

2. **per-issue override** in frontmatter:
   - `workflow: direct | pr` (optional)
   - overrides repo default when set

### how agents use it

**AGENTS.md block** (workflow-agnostic):
- documents *both* workflows and their commands
- does not suggest a preference by default
- users can switch between workflows freely without editing AGENTS.md
- explains how to use brd commands to make either workflow smooth

**config + issue frontmatter** (preference signaling):
- `workflow = "direct" | "pr"` in config.toml
- `workflow: direct | pr` in issue frontmatter (optional override)
- machine-readable, used by commands for contextual hints

**command adaptations** (separate design issue):
- commands become workflow-aware and give contextual warnings
- e.g. `brd done` warns if on main but config says "pr"
- e.g. `brd agent ship` warns if config says "pr"
- helps users stay on the expected workflow path

## decision

proceed with layered approach:
1. add `workflow` field to config (default: "direct")
2. add optional `workflow` field to issue frontmatter
3. update AGENTS.md block to document both workflows
4. create separate design issue for command adaptations

## implementation issues

- brd-w3ao: add workflow config field and schema v5 migration
- brd-c7t7: add workflow field to issue frontmatter
- brd-fmiv: update AGENTS.md block to document both workflows
- brd-fzk1: design workflow-aware command hints and warnings