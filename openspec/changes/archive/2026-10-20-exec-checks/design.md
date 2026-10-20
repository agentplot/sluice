# Design

## Context

A session run (session-runs-1 clauses 89 to 97) counts against the
host's bound from a record in the host's own state,
`$XDG_STATE_HOME/sluice/sessions/<worktree>.json`, live while its
supervisor holds its run directory. The record holds the worktree, the
place, the branch, the starting process, the start and the run
directory. The bound reads only how many there are (`Tests::sessions`).
A round's `capacity` and `sluice exec`'s `session_capacity` subtract
them, with the combinations under test, from `Tests::bound`. Nothing
writes a session run's tasks while it goes, and the overview never reads
the host's state, so a person sees the queue held to fewer runs than its
window with nothing to say why.

The overview lists the branches with records in name order:
`LocalFiles::branches` sorts them, and the policy loses its own order when
it is parsed, since its `[branches]` tables are read into a `BTreeMap`.

herdr runs a tab-bar command as one string through `/bin/sh -lc`, with
the focused pane's directory as its working directory and none of the
plugin's own variables. The plugin keeps a copy of its segment in its
state directory, which outlives a reinstall, and setup prints that copy's
absolute path. swancloud renders one herdr `config.toml` for every host,
so it leaves the entry out: the path is under `/Users/chuck` on a Mac and
`/home/chuck` on a box.

## Goals / Non-Goals

**Goals:**

- A check's tasks, its command and the host's room in
  `sluice/overview@1`, so a reader can tell agents' checks from the
  queue's combinations and see when they hold the queue back.
- A finished check that says what it is worth when its head is
  submitted.
- Branches in the policy's order, from the overview to both plugins.
- A tab-bar command that is the same text on every host.

**Non-Goals:**

- Changing the bound, the reserve or which runs start first (91).
- A remote session run's tasks (96).
- Sending a host's session runs to other hosts.

## Decisions

### D1. `sluice exec` writes its own tasks, into its host record

`sluice exec` already polls its run (`Tests::progress`) to copy its
output. On each pass it also asks `Tests::tasks(&run)`, the same moon
progress a combination's round reads (ADR 0028), and when the tasks
differ from what it last wrote, it rewrites its record in the host's
state with them, under the host's lock and by a rename. The record also
gains the command, for a run of a command. The record is the run's own,
written only by the process that started it, and goes when the run ends
as it does today. A run of a command has no tasks: the command runner
gives none.

If `sluice exec` dies while its run's supervisor goes on, the tasks stay
as last written until the supervisor ends and the record stops counting.

**Alternatives considered:**

- *The overview asks the check seam for each running session's tasks.*
  ADR 0028 turned that down for combinations: the overview reads what a
  writer kept, and never a run's directory.
- *A round writes them, as it does a combination's.* A round holds a
  branch's lock and writes the branch's progress; a session run is no
  branch's (C.3), and a round may not run at all while an agent checks.

### D2. Read from the host's state through the check seam

The overview reads session runs through `Tests::session_runs`, a new
method of the check seam beside `Tests::sessions`, which already reads
the same records for the bound. It returns each live run's worktree,
branch, start, command and tasks; `sessions` stays its count.

This departs from contract item 12's "through the records seam": the
host's session runs are kept "in the host's own state beside its port
blocks … and nothing of them reaches the records" (session-runs-1 C.3),
and rule core.18 rules out a session run's count in a branch's records.
Rule core.11 is about the records a reader on any host may need; a
session run is a fact of one host, read there alone, as a run's tasks are
(ADR 0028). ADR 0035 records it.

The overview attaches a run to the `sessions[]` entry of the same
canonical worktree. A run in another repository on the host has no
entry here, and counts only in `room`.

### D3. `room` is the arithmetic `session_capacity` does

`Sluice::room(reserved) -> Room {bound, queue, sessions, reserve}`:
`bound` from `Tests::bound`, `queue` the candidates under test on every
branch of the repository, `sessions` from `Tests::sessions`, `reserve`
the host's `[runs] reserved`. `session_capacity` becomes `bound - reserve
- queue - sessions`, computed on `room`, so what the overview shows and
what `sluice exec` decides cannot disagree. `sessions` counts every run
the host's session store holds, a preview's command included, since it
takes a run of the bound as a session run does (previews-1).

### D4. One definition of held back

The queue is held back by checks when the host has no free run (`queue
+ sessions >= bound`), at least one session run holds one, and some
branch has a submission queued, in no candidate, while fewer of its
candidates are under test than its window; a submission whose candidate
passed waits on no run. Only then do the counts line, the band, the status
line and the tab bar add `· 1 slot to an agent's checks`, or `· N slots
to agents' checks`. The CLI and both plugins implement this one rule,
each tested on the same cases.

### D5. The finished check's payoff, told truly

`last_result` (ADR 0027) gains:

- `tasks[]`: one entry per task of the finished run, from its report,
  `passed`, `failed` or `replayed`, as item 11 shapes a task.
- `reused`: the run was recorded under its keys (92), every check
  passed, and the branch's gate trusts the executor the run reports
  (`Rules::trusts_run(kind, confined)`, as `verdict.rs` reads a ran
  result). `finish_session` sets it from the policy the session's plan
  read at the branch's current revision (56, rule core.15).

So the plugins say `14 passed · reused when submitted` only when the
gate will take those passes for that head (93), and `14 passed · not
kept: uncommitted changes` when the worktree held more than its head.
Neither field is read by a round, a gate or a replay (rule core.18).

### D6. Checks as load, never as entries

The board's pane and herdr's popup show one CHECKING section after the
branches, since each row names the branch the check is held to: the
worktree's name, `→ <branch>`, the count and bar of item 11, and the
time since it started; a run of a command shows its command instead of
the count. A finished check stays for 15 minutes after it ended, read
from `last_result`, as recent events do. Under it, the host line: `host:
4 of 6 runs · 3 queue · 1 agent`. herdr's sidebar shows `checking 3/8` on
the agent's own pane, after a live submission and an unanswered blame or
conflict and before the ahead forecast and a landing.

### D7. The overview orders branches by the default branch's policy

The default branch is the repository's HEAD: the bare repository's, or
the main checkout's branch. The pull-request driver already reads it;
that read moves to the profiles beside the other git reads, and the
overview gets it through `Host::default_branch`. The policy is read at
that branch's records' base when it has records, as the overview reads
every branch, never fetching (ADR 0026), and otherwise at its local ref.
With neither, the branches stay in name order.

A branch's place is the index of the `[branches]` table its gate comes
from (`rules_for`: its exact name, else the closest pattern), so a
pattern's matches stand together where the pattern stands, in name order
among themselves; branches no table names come last, in name order. The
policy keeps its tables in file order by reading their keys as
`toml::Spanned<String>` and sorting by position.

Rule core.15 still holds: every branch's gate, limits and rules are read
from its own revision. The order is the only thing read from the default
branch, and no round, gate, landing or submission reads the order. ADR
0036 records it.

### D8. The tab-bar command names `$HOME`

Setup prints the command as `sh "$HOME/<the state directory below the
home directory>/segment/bin/segment.sh"`, or as the absolute path when
the state directory is not under `$HOME`. herdr's `/bin/sh -lc` expands
it on each host, so where herdr keeps its state at the same place below
the home directory, which is its default
(`~/.local/state/herdr/plugins/agentplot.sluice`), the line is the same on
a Mac and a Linux box. The copy already finds its state from where it
lies.

**Alternatives considered:**

- *A command on PATH, installed by the plugin's package.* herdr's install
  from GitHub puts nothing on PATH, so the README's snippet would hold
  for packaged installs alone, and the wrapper would still need the
  state directory.
- *Asking herdr for the plugin's directory on each run.* `herdr plugin
  list --json` with jq every 5 s, and herdr has no command for the state
  directory.

## Risks / Trade-offs

- **A stale task list** when `sluice exec` dies before its supervisor:
  shown until the record stops counting, never read by anything that
  decides.
- **A host with `XDG_STATE_HOME` set** on some hosts and not on others
  gets a different tab-bar line on each; setup prints the line that holds
  on the host it runs on.
- **`reused` is told when the run ends.** A policy changed later is not
  reflected until the next run.

## swancloud

Its own change, after a sluice release holds this one:

- In `lib/herdr-plugins.nix`, give the sluice entry `settings.ui.tab_bar_right
  = [ { type = "command"; command = ''sh
  "$HOME/.local/state/herdr/plugins/agentplot.sluice/segment/bin/segment.sh"'';
  interval_seconds = 5; timeout_seconds = 2; } ];`.
- Replace the comment that keeps the segment out ("Its tab-bar segment
  stays out …") with why the `$HOME` form is the same on every host.
- `render`'s check for store paths passes it, and `herdr config check`
  at build checks it.
- Move `packages/sluice`'s tag to the release.
