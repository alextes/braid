---
schema_version: 8
id: brd-qas3
title: 'design: interact with running claude agent'
priority: P2
status: done
type: design
deps: []
tags:
- agent
owner: null
created_at: 2026-01-18T14:18:21.998249Z
started_at: 2026-01-27T21:24:16.474681Z
completed_at: 2026-01-27T21:33:27.071946Z
---

Design how users can interact with a claude agent that's working on an issue.

## current state

already implemented:
- `brd agent logs --follow` - view real-time output from running agent
- `brd agent send` - send message to **stopped** agent (uses --resume)
- `brd agent attach` - attach interactively to **stopped** agent

gap: can't send input to a **running** agent without killing it first.

## approaches

### approach 1: stream-json stdin pipe (recommended)

claude code supports `--input-format stream-json` for "realtime streaming input".

spawn agent with:
```
claude -p --verbose \
  --input-format stream-json \
  --output-format stream-json \
  < stdin_pipe > log_file
```

keep stdin pipe open, write JSON messages when user wants to interact.

**pros:**
- built into claude code
- structured input format
- can send at any time

**cons:**
- need to keep pipe handle alive for session lifetime
- need to figure out JSON message format for user input

### approach 2: named pipe (FIFO)

create a named pipe for stdin:
```
mkfifo .braid/sessions/agent-xxx.stdin
```

spawn with stdin from pipe. `brd agent send` writes to pipe.

**pros:**
- simple unix semantics
- works with existing spawn code

**cons:**
- agent blocks if pipe empty and nothing written
- platform-specific (no windows)

### approach 3: kill + resume (current workaround)

kill running agent, then `brd agent send` to resume with new input.

**pros:**
- already works today

**cons:**
- interrupts agent mid-task
- loses any in-flight work
- jarring UX

### approach 4: don't bother

agents rarely need mid-run input. if they do, user can:
1. watch logs with `--follow`
2. kill agent
3. use `send` or `attach` to continue

**pros:**
- no new code needed

**cons:**
- not great UX when agent asks a question

## detecting when agent needs input

look for `type: "user"` events in the stream-json output with a question/prompt.
could surface this in `brd agent ps` or logs output.

## recommendation

start with **approach 4** (status quo) since it works today via kill+send.

if we want better UX, implement **approach 1** (stream-json stdin):
1. spawn agents with stdin as a pipe we hold open
2. add `brd agent input <session> "message"` to write to running agent
3. detect "waiting for input" state in ps/logs