# Design

## Context

See proposal.md for why. The clauses are session-runs-1 89 to 97
(`agentplot/blueprints:design/sluice/proposals/session-runs-1.md`); 97
lists each session place with the time, head and outcome of its last
run, and 92's "records nothing" governs the records under keys alone.
What exists:

- `sluice exec` (`crates/sluice/src/exec.rs`) adopts the worktree
  through `Checkout::adopt`, which writes the session place's record
  under `<common>/sluice/.places/<flat>.json` with `last_run`, the time
  the run starts (97). It ends one of four ways: finished, lost (the
  supervisor stopped before it reported), stopped (the caller ended
  it), or refused before it started.
- `Sluice::finish_session` (`sluice-core/src/session.rs`) returns
  `Recorded::Written(n)` for a clean run of the checks, `NotClean` for a
  worktree holding more than its head, `Nothing` for a command or a run
  that hit its time limit; only `Written` writes `results.jsonl` lines,
  each with `worktree` (92).
- `Checkout::session_places` lists `SessionPlace{name, worktree, branch,
  ports, last_run, gone}`; `sessions_json` and `worktree_json` in
  `crates/sluice/src/lib.rs` render it for `status --json` and
  `overview --json`.
- A run's directory `.runs/<place>/<n>/` holds `request.json`, whose
  `meta` names the runner, revision and targets, and `result.json`, the
  exit; a moon run's task outcomes are read from moon's report inside
  the worktree.

## Goals / Non-Goals

**Goals:**

- A consumer can tell, from one read of the overview, whether the last
  session run in a worktree was a recorded pass of its checks on a
  given revision, and can never mistake a command's run or an unclean
  run for one (rule core.18).
- The result is a fact about the host, kept and read through the place
  seam beside the ports and `last_run` (rules core.2, core.11).
- `--watch` redraws when a run ends, since the document changes.

**Non-Goals:**

- Reading each worktree's head or files on every overview. A `--watch`
  reads about once a second; a `git status` per held worktree each time
  is the cost this change does not take. The consumer has the worktree
  open and compares `head` itself.
- A history of runs: only the last is kept.
- Anything a round, a gate, a submission or a replay reads.

## Decisions

### The result is kept on the session place's record, not derived

Deriving from what is recorded was the first choice and does not hold:

- `record_session` skips a line `same_as` one already in the log, and
  `same_as` leaves out `at` and `worktree`. A worktree that fails a
  check, passes it, then fails it again on the same head writes the
  second fail as nothing, so the newest line for the key is the pass.
  "Latest line per task" would call that worktree ready.
- A rerun that matches every line writes nothing, so the log cannot say
  a run happened at all.
- A command's run and an unclean run write nothing to the log (92).
  Their run directory keeps `result.json`, but `request.json` cannot
  tell a command given to `sluice exec --` from a policy whose runner is
  `command`, nor says the worktree was unclean, and a moon run's task
  outcomes are re-read from moon's report in the worktree, which the
  next moon run there overwrites.

Only the core knows, as the run ends, its kind, whether the worktree
was clean and what `Recorded` it returned. So the core hands that to
the place seam once, and the place profile keeps it on the record that
already holds `last_run`. The record is removed with the place's
release (95), so the result goes with it; a gone worktree's place is
released by the next run or round (97), and its result with it.

Alternative: the host's session-run state (`$XDG_STATE_HOME/sluice/
sessions/`), which `end_session` deletes. It is the `Host` trait's, read
by the CLI alone; overview reads session places through the core and
the place seam, so keeping it there would give `sessions[]` a second
source. Rejected.

Alternative: change `record_session`'s dedupe to compare against the
newest line for the key. That changes what the results log holds and
what replays read (37, 93), for a display. Rejected here.

### The shape

```json
"last_result": {
  "at": "2026-10-16T09:12:40Z",
  "head": "<full revision>",
  "kind": "checks",
  "outcome": "pass",
  "recorded": true
}
```

- `at`: when the run ended. `last_run` stays the time it started (97).
  A run's start drops the result of the one before, so a run under way,
  or one whose starter was killed, has no `last_result`.
- `head`: the worktree's head the run was given, full.
- `kind`: `checks` or `command`.
- `outcome`: `pass` when every check the run named reported and passed
  (the predicate `exec` already exits 0 on), or a command exited 0;
  `fail` otherwise; `timed_out`; `lost`; `stopped`. `lost` and
  `stopped` have no report.
- `recorded`: true exactly when `finish_session` returned `Written`,
  including `Written(0)`, every result already under its key. The core
  reads the worktree again through the driver as the run ends; a head
  or files changed since it started record nothing under keys
  (`Recorded::Changed`; 92, rule core.18), and the result names the
  head the run started on.

The name is `last_result`, not `last`, beside `last_run`: `result` is
the user word for a check's outcome, and `last` alone would read as a
second spelling of `last_run`. Absent when the place has had no run end
since this change, as `last_run` is absent before a first run.

### The overview reads no worktree; "ready" is the consumer's

The overview reads neither a worktree's head nor whether it holds more
than its head: it lists what the last run left, and nothing it would
have to read from each worktree on every `--watch` poll. "Ready" is the
consumer's rule: a recorded pass on the worktree's current head, that is
`kind == "checks"`, `outcome == "pass"`, `recorded`, and `head` equal to
the head the consumer reads from the worktree it has open. A worktree
whose files have changed since still has that head's recorded pass,
which is what a submission of the head would replay (93); a consumer
may mark such a worktree separately.

### Written after the results, and on every end

`finish_session` writes the result after `record_session` and the cache
save, so a reader that sees `recorded: true` finds the lines (rule
core.7's order). `exec` calls `Sluice::session_ended` on the lost and
stopped paths, which have no report. A run refused before it started
leaves the result as it was: nothing ran. A run that starts drops the
result of the one before as `adopt` writes its `last_run`, so a starter
killed outright leaves no result rather than one that is not the last
run's.

### The seam

`Checkout::ran(name, &LastResult)`: keep `LastResult` as the session
place `name`'s last result, replacing any; nothing for a place already
released; an error for a candidate's or a preview's place. The place
profile reads and writes the record under the repository's lock over
session places' records, beside them in its git directory, and removes
a released session place's record under the same lock, so a release
between the read and the write never sees the record written again.
The host's lock for blocks of ports is not used: a sandboxed check that
releases a place may not write the host's state. `SessionPlace.last_result: Option<LastResult>`.
One interface, implemented by the place profile and the fakes (rule
core.2); the conformance suite covers it.

## Risks / Trade-offs

- [Two runs in one worktree at once write the record in turn] → the
  later write wins, and the record is written whole by rename, as
  `adopt` writes it; `last_result` is the last run to end, which is what
  it says.
- [A consumer reads `outcome: pass` alone] → `recorded` and `kind` sit
  beside it, the reference page says what "ready" needs, and the spec
  pins that a command's run and an unclean run never say `recorded:
  true`.
- [The record grows a field older binaries do not know] → the record is
  read with serde defaults; an older binary drops `last_result` on its
  next write, and the next run writes it again.

## Migration Plan

None: an existing record has no `last_result` until its next run ends.
