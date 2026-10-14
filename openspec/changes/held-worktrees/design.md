# Design

## Context

See proposal.md for why. The blueprint is
`agentplot/blueprints:design/sluice/proposals/session-runs-1.md`,
clause 97. What `session-runs` built:

- A session's place is a record `<common>/sluice/.places/<flat>.json`
  with `kind: "session"`, the worktree's canonical path, its branch, its
  ports and its block, written by `Places::adopt` on every `sluice
  exec`. `Checkout::release_session(name, policy)` ends its runs, runs
  the release command once, removes `.tmp/<flat>`, frees the block and
  removes the record; the worktree is never touched.
- `sluice release` reads the policy at the branch's current revision
  and passes it; with the branch unreadable it passes none and the
  command the last run carried runs.
- Releases of candidates' places are logged as `Event::Released{place}`
  in the branch's log by `perform_releases` and `recover`.
- Rule core.16 forbids a round to release a session's place.

## Goals / Non-Goals

**Goals:**

- A place whose worktree is gone is released by the core, on the host,
  at points every host passes through anyway, with no daemon and no
  timer.
- The release is the same as `sluice release`'s: one path through
  `release_session`, the policy read the same way.
- A place whose worktree exists is never touched; listing it is a pure
  read.

**Non-Goals:**

- A lapse after a time without runs (ruled out by 97).
- Places on other hosts: `.places/` is the repository's on this host.
- Telling whether an existing directory is still a git worktree.

## Decisions

### D1. The place seam lists session places and says which are gone

`Checkout::session_places() -> Vec<SessionPlace{name, worktree,
branch, ports, last_run, gone}>`. The places profile reads every
`.places/*.json` of kind `session`, names it `session/<path>` as
`adopt` does, and sets `gone` when the path is no directory. Existence
is the profile's to judge (I/O), the decision the core's (rule core.2).

`adopt(worktree, branch, checkout, at)` takes the run's start from the
core's clock and keeps it as the record's `last_run` (serde default
none, so older records read). The core owns the time so the fakes and
the CLI tests see the clock they script.

Alternative rejected: `last_run` from the host's `sessions/` files,
which are dropped when a run ends, and are the host's, not the
repository's.

### D2. The sweep is the core's, before a round's lock and before an exec counts

`Sluice::release_abandoned() -> Vec<Abandoned>`: for each gone place,
try its branch's lock (`State::lock`, never `wait_lock`); busy, skip it
for the next round or run. Read the policy with `release_policy(branch)`
(56; the same read `sluice release` now uses), fall back to the last
run's command when the branch cannot be read, call `release_session`,
and log `Released{place, worktree}` in the branch's log through the
versioned update (62). A failed release is returned with its reason and
logged as a warning; the sweep goes on.

It runs from `Sluice::round(branch)`, which `step` wraps, before the
round takes its own branch's lock, so a waiting submit, `sluice step`
and `sluice run` all sweep (core.4: a round behaves the same whoever
runs it); `sluice run` also sweeps once a pass, whether or not any
branch has a queue. `sluice exec` sweeps before it counts its run.
`sluice status` never does.

Why before the lock: the sweep takes the lock of each place's branch,
which may be the round's own; taking it inside the round's lock would
deadlock against itself, and waiting for another branch's lock inside
one would deadlock two rounds. Non-blocking takes keep core.14 (one
actor per branch) with no wait.

Rule core.16 changes to say the core releases a session's place when
its working copy is gone, because it is then no session's (97, 25);
ADR 0020 records it.

Alternatives rejected: a sweep in the CLI only (a round run by any
other caller would not do it, against core.4); a sweep in `sluice
status` (a reader that writes).

### D3. A release command whose worktree is gone runs in the scratch directory

`Runs::release` runs the command with `cwd` the place; for a removed
worktree that directory is gone and the spawn fails. It falls back to
the request's `SLUICE_TMP`, which `let_go` removes only after the
release, else the release's own directory. The environment, ports and
executor are unchanged.

### D4. Reporting

`Round{step, released}` carries what a round released; `tell_abandoned`
prints one line for each, "released <worktree>: the worktree no longer
exists; its runs ended, the release command run, its ports free", with
the reason when the last run's command ran, or "could not release …"
with the error. `exec --json` and `submit --json` print it on stderr.

### D5. `sluice status` with no argument

`Status{target: Option<String>}`. With none, it prints "worktrees held
until released:", one line per place (worktree, branch, `last run`,
ports, and "(removed: released by the next run or round)" when gone),
and how to release one; `--json` prints `{"worktrees": [{worktree,
branch, ports, last_run, removed}]}`, the ports in the field names
`sluice exec --json` uses.

## Risks / Trade-offs

- [Two sweeps race on one place] → both may run the release command;
  ADR 0016 already requires it be safe to repeat, and `let_go` and
  `free_block` tolerate a second pass.
- [A broken place is retried every round] → reported each time, never
  fatal; `sluice release` by hand or fixing the release command ends
  it.
- [A release logged on a branch with no records gives it records] →
  `sluice run` then steps an empty queue, which does nothing.
- [A path recreated as a directory after removal] → counts as the
  worktree: held until released, as a worktree is.
