---
schema_version: 4
id: brd-8vcu
title: add branch switching helpers for writing to issues branch
priority: P2
status: skip
deps:
- brd-vwel
owner: null
created_at: 2025-12-28T23:58:33.893601Z
updated_at: 2025-12-30T15:54:10.514561Z
---

Add infrastructure for switching to issues branch, making changes, and switching back.

## Changes

**src/repo.rs:**
```rust
/// Execute a closure while on the issues branch
pub fn with_issues_branch<F, T>(&self, config: &Config, f: F) -> Result<T>
where F: FnOnce() -> Result<T>
{
    // 1. Get current branch
    // 2. Stash if dirty (git stash push -m "brd: temp stash")
    // 3. Checkout issues branch
    // 4. Run closure
    // 5. Checkout original branch
    // 6. Pop stash if we stashed
}
```

This provides a clean abstraction for all write operations.

## Acceptance criteria
- [ ] Helper correctly stashes and restores dirty state
- [ ] Helper switches branches and returns
- [ ] Errors during closure still restore original branch