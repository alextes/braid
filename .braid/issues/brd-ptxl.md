---
schema_version: 9
id: brd-ptxl
title: 'design: scheduled and recurring issues'
priority: P3
status: done
type: design
deps: []
tags:
- ux
owner: null
created_at: 2026-01-27T21:49:04.882586Z
started_at: 2026-01-27T22:01:10.494725Z
completed_at: 2026-01-27T22:29:47.505026Z
---

## problem

sometimes work doesn't need to be done immediately but should be tracked for the future. currently there's no way to:
1. schedule an issue to become "ready" at a future date
2. have recurring tasks that auto-create issues on a schedule

## use cases

- **scheduled**: "update dependencies" - not urgent now, but do it next month
- **recurring daily**: "check monitoring dashboards" 
- **recurring weekly**: "review open PRs", "triage new issues"
- **recurring monthly**: "update changelog", "security audit"

## questions to explore

1. **storage**: where does schedule info live?
   - in frontmatter (`scheduled_for: 2024-02-01`, `recurs: weekly`)?
   - separate schedule file?

2. **visibility**: how do scheduled issues appear in `brd ls`?
   - hidden until scheduled date?
   - shown but greyed out with "scheduled for X"?
   - separate `brd scheduled` command?

3. **recurrence mechanics**:
   - when does the new issue get created? on `brd ls`? background daemon? manual `brd cron`?
   - what happens to completed recurring issues? archive? delete?
   - how to stop a recurring issue?

4. **implementation options**:
   - simple: just a `scheduled_for` date field, issue hidden until then
   - medium: add `recurs` field, `brd cron` command to process
   - complex: full cron-like scheduling with daemon

## suggested approach

start simple:
1. add `scheduled_for: date` frontmatter field
2. `brd ready` excludes issues where `scheduled_for > today`
3. `brd ls --scheduled` to see upcoming scheduled issues
4. defer recurrence to a follow-up issue