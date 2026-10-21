# Design

## Context

The herdr plugin's watcher lists herdr's panes (`lib/panes.sh`), finds
each one's worktree and repository, and sets the `sluice` token on each
pane from its repository's overview documents (`pane_token` in
`lib/words.jq`), choosing the first that holds: a live submission from
the pane's worktree, an unanswered blame or conflict of its own, a check
under way in it, commits ahead (from `bin/forecast.sh`), a landing in the
last 30 minutes. herdr 0.9.5's `pane list` gives each pane's `tokens`, one
map of name to value merged across every source that reports to the pane;
`panes_json` drops it today. The popup (`bin/status.sh`) knows the pane it
was opened over from `HERDR_PLUGIN_CONTEXT_JSON`'s `focused_pane_id`.

The board runs inside the Claude Code process. `$.env.get(name)` reads
that process's environment, the one the session was started with, and
`claude plugin validate` lists each name a module reads. The status line
names this session's submissions (`isMine`: its `from.path` is the
session's worktree, or the session's own submit printed its number or
pull request).

## Goals / Non-Goals

**Goals:**

- A pane or session an orchestrator names a branch for shows that
  branch's state, the same words in both plugins.
- The named branch first in every detail view.

**Non-Goals:**

- Any change to `sluice overview` or another document.
- A hint naming a repository, a host or more than one branch.
- Any word of a particular orchestrator in the plugins.

## Decisions

### D1. Two sources, one name each

herdr: the pane token `sluice_branch`, read from `pane list`'s `tokens`
for every pane, whichever source set it. Claude Code: `SLUICE_BRANCH` in
the session's environment, read once when the session starts. An empty
value is no hint. A hint applies within the pane's or session's own
repository: a pane in no git repository still gets no token. herdr merges
every source's tokens into one map, so the name carries the plugin's
prefix, as `sluice` and `sluice_q` do.

### D2. Own submissions first, then the hint

The pane's or session's own submissions still win: a live submission, or
an unanswered blame or conflict of its own. Otherwise the named branch's
line replaces the rest: a check under way in the worktree, commits ahead,
a recent landing. A worktree shared by several agents is why the hint is
set, and there those three describe the shared worktree, not the agent.

### D3. The words

`release · 3 queued · #12 testing 3/8 · 2 checking`:

- `3 queued`: the branch's submissions in no combination under test.
- `#12 testing 3/8`: the oldest combination under test, by its
  submissions, with how many of its tasks have ended, or its minutes when
  its runner lists no tasks; `+1 more` when others are under test too.
  No bar: the line is a sidebar token, at most 80 characters.
- `2 checking`: agents' checks held to that branch (`sessions[]` whose
  `branch` it is, with a `run`), the work of the agents the orchestrator
  coordinates for it.

A part at zero is left out. With all three at zero the line is `release ·
nothing queued`. A branch no document of the repository lists is
`release · no records`. The board's status line is the same words after
`sluice: `, then `· 1 slot to an agent's checks` as it says today.

### D4. Detail views put the named branch first

The herdr popup and dashboard put the focused pane's named branch first,
and the board's pane and band the session's, then the rest in the
overview's order (branch-order). The detail views show it even when idle
and leave it out of the count of branches with nothing queued; with no
records it is one line, `release · no records`. The band keeps showing
only busy branches. The tab bar is the host's and is unchanged.

### D5. A named pane gets no forecast

The forecast runs per worktree. It runs for a worktree only when one of
its panes has no named branch, since a named pane's row never shows
`ahead`, so no `sluice submit --check` runs for a line nobody sees.

### D6. `SLUICE_BRANCH` is shared with previews

A preview's command already gets `SLUICE_BRANCH`, the branch the preview
was asked for (previews-1). A session started from it reads that branch,
which is the branch its work is for: the same meaning, so no second name.

### D7. herdr's token outlives no restart

herdr keeps a token reported with no `--ttl-ms` until it is replaced,
cleared or the pane closes, and restores no token after its server
restarts. The guides say an orchestrator reports it again when it
restarts or resumes an agent. The watcher reads tokens at each tick
(15 seconds), so a new hint shows within one.

## Risks / Trade-offs

- [Two sources of one token name] → herdr keeps the latest; the guide
  says one orchestrator owns `sluice_branch` on a pane.
- [A hint hides a worktree's own `ahead`] → set it only where the
  worktree doesn't say; the guide says so.
