# design issue workflow

design issues (`type: design`) are for planning and decision-making before implementation. they require human collaboration, should not be closed autonomously, and must spawn follow-up issues before closing.

## workflow

### 1. claim the issue

```bash
brd start <design-issue-id>
```

### 2. research and analyze

- explore the codebase to understand constraints
- identify possible approaches
- consider trade-offs (complexity, maintainability, performance, etc.)

### 3. write up findings

update the issue body with:

- **options considered** — list each approach with pros/cons
- **recommendation** — your suggested approach and why
- **open questions** — anything you're unsure about

### 4. discuss with human

**do not close the issue yet.** present your analysis and wait for feedback:

- the human may approve your recommendation
- they may ask clarifying questions
- they may suggest a different approach
- they may want to defer the decision

### 5. produce output

once a design is approved, create concrete next steps. follow-up issues are required:

- **implementation issues** — create one or more new issues with actionable tasks (preferred)
- **documentation issues** — create a new issue if the decision only needs docs/notes
- if the design affects existing issues, update them after creating the follow-up issues

example:
```bash
brd add "implement crates.io publishing" -p P2 --dep <design-issue-id>
brd add "set up cargo-dist for releases" -p P2 --dep <design-issue-id>
```

### 6. close the design issue

only after:
- human has approved the design
- follow-up issues have been created

```bash
brd done <design-issue-id> --result <impl-issue-1> --result <impl-issue-2>
```

the `--result` flag is **required** for design issues. it:
- verifies the result issues exist
- automatically propagates dependencies (issues depending on this design now also depend on the results)

use `--force` to close without specifying results (not recommended).

## transitive dependencies

if issue X depends on a design issue D, closing D with `--result A, B` makes X depend on A and B. this allows you to declare dependencies on design issues before knowing what implementation work will result.

example:
```bash
# v1.0 depends on a design issue
brd dep add brd-v1 brd-design

# later, close the design with its results
brd done brd-design --result brd-impl-1 --result brd-impl-2

# now brd-v1 also depends on brd-impl-1 and brd-impl-2
```

## when to skip discussion

you may close a design issue autonomously only if:

- the design is trivially obvious (e.g., "which error message to use")
- the human explicitly said "just pick one"
- it's a minor decision with easily reversible consequences

when in doubt, discuss first.

## example

**bad:** agent claims design issue → writes recommendation → immediately closes

**good:** agent claims design issue → writes recommendation → asks human "does this approach work for you?" → human approves → agent creates implementation issues → agent closes design issue
