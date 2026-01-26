# TUI specification

living spec for `brd tui` - the terminal user interface for braid.

## overview

a full-screen terminal interface for browsing and managing issues. optimized for keyboard navigation with vim-style bindings.

## layout

```
┌─ header ──────────────────────────────────────────────────┐
│ brd tui — agent: <agent_id>                               │
├─ main area ───────────────────────────────────────────────┤
│                                                           │
│  (view content: dashboard or issues)                      │
│                                                           │
├─ footer ──────────────────────────────────────────────────┤
│ [message] │ [keybinding hints]                            │
└───────────────────────────────────────────────────────────┘
```

## views

### dashboard (key: `1`)

statistics overview with visual bars:
- **status**: open/doing/done/skip counts with stacked bar
- **priority**: P0-P3 distribution for active issues
- **health**: ready/blocked counts, stale indicator (doing >24h)
- **active agents**: list of agents with their current issues
- **recent activity**: completed/started in last 24h

### issues (key: `2`, default)

two-pane layout (toggleable):

```
┌─ Issues (N) ──────────────┬─ Detail ──────────────────────┐
│ status ready id  pri age  │ ID:       brd-xxxx            │
│ →◉ brd-xxx P1 2h owner... │ Title:    ...                 │
│   brd-yyy P2 1d -    ...  │ Priority: P1                  │
│ ✓  brd-zzz P3 3d -    ... │ Status:   doing               │
│                           │ Owner:    agent-name          │
│                           │                               │
│                           │ Dependencies:                 │
│                           │   > brd-dep1 (resolved)       │
│                           │   - brd-dep2 (open)           │
└───────────────────────────┴───────────────────────────────┘
```

**list columns:**
- status icon: ` ` open, `→` doing, `✓` done, `⊘` skip
- ready indicator: `◉` if ready (no blockers), ` ` otherwise
- issue id
- priority (P0-P3)
- age since creation
- owner (truncated)
- title + tags

**detail pane shows:**
- issue metadata (id, title, priority, status, owner, tags)
- state (READY/BLOCKED)
- dependencies with resolution status
- dependents (reverse deps)
- acceptance criteria
- description body

**details pane toggle:**
- `Tab` hides/shows the details pane
- when hidden, list takes full width
- `Enter` opens full-screen detail overlay when pane is hidden

## modes

### normal mode

default state. all navigation and action keys work.

### input modes

modal dialogs that capture keyboard input:

- **title input**: entering new issue title
- **priority selection**: choosing P0-P3
- **type selection**: choosing (none)/design/meta
- **deps selection**: multi-select existing issues as dependencies
- **filter input**: typing search query (live filtering)

### overlay modes

full-screen overlays on top of current view:

- **help overlay** (`?`): keybinding reference
- **detail overlay** (`Enter` when details pane hidden): full-screen issue details

## keybindings

### navigation
| key | action |
|-----|--------|
| `↑` / `k` | move selection up |
| `↓` / `j` | move selection down |
| `g` | go to top |
| `G` | go to bottom |
| `←` / `h` | select previous dependency (in detail pane) |
| `→` / `l` | select next dependency (in detail pane) |

### actions
| key | action |
|-----|--------|
| `a` / `n` | add new issue |
| `e` | edit selected issue in $EDITOR |
| `s` | start selected issue (claim it) |
| `d` | mark selected issue as done |
| `Enter` | open selected dependency / show detail overlay |
| `r` | refresh issues from disk |

### views
| key | action |
|-----|--------|
| `1` | switch to dashboard |
| `2` | switch to issues |
| `Tab` | toggle details pane visibility |

### filter
| key | action |
|-----|--------|
| `/` | enter filter mode |
| `R` | toggle ready-only filter |
| `Enter` | confirm filter (in filter mode) |
| `Esc` | clear filter |

### other
| key | action |
|-----|--------|
| `?` | toggle help overlay |
| `q` | quit |
| `Ctrl+C` | quit |

## styling

- **selected row**: yellow background, black text, bold
- **done/skip issues**: dark gray text
- **doing issues**: colored by age (green <1h, yellow <24h, red >24h)
- **design type**: italic
- **meta type**: bold
- **ready indicator**: green `◉`
- **blocked deps**: red text
- **resolved deps**: green text

## state persistence

no state persists between sessions. on startup:
- view defaults to issues
- selection starts at top
- no filters active
- details pane shown

## future considerations

(items to potentially add to spec as we implement them)

- scrolling in detail pane for long descriptions
- keyboard shortcuts for status filters (show only open, only doing, etc.)
- search across issue body, not just title
- bulk operations (multi-select)
- issue preview on hover/delay
