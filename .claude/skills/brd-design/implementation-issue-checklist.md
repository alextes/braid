# implementation issue checklist

use this checklist when creating implementation issues from a design issue. each issue should pass all applicable items.

## quality checklist

- [ ] **small scope** — one clear task, completable in a single session. if you're writing more than a paragraph to describe it, it's probably too big.
- [ ] **clear title** — imperative, specific, lowercase. good: "add --json flag to brd show". bad: "improvements to show command".
- [ ] **independence** — can be worked on without waiting for other impl issues (unless explicitly ordered with `--dep`).
- [ ] **acceptance criteria** — if the "done" state isn't obvious from the title, add `--ac` criteria. e.g. `--ac "brd show --json outputs valid JSON for all issue types"`.
- [ ] **correct dependencies** — depends on the design issue (`--dep <design-id>`). if it must come after another impl issue, add that dep too.
- [ ] **appropriate priority** — usually inherits from the design issue. only change if there's a reason (e.g. a foundational piece is higher priority).
- [ ] **prefer what over how** — be helpful where you can, but don't over-specify. the implementer is smart too.

## common splitting patterns

**feature by feature:**
- implement feature A end-to-end
- implement feature B end-to-end
- (good when features are independent)

**incremental delivery:**
- implement the minimal version
- add enhancement X
- add enhancement Y
- (good when there's a useful subset that works on its own)

## dependency ordering

when impl issues have a natural order, express it with `--dep`:

```bash
brd add "add config file support" --dep design-123
brd add "migrate CLI flags to use config" --dep design-123 --dep <config-issue>
```

this ensures `brd ready` surfaces issues in the right order. issues without ordering constraints can be worked in parallel.

## example

for a design issue "design: add JSON output to all commands":

```bash
brd add "add --json flag to brd ls" --dep design-42 -p P2 --ac "brd ls --json outputs valid JSON array"
brd add "add --json flag to brd show" --dep design-42 -p P2 --ac "brd show --json outputs valid JSON object"
brd add "add --json flag to brd ready" --dep design-42 -p P2
```

each issue implements the feature end-to-end for one command (shared helpers emerge naturally). then close the design issue:

```bash
brd done design-42 --result <ls-issue> --result <show-issue> --result <ready-issue>
```
