---
schema_version: 4
id: brd-qo6j
title: add integration tests for CLI commands
priority: P1
status: done
deps: []
owner: null
created_at: 2025-12-26T19:15:13.156396Z
updated_at: 2025-12-26T20:23:27.190438Z
acceptance:
- create tests/ directory with integration test infrastructure
- 'test full workflow: init → add → ls → start → done'
- test dependency management (dep add, blocked/ready states)
- test JSON output mode
- test error cases (missing issues, ambiguous IDs)
---

the project has zero integration tests. all existing tests are unit tests for pure logic modules.

for a CLI tool, integration tests are crucial because they test the actual user experience and catch wiring issues between components.

suggested approach:
- use a temp directory with a git repo for each test
- either use assert_cmd crate or std::process::Command
- create helper functions for common setup/teardown