---
schema_version: 4
id: brd-48y6
title: add unit tests for TUI state management
priority: P1
status: done
type: design
deps: []
tags:
- testing
owner: null
created_at: 2025-12-28T08:48:43.326404Z
updated_at: 2025-12-28T16:55:05.866369Z
---

the TUI has 7 input modes and complex state transitions with zero test coverage.

## InputMode states
- Normal
- Title (new issue)
- Priority (new issue)
- EditSelect
- EditTitle
- EditPriority
- EditStatus

## test cases needed
- state transitions between modes
- selection clamping on reload
- pane switching
- input handling in each mode
- message display/clearing

---

## design analysis

### current structure

**app.rs** - `App` struct with:
- state fields: `issues`, `ready_issues`, `all_issues`, `ready_selected`, `all_selected`, `active_pane`, `input_mode`, `message`
- pure methods: `move_up`, `move_down`, `switch_pane`, `toggle_help`, `start_add_issue`, `cancel_add_issue`, `confirm_title`, `start_edit_issue`, `cancel_edit`, `confirm_edit_field`, `selected_issue_id`, `selected_issue`
- I/O methods: `new`, `reload_issues`, `start_selected`, `done_selected`, `create_issue`, `save_edit`

**event.rs** - input handling that:
- updates `input_mode` state (pure)
- calls I/O methods with `paths` parameter

### testability assessment

**easily testable now** (pure state):
- all navigation: `move_up`, `move_down`, `switch_pane`
- mode transitions: `start_add_issue` → `cancel_add_issue`, `confirm_title`
- edit flow: `start_edit_issue` → `confirm_edit_field` → mode-specific state
- selection logic: `selected_issue_id` with various pane/selection states

**needs refactoring** (I/O dependent):
- `App::new` requires `RepoPaths` → filesystem
- `reload_issues` reads filesystem
- `start_selected`, `done_selected`, `create_issue`, `save_edit` write filesystem

### recommended approach

**add test-friendly constructor:**

```rust
impl App {
    /// create app with pre-loaded state (for testing)
    #[cfg(test)]
    pub fn with_state(
        issues: HashMap<String, Issue>,
        config: Config,
        agent_id: String,
    ) -> Self {
        let mut app = Self {
            issues: HashMap::new(),
            ready_issues: Vec::new(),
            all_issues: Vec::new(),
            ready_selected: 0,
            all_selected: 0,
            active_pane: ActivePane::Ready,
            agent_id,
            message: None,
            show_help: false,
            input_mode: InputMode::Normal,
            config,
        };
        app.issues = issues;
        app.rebuild_lists();  // new helper to sort issues into ready/all
        app
    }

    fn rebuild_lists(&mut self) {
        // extract list-building logic from reload_issues
    }
}
```

**test categories:**

1. **state transition tests** - use `App::with_state`, test pure methods
2. **selection clamping tests** - inject issues, call `rebuild_lists`, verify clamping
3. **input mode tests** - verify mode transitions without I/O
4. **integration tests** - use temp dirs for I/O methods (like existing CLI tests)

### implementation tasks

1. extract `rebuild_lists()` helper from `reload_issues()`
2. add `#[cfg(test)] App::with_state()` constructor
3. add unit tests in `src/tui/app.rs`:
   - `test_move_up_down`
   - `test_switch_pane`
   - `test_selection_clamping`
   - `test_add_issue_flow`
   - `test_edit_issue_flow`
   - `test_cancel_returns_to_normal`
4. optionally: add integration tests for I/O methods