---
schema_version: 8
id: brd-xjub
title: 'design: explore issue relationships and connections'
priority: P2
status: open
type: design
deps: []
tags:
- cli
- tui
owner: null
created_at: 2026-01-27T22:00:35.661732Z
---

## context

issues have explicit dependencies (deps/dependents) but understanding the broader relationship graph is hard. how do related issues connect? what's the big picture?

## approaches to explore

### 1. explicit "related" links
- add `related: [brd-xxx, brd-yyy]` field to issues
- bidirectional? or one-way?
- different from deps (no blocking semantics)
- pros: explicit, simple
- cons: manual maintenance burden

### 2. deps-based exploration
- `brd graph` or `brd tree` to visualize dep graph
- show transitive deps/dependents
- find orphan issues, cycles, clusters
- pros: uses existing data
- cons: only shows blocking relationships

### 3. semantic/embedding graph
- generate embeddings for issue content
- find similar issues automatically
- "issues like this one" feature
- cluster visualization
- pros: discovers hidden connections, zero manual work
- cons: complexity, needs embedding model, overkill?

### 4. tag-based grouping
- enhance tag system for relationship discovery
- `brd ls --tag foo` already works
- could add tag hierarchy or tag co-occurrence analysis
- pros: builds on existing feature
- cons: requires good tagging discipline

## questions

- what problem are we actually solving?
- how often do users need to explore relationships?
- is the issue graph small enough that a simple tree view suffices?
- is embedding-based discovery worth the complexity?