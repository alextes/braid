---
name: brd-design
description: work on a BRD design issue — explore the code, think through approaches, propose a design, get human approval, and produce implementation issues
argument-hint: "[issue-id]"
disable-model-invocation: true
---

# brd-design — propose a design and produce implementation issues

you are working through a design issue end-to-end: understand the problem, explore the code, think through approaches, propose a design, get human approval, and create implementation issues. prior research via `/brd-research` is optional — most design issues start here directly.

**this is a collaborative process.** design is not "build the whole plan in isolation, then present it." as you explore and think through approaches, ask the human about interesting trade-offs, surprising constraints, or forks in the road. use AskUserQuestion (or your harness's equivalent) whenever you hit a decision point where human input would save you from building out a branching tree of possibilities. the final approval step is mandatory, but don't wait until the end to start the conversation.

## 1. pre-flight

```bash
brd show $ARGUMENTS --context
```

- confirm this is a `type: design` issue. if not, stop and tell the human.
- if not already claimed, claim it:

```bash
brd start $ARGUMENTS
```

## 2. understand the problem

**read everything relevant:**
- the issue body and any existing `## research findings` section (from prior `/brd-research`)
- all dependency and dependent issues (already shown by `--context`)
- if prior research was done, incorporate those findings rather than re-doing the exploration

**explore the code:**
- use Glob to find files in the affected area
- use Grep to trace how related functionality works
- use Read to understand the key modules, types, and interfaces
- note patterns the codebase already uses — your design should be consistent with them

**understand constraints:**
- what depends on this design issue? those dependents set the scope of what your design must enable.
- are there backwards compatibility concerns?
- are there performance constraints?
- anything else that shapes the solution space

if you notice something surprising or find an interesting constraint, surface it to the human early — don't stockpile questions for later.

## 3. think through approaches

identify at least two viable designs. useful lenses to consider (not all will apply):
- **complexity** — how much code, how many files, how many new concepts?
- **maintainability** — will this be easy to understand and modify later?
- **performance** — does it matter here? if so, how do the options compare?
- **backwards compatibility** — does this break existing behavior?
- **consistency** — does this follow patterns already used in the codebase?
- whatever else is relevant to this particular problem

if the trade-offs between approaches are interesting or non-obvious, this is a great moment to ask the human before committing to a recommendation.

get the issue file path and write a `## design` section in the issue body:

```bash
brd path $ARGUMENTS
```

the design section should include:
- **options considered** — brief description of each approach with trade-offs
- **recommendation** — which approach and why
- **scope** — what the implementation will touch (files, modules, interfaces)
- **planned implementation issues** — numbered list of the issues you intend to create

## 4. get approval before creating issues

if you've been collaborating throughout (asking questions, discussing trade-offs), this step may feel like a natural conclusion. if you've been heads-down exploring, this is where you bring the human in.

either way, before creating implementation issues, make sure the human has signed off. summarize your design:

- state your recommendation and the reasoning
- explain the key trade-off
- list the implementation issues you plan to create
- ask if this works or if they'd like to adjust anything

then **stop and wait**. do not create implementation issues until the human approves. they may:
- approve as-is
- request changes
- ask clarifying questions
- suggest a different approach
- ask you to do `/brd-research $ARGUMENTS` first for deeper exploration

## 5. when to skip discussion

you may skip waiting for approval only if:
- the design is trivially obvious (e.g. "which error message to use")
- the human explicitly said "just pick one" or "go ahead"
- it's a minor decision with easily reversible consequences

when in doubt, discuss first.

## 6. create implementation issues

after the human approves, create issues using `brd add`. each issue should depend on the design issue:

```bash
brd add "<title>" --dep $ARGUMENTS [other flags]
```

use the [implementation-issue-checklist.md](implementation-issue-checklist.md) to verify each issue is well-formed. key qualities:

- **small scope** — one clear task per issue, completable in a single session
- **independence** — issues can be worked on in parallel where possible
- **clear acceptance** — use `--ac` to add acceptance criteria when the "done" state isn't obvious
- **correct dependencies** — use `--dep` for the design issue and for ordering between impl issues when one must come before another
- **appropriate priority** — inherit from the design issue unless there's reason to differ

if implementation issues have ordering constraints (e.g. "add the type" before "use the type"), express that with `--dep` between them.

## 7. close the design issue

after all implementation issues are created:

```bash
brd done $ARGUMENTS --result <impl-1> --result <impl-2> ...
```

the `--result` flag is **required** for design issues. it does two things:
- verifies the result issues exist
- **propagates dependencies transitively** — any issue that depended on this design issue now also depends on all the result issues

this is how braid tracks "design issue X must be done before Y" through to the actual implementation work.

## 8. anti-patterns

- **don't close before approval.** the human must sign off on the design.
- **don't create implementation issues before approval.** the design might change.
- **don't create one giant issue.** split the work into small, focused pieces.
- **don't forget `--result`.** without it, dependency propagation doesn't happen.
- **don't start writing code.** this is a design skill, not an implementation skill. code comes after the impl issues are created and picked up.
- **don't skip the issue body.** write the design into the issue so it's preserved. chat messages disappear; the issue persists.
