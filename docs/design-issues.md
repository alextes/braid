# design issue workflow

design issues (`type: design`) are for planning and decision-making before implementation. they require human collaboration and should not be closed autonomously.

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

once a design is approved, create concrete next steps:

- **implementation issues** — break down into actionable tasks
- **or update existing issues** — if the design affects them
- **or document the decision** — if no code changes needed

example:
```bash
brd add "implement crates.io publishing" -p P2 --dep <design-issue-id>
brd add "set up cargo-dist for releases" -p P2 --dep <design-issue-id>
```

### 6. close the design issue

only after:
- human has approved the design
- output has been created (issues, plan, or documentation)

```bash
brd done <design-issue-id>
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
