---
schema_version: 4
id: brd-qfiu
title: add README section comparing braid with beads
priority: P2
status: done
type: design
deps: []
owner: null
created_at: 2025-12-27T22:13:47.855608Z
updated_at: 2025-12-28T18:35:30.418591Z
---

add a section to the README that compares braid with beads, the project that inspired it. this should help users understand the relationship between the two projects and what makes braid different.

## options considered
1. short paragraph near the top (after "takes inspiration from beads") with a few key differences
   - pros: minimal surface area, keeps readme compact
   - cons: easy to miss, harder to scan
2. dedicated "braid vs beads" section with a small comparison table
   - pros: scannable, balanced, easy to update
   - cons: adds visible weight to the readme
3. faq-style bullets under "why braid?" describing when to pick each tool
   - pros: clear decision framing
   - cons: more opinionated, longer text

## recommendation
option 2: add a "braid vs beads" section after "why braid?" with a short table and a one-line "choose braid if / choose beads if" note. keep it factual and friendly.

## draft copy (proposal)
### braid vs beads

| | braid | beads |
| --- | --- | --- |
| storage | markdown issues in `.braid/issues/` | jsonl issues in `.beads/` |
| workflow | repo-local, git pull/push | distributed with auto-sync + daemon |
| ids | human-readable prefix + random suffix | hash-based ids |
| extras | minimal cli, no background services | sqlite cache, compaction, hierarchy |

choose braid if you want a small, repo-local tracker that feels like git-native docs. choose beads if you want a distributed, long-horizon memory system with background sync and compaction.

## open questions
- ok to include the "choose braid / choose beads" line, or keep it purely factual?
- placement: after "why braid?" or earlier near the top?

## decision
- keep it factual (no "choose braid / choose beads" line)
- place after "why braid?"

## follow-up
- brd-usm3
