# Design

## Context

See proposal.md for why. The blueprint is
`agentplot/blueprints:design/sluice/proposals/session-runs-1.md`
(clauses 89 to 95 here; 96, remote, is a later change). What exists:

- A place is made by `profiles::places::Places` (`Checkout::materialize`),
  which writes a record under `<common>/sluice/.places/<flat>.json`, holds
  a port slot under `$XDG_STATE_HOME/sluice/slots` (ADR 0016), adds a
  detached worktree, and on `Checkout::release` ends the place's runs,
  runs `[tests] release` through `runs::Runs`, removes the worktree,
  frees the slot and removes the record.
- A run is started by `profiles::checks::Checks` (`Tests::start`) from a
  `RunRequest{place, revision, required, since, limit, limits, tests,
  replay}`, supervised by `sluice supervise <dir>` under
  `<common>/sluice/.runs/<name>/`, held to the executor `choose(&Limits)`
  picks (open or sandbox) and recorded with `Enforced`.
- The bound is `Instance{bound}` from `~/.config/sluice/config.toml`
  `[runs] max`, counted by `Sluice::capacity` in `core/step.rs` from the
  candidates under test in the records (ADR 0012, core.14).
- Check-run records are `ResultLine{kind, task, hash, outcome, host,
  executor, branch, combination}` in `<common>/sluice/results.jsonl`,
  written by `Sluice::poll` before any verdict (core.7) and read by
  `verdict::read` for replays (69).
- `Driver::session(worktree)` (worktrunk profile) gives head, branch,
  `dirty` (porcelain with untracked files) and `detached`.
- `wt-submit` runs `sluice submit` and, on exit 0, `wt remove`.

## Goals / Non-Goals

**Goals:**

- One contract for a run wherever its place came from: the same
  `RunRequest`, executor, limits, environment and `Enforced` record.
- The count stays in the records and the core counts it (core.14);
  nothing new is held in a process's memory.
- An agent's worktree is never written, moved or removed by sluice.

**Non-Goals:**

- `--remote` (96): waits on the remote submit profile and C.8.
- Counting or capping what a worktree keeps running between runs.
- Timing out an abandoned worktree's ports and stack.
- A container executor for session runs (not built for candidates
  either).

## Decisions

### D1. A session place is a second kind of place record, adopted, never made

`Places` gains `adopt(worktree, branch, settings) -> Place`, writing
`.places/<flat>.json` with `Made{kind: Session, path: <worktree>, branch,
slot, ...}` (a `kind` field, `Made` default for existing records), holding
a slot through the existing `hold_slot` with the record path as holder,
and making `<common>/sluice/.tmp/<flat>` once; the record and slot stay
until release. `Checkout` gains `adopt`, and `release(name, passed)` on a
session record ends its runs, runs the release command, removes tmp,
frees the slot and removes the record, and never runs `git worktree
remove`. The flat name is `session%2F<flat path>`, so a session place and
a candidate's place never share a name.

Why: the slot, the env (`SLUICE_PLACE`, `SLUICE_PORT_*`) and the release
already hang off the record; a second profile would duplicate them and
core.3 would want an ADR for each. Alternative rejected: a separate
`Sessions` profile with its own slot directory, which could collide with
candidates' ports.

### D2. A session run is a `RunRequest` on the adopted place, named per run

`sluice exec` builds `RunRequest{place: adopted, revision: head,
required: gate.required, since: Some(branch rev), limit: rules.timeout,
limits: policy.executor, tests, replay: false}` and calls
`Tests::start`. Run directories are `.runs/<session flat>/<n>`, a fresh
`n` per run, because `Runs::clear` on a reused name runs the release
command (94 forbids that between runs). `Runs::remove` for a session
place iterates every `<n>` and runs the release command once, from a
request built at release time from the place's branch policy.

`sluice exec -- <command>` uses the `command` runner path of `Checks`
with the given argv and `required` empty, and marks the request
`record: false`.

Why: every limit, env and log path already exists on this route; the
executor's `Enforced` comes free. Alternative rejected: a plain
`Command::spawn` in the CLI under the toolchain, which would bypass the
sandbox and record nothing of what was enforced (84).

### D3. The count: session runs are records the core reads

`State` gains `sessions() -> Vec<SessionRun{worktree, place, branch, pid,
started, run_dir}>`, `begin_session(SessionRun) -> Result<()>` and
`end_session(worktree)`, implemented by `local_files` as
`<common>/sluice/sessions/<flat>.json` under the repository `.lock`. A
reader drops a record whose `pid` is gone (`kill(pid, 0)` fails) before
counting. `Sluice::capacity` subtracts live sessions as it subtracts
under-test candidates, under `Serves::Every` and `Serves::Stepped`
alike, so a waiting `wt submit` running its own rounds sees them too.

Taking a slot for a session run: `exec` locks the target branch
(`State::lock`, as a round does, core.14) and the repository `.lock`,
reads capacity less the reserve, and if there is room writes the session
record and starts the run before releasing either lock; otherwise it
releases both, prints "waiting for a slot", pauses (`Pause`, 3 s) and
tries again. Ctrl-C while waiting exits 130 with nothing recorded.

Why: ADR 0012 put the count in the records and rejected a lock-file
semaphore; this keeps the count where the core reads it. Known race: a
round on branch A and an exec on branch B hold different branch locks
and may both see the last slot; the host over-runs by one at most. Stated
under Risks.

### D4. The reserve is the host's, in `Instance`

`[runs] reserved = N` (default 0) joins `[runs] max` in
`instance::Instance`; `reserved >= bound` is refused at read with the
setting named. Session capacity is `bound - reserved - under - sessions`;
a round's capacity is `bound - under - sessions`. A session run never
starts into the reserve; a round starts into any free slot. This is the
whole of 91's "started first": a reserve, not a queue between starters.

### D5. Recording a clean session run

`exec` records the run's `Ran` results only when `Driver::session` says
`dirty == false` and not detached-ambiguous (a detached head is allowed,
the head is still exact). The write reuses the `ResultLine` builder from
`Sluice::poll`, moved to a function `records::result_lines(report, host,
branch, combination)` both call, with `combination = head` and a new
optional field `worktree`. `Replayed` results are never written (core.9).
A dirty worktree runs with `replay: false` as well and writes nothing;
the output says "not recorded: worktree is not clean". A command run
writes nothing.

`Enforced.confined` is `false` for every session run; the existing
`choose` never sets it today, and `verdict::read` with the default trust
(open, sandbox) reads the record as a pass for the same key. A gate that
trusts only confined runs ignores it (29).

### D6. `sluice release` and the worktrunk driver

`Command::Release{--worktree "."}` finds the session record by the
worktree's canonical path, calls `Checkout::release(name, false)` and
`State::end_session`, and exits 0 with "nothing to release" when no
record exists. `Command::Exec{--release}` runs release after the run
ends. `wt-submit` runs `sluice release --worktree <wt>` before `wt
remove`, and not under `--no-remove`, `--no-wait` or `--check`.

Why: 25 and core.16 forbid a round from touching a session's place;
release stays a command the session or its driver issues.

### D7. Vocabulary at the surface

CLI and site say worktree, run, ports, release, bound and reserve; code
and specs say session run, session place, slot. Error and status text
never says "line".

## Risks / Trade-offs

- [Two starters see the last slot] → at most one run over the bound;
  stated in the ADR; a later change may take the repository lock in the
  round's start path too.
- [A killed `exec` leaves a session record] → readers drop records whose
  pid is gone before counting; the run directory's supervisor ends the
  group as today.
- [An abandoned worktree holds a slot and a stack] → left open by the
  blueprint; `sluice release` by hand; documented on the site.
- [Session results from a worktree on another disk or with other
  toolchain state than a candidate's] → the key is moon's content hash,
  so a differing input is a differing key; the script runner keys by
  revision and the clean-worktree rule guards it.
- [Forcing runs (`--force`) in an iterating agent's worktree costs
  time] → moon's dependency cache still applies to deps; the agent can
  use `sluice exec -- moon run …` for cached iteration, which records
  nothing.

## Migration Plan

No records change shape; new files appear under `<common>/sluice/
sessions/` and `.places/session%2F…`. An older sluice ignores both. No
rollback needed beyond removing them.

## Open Questions

- Whether `sluice exec` should accept `--cached` to let moon replay and
  skip recording. Deferrable: adds a flag, changes no spec.

## Notes from the build

- Task 1.2, D3: session runs are recorded under
  `<common>/sluice/.sessions/<flat>.json`, not `sessions/`. A branch's
  records live at `<common>/sluice/<branch>/`, so a branch named
  `sessions` would share the directory; `LocalFiles::dir` refuses only
  dot-named components, as `.places`, `.runs` and `.logs` rely on.
- Task 1.2, D3: the `State` trait gains a fourth method beside
  `sessions`, `begin_session` and `end_session`: `lock_sessions`, the
  repository `.lock` as a guard. `exec` must hold it across reading the
  count and writing its run, and a second `flock` of the same file from
  one process waits on itself, so `begin_session`, `end_session`,
  `record_results` and `allocate_id` take the lock only when this
  process's guard does not hold it.
- Task 1.4, D4: `Sluice::session_capacity(reserved)` takes the reserve as
  an argument (the reserve is the host's, read by the CLI) and counts the
  candidates under test on every branch whatever the `Serves`, because a
  session run is the host's and no instance serves it.
