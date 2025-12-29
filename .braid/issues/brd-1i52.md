---
schema_version: 4
id: brd-1i52
title: 'design: track PR URL in issue frontmatter'
priority: P3
status: todo
type: design
deps: []
owner: null
created_at: 2025-12-29T23:15:06.307669Z
updated_at: 2025-12-29T23:15:06.307669Z
---

when `brd agent pr` creates a PR, store the URL in the issue frontmatter so agents and humans can easily find it.

## edge cases to consider

- **reopened issues**: if an issue is reopened after PR merge, the old PR URL is stale. clear it? keep history?
- **git-native mode flow**:
  1. claim issue, commit to main
  2. create feature branch
  3. work, mark done, commit (on feature branch)
  4. create PR - now need another commit to main just to store PR URL
  5. this is awkward - the PR URL commit would be separate from the issue work
- **multiple PRs**: what if work spans multiple PRs? array of URLs?

these edge cases may make this not worth the complexity.
