---
name: brd-research
description: research phase for a BRD design issue — explores the codebase and documents findings before proposing solutions
argument-hint: "[issue-id]"
disable-model-invocation: true
---

# brd-research — explore and document findings

you are doing **research only**. your job is to explore the problem space, document what you find, and present findings to the human. you do not propose a final design, write code, or close the issue.

this is a collaborative process. if you run into interesting findings, surprising constraints, or forks in the road as you explore, surface them to the human early — use AskUserQuestion (or your harness's equivalent) rather than stockpiling everything for the end.

## 1. pre-flight

```bash
brd show $ARGUMENTS --context
```

- confirm this is a `type: design` issue. if not, stop and tell the human.
- if not already claimed, claim it:

```bash
brd start $ARGUMENTS
```

## 2. exploration strategy

work through these steps in order. skip any that aren't relevant.

**understand the problem:**
- read the issue body carefully — what's being asked and why?
- read all dependency and dependent issues (`brd show $ARGUMENTS --context` already gave you these)
- identify the key constraints and success criteria

**map the relevant code:**
- use Glob to find files in the affected area
- use Grep to trace how existing functionality works
- use Read to understand the key modules, types, and interfaces
- note any relevant patterns the codebase already uses

**check history:**
- look at recent git log for the affected files — is there prior work, reverted attempts, or relevant context in commit messages?

**identify approaches:**
- come up with at least two viable approaches
- for each, note: what changes, what's easy, what's hard, what could go wrong

**external research (when relevant):**
- if the issue involves libraries, protocols, or patterns you're unsure about, use WebSearch/WebFetch
- look for prior art, known pitfalls, or best practices
- this step is optional — skip it for purely internal design questions

## 3. document findings

get the issue file path:

```bash
brd path $ARGUMENTS
```

edit the issue body to add a `## research findings` section. use the structure from [findings-template.md](findings-template.md):

- **context** — what you explored and key observations
- **options considered** — at least two approaches with pros, cons, effort, and risk
- **open questions** — things you couldn't determine or that need human input
- **recommendation** — which option you lean toward and why (this is a suggestion, not a decision)

write findings directly into the issue file so they're preserved and visible to anyone who runs `brd show`.

## 4. present to human

after documenting findings, summarize them in your response:

- state what the issue is asking for (one sentence)
- list the options you found (brief)
- highlight the key trade-off or open question
- state your recommendation

then **stop and wait for feedback**. the human may:
- ask clarifying questions
- point you toward something you missed
- approve moving to design phase
- change direction entirely

## 5. anti-patterns

- **don't write code.** not even "rough sketches." research is read-only exploration.
- **don't close the issue.** research is not completion.
- **don't create implementation issues.** that's the design phase's job.
- **don't skip documenting in the issue body.** your chat messages disappear; the issue body persists.
- **don't propose a final design.** present options and a recommendation, but frame it as input for the design phase.
- **don't boil the ocean.** focus on what's relevant to this specific issue. if you find tangential problems, note them briefly but don't deep-dive.

## 6. next step

after the human reviews your findings and is ready to move forward:

```
/brd-design $ARGUMENTS
```
