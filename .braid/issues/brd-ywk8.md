---
schema_version: 8
id: brd-ywk8
title: set up npm release with cargo-dist
priority: P3
status: open
type: design
deps: []
owner: null
created_at: 2025-12-27T13:58:15.293157Z
---

cargo-dist supports npm publishing for wider distribution.

## tasks
- configure cargo-dist for npm installer
- set up NPM_TOKEN secret in github repo
- test npm install workflow

## notes
- requires human to create npm account/token and add to repo secrets
- see: https://opensource.axo.dev/cargo-dist/book/installers/npm.html